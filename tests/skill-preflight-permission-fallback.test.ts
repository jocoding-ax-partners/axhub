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
  test("Shell-prefix variant — exit 0 + Korean systemMessage JSON on stdout + stderr empty", () => {
    // Canonical Shell-prefix denial fixture
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

  test("Bash-prefix variant — M1 review (PR #99): regex covers (?:Shell|Bash)", () => {
    // Verifies the M1 review broadening to (?:Shell|Bash) catches the Bash-prefix
    // wording that Claude Code may emit on bash-tool denial.
    const denialText =
      'Bash command permission check failed for pattern "!`test`": This command requires approval';
    makeHelper(`#!/bin/sh\n>&2 printf '${denialText}\\n'\nexit 1\n`);
    const out = runNode(buildScript());
    expect(out.status).toBe(0);
    const parsed = JSON.parse(out.stdout.trim());
    expect(parsed).toHaveProperty("systemMessage");
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

describe("Case G — secret token redaction in stderr passthrough (PR #99 security M2)", () => {
  test("sk- / gho_ / github_pat_ / axhub_ / Bearer tokens redacted before parent forward", () => {
    const tokenStderr =
      "Authorization: Bearer abc123XYZ_token.example=\n" +
      "OpenAI key sk-proj1234567890abcdefghij found in env\n" +
      "GitHub gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa exposed\n" +
      "GitHub fine-grained github_pat_11AA22BB33CC44DD55EE66_77FF88GG99HH00II11JJ22KK33LL44MM55NN exposed\n" +
      "axhub_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa internal token\n";
    makeHelper(`#!/bin/sh\ncat >&2 << 'STDERR_EOF'\n${tokenStderr}STDERR_EOF\nexit 1\n`);
    const out = runNode(buildScript());
    expect(out.status).toBe(1);
    // Verify passthrough still happened (not silenced) — non-empty stderr
    expect(out.stderr.length).toBeGreaterThan(0);
    // Token patterns must be redacted
    expect(out.stderr).not.toContain("sk-proj1234567890abcdefghij");
    expect(out.stderr).not.toContain("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    expect(out.stderr).not.toContain("github_pat_11AA22BB33CC44DD55EE66");
    expect(out.stderr).not.toContain("axhub_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    expect(out.stderr).toContain("<redacted>");
    expect(out.stdout).not.toContain("systemMessage");
  });
});

describe("Case F — binary not found (ENOENT) — PR #99 review M2", () => {
  // result.error truthy 분기 — helper binary 자체가 없을 때 spawnSync 가 ENOENT.
  // 현재 codegen 의 분기 1 (result.error || denialRegex.test(stderrText)) 가
  // 한국어 systemMessage 출력 — ADR-0011 Consequences 의 의도된 trade-off.
  // Mental model 은 "권한 prompt" 가 아니라 "binary 부재" 지만 systemMessage 안내가
  // "허용 클릭" 으로 일관 — 사용자가 한 번 클릭해도 다음에도 같은 메시지를 본다는 limitation.
  // M2 follow-up: 별도 systemMessage 분기는 Phase 27.y RFC 로 deferred.
  test("ENOENT result.error path → systemMessage swallow + exit 0 (current trade-off)", () => {
    // Helper 파일 삭제 — buildScript() 가 가리키는 HELPER 가 없어서 spawnSync ENOENT
    if (existsSync(HELPER)) rmSync(HELPER);
    const out = runNode(buildScript());
    expect(out.status).toBe(0);
    expect(out.stdout).toContain("systemMessage");
    expect(out.stdout).toContain("허용");
    expect(out.stderr).toBe("");
  });
});
