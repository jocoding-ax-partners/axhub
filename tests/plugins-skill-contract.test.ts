import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

// plugins 는 0 확인 → 1 찾기·받기 → 2 설치 → 3 올리기 → 4 상태·내리기 → 5 실패하면 → 끝 → 알아둘 것 순서의 단일 SKILL.md 예요.
// 2026-11 개편 때 "짧고 목표 중심, 판단은 CLI 사실 + AI, 맥락은 docs" 로 다시 썼어요.
// 옛 판의 AP-17 CLI 경로 찾기 3단계(지금은 preflight 실패 → setup), CLI 0.50.0 버전 가드(지금은 명령별 `--help`),
// 상태 17개와 next_action 대응표(지금은 CLI 의 `headline`·`next_action`), PAT 발급·로그인 복구 절차, 콘솔 주소 목록,
// 승인 카드 문구 세부(Codex 질문 프로토콜은 번들 빌드가 주입), references/publish.md 는 의도적으로 뺐어요.
// 새로 더한 것은 내리기(`plugin delist`)예요.

const ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(ROOT, path), "utf8");
const SKILL = readRepo("skills/plugins/SKILL.md");
const FRONTMATTER = SKILL.slice(0, SKILL.indexOf("\n---\n", 4));
const CLAUDE_BUNDLE = readRepo("plugins/axhub/skills/plugins/SKILL.md");
const CODEX_BUNDLE = readRepo("plugins/axhub-codex/skills/plugins/SKILL.md");

const sectionOf = (start: string, end: string): string => {
  const from = SKILL.indexOf(start);
  const to = SKILL.indexOf(end, from + start.length);
  expect(from, `missing section start: ${start}`).toBeGreaterThanOrEqual(0);
  expect(to, `missing section end: ${end}`).toBeGreaterThan(from);
  return SKILL.slice(from, to);
};

// 순서 계약: 앞 문자열이 뒤 문자열보다 먼저 나와야 해요.
const expectOrder = (text: string, ...needles: string[]): void => {
  const offsets = needles.map((needle) => {
    const offset = text.indexOf(needle);
    expect(offset, `missing: ${needle}`).toBeGreaterThanOrEqual(0);
    return offset;
  });
  for (let i = 1; i < offsets.length; i += 1) {
    expect(offsets[i - 1], `${needles[i - 1]} → ${needles[i]}`).toBeLessThan(offsets[i]);
  }
};

describe("plugins skill contract (2026-11 개편판)", () => {
  test("stays one compact SKILL.md without references", () => {
    expect(FRONTMATTER).toContain("name: plugins");
    expect(SKILL).not.toContain("references/");
    for (const root of ["skills", "plugins/axhub/skills", "plugins/axhub-codex/skills"]) {
      expect(existsSync(join(ROOT, root, "plugins", "references")), `${root}/plugins/references`).toBe(false);
    }
    expect(CLAUDE_BUNDLE).toBe(SKILL);
    expect(Buffer.byteLength(CODEX_BUNDLE, "utf8")).toBeLessThanOrEqual(8_000);
  });

  test("frontmatter takes company marketplace requests and yields deploy, bootstrap and setup", () => {
    expect(FRONTMATTER).toContain("회사 플러그인 마켓플레이스에서 플러그인을 찾고, Claude Code·Codex 에 설치하고, 내 플러그인을 올려 게시 심사를 신청해요");
    for (const utterance of [
      "플러그인 목록 보여줘",
      "OO 플러그인 설치해줘",
      "OO 1.2.0 받아줘",
      "이 스킬 플러그인으로 올려줘",
      "플러그인 심사 어떻게 됐어?",
      "회사 마켓플레이스 연결해줘",
      "플러그인 내려줘",
    ]) {
      expect(FRONTMATTER, utterance).toContain(`"${utterance}"`);
    }
    expect(FRONTMATTER).toContain("앱 배포는 deploy, 새 앱은 bootstrap, axhub 플러그인 자체 업데이트는 setup 이에요");
    expect(FRONTMATTER).toContain('intent: "yield to deploy"');
  });

  test("asks the live CLI for commands and reads the marketplace docs verbatim", () => {
    expect(SKILL).toContain("명령은 지금 CLI 에 묻고, 제공 조건·자동 검수·내리기와 회수의 뜻은 문서 원문으로 읽어요");
  });

  test("runs the steps in order 0 → 5 and ends with a short result", () => {
    expectOrder(
      SKILL,
      "### 0. 확인",
      "### 1. 찾기·받기",
      "### 2. 설치",
      "### 3. 올리기",
      "### 4. 상태·내리기",
      "### 5. 실패하면",
      "## 끝",
      "## 알아둘 것",
    );
  });

  test("0 확인: preflight 실패는 setup, --tenant, 올리기·연결·내리기는 문서 원문, --help 먼저", () => {
    const step = sectionOf("### 0. 확인", "### 1. 찾기·받기");
    expect(step).toContain("`axhub plugin-support preflight --json` 으로 확인해요. 안 되면 setup 으로 보내고 멈춰요");
    expect(step).toContain("작업공간이 여럿이면 `--tenant` 를 붙여요");
    expect(step).toContain("회사 UUID(`axhub auth status --json` 의 `tenants[].tenant_id`)");
    expect(step).toContain("`https://docs.axhub.ai/llms.txt` 의 「회사 플러그인 빠른 시작」·「회사 플러그인 마켓플레이스」를 `curl -fsSL` 로 원문 그대로 읽고 해요");
    expect(step).toContain("쓰기 전에 그 명령의 `--help` 를 읽고, 거기 있는 인자만 써요");
  });

  test("1 찾기·받기: 목록은 좁혀 보고, 받기는 정확한 버전만 새 파일로", () => {
    const step = sectionOf("### 1. 찾기·받기", "### 2. 설치");
    expect(step).toContain("`axhub plugin list --json` 을 검색어·페이지로 좁혀 봐요");
    expect(step).toContain("받기는 정확한 버전만 새 파일로 받아요(`plugin download --app --version --output`)");
  });

  test("2 설치: 미리보기와 위험 요약 → 한 번 묻고 → --execute --yes, 마켓플레이스 연결도 한 번 물어요", () => {
    const step = sectionOf("### 2. 설치", "### 3. 올리기");
    expect(step).toContain("플러그인·정확한 버전·도구(claude 또는 codex) 하나를 정해요. 도구를 모르면 물어요");
    expect(step).toContain("미리보기(`plugin install … --json`)와 목록의 위험 요약(hooks·MCP·실행 파일 수)을 보여줘요");
    expect(step).toContain("설치하려는 버전이 지금 제공 중인 버전과 다르면 함께 알려요");
    expect(step).toContain("한 번 물은 뒤 `--execute --yes` 로 설치하고, 그 도구를 다시 시작하라고 알려요");
    expect(step).toContain("30일짜리 읽기 키를 저장하는 일이라 한 번 물어요");
    expectOrder(step, "미리보기(`plugin install … --json`)", "위험 요약", "한 번 물은 뒤 `--execute --yes`");
  });

  test("3 올리기: 미리보기 → 권리 확인 질문 → --attest-distribution-rights --execute --submit, 같은 멱등 키, 신청만 실패면 submit", () => {
    const step = sectionOf("### 3. 올리기", "### 4. 상태·내리기");
    expect(step).toContain("SKILL.md 하나(이름·버전), 플러그인 폴더(`.claude-plugin/plugin.json`), 이미 있는 플러그인의 새 버전(`--app` + 새 버전)");
    expect(step).toContain("남이 만든 플러그인을 새로 재배포하면 `--third-party` 를 붙여요");
    expect(step).toContain("`plugin publish <경로> --json` 미리보기로 파일·크기·위험 요약을 보여줘요");
    expect(step).toContain("비밀 파일(`.env`·`*.pem`·`*.key`·`id_rsa*`)이 있으면 빼 달라고 해요");
    expect(step).toContain("\"이 플러그인을 배포할 권리가 있음을 확인하고, 올린 뒤 게시 심사를 신청할까요?\"라고 한 번 묻고");
    expect(step).toContain("`--attest-distribution-rights --execute --submit` 로 올려요");
    expect(step).toContain("다시 시도할 때는 같은 `--idempotency-key` 를 써요");
    expect(step).toContain("올리기는 됐는데 신청만 안 됐으면 `plugin submit` 으로 신청만 해요");
    expect(step).toContain("\"결과 알림은 없어요. `plugin manage status` 로 확인해요\"");
    expectOrder(step, "`plugin publish <경로> --json` 미리보기", "배포할 권리가 있음을 확인하고", "--attest-distribution-rights --execute --submit");
  });

  test("4 상태·내리기: headline·next_action, --watch 한 번, delist 는 미리보기 → 한 번 묻고 실행", () => {
    const step = sectionOf("### 4. 상태·내리기", "### 5. 실패하면");
    expect(step).toContain("`plugin manage status --app … --release-version … --json` 으로 먼저 한 번 보고, `headline` 과 `next_action` 을 쉬운 말로 알려줘요");
    expect(step).toContain("업로드 처리·자동 검수 중일 때만 `--watch` 로 한 번(5분) 기다려요");
    expect(step).toContain("사람의 결정을 기다리는 상태면 기다리지 않고 나중에 볼 명령만 알려줘요");
    expect(step).toContain("내리기는 `plugin delist` 미리보기를 보여주고 한 번 물은 뒤 실행해요");
  });

  test("5 실패하면: message_ko·next_action 대로, 키 직접 발급은 next_action 일 때만, 끝나면 pat unset", () => {
    const step = sectionOf("### 5. 실패하면", "## 끝");
    expect(step).toContain("출력의 `message_ko`·`next_action` 대로 해요");
    expect(step).toContain("게시용 키는 CLI 가 준비하고, 직접 발급은 `next_action` 이 그렇게 말할 때만 한 번 물은 뒤 해요");
    expect(step).toContain("`axhub auth pat unset` 으로 내려요");
  });

  test("끝: 한두 줄 결과, 목록은 표, 다음 일은 한 줄", () => {
    const end = sectionOf("## 끝", "## 알아둘 것");
    expect(end).toContain("무엇을 했고 결과가 어떤지 한두 줄로 말해요. 목록은 표로 보여줘요");
    expect(end).toContain("\"승인되면 '설치해줘'라고 하면 돼요.\"");
  });

  test("알아둘 것: headless 는 미리보기까지, 받기≠설치, 검사 안 됨≠없음, servable 일 때만 설치 가능, 심사 용어, 키 숨기기", () => {
    const notes = SKILL.slice(SKILL.indexOf("## 알아둘 것"));
    expect(notes).toContain("질문에 답할 수 없는 실행(headless)이면 목록·받기·상태만 하고, 설치·올리기·내리기는 미리보기까지만 해요");
    expect(notes).toContain("받아 달라는 요청을 설치 허락으로 보지 않아요. 받은 코드를 풀거나 실행하지 않아요");
    expect(notes).toContain("위험 요약이 비어 있으면 \"검사 안 됨\", 조회가 실패하면 \"확인 못 함\"이에요. \"없음\"이라고 하지 않아요");
    expect(notes).toContain("업로드가 끝났거나 심사를 신청한 것은 아직 설치할 수 없어요. `servable` 일 때만 \"설치할 수 있어요\"라고 해요");
    expect(notes).toContain("플러그인 버전 심사는 \"게시 심사\", 앱 공개범위 심사는 \"공개범위 변경 심사\"예요. 섞어 부르지 않아요");
    expect(notes).toContain("`next_action` 의 주소만 넘기고 CLI 로 대신하지 않아요");
    expect(notes).toContain("반려된 버전을 같은 버전으로 다시 신청(`--reason`)하는 건 사용자가 고를 때만 해요");
    expect(notes).toContain("보안 회수는 운영팀이 하는 일이라 문서 안내만 해요");
    expect(notes).toContain("키(token) 값은 화면에 내지 않아요");
  });

  test("옛 판의 경로 찾기·버전 가드·상태표·PAT 절차·콘솔 주소는 다시 들어오지 않아요", () => {
    for (const retired of [
      "bare `axhub` 실패는 미설치가 아니에요",
      "repair-path",
      "0.50.0",
      "axhub update apply",
      "| `takedown`",
      "issue_plugin_pat",
      "auth pat issue",
      "--show-token",
      "/discovery?type=plugin",
      "/console/review",
      "네이티브 선택 UI",
      "review_ready",
      "공개 신청",
    ]) {
      expect(SKILL, retired).not.toContain(retired);
    }
  });

  test("codex 판은 승인 게이트 요약을 받고 본문 전체가 8,000B 안에 들어가요", () => {
    expect(CODEX_BUNDLE).toContain("## 승인 게이트 계약 (요약)");
    expect(CODEX_BUNDLE).toContain("미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요");
    expect(CODEX_BUNDLE).toContain("카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요");
    expect(CODEX_BUNDLE).not.toContain("## 질문 계약 (요약)");
    expect(CODEX_BUNDLE).not.toContain("examples:");
    // 도구 이름은 소문자 인자라 치환되지 않아요.
    expect(CODEX_BUNDLE).toContain("도구(claude 또는 codex)");
    expect(CODEX_BUNDLE).toContain("키(token) 값은 화면에 내지 않아요");
  });
});

describe("plugins 라우팅", () => {
  test("라우팅 fixture 가 마켓플레이스 요청은 plugins, 배포·새 앱·플러그인 자체 업데이트는 각 스킬로 보내요", () => {
    const fixture = JSON.parse(readRepo("tests/routing/plugins-routing.fixture.json")) as {
      cases: Array<{ utterance: string; expected: string }>;
    };
    const expectedFor = (utterance: string): string[] =>
      fixture.cases.filter((entry) => entry.utterance === utterance).map((entry) => entry.expected);
    for (const utterance of [
      "회사 플러그인 목록 보여줘",
      "scaffold 1.23.0 을 Claude 에 설치해줘",
      "scaffold 1.23.0 이 폴더에 받아줘",
      "이 폴더 스킬들 하나의 플러그인으로 올려줘",
      "내 플러그인 심사 어떻게 됐어?",
      "회사 마켓플레이스 연결해줘",
      "플러그인 내려줘",
    ]) {
      expect(expectedFor(utterance), utterance).toEqual(["plugins"]);
    }
    expect(expectedFor("배포해줘")).toEqual(["deploy"]);
    expect(expectedFor("새 앱 만들어줘")).toEqual(["bootstrap"]);
    expect(expectedFor("axhub 플러그인 업데이트해줘")).toEqual(["setup"]);
  });

  test("codex description 이 마켓플레이스 트리거를 들고 배포·새 앱·자체 업데이트를 넘겨요", () => {
    const codexDescriptions = JSON.parse(readRepo("codex-overrides/routing/descriptions.json")) as Record<string, string>;
    const codex = codexDescriptions.plugins ?? "";
    for (const needle of [
      "\"플러그인 목록 보여줘\"",
      "\"OO 플러그인 설치해줘\"",
      "\"OO 1.2.0 받아줘\"",
      "\"이 스킬 플러그인으로 올려줘\"",
      "\"플러그인 심사 어떻게 됐어?\"",
      "\"회사 마켓플레이스 연결해줘\"",
      "\"플러그인 내려줘\"",
      "앱 배포는 deploy, 새 앱은 bootstrap, axhub 플러그인 자체 업데이트는 setup 이에요",
    ]) {
      expect(codex, needle).toContain(needle);
    }
    // 소스 description 의 "Claude Code·Codex" 가 일반 치환으로 "Codex·Codex" 가 되지 않게 override 해요.
    expect(CODEX_BUNDLE).toContain(`description: ${JSON.stringify(codex)}`);
    expect(CODEX_BUNDLE).not.toContain("Codex·Codex");
  });
});

describe("plugins 공개 문서·정책", () => {
  const readme = readRepo("README.md");
  const codexReadme = readRepo("codex-overrides/README.md");
  const policy = readRepo("POLICY.md");
  const codexPolicy = readRepo("codex-overrides/POLICY.md");
  const agentPolicy = readRepo("docs/policy/agent-policy.md");

  test("README·POLICY 는 App Console·Console Review·scoped PAT·게시 심사 계약을 유지해요", () => {
    for (const document of [readme, codexReadme, policy, codexPolicy]) {
      expect(document).toContain("App Console");
      expect(document).toContain("Console Review");
      expect(document).toContain("plugins:read");
      expect(document).toContain("plugin install");
      expect(document).toContain("upload_complete");
      expect(document).toContain("review_submitted");
      expect(document).toContain("axhub plugin submit");
      expect(document).toContain("axhub plugin manage status");
      expect(document).not.toContain("/settings/integrations/plugin");
      expect(document).not.toContain("plugin admin");
      expect(document).not.toContain("review_ready");
      expect(document).not.toContain("--show-token");
      expect(document).not.toContain("references/publish.md");
    }
    for (const document of [policy, codexPolicy]) {
      expect(document).toContain("~/.axhub/plugins/");
      expect(document).toContain("`plugin delist`는 내가 올린 플러그인을 회사 마켓플레이스에서 내려요");
    }
    expect(agentPolicy).toContain("## AP-21 app-backed plugin marketplace");
    expect(agentPolicy).toContain("받아 달라는 요청을 설치 허락으로 보지 않으며");
  });

  test("POLICY 는 설치 상태 경로를 모두 공개해요", () => {
    for (const document of [policy, codexPolicy]) {
      for (const path of [
        "<host>/.install.lock",
        ".marketplace-transaction.json",
        ".marketplace-host-mutating.json",
        ".marketplace-rollback-pending.json",
        ".marketplace-host-installed.json",
        ".marketplace-staging-",
        ".marketplace-backup-",
        "AXHUB_PLUGIN_HOME",
      ]) {
        expect(document, path).toContain(path);
      }
    }
  });
});
