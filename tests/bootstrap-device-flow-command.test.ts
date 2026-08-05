import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("bootstrap device-flow command contract", () => {
  // 백엔드에 연동된 GitHub 계정이 있으면 bootstrap 은 device flow 를 아예 타지 않아요.
  // 아래 안무 계약은 전부 "미연동·만료" fallback 에만 적용돼요 — 그 프레이밍이
  // 스킬 본문과 reference 에서 같이 유지되는지 먼저 고정해요.
  test("frames the whole device-flow choreography as the not-linked fallback", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");
    const errorsReference = readRepo("skills/bootstrap/references/errors-and-followups.md");

    // Step 6: 정상 응답 = 연동됨 = 인증 단계 없음.
    expect(bootstrap).toContain("이 조회가 정상 응답하면 GitHub 계정이 이미 연동된 상태라 **인증 단계가 없어요**");
    expect(bootstrap).toContain("device flow 를 미리 시작하거나 인증이 필요하다고 말하지 않아요");
    expect(bootstrap).toContain("9단계 device flow 안무는 이 fallback 전용이에요");
    // Step 9: 안무 진입 자체가 fallback.
    expect(bootstrap).toContain("연동된 계정이면 execute 는 device flow 없이 끝나요");
    // relogin 은 axhub 재로그인이 아니라 연동 없음/만료 신호예요.
    expect(bootstrap).toContain("`github_relogin_required` 는 연동이 없거나 만료된 상태라");
    expect(errorsReference).toContain("연동이 살아 있으면 이 경로 자체가 안 나와요");
    expect(templateReference).toContain("bootstrap continues with **no authentication step at all**");
    expect(templateReference).toContain("Device flow is the fallback for this case only");
    expect(localReference).toContain("A linked GitHub account makes execute finish with no device flow at all");
    expect(resumeReference).toContain("a linked GitHub account never reaches a pending device flow");
    // 영속을 약속하지 않아요 — refresh token 만료 뒤 재연동은 정상 경로예요.
    expect(bootstrap).not.toContain("다시 묻지 않아요");
    expect(templateReference).not.toContain("only once and never again");
  });

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
    expect(preCloneReference).toContain("axhub --no-input apps bootstrap");
    expect(preCloneReference).not.toContain("--execute --watch");
    expect(preCloneReference).not.toContain("--resume-last --watch");
    expect(preCloneReference).not.toMatch(/axhub --no-input apps bootstrap[^\n]*--execute[^\n]*--json/);
    expect(resumeReference).not.toMatch(/axhub --no-input apps bootstrap[^\n]*--execute[^\n]*--json/);
  });

  test("surfaces the code via the short github link fast path instead of re-running a silent execute", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");

    expect(bootstrap).toContain("인증 URL: `https://github.com/login/device`");
    expect(bootstrap).toContain("입력 코드: <USER_CODE>");
    expect(bootstrap).toContain("URL 부분만 inline code span");
    expect(bootstrap).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link");
    expect(bootstrap).toContain("같은 `--execute` 를 절대 다시 실행하지 않아요");
    expect(bootstrap).toContain("새 device code 를 발급해 사용자가 보고 있는 코드를 무효로 만들어요");
    expect(bootstrap).toContain("코드를 보여주기 전에 `인증 확인` 을 먼저 실행하거나, 코드만 보여주고 응답을 끝내면 실패예요");
    expect(bootstrap).toContain("`github link` fast path 로 코드를 먼저 보여주고 `--resume-last` 로만 이어가요");
    expect(bootstrap).toContain("NEVER 코드가 안 보인 채 끝난 execute 를 같은 `--execute` 로 재실행하지 않아요");
  });
});
