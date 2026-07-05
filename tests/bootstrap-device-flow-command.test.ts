import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("bootstrap device-flow command contract", () => {
  test("keeps the first desktop execute and resume out of watch mode", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");
    const preCloneReference = localReference.slice(
      localReference.indexOf("## Execute And Watch"),
      localReference.indexOf("## Clone Current Directory"),
    );

    expect(bootstrap).toContain("첫 execute/resume 에 `--watch`/`--watch-timeout` 을 붙이지 않아요");
    expect(localReference).toContain("Do not attach `--watch` or `--watch-timeout` to the first execute");
    expect(localReference).toContain("background task output file");
    expect(bootstrap).toContain("`timeout` 을 12000~15000ms 로 짧게 지정");
    expect(localReference).toContain("Set the Bash tool `timeout` to 12000-15000ms");
    expect(localReference).toContain("do not run the emitted `resume_command`");
    expect(localReference).toContain("never verbatim");
    expect(localReference).toContain("strip `--watch --watch-timeout <value>` and `--json` from the first Desktop resume");
    expect(bootstrap).toContain("output file 을 알려주면 그 파일을 즉시 읽어");
    expect(bootstrap).toContain("원래 execute background task 가 `auto_poll:true` 로 아직 돌고 있으면 중복 resume 을 실행하지 말고");
    expect(bootstrap).toContain("승인되면 이 화면에서 자동으로 계속 확인");
    expect(localReference).toContain("do not write wording that asks them to report back after approval");
    expect(localReference).toContain("Pending messages must say that approval will be detected automatically");
    expect(bootstrap).not.toContain("승인해주시면 알려주세요");
    expect(bootstrap).not.toContain("승인 후 알려주세요");
    expect(bootstrap).not.toContain("승인 완료 후 알려주세요");
    expect(resumeReference).toContain("strip `--watch --watch-timeout <value>` and `--json` from the first Desktop resume");
    expect(resumeReference).toContain("never run it verbatim");
    expect(resumeReference).toContain("original execute is still running as an `auto_poll:true` background task");
    expect(preCloneReference).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap");
    expect(preCloneReference).not.toContain("--execute --watch");
    expect(preCloneReference).not.toContain("--resume-last --watch");
    expect(preCloneReference).not.toMatch(/AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap[^\n]*--execute[^\n]*--json/);
    expect(resumeReference).not.toMatch(/AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap[^\n]*--execute[^\n]*--json/);
  });
});
