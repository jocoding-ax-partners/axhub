#!/usr/bin/env bun
/**
 * Phase 6 US-602: AXHUB_PLUGIN_RELEASE auto-sync from package.json.
 *
 * Reads package.json `version` field and rewrites the RELEASE_VERSION default
 * in bin/install.sh so it stays in lockstep with releases. Idempotent: if the
 * default already matches package.json version, exits 0 with no diff.
 *
 * Pattern matched + replaced (preserves env override):
 *   RELEASE_VERSION="${AXHUB_PLUGIN_RELEASE:-vX.Y.Z}"
 *
 * Maintainer flow: bump package.json + .claude-plugin/{plugin,marketplace}.json
 * → run `bun run codegen:version` → commit. smoke:full chains it for drift
 * detection.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import packageJson from "../package.json" with { type: "json" };

const REPO_ROOT = join(import.meta.dir, "..");
const INSTALL_SH = join(REPO_ROOT, "bin/install.sh");
const INSTALL_PS1 = join(REPO_ROOT, "bin/install.ps1");
const INDEX_TS = join(REPO_ROOT, "src/axhub-helpers/index.ts");
const TELEMETRY_TS = join(REPO_ROOT, "src/axhub-helpers/telemetry.ts");

const VERSION_LINE_RE = /^(RELEASE_VERSION="\$\{AXHUB_PLUGIN_RELEASE:-)v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(\}")/m;
// PowerShell single-quote literal — preserves quote pair via captured groups.
const PS_VERSION_LINE_RE = /^(\$ReleaseVersion = if \(\$env:AXHUB_PLUGIN_RELEASE\) \{ \$env:AXHUB_PLUGIN_RELEASE \} else \{ ')v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(' \})$/m;
const TS_PLUGIN_VERSION_RE = /^(const PLUGIN_VERSION = ")\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(";)$/m;
const TS_HELPER_VERSION_RE = /^(const HELPER_VERSION = ")\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(";)$/m;

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
  // Extract the existing version literal (digits between the captured groups)
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

  // 1. bin/install.sh — RELEASE_VERSION default
  const installResult = syncFile(INSTALL_SH, VERSION_LINE_RE, pkgVersion, (m) => m.replace(/v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, targetTag));
  if (installResult.changed) filesUpdated.push("bin/install.sh");
  if (installResult.before !== null) beforeTag = `v${installResult.before}`;

  // 1b. bin/install.ps1 — Windows installer $ReleaseVersion default (Phase 11 US-1101)
  const ps1Result = syncFile(INSTALL_PS1, PS_VERSION_LINE_RE, pkgVersion, (m) => m.replace(/v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, targetTag));
  if (ps1Result.changed) filesUpdated.push("bin/install.ps1");

  // 2. src/axhub-helpers/index.ts — PLUGIN_VERSION constant (US-602 follow-up: Phase 6 architect 권고)
  const indexResult = syncFile(INDEX_TS, TS_PLUGIN_VERSION_RE, pkgVersion, (m) => m.replace(/\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, pkgVersion));
  if (indexResult.changed) filesUpdated.push("src/axhub-helpers/index.ts (PLUGIN_VERSION)");

  // 3. src/axhub-helpers/telemetry.ts — PLUGIN_VERSION + HELPER_VERSION constants
  const tlmContent = readFileSync(TELEMETRY_TS, "utf8");
  let tlmUpdated = tlmContent;
  let tlmChanged = false;
  if (TS_PLUGIN_VERSION_RE.test(tlmContent)) {
    const m = tlmContent.match(/PLUGIN_VERSION = "(\d+\.\d+\.\d+(?:-[a-z0-9.]+)?)"/);
    if (m && m[1] !== pkgVersion) {
      tlmUpdated = tlmUpdated.replace(TS_PLUGIN_VERSION_RE, (s) => s.replace(/\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, pkgVersion));
      tlmChanged = true;
    }
  }
  if (TS_HELPER_VERSION_RE.test(tlmUpdated)) {
    const m = tlmUpdated.match(/HELPER_VERSION = "(\d+\.\d+\.\d+(?:-[a-z0-9.]+)?)"/);
    if (m && m[1] !== pkgVersion) {
      tlmUpdated = tlmUpdated.replace(TS_HELPER_VERSION_RE, (s) => s.replace(/\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/, pkgVersion));
      tlmChanged = true;
    }
  }
  if (tlmChanged) {
    writeFileSync(TELEMETRY_TS, tlmUpdated);
    filesUpdated.push("src/axhub-helpers/telemetry.ts (PLUGIN_VERSION + HELPER_VERSION)");
  }

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
