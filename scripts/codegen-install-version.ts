#!/usr/bin/env bun
/**
 * AXHUB_PLUGIN_RELEASE auto-sync from package.json.
 *
 * Reads package.json `version` field and rewrites version constants in:
 *   - bin/install.sh         RELEASE_VERSION default
 *   - bin/install.ps1        $ReleaseVersion default
 *   - Cargo.toml             workspace.package.version (Rust env!("CARGO_PKG_VERSION"))
 *
 * TS shadow constants (src/axhub-helpers/index.ts + telemetry.ts) removed in
 * v0.2.0 TS-helper migration. Rust binary now reads CARGO_PKG_VERSION at compile
 * time — no version-source duplication.
 *
 * Idempotent: re-running with no version change is a no-op.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import packageJson from "../package.json" with { type: "json" };

const REPO_ROOT = join(import.meta.dir, "..");
const INSTALL_SH = join(REPO_ROOT, "bin/install.sh");
const INSTALL_PS1 = join(REPO_ROOT, "bin/install.ps1");
const CARGO_TOML = join(REPO_ROOT, "Cargo.toml");

const VERSION_LINE_RE = /^(RELEASE_VERSION="\$\{AXHUB_PLUGIN_RELEASE:-)v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(\}")/m;
const PS_VERSION_LINE_RE = /^(\$ReleaseVersion = if \(\$env:AXHUB_PLUGIN_RELEASE\) \{ \$env:AXHUB_PLUGIN_RELEASE \} else \{ ')v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(' \})$/m;
const CARGO_WORKSPACE_VERSION_RE = /^(version = ")\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(")$/m;

// sh/ps1-absorption Phase 3.1 (T7): disclosure marker version 도 release version 과
// sync. 이전엔 install.sh 의 `_AXHUB_DISCLOSURE_VER` 가 v0.5.13 에 하드코딩
// 되어 v0.8.0 release 시점에도 drift. codex outside voice finding #9 fix.
const DISCLOSURE_VER_SH_RE = /^(_AXHUB_DISCLOSURE_VER=")v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(")$/m;
const DISCLOSURE_VER_PS_RE = /^(\$AxhubDisclosureVer = ')v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(')$/m;

export interface SyncResult {
  changed: boolean;
  before_version: string | null;
  after_version: string;
  install_sh_path: string;
  files_updated: string[];
}

const syncFile = (path: string, regex: RegExp, targetVersionLiteral: string, replacement: (m: string) => string): { changed: boolean; before: string | null } => {
  const content = readFileSync(path, "utf8");
  const match = content.match(regex);
  if (!match) {
    throw new Error(`${path} missing expected version line — has the file been refactored?`);
  }
  const beforeMatch = match[0].match(/\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/);
  const before = beforeMatch ? beforeMatch[0] : null;
  if (before === targetVersionLiteral) return { changed: false, before };
  const updated = content.replace(regex, replacement);
  writeFileSync(path, updated);
  return { changed: true, before };
};

export function syncInstallVersion(): SyncResult {
  const pkgVersion = packageJson.version;
  if (!/^\d+\.\d+\.\d+(?:-[a-z0-9.]+)?$/.test(pkgVersion)) {
    throw new Error(`package.json version "${pkgVersion}" is not valid semver`);
  }
  const targetTag = `v${pkgVersion}`;
  const filesUpdated: string[] = [];
  let beforeTag: string | null = null;

  const installResult = syncFile(INSTALL_SH, VERSION_LINE_RE, pkgVersion, (m) => m.replace(/v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, targetTag));
  if (installResult.changed) filesUpdated.push("bin/install.sh");
  if (installResult.before !== null) beforeTag = `v${installResult.before}`;

  const ps1Result = syncFile(INSTALL_PS1, PS_VERSION_LINE_RE, pkgVersion, (m) => m.replace(/v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, targetTag));
  if (ps1Result.changed) filesUpdated.push("bin/install.ps1");

  const cargoResult = syncFile(CARGO_TOML, CARGO_WORKSPACE_VERSION_RE, pkgVersion, (m) => m.replace(/\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, pkgVersion));
  if (cargoResult.changed) filesUpdated.push("Cargo.toml (workspace.package.version)");

  // sh/ps1-absorption Phase 3.1 (T7): disclosure marker version sync.
  // codex outside voice finding #9 fix — `_AXHUB_DISCLOSURE_VER` 가 release
  // version 과 drift 누적 위험을 codegen 으로 잠금. disclosure 텍스트는 그대로
  // 두고 version 만 sync 해서 marker 비교 시 새 release 가 항상 fresh marker
  // 로 인식되도록 보장. install.sh / install.ps1 의 disclosure 텍스트 자체를
  // 사용자가 변경한 release 에서는 marker version 도 자동 bump 돼서 사용자가
  // 다시 disclosure 보게 돼요 (ADR-0012 dual-channel 의도와 일치).
  const discloseShResult = syncFile(INSTALL_SH, DISCLOSURE_VER_SH_RE, pkgVersion, (m) => m.replace(/v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, targetTag));
  if (discloseShResult.changed) filesUpdated.push("bin/install.sh (_AXHUB_DISCLOSURE_VER)");

  const disclosePsResult = syncFile(INSTALL_PS1, DISCLOSURE_VER_PS_RE, pkgVersion, (m) => m.replace(/v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, targetTag));
  if (disclosePsResult.changed) filesUpdated.push("bin/install.ps1 ($AxhubDisclosureVer)");

  return {
    changed: filesUpdated.length > 0,
    before_version: beforeTag,
    after_version: targetTag,
    install_sh_path: INSTALL_SH,
    files_updated: filesUpdated,
  };
}

if (import.meta.main) {
  const result = syncInstallVersion();
  if (result.changed) {
    process.stdout.write(`codegen-install-version: ${result.before_version} → ${result.after_version} (${result.files_updated.length} file(s) updated)\n`);
    for (const f of result.files_updated) {
      process.stdout.write(`  · ${f}\n`);
    }
  } else {
    process.stdout.write(`codegen-install-version: already in sync at ${result.after_version} (no change)\n`);
  }
}
