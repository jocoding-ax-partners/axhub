// Phase 25 PR 25.2 — Shell wrapper kill switch verification.
//
// Companion to crates/axhub-helpers/tests/hook_safety_cli.rs which covers the
// Rust helper subcommands. This file verifies the POSIX shell wrappers exit
// silently before invoking the helper binary so kill-switched environments
// never even spawn axhub-helpers.

import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { join } from "node:path";

const root = join(import.meta.dir, "..");

function runShell(script: string, env: Record<string, string>) {
  // `CLAUDE_PLUGIN_ROOT` is required by both shell wrappers; we point it at a
  // nonexistent path so any code path past the kill switch would emit a
  // systemMessage about install.sh missing. That makes "empty stdout" a
  // strong signal that the kill switch fired first.
  const finalEnv: Record<string, string> = {
    ...process.env,
    CLAUDE_PLUGIN_ROOT: "/tmp/axhub-test-nonexistent",
    ...env,
  };
  for (const key of ["AXHUB_DISABLE_HOOKS", "AXHUB_DISABLE_HOOK", "DISABLE_AXHUB"]) {
    if (!(key in env)) delete finalEnv[key];
  }
  return spawnSync("bash", [join(root, script)], {
    env: finalEnv,
    input: "",
    timeout: 5_000,
  });
}

describe("hooks/session-start.sh kill switch", () => {
  test("global AXHUB_DISABLE_HOOKS=1 short-circuits before binary check", () => {
    const out = runShell("hooks/session-start.sh", { AXHUB_DISABLE_HOOKS: "1" });
    expect(out.status).toBe(0);
    expect(out.stdout.toString()).toBe("");
    expect(out.stderr.toString()).toBe("");
  });

  test("per-hook AXHUB_DISABLE_HOOK=session-start short-circuits", () => {
    const out = runShell("hooks/session-start.sh", {
      AXHUB_DISABLE_HOOK: "session-start,other",
    });
    expect(out.status).toBe(0);
    expect(out.stdout.toString()).toBe("");
  });

  test("legacy DISABLE_AXHUB=1 short-circuits (deprecation warning surfaces from helper, not wrapper)", () => {
    const out = runShell("hooks/session-start.sh", { DISABLE_AXHUB: "1" });
    expect(out.status).toBe(0);
    expect(out.stdout.toString()).toBe("");
  });

  test("without kill switch, wrapper proceeds past kill-switch gate (emits systemMessage about missing install)", () => {
    const out = runShell("hooks/session-start.sh", {});
    expect(out.status).toBe(0);
    const stdout = out.stdout.toString();
    // CLAUDE_PLUGIN_ROOT points to a nonexistent dir → install.sh missing →
    // wrapper emits a systemMessage. We only assert that the wrapper got past
    // the kill switch (i.e. it produced output instead of silent exit).
    expect(stdout.length).toBeGreaterThan(0);
  });

  test("per-hook list naming a different hook does NOT skip session-start", () => {
    const out = runShell("hooks/session-start.sh", {
      AXHUB_DISABLE_HOOK: "preauth-check,prompt-route",
    });
    expect(out.status).toBe(0);
    // Should produce some output (wrapper kept going past kill switch).
    expect(out.stdout.toString().length).toBeGreaterThan(0);
  });
});

// sh/ps1-absorption Phase 4 (F1): hooks/token-freshness-gate.sh kill switch
// cases removed — the shim was deleted after T8 SKILL migration. Coverage
// migrated to crates/axhub-helpers/tests/token_gate_test.rs (kill switch
// AXHUB_DISABLE_HOOKS / AXHUB_DISABLE_HOOK / DISABLE_AXHUB) which exercises
// the Rust subcommand the SKILL now calls directly.
