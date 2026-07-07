import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("bootstrap device-flow command contract", () => {
  test("keeps the first desktop execute and resume out of watch and monitor mode", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");
    const preCloneReference = localReference.slice(
      localReference.indexOf("## Execute And Watch"),
      localReference.indexOf("## Clone Current Directory"),
    );

    expect(bootstrap).toContain("첫 execute/resume 에 `--watch`/`--watch-timeout` 금지");
    expect(localReference).toContain("Do not attach `--watch` or `--watch-timeout` to the first execute");
    expect(bootstrap).toContain("GitHub device flow 는 평문 URL+코드를 본문에 다시 써요");
    expect(bootstrap).toContain("`Monitor`, `ScheduleWakeup`, `TaskOutput`, `읽는 중 <output>`, 임시 출력 파일 읽기 카드로 코드 노출 금지");
    expect(localReference).toContain("Do not set an intentionally short tool timeout to force Claude Desktop backgrounding");
    expect(localReference).toContain("not from `Monitor`, `ScheduleWakeup`, `TaskOutput`, or reading a generated output file");
    expect(localReference).toContain("use one direct `axhub github accounts list --tenant <tenant> --json` check");
    expect(resumeReference).toContain("Do not use Monitor, ScheduleWakeup, TaskOutput, or background output file reads as the resume control plane");
    expect(localReference).toContain("never verbatim");
    expect(localReference).toContain("strip `--watch --watch-timeout <value>` and `--json` from the first Desktop resume");
    expect(bootstrap).toContain("CLI 가 pending 으로 끝나면 URL·코드를 본문에 쓰고");
    expect(bootstrap).toContain("`인증 확인` 제목의 단일 `axhub github accounts list --tenant <tenant> --json`");
    expect(bootstrap).toContain("승인 반영을 확인해요");
    expect(localReference).toContain("do not write wording that asks them to report back after approval");
    expect(bootstrap).not.toContain("승인해주시면 알려주세요");
    expect(bootstrap).not.toContain("승인 후 알려주세요");
    expect(bootstrap).not.toContain("승인 완료 후 알려주세요");
    expect(resumeReference).toContain("strip `--watch --watch-timeout <value>` and `--json` from the first Desktop resume");
    expect(resumeReference).toContain("never run it verbatim");
    expect(preCloneReference).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap");
    expect(preCloneReference).not.toContain("--execute --watch");
    expect(preCloneReference).not.toContain("--resume-last --watch");
    expect(preCloneReference).not.toMatch(/AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap[^\n]*--execute[^\n]*--json/);
    expect(resumeReference).not.toMatch(/AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap[^\n]*--execute[^\n]*--json/);
  });
});
