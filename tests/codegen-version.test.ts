// Tests for AXHUB_PLUGIN_RELEASE auto-sync codegen (sh + ps1 + ts version sites).

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { syncInstallVersion } from "../scripts/codegen-install-version";
import packageJson from "../package.json" with { type: "json" };

const REPO_ROOT = join(import.meta.dir, "..");
const INSTALL_SH = join(REPO_ROOT, "bin/install.sh");
const INSTALL_PS1 = join(REPO_ROOT, "bin/install.ps1");

describe("codegen-install-version (US-602)", () => {
  test("package.json version is valid semver", () => {
    expect(packageJson.version).toMatch(/^\d+\.\d+\.\d+(?:-[a-z0-9.]+)?$/);
  });

  test("syncInstallVersion is idempotent (re-run produces no change)", () => {
    syncInstallVersion(); // first run brings to sync
    const result = syncInstallVersion(); // second run should be no-op
    expect(result.changed).toBe(false);
    expect(result.after_version).toBe(`v${packageJson.version}`);
  });

  test("bin/install.sh RELEASE_VERSION default matches package.json version", () => {
    syncInstallVersion(); // ensure sync
    const content = readFileSync(INSTALL_SH, "utf8");
    const expected = `RELEASE_VERSION="\${AXHUB_PLUGIN_RELEASE:-v${packageJson.version}}"`;
    expect(content).toContain(expected);
  });

  test("AXHUB_PLUGIN_RELEASE env override syntax preserved (codegen does not break override)", () => {
    syncInstallVersion();
    const content = readFileSync(INSTALL_SH, "utf8");
    expect(content).toMatch(/RELEASE_VERSION="\$\{AXHUB_PLUGIN_RELEASE:-v\d+\.\d+\.\d+/);
  });

  test("syncInstallVersion result reports before/after when no change", () => {
    syncInstallVersion();
    const result = syncInstallVersion();
    expect(result.before_version).toBe(`v${packageJson.version}`);
    expect(result.after_version).toBe(`v${packageJson.version}`);
    expect(result.install_sh_path).toContain("bin/install.sh");
  });

  test("bin/install.ps1 $ReleaseVersion default matches package.json version (Phase 11 US-1101)", () => {
    syncInstallVersion();
    const content = readFileSync(INSTALL_PS1, "utf8");
    const expected = `$ReleaseVersion = if ($env:AXHUB_PLUGIN_RELEASE) { $env:AXHUB_PLUGIN_RELEASE } else { 'v${packageJson.version}' }`;
    expect(content).toContain(expected);
  });

  test("PowerShell single-quote literal preserved + pre-release tag regex round-trips", () => {
    // The PS regex must accept pre-release tags like '0.1.8-rc.1' and preserve
    // the surrounding single quotes via captured groups.
    syncInstallVersion();
    const content = readFileSync(INSTALL_PS1, "utf8");
    // Both single quotes around the version literal must be present
    expect(content).toMatch(/'v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?'/);
    // Regex itself must accept pre-release pattern (compile-time check)
    const PRE_RELEASE_PATTERN = /^(\$ReleaseVersion = if \(\$env:AXHUB_PLUGIN_RELEASE\) \{ \$env:AXHUB_PLUGIN_RELEASE \} else \{ ')v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(' \})$/m;
    expect(PRE_RELEASE_PATTERN.test(content)).toBe(true);
    // Verify pre-release sample literal would match if substituted
    const sampleWithPreRelease = content.replace(
      /'v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?'/,
      "'v0.1.8-rc.1'",
    );
    expect(PRE_RELEASE_PATTERN.test(sampleWithPreRelease)).toBe(true);
  });
});
