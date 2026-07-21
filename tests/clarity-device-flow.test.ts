import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("clarity device flow UX contract", () => {
  test("surfaces the verification URL and user code in normal chat text", () => {
    const clarity = readRepo("skills/clarity/SKILL.md") + readRepo("skills/clarity/references/execution-guardrails.md");

    expect(clarity).toContain("## Device Flow 코드 표시");
    expect(clarity).toContain("일반 채팅 본문에 URL과 입력 코드를 다시 써요");
    expect(clarity).toContain("shell loop, background watcher, persistent monitor 를 쓰지 않아요");
    expect(clarity).toContain("`Monitor 사용` 권한 카드가 뜨는 명령은 실패");
    expect(clarity).toContain("device flow fast path 에서는 다른 사전 점검을 건너뛰어요");
    expect(clarity).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --tenant <tenant>");
    expect(clarity).toContain("device flow 를 시작하는 Bash/tool call 제목과 description 은 모두 정확히 `계정 인증 시작`");
    expect(clarity).toContain("사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL");
    expect(clarity).toContain("Markdown URL 링크 문법은 전부 금지");
    expect(clarity).toContain("device-flow URL 은 Claude Desktop 이 자동 링크로 바꾸지 못하도록 URL 부분만 inline code span 으로 써요");
    expect(clarity).toContain("[https://github.com/login/device](github.com/login/device)");
    expect(clarity).toContain("인증 URL: \\`https://github.com/login/device\\`");
    expect(clarity).toContain("입력 코드: <USER_CODE>");
    expect(clarity).toContain("승인 확인이나 계정 목록 조회를 시작하기 전에 먼저 assistant 본문 문장으로 URL과 코드를 노출");
    expect(clarity).toContain("`실행됨 명령 N개`, `TaskOutput 사용함`, tool 카드, 접힌 로그만 남기고 응답을 끝내면 실패");
    expect(clarity).toContain("코드를 명령 출력이나 로그 읽기 결과 안에만 남기지 않아요");
    expect(clarity).toContain("코드 표시 뒤 assistant 응답을 끝내지 말고");
    expect(clarity).toContain("`계정 인증 시작` command 뒤 같은 assistant turn 에서 title 과 description 이 모두 정확히 `인증 확인`");
    expect(clarity).toContain("자동 브라우저 열기는 입력 코드가 포함된 직접 URL을 우선 열 수 있어요");
    expect(clarity).toContain("CLI 가 pending 으로 끝나도 `sleep`, `&&`, shell loop, watcher 로 감시하지 말고");
    expect(clarity).toContain("승인 확인용 `while true ... accounts list ... sleep ...` 루프나 persistent monitor 는 쓰지 않아요");
    expect(clarity).toContain("device_code 같은 내부 교환용 값은 절대 쓰지 않아요");
    expect(clarity).toContain("axhub github link");
    expect(clarity).not.toContain("operational-lookups.md");
  });
});
