// Phase 27.x — Node runner stderr fallback behaviour regression test.
// ADR-0011 §Decision: 5 케이스 (A/B/C/D/E).
//
// The Node runner is extracted from codegen-preflight-injection.ts and run with
// a temporary helper stub that simulates each scenario. ${CLAUDE_PLUGIN_ROOT} is
// replaced with a temp dir so the runner resolves the stub as the helper binary.

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { getPreflightInjectionLine } from "../scripts/codegen-preflight-injection";

const REPO_ROOT = join(import.meta.dir, "..");
const TMP_ROOT = join(REPO_ROOT, "tests/.tmp-preflight-fallback");
const TMP_BIN = join(TMP_ROOT, "bin");
const HELPER = join(TMP_BIN, "axhub-helpers");

beforeAll(() => {
  mkdirSync(TMP_BIN, { recursive: true });
});

afterAll(() => {
  if (existsSync(TMP_ROOT)) rmSync(TMP_ROOT, { recursive: true });
});

/**
 * Extract the JS body from `!`node -e "SCRIPT"`` and substitute plugin root.
 *
 * The codegen output is shell-escaped — `\"` pairs in SCRIPT are meant to be
 * unescaped by the shell before reaching `node -e`. Tests invoke node directly
 * via `spawnSync('node', ['-e', script])` with no shell layer, so the unescape
 * step must happen here. Without it, Node sees a literal `\"` and fails with
 * `SyntaxError: Expected unicode escape`.
 */
function buildScript(): string {
  const injLine = getPreflightInjectionLine();
  const m = injLine.match(/^!`node -e "(.+)"`$/);
  if (!m) throw new Error(`Unexpected injection line format: ${injLine.slice(0, 60)}`);
  return m[1]
    .replace(/\\"/g, '"')
    .replace(/\$\{CLAUDE_PLUGIN_ROOT\}/g, TMP_ROOT);
}

function makeHelper(content: string): void {
  writeFileSync(HELPER, content, { mode: 0o755 });
}

function runNode(script: string): { stdout: string; stderr: string; status: number | null } {
  const result = spawnSync("node", ["-e", script], { timeout: 8_000 });
  return {
    stdout: result.stdout.toString(),
    stderr: result.stderr.toString(),
    status: result.status,
  };
}

describe("Case A — normal path: helper exits 0, no stderr", () => {
  test("exit code propagates (0), no systemMessage, no stderr", () => {
    makeHelper('#!/bin/sh\nprintf \'{"auth_status":"authenticated"}\'\nexit 0\n');
    const out = runNode(buildScript());
    expect(out.status).toBe(0);
    expect(out.stdout).not.toContain("systemMessage");
    expect(out.stderr).toBe("");
  });
});

describe("Case B — permission denial: strict-anchor regex matches", () => {
  test("exit 0 + Korean systemMessage JSON on stdout + stderr empty", () => {
    // Use the canonical denial fixture
    const denialText =
      'Shell command permission check failed for pattern "!`test`": This command requires approval';
    makeHelper(`#!/bin/sh\n>&2 printf '${denialText}\\n'\nexit 1\n`);
    const out = runNode(buildScript());
    expect(out.status).toBe(0);
    const parsed = JSON.parse(out.stdout.trim());
    expect(parsed).toHaveProperty("systemMessage");
    expect((parsed as { systemMessage: string }).systemMessage).toContain("axhub");
    expect((parsed as { systemMessage: string }).systemMessage).toContain("허용");
    expect(out.stderr).toBe("");
  });
});

describe("Case C — false-positive guard: generic 'permission' word only", () => {
  test("strict-anchor NOT matched → stderr passthrough + helper exit code", () => {
    makeHelper("#!/bin/sh\n>&2 echo 'Error: permission denied to read file'\nexit 1\n");
    const out = runNode(buildScript());
    expect(out.status).toBe(1);
    expect(out.stdout).not.toContain("systemMessage");
    expect(out.stderr).toContain("permission denied");
  });
});

describe("Case D — wording fuzz: 'permission check failed' without 'Shell command' prefix", () => {
  test("strict-anchor NOT matched → stderr passthrough + helper exit code", () => {
    makeHelper(
      "#!/bin/sh\n>&2 echo 'permission check failed for pattern: requires user approval'\nexit 1\n",
    );
    const out = runNode(buildScript());
    expect(out.status).toBe(1);
    expect(out.stdout).not.toContain("systemMessage");
    expect(out.stderr).toContain("permission check failed");
  });
});

describe("Case E — unrecognized stderr passthrough (ADR-0010 §42 정합)", () => {
  test("Rust panic backtrace passthrough + helper exit code, NOT swallowed", () => {
    const rustPanic =
      "thread 'main' panicked at 'unwrap() on Err', crates/axhub-helpers/src/main.rs:42";
    makeHelper(`#!/bin/sh\ncat >&2 << 'STDERR_EOF'\n${rustPanic}\nSTDERR_EOF\nexit 1\n`);
    const out = runNode(buildScript());
    expect(out.status).toBe(1);
    expect(out.stderr).toContain("panicked");
    expect(out.stdout).not.toContain("systemMessage");
  });

  test("deprecation warning passthrough + helper exit code, NOT swallowed", () => {
    makeHelper(
      "#!/bin/sh\n>&2 echo 'warning: axhub-helpers v0.5.8 is deprecated. Update to v0.5.9.'\nexit 0\n",
    );
    const out = runNode(buildScript());
    expect(out.status).toBe(0);
    expect(out.stderr).toContain("deprecated");
    expect(out.stdout).not.toContain("systemMessage");
  });
});
