// Phase 27.x hotfix — Step 0.5: 권한 layer probe + Node runner stdio capture mechanism.
//
// Deterministic mock harness: "helper stub" 이 특정 stderr 를 emit 하고,
// Node runner (codegen-preflight-injection 이 emit 할 코드와 동일 로직) 가
// stdio:['inherit','inherit','pipe'] 로 capture 해서 denialRegex 분기를 실행해요.
//
// pass criteria (iteration 4 Minor (c)):
//  (a) helper stub stderr → runner 가 buffer 로 capture (buffer.length > 0)
//  (b) strict-anchor regex 매칭 시 → runner stdout 에 systemMessage JSON + exit 0
//  (c) 미매칭 시 → runner stderr passthrough + helper exit code propagate

import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync, chmodSync, mkdtempSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

const REPO_ROOT = join(import.meta.dir, "../../..");
const FIXTURES = join(REPO_ROOT, "tests/fixtures/preflight-permission-denied");

const DENIAL_STDERR = readFileSync(join(FIXTURES, "stderr.txt"), "utf8").trimEnd();
const EXPECTED_MSG = JSON.parse(
  readFileSync(join(FIXTURES, "expected-systemmessage.json"), "utf8"),
);

// Runner 본문 — codegen-preflight-injection.ts 의 lite variant 와 동일 로직.
// helper 경로는 stub 으로 교체해서 deterministic 하게 테스트해요.
const makeRunnerScript = (helperPath: string): string => `
const cp = require('child_process');
const env = {...process.env};
const helper = ${JSON.stringify(helperPath)};
const result = cp.spawnSync(helper, ['preflight', '--json'], {
  stdio: ['inherit', 'inherit', 'pipe'],
  env,
});
const stderrText = String(result.stderr ?? '');
const denialRegex = /^Shell command permission check failed.*requires approval/im;
if (result.error || (result.status !== 0 && denialRegex.test(stderrText))) {
  console.log(JSON.stringify({systemMessage:"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)"}));
  process.exit(0);
} else if (stderrText.length > 0) {
  process.stderr.write(stderrText);
}
process.exit(typeof result.status === 'number' ? result.status : 0);
`;

// Helper stub — 주어진 stderrContent 를 stderr 에 쓰고 exitCode 로 종료해요.
// node shebang 으로 실행 가능한 stub 이에요 (chmod +x).
const makeHelperStub = (stderrContent: string, exitCode: number): string =>
  `#!/usr/bin/env node\nprocess.stderr.write(${JSON.stringify(stderrContent)});\nprocess.exit(${exitCode});\n`;

interface RunResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

function runWithHelper(
  helperStderr: string,
  helperExitCode: number,
): RunResult {
  const tmp = mkdtempSync(join(tmpdir(), "axhub-runner-test-"));

  // Helper stub — spawnSync 이 직접 호출할 실행 파일
  const helperPath = join(tmp, "axhub-helpers-stub");
  writeFileSync(helperPath, makeHelperStub(helperStderr, helperExitCode));
  chmodSync(helperPath, 0o755);

  // Runner script — codegen 이 emit 할 로직 그대로
  const runnerPath = join(tmp, "runner.js");
  writeFileSync(runnerPath, makeRunnerScript(helperPath));

  // Runner 를 node 로 실행, stdout/stderr capture
  const proc = spawnSync("node", [runnerPath], {
    stdio: ["inherit", "pipe", "pipe"],
    encoding: "utf8",
  });

  return {
    stdout: proc.stdout ?? "",
    stderr: proc.stderr ?? "",
    exitCode: proc.status ?? -1,
  };
}

describe("Step 0.5 — Node runner stdio capture mechanism (permission-prompt-surface)", () => {
  test(
    "(a)+(b) denial regex 매칭: helper stderr capture (buffer > 0) + systemMessage JSON stdout + exit 0",
    () => {
      // helper 가 실제 Claude Code denial 텍스트를 stderr 로 emit, non-zero exit
      const result = runWithHelper(DENIAL_STDERR, 1);

      // (a) runner 가 내부 stderr 를 capture 했다는 근거: regex 가 매칭되어 systemMessage 출력됨.
      // 만약 stdio:'inherit' 였다면 result.stderr 가 null/Buffer(0) 라 regex 실패했을 것.
      expect(result.stdout.trim().length).toBeGreaterThan(0);

      // (b-1) stdout 이 valid JSON 이고 systemMessage 필드가 있어야 해요
      const parsed = JSON.parse(result.stdout.trim());
      expect(parsed).toHaveProperty("systemMessage");
      expect(parsed.systemMessage).toBe(EXPECTED_MSG.systemMessage);

      // (b-2) exit 0 — SKILL Step 0 계속 진행 신호
      expect(result.exitCode).toBe(0);

      // (b-3) runner stderr 에 denial 텍스트 그대로 노출되지 않아야 해요 (swallow)
      expect(result.stderr).not.toContain("Shell command permission check failed");
    },
  );

  test(
    "(c) 미매칭 stderr: passthrough + helper exit code propagate",
    () => {
      const unrecognized = "warning: some unrecognized informational stderr from helper";
      const helperExitCode = 2;

      const result = runWithHelper(unrecognized, helperExitCode);

      // (c-1) stdout 에 systemMessage 없어야 해요 (false-positive 차단)
      expect(result.stdout.trim()).toBe("");

      // (c-2) runner stderr 에 원본 unrecognized stderr 가 passthrough 돼야 해요
      expect(result.stderr).toContain(unrecognized);

      // (c-3) runner 가 helper exit code 를 그대로 propagate 해야 해요
      expect(result.exitCode).toBe(helperExitCode);
    },
  );

  test(
    "(c-zero) exit 0 + no stderr: clean path propagates exit 0, no output",
    () => {
      // 정상 path — helper 가 stderr 없이 exit 0
      const result = runWithHelper("", 0);

      expect(result.stdout.trim()).toBe("");
      expect(result.stderr.trim()).toBe("");
      expect(result.exitCode).toBe(0);
    },
  );

  test(
    "strict-anchor regex 는 generic 'permission' 단어만으로 false-positive 발생하지 않아요",
    () => {
      const generic = "Error: permission denied (unrelated error)";
      const result = runWithHelper(generic, 1);

      // systemMessage 가 출력되면 안 돼요 (strict-anchor 미매칭)
      expect(result.stdout.trim()).toBe("");
      // passthrough 로 generic stderr 보여야 해요
      expect(result.stderr).toContain(generic);
      expect(result.exitCode).toBe(1);
    },
  );
});
