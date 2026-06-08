// Option 3 — Bun PostToolUse deploy verifier removal.
//
// Behavioral coverage lives in crates/axhub-helpers/tests/verify_deploy_artifact_test.rs.
// This Bun-side regression locks the repository surface so the runtime hook no
// longer depends on Bun or TS-only helpers.

import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");

describe("Bun deploy verifier runtime removal", () => {
  test("old Bun hook/verifier/helper files are deleted", () => {
    for (const relPath of [
      "hooks/post-tool-verify-deploy-artifacts.ts",
      "hooks/_helpers.ts",
      "scripts/verify-user-app-artifact.ts",
    ]) {
      expect(existsSync(join(root, relPath)), relPath).toBe(false);
    }
  });

  test("package.json no longer exposes a Bun verifier script", () => {
    const packageJson = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
    expect(packageJson.scripts["verify:user-app-artifact"]).toBeUndefined();
  });

  test("hooks.json dispatches deploy verification through axhub-helpers.sh", () => {
    const hooksJson = JSON.parse(readFileSync(join(root, "hooks/hooks.json"), "utf8"));
    const bashHooks = hooksJson.hooks.PostToolUse[0].hooks.map((h: { command: string }) => h.command);
    expect(bashHooks).toContain(
      "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh verify-deploy-artifact",
    );
    expect(bashHooks.some((command: string) => /^bun\s/.test(command))).toBe(false);
  });
});
