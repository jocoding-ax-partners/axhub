// Phase 6 US-602: tests for AXHUB_PLUGIN_RELEASE auto-sync codegen.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { syncInstallVersion } from "../scripts/codegen-install-version";
import packageJson from "../package.json" with { type: "json" };

const REPO_ROOT = join(import.meta.dir, "..");
const INSTALL_SH = join(REPO_ROOT, "bin/install.sh");

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
});
