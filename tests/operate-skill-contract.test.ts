import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

// operate 는 0 확인 → 1 뜻 알기(바꾸는 일일 때) → 2 명령 찾기 → 3 실행 → 끝 → 알아둘 것 순서의 단일 SKILL.md 예요.
// 2026-11 개편 때 이름을 clarity 에서 operate 로 바꾸고 "짧고 목표 중심, 판단은 CLI 사실 + AI, 맥락은 docs" 로 다시 썼어요.
// 옛 판의 Desktop 첫 문장·도구 제목 고정, 단일 명령 allowlist, 영어 직접 지목, GitHub 재연결의 긴 device flow 절차(지금은 3번의 브라우저 승인 한 줄),
// hidden `authz grants create` 예외, references/execution-guardrails.md(진행 알림·TodoWrite) 는 의도적으로 뺐어요.

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");
const OPERATE = readRepo("skills/operate/SKILL.md");
const CODEX_OPERATE = readRepo("plugins/axhub-codex/skills/operate/SKILL.md");
const FRONTMATTER = OPERATE.slice(0, OPERATE.indexOf("\n---\n", 4));

const sectionOf = (start: string, end: string): string => {
  const from = OPERATE.indexOf(start);
  const to = OPERATE.indexOf(end, from + start.length);
  expect(from, `missing section start: ${start}`).toBeGreaterThanOrEqual(0);
  expect(to, `missing section end: ${end}`).toBeGreaterThan(from);
  return OPERATE.slice(from, to);
};

// 순서 계약: 앞 문자열이 뒤 문자열보다 먼저 나와야 해요.
const expectOrder = (...needles: string[]): void => {
  const offsets = needles.map((needle) => {
    const offset = OPERATE.indexOf(needle);
    expect(offset, `missing: ${needle}`).toBeGreaterThanOrEqual(0);
    return offset;
  });
  for (let i = 1; i < offsets.length; i += 1) {
    expect(offsets[i - 1], `${needles[i - 1]} → ${needles[i]}`).toBeLessThan(offsets[i]);
  }
};

describe("operate skill contract (2026-11 개편판, 옛 clarity)", () => {
  test("is named operate, stays one compact SKILL.md and leaves no clarity behind", () => {
    expect(FRONTMATTER).toContain("name: operate");
    expect(OPERATE).not.toContain("references/");
    expect(OPERATE).not.toContain("clarity");
    for (const root of ["skills", "plugins/axhub/skills", "plugins/axhub-codex/skills"]) {
      expect(existsSync(join(REPO_ROOT, root, "clarity")), `${root}/clarity`).toBe(false);
      expect(existsSync(join(REPO_ROOT, root, "operate", "references")), `${root}/operate/references`).toBe(false);
    }
    expect(Buffer.byteLength(CODEX_OPERATE, "utf8")).toBeLessThanOrEqual(8_000);
  });

  test("frontmatter takes operations on existing apps and yields deploy, code and setup", () => {
    expect(FRONTMATTER).toContain("이미 axhub 에 있는 앱과 회사를 운영하는 명령을 찾아 실행해요");
    for (const utterance of [
      "로그 보여줘",
      "앱 이름·아이콘 바꿔줘",
      "앱 DB 에서 주문 몇 건인지 봐줘",
      "환경변수 바꿔줘",
      "이전 버전으로 되돌려줘",
      "앱 삭제해줘",
      "OO 에게 권한 줘",
    ]) {
      expect(FRONTMATTER, utterance).toContain(`"${utterance}"`);
    }
    for (const utterance of [
      "내 앱들 지금 상태 어때?",
      "어제 버전으로 되돌려줘",
      "내가 쓸 수 있는 커넥터 뭐 있어?",
      "GitHub 계정 다시 연결해줘",
    ]) {
      expect(FRONTMATTER, utterance).toContain(`utterance: "${utterance}"`);
    }
    expect(FRONTMATTER).toContain("배포와 실패 원인은 deploy, 앱 코드는 development, 설치·업데이트·로그인은 setup 이에요");
    expect(FRONTMATTER).toContain('intent: "yield to deploy"');
  });

  test("never memorizes commands — always asks the live CLI", () => {
    expect(OPERATE).toContain("명령 목록은 외우지 않아요. 무엇이 있는지와 쓰는 법은 항상 지금 CLI 에 물어요");
  });

  test("runs the steps in order 0 → 3 and ends with a short result", () => {
    expectOrder("### 0. 확인", "### 1. 뜻 알기 (바꾸는 일일 때)", "### 2. 명령 찾기", "### 3. 실행", "## 끝", "## 알아둘 것");
    expectOrder(
      "axhub plugin-support preflight --json",
      "https://docs.axhub.ai/llms.txt",
      "axhub --json-schema --field-expr",
      "`--help` 를 실행 전에 한 번 읽고",
      "조회는 바로 실행해요",
      "`--execute`",
    );
  });

  test("0 확인: preflight 실패는 setup, 작업공간이 여럿이면 앱의 작업공간을 --tenant 로 붙여요", () => {
    const step = sectionOf("### 0. 확인", "### 1. 뜻 알기");
    expect(step).toContain("`axhub plugin-support preflight --json` 으로 확인해요. 안 되면 setup 스킬로 보내고 멈춰요");
    expect(step).toContain("사용자가 말한 앱 이름이나 현재 폴더의 앱을 써요");
    expect(step).toContain("작업공간이 여럿이면 그 앱의 작업공간을 `--tenant` 로 붙여요(`axhub apps get <앱> --json` 의 `tenant_id`)");
  });

  test("1 뜻 알기: 결과가 큰 일은 운영 문서를 curl 원문으로 읽고, 비슷한 일은 차이를 보여주고 물어요", () => {
    const step = sectionOf("### 1. 뜻 알기", "### 2. 명령 찾기");
    expect(step).toContain("삭제·멈추기·되돌리기·공개 범위·권한처럼 결과가 큰 일은");
    expect(step).toContain("`https://docs.axhub.ai/llms.txt` 목차의 운영 문서(`.md`)를 `curl -fsSL` 로 원문 그대로 읽고");
    expect(step).toContain("뜻과 되돌릴 수 있는지 확인해요");
    expect(step).toContain("삭제와 절전·일시정지의 차이");
    expect(step).toContain("비슷한 일이 여럿이면(예: \"멈춰줘\" → 절전·일시정지·삭제) 하나를 고르지 말고 차이를 한 줄씩 보여주고 어느 쪽인지 물어요");
  });

  test("2 명령 찾기: 스키마는 잘라 읽고, --help 먼저, 내 범위는 mine, 없으면 없다고 말해요", () => {
    const step = sectionOf("### 2. 명령 찾기", "### 3. 실행");
    expect(step).toContain("`axhub --json-schema --field-expr '.commands[\"<명사>\"][\"<동사>\"]'` 처럼 필요한 부분만 잘라 찾아요");
    expect(step).toContain("없을 때만 한 단계씩 넓혀요. 전체를 통째로 읽지 않아요");
    expect(step).toContain("고른 명령의 `--help` 를 실행 전에 한 번 읽고, 거기 있는 인자·플래그만 써요");
    expect(step).toContain("볼 수 있는 커넥터는 `axhub connectors mine`");
    expect(step).toContain("바꿀 수 있는지는 `axhub authz grants mine`");
    expect(step).toContain("회사 전체 목록으로 대신하지 않아요");
    expect(step).toContain("찾지 못하면 \"axhub 에 그 기능은 없어요\"라고 말하고 가장 가까운 명령을 알려줘요");
    expect(step).toContain("없는 명령을 지어내지 않아요");
  });

  test("3 실행: 조회는 바로, 바꾸는 명령은 미리보기 → 한 번 묻고 → --execute, 되돌릴 수 없으면 먼저 말해요", () => {
    const step = sectionOf("### 3. 실행", "## 끝");
    expect(step).toContain("조회는 바로 실행해요");
    expect(step).toContain("바꾸는 명령은 먼저 미리보기(실행 플래그 없이)로 무엇이 바뀌는지 받아요");
    // 11-11 실측: 정확히 말한 되돌릴 수 있는 바꾸기는 바로, 되돌리기 어렵거나 모호하면 묻기
    expect(step).toContain("사용자가 정확히 말한, 되돌릴 수 있는 바꾸기(이름·소개·아이콘·환경변수 추가 등)는 미리보기가 요청과 같으면 바로 `--execute` 로 실행해요");
    expect(step).toContain("삭제·일시정지·롤백·앱 DB 쓰기·권한 주기처럼 되돌리기 어렵거나 영향이 큰 일, 또는 요청이 모호하면 미리보기를 보여주고 한 번 물은 뒤 `--execute`(필요하면 `--yes`)로 실행해요");
    expect(step).toContain("되돌릴 수 없으면 그 사실을 먼저 말해요");
    // 미리보기 → 묻기 → 실행 순서
    expect(step.indexOf("미리보기(실행 플래그 없이)")).toBeLessThan(step.indexOf("한 번 물은 뒤"));
    expect(step.indexOf("한 번 물은 뒤")).toBeLessThan(step.indexOf("`--execute`(필요하면"));
    // 앱 DB 쓰기 SQL 은 바꾸는 일이에요
    expect(step).toContain("앱 DB 에 쓰는 SQL(INSERT·UPDATE·DELETE·테이블 변경)은 묻는 쪽이에요. 조회 SQL 만 바로 실행해요");
    // 값은 채팅으로 받지 않아요
    expect(step).toContain("값(환경변수 등)은 채팅으로 받지 않고 `.env` 에서 읽게 해요(`--from-dotenv`)");
    // 오류는 쉬운 말로, exit 64 는 --help 를 다시 읽고 한 번만
    expect(step).toContain("실패하면 `message_ko`·`recovery_action` 을 쉬운 말로 설명해요");
    expect(step).toContain("인자 오류(exit 64)면 `--help` 를 다시 읽고 한 번만 고쳐 실행해요");
  });

  test("3 실행: 브라우저 승인이 필요한 명령(GitHub 다시 연결)은 코드를 채팅에 바로, 한 번만 기다리고, 만료는 --fresh", () => {
    const step = sectionOf("### 3. 실행", "## 끝");
    // GitHub 다시 연결은 operate, 첫 연결은 bootstrap 4번이 맡아요 (AP-18). 옛 clarity Device Flow 절의 정본이에요.
    expect(step).toContain(
      "브라우저 승인이 필요한 명령(예: GitHub 다시 연결 `axhub github link`)은 출력의 주소와 입력 코드를 채팅 본문에 바로 보여주고, 승인은 한 번만(`--wait-approval 90s`) 기다려요",
    );
    expect(step).toContain("그래도 없으면 \"승인하고 알려 주세요\"라고 하고 멈춰요");
    expect(step).toContain("코드가 만료됐으면 `--fresh` 로 새로 받아요");
    // 코드 표시가 기다리기보다 먼저예요.
    expect(step.indexOf("채팅 본문에 바로 보여주고")).toBeLessThan(step.indexOf("--wait-approval 90s"));
    // 값은 채팅으로 받지 않는다는 줄 바로 앞이에요.
    expect(step.indexOf("`--fresh` 로 새로 받아요")).toBeLessThan(step.indexOf("값(환경변수 등)은 채팅으로 받지 않고"));
    expect(step).not.toContain("device_code");
  });

  test("끝: 한두 줄 결과, 목록은 표, 다음 일은 한 줄로 이어줘요", () => {
    const end = sectionOf("## 끝", "## 알아둘 것");
    expect(end).toContain("무엇을 했고 결과가 어떤지 한두 줄로 말해요. 목록은 표로 보여줘요");
    expect(end).toContain("결과가 다음 일로 이어지면 한 줄로 알려줘요");
    expect(end).toContain("'대시보드 만들어줘'라고 하면 돼요");
  });

  test("알아둘 것: 섞인 요청은 넘기고, headless 는 미리보기까지, 숨김 명령·내부 id·비밀값은 쓰지 않아요", () => {
    const notes = OPERATE.slice(OPERATE.indexOf("## 알아둘 것"));
    expect(notes).toContain(
      "배포·배포 실패 원인은 deploy, 앱 코드는 development, 새 앱은 bootstrap, 설치·업데이트·로그인은 setup 이 맡아요",
    );
    expect(notes).toContain("섞여 오면 그 부분은 넘기고 운영 부분만 해요");
    expect(notes).toContain("질문에 답할 수 없는 실행(headless)이면 조회만 하고, 바꾸는 명령은 어떤 것이든 미리보기까지만 해요");
    expect(notes).toContain("`axhub plugin-support` 명령은 운영에 쓰지 않아요");
    expect(notes).toContain("사용자에게 보이는 문장에는 내부 id·raw 출력·비밀값을 쓰지 않아요. 앱은 이름으로 말해요");
  });

  test("옛 clarity 의 Desktop 고정 문구·device flow·hidden 예외·진행 알림은 다시 들어오지 않아요", () => {
    for (const retired of [
      "요청하신 운영 명령을 확인할게요",
      "명령 찾기\"",
      "Desktop-visible command allowlist",
      "Use the axhub clarity skill",
      "계정 인증 시작",
      "TodoWrite",
      "[1/4]",
      "execution-guardrails",
    ]) {
      expect(OPERATE, retired).not.toContain(retired);
    }
  });

  test("codex 판은 질문 계약 요약을 받고 본문 전체가 8,000B 안에 들어가요", () => {
    expect(CODEX_OPERATE).toContain("## 질문 계약 (요약)");
    expect(CODEX_OPERATE).toContain("미리 넣어 둔 문구·유사 표현·무응답은 답이 아니에요");
    expect(CODEX_OPERATE).toContain("카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요");
    // 새로 만드는 일의 canonical 승인 문구가 없는 스킬이라 승인 게이트 요약은 받지 않아요.
    expect(CODEX_OPERATE).not.toContain("## 승인 게이트 계약 (요약)");
    expect(CODEX_OPERATE).toContain("\n## 순서\n");
    expect(CODEX_OPERATE).toContain("사용자에게 보이는 문장에는 내부 id·raw 출력·비밀값을 쓰지 않아요");
    expect(CODEX_OPERATE).not.toContain("examples:");
  });
});

describe("operate 라우팅", () => {
  test("라우팅 fixture 가 운영 요청은 operate, 배포·코드·새 앱·설치는 각 스킬로 보내요", () => {
    const fixture = JSON.parse(readRepo("tests/routing/operate-routing.fixture.json")) as {
      cases: Array<{ utterance: string; context?: string; expected: string }>;
    };
    const expectedFor = (utterance: string): string[] =>
      fixture.cases
        .filter((entry) => entry.utterance === utterance && !(entry.context ?? "").includes("axhub 무관"))
        .map((entry) => entry.expected);
    for (const utterance of [
      "내 앱들 지금 상태 어때?",
      "현재 앱 상태 확인해줘",
      "로그 보여줘",
      "앱 이름이랑 아이콘 바꿔줘",
      "앱 DB 에서 이번 주 주문 몇 건인지 봐줘",
      "환경변수 바꿔줘",
      "어제 버전으로 되돌려줘",
      "앱 삭제해줘",
      "지원팀한테 그 폴더 업로드 권한 줘",
      "내가 쓸 수 있는 커넥터 뭐 있어?",
    ]) {
      expect(expectedFor(utterance), utterance).toEqual(["operate"]);
    }
    expect(expectedFor("배포해줘")).toEqual(["deploy"]);
    expect(expectedFor("왜 배포 실패했어")).toEqual(["deploy"]);
    expect(expectedFor("할 일에 마감일 추가해줘")).toEqual(["development"]);
    expect(expectedFor("회사 데이터로 대시보드 만들어줘")).toEqual(["development"]);
    expect(expectedFor("새 앱 만들어줘")).toEqual(["bootstrap"]);
    expect(expectedFor("axhub 업데이트해줘")).toEqual(["setup"]);
    // GitHub 다시 연결은 operate, 첫 연결은 bootstrap 4번이 맡아요. setup 은 GitHub 연결을 하지 않아요.
    expect(expectedFor("GitHub 다시 연결해줘")).toEqual(["operate"]);
    expect(expectedFor("reconnect my GitHub account")).toEqual(["operate"]);
  });

  test("codex description 이 운영 트리거를 들고 배포·코드·새 앱·설치를 넘겨요", () => {
    const codexDescriptions = JSON.parse(readRepo("codex-overrides/routing/descriptions.json")) as Record<string, string>;
    const codex = codexDescriptions.operate ?? "";
    for (const needle of [
      "\"내 앱들 지금 상태 어때?\"",
      "\"로그 보여줘\"",
      "\"앱 이름·아이콘 바꿔줘\"",
      "\"앱 DB 에서 이번 주 주문 몇 건인지 봐줘\"",
      "\"환경변수 바꿔줘\"",
      "\"어제 버전으로 되돌려줘\"",
      "\"앱 삭제해줘\"",
      "\"OO 에게 권한 줘\"",
      "\"내가 쓸 수 있는 커넥터 뭐 있어?\"",
      "배포와 배포 실패 원인은 deploy",
      "앱 코드(기능 추가·커넥터 데이터로 화면 만들기)는 development",
      "새 앱은 bootstrap",
      "\"GitHub 계정 다시 연결해줘\"",
      "설치·업데이트·로그인은 setup",
    ]) {
      expect(codex, needle).toContain(needle);
    }
    // 번들 description 은 override 그대로예요.
    expect(CODEX_OPERATE).toContain("내가 쓸 수 있는 커넥터 뭐 있어?");
    // development codex description 도 운영 명령을 operate 로 넘겨요.
    expect(codexDescriptions.development).toContain("axhub 운영 명령은 operate 예요");
  });
});

describe("operate hidden admin commands", () => {
  test("hidden admin commands like authz grants create are checked with --help", () => {
    const body = require("node:fs").readFileSync(require("node:path").join(import.meta.dir, "..", "skills/operate/SKILL.md"), "utf8");
    expect(body).toContain("목록에 안 보이는 관리자 명령(예: 권한 주기 `axhub authz grants create`)은 그 명령의 `--help` 로 있는지 확인해요");
  });
});

