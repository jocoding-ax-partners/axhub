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

const VERSION_LINE_RE = /^(RELEASE_VERSION="\$\{AXHUB_PLUGIN_RELEASE:-)v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(\}")/m;

export interface SyncResult {
  changed: boolean;
  before_version: string | null;
  after_version: string;
  install_sh_path: string;
}

export function syncInstallVersion(): SyncResult {
  const pkgVersion = packageJson.version;
  if (!/^\d+\.\d+\.\d+(?:-[a-z0-9.]+)?$/.test(pkgVersion)) {
    throw new Error(`package.json version "${pkgVersion}" is not valid semver`);
  }
  const targetTag = `v${pkgVersion}`;

  const content = readFileSync(INSTALL_SH, "utf8");
  const match = content.match(VERSION_LINE_RE);
  if (!match) {
    throw new Error(`bin/install.sh missing expected RELEASE_VERSION line — has the file been refactored?`);
  }
  const beforeTag = match[0].match(/v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?/)?.[0] ?? null;

  if (beforeTag === targetTag) {
    return { changed: false, before_version: beforeTag, after_version: targetTag, install_sh_path: INSTALL_SH };
  }

  const updated = content.replace(VERSION_LINE_RE, `$1${targetTag}$2`);
  writeFileSync(INSTALL_SH, updated);
  return { changed: true, before_version: beforeTag, after_version: targetTag, install_sh_path: INSTALL_SH };
}

if (import.meta.main) {
  const result = syncInstallVersion();
  if (result.changed) {
    process.stdout.write(`codegen-install-version: ${result.before_version} → ${result.after_version} (bin/install.sh updated)\n`);
  } else {
    process.stdout.write(`codegen-install-version: already in sync at ${result.after_version} (no change)\n`);
  }
}
