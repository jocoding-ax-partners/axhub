import { describe, expect, test } from "bun:test";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// DP-6 ② 실 CLI 계약 스냅샷 — AXHUB_REAL_CLI=1 일 때만 실행돼요 (nightly CI /
// 로컬 opt-in). fixture shim 이 아니라 PATH 의 실제 axhub 바이너리를 실행해
// plugin 이 의존하는 읽기 전용 표면(스키마 모양·exit 계약·플래그 존재)을 검증해요.
// 인증·네트워크 상태와 무관하게 참이어야 하는 것만 assert 해요.

const REAL = process.env.AXHUB_REAL_CLI === "1";
const t = REAL ? test : test.skip;

interface RunResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

const run = (args: string[]): RunResult => {
  const result = Bun.spawnSync({
    cmd: ["axhub", ...args],
    cwd: mkdtempSync(join(tmpdir(), "axhub-contract-")),
    stdout: "pipe",
    stderr: "pipe",
    timeout: 90_000,
  });
  return { exitCode: result.exitCode ?? -1, stdout: result.stdout.toString(), stderr: result.stderr.toString() };
};

describe("real CLI contract snapshot (AXHUB_REAL_CLI=1)", () => {
  t("onboarding-detect: exit 0 + gaps 배열 + null 완료 계약 (no_gap 문자열 강제 금지)", () => {
    const res = run(["plugin-support", "onboarding-detect", "--json"]);
    expect(res.exitCode).toBe(0);
    const detect = JSON.parse(res.stdout) as { first_gap?: string | null; gaps?: string[] };
    expect(Array.isArray(detect.gaps)).toBe(true);
    const gaps = detect.gaps ?? [];
    if (gaps.length === 0) {
      // 완료 상태 계약(D1): gaps 가 비면 first_gap 은 null/부재 또는 "no_gap" — plugin 은 둘 다 완료로 수용해요.
      expect(detect.first_gap === null || detect.first_gap === undefined || detect.first_gap === "no_gap").toBe(true);
    } else {
      expect(detect.first_gap).toBe(gaps[0]);
    }
  });

  t("preflight: exit ∈ {0,64,65} + JSON envelope", () => {
    const res = run(["plugin-support", "preflight", "--json"]);
    expect([0, 64, 65]).toContain(res.exitCode);
    if (res.stdout.trim().length > 0) {
      const pf = JSON.parse(res.stdout) as { cli_present?: boolean };
      expect(typeof pf.cli_present).toBe("boolean");
    }
  });

  t("plugin-support import 표면 존재 (v0.21.3+): unknown-subcommand exit 2 가 아니어야 해요", () => {
    const res = run(["--json", "plugin-support", "import", "--mode", "preview"]);
    expect(res.exitCode).not.toBe(2);
  });

  t("deploy logs: 앱 단위 표면 + --since/--until 시간창 플래그", () => {
    const res = run(["deploy", "logs", "--help"]);
    expect(res.exitCode).toBe(0);
    expect(res.stdout).toContain("--since");
    expect(res.stdout).toContain("--until");
    expect(res.stdout).toContain("--app");
  });

  t("deploy verify / deploy diagnose 표면 존재", () => {
    expect(run(["deploy", "verify", "--help"]).exitCode).toBe(0);
    expect(run(["deploy", "diagnose", "--help"]).exitCode).toBe(0);
  });

  t("github device-flow 표면: link/accounts 존재", () => {
    const res = run(["github", "--help"]);
    expect(res.exitCode).toBe(0);
    expect(res.stdout.toLowerCase()).toContain("accounts");
  });

  t("feedback 표면 (AP-19, hidden): 탑재된 CLI 면 -m 단일 인자 + --dry-run 계약", () => {
    // hidden 명령이라 구/미배포 CLI 에는 없어요 — 부재(비 0 exit)는 계약 위반이
    // 아니라 skip 이에요 (AP-19 는 best-effort, 조용한 실패가 곧 가용성 게이트).
    const res = run(["feedback", "--help"]);
    if (res.exitCode !== 0) return;
    expect(res.stdout).toContain("-m");
    expect(res.stdout).toContain("--dry-run");
  });
});
