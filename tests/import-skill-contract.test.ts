import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

// import 는 0 올릴 수 있나 → 1 숙지 → 2 비교 → 3 안내와 확인 → 4 고치기 → 5 다시 비교 → 6 올리기 →
// 결과 표 순서의 단일 SKILL.md 예요. 옛 판(import/v1 envelope 필드·enum 검증 목록, Claude Desktop
// cd·도구 제목 규칙, 실패 유형 표, `.axhub/import-resume.json` 이어 하기, 배포 방식 카드(AP-28),
// 첫 문장 고정 문구, references/)을 잠그던 계약은 2026-11 개편 때 의도적으로 뺐어요.

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");
const IMPORT = readRepo("skills/import/SKILL.md");
const CODEX_IMPORT = readRepo("plugins/axhub-codex/skills/import/SKILL.md");
const FRONTMATTER = IMPORT.slice(0, IMPORT.indexOf("\n---\n", 4));

const sectionOf = (start: string, end: string): string => {
  const from = IMPORT.indexOf(start);
  const to = IMPORT.indexOf(end, from + start.length);
  expect(from, `missing section start: ${start}`).toBeGreaterThanOrEqual(0);
  expect(to, `missing section end: ${end}`).toBeGreaterThan(from);
  return IMPORT.slice(from, to);
};

// 순서 계약: 앞 문자열이 뒤 문자열보다 먼저 나와야 해요.
const expectOrder = (...needles: string[]): void => {
  const offsets = needles.map((needle) => {
    const offset = IMPORT.indexOf(needle);
    expect(offset, `missing: ${needle}`).toBeGreaterThanOrEqual(0);
    return offset;
  });
  for (let i = 1; i < offsets.length; i += 1) {
    expect(offsets[i - 1], `${needles[i - 1]} → ${needles[i]}`).toBeLessThan(offsets[i]);
  }
};

describe("import skill contract (2026-11 개편판)", () => {
  test("stays one compact SKILL.md that fits the codex 8,000B window", () => {
    // codex 는 SKILL.md 를 파일 앞에서부터 8,000B 만 읽어요 — 통째로 들어가야 절차가 잘리지 않아요.
    expect(Buffer.byteLength(CODEX_IMPORT, "utf8")).toBeLessThanOrEqual(8_000);
    expect(IMPORT).not.toContain("references/");
    for (const root of ["skills", "plugins/axhub/skills", "plugins/axhub-codex/skills"]) {
      expect(existsSync(join(REPO_ROOT, root, "import", "references")), root).toBe(false);
    }
  });

  test("frontmatter routes existing code here and yields new apps and redeploys", () => {
    expect(FRONTMATTER).toContain("다른 곳에서 만든 앱(이미 코드가 있는 폴더)을 axhub 에 맞게 고쳐서 처음 올리고 첫 배포까지 해요");
    expect(FRONTMATTER).toContain("빈 폴더 새 앱은 bootstrap, 이미 올린 앱 재배포는 deploy 예요");
    expect(FRONTMATTER).toContain("intent: \"yield to bootstrap\"");
  });

  test("runs the steps in order 0 → 6 and ends with the result table", () => {
    expectOrder(
      "### 0. 올릴 수 있나",
      "### 1. 숙지",
      "### 2. 비교",
      "### 3. 안내와 확인",
      "### 4. 고치기",
      "### 5. 다시 비교",
      "### 6. 올리기",
      "## 끝",
    );
    expectOrder(
      "axhub plugin-support preflight --json",
      "axhub plugin-support tenant-resolve --json",
      "--mode preview",
      "지금 이대로 고쳐서 axhub 에 올릴까요?",
      "--mode execute --approved --commit-manifest --verify-wait none",
      "axhub deploy verify <deployment_id> --app <앱> --wait --wait-timeout 9m --json",
    );
  });

  test("0: checks the CLI, workspace, and company policy like bootstrap before touching code", () => {
    const gate = sectionOf("### 0. 올릴 수 있나", "### 1. 숙지");
    expect(gate).toContain("폴더가 비어 있으면 bootstrap 으로 보내고 멈춰요");
    expect(gate).toContain("안 되면 setup 스킬로 보내고 멈춰요");
    expect(gate).toContain("`capabilities.import.supported`·`capabilities.self_hosted_git.git_policy_decision`");
    expect(gate).toContain("\"앱을 어느 작업공간에 올릴까요?\"");
    expect(gate).toContain("앱 서비스 권한·저장 위치·GitHub 연결은 bootstrap 스킬 3·4단계와 똑같이 해요");
  });

  test("1: reads the reference doc verbatim with curl and finds feature docs via llms.txt", () => {
    const learn = sectionOf("### 1. 숙지", "### 2. 비교");
    expect(learn).toContain("`capabilities.readiness.context_url`");
    expect(learn).toContain("`curl -fsSL <주소>` 로 한 번 원문 그대로 받아 읽어요");
    expect(learn).toContain("요약해 주는 웹 읽기 도구는 쓰지 않아요");
    expect(learn).toContain("`https://docs.axhub.ai/llms.txt` 목차에서 그 기능 문서를 찾아");
    expect(IMPORT).not.toContain("WebFetch");
  });

  test("2: compares with the CLI preview, deploy check, outside services, and a local build", () => {
    const compare = sectionOf("### 2. 비교", "### 3. 안내와 확인");
    expect(compare).toContain("axhub --json plugin-support import --mode preview --tenant <작업공간> --git-choice <key>");
    expect(compare).toContain("`--github-owner <계정>`");
    expect(compare).toContain("`axhub deploy check --json`");
    expect(compare).toContain("Supabase·Firebase 같은 외부 DB·로그인");
    // 외부 DB·로그인은 그대로 연결하지 않고 axhub 것으로 바꾸기를 추천하며 물어요.
    expect(compare).toContain("외부 DB·로그인은 그대로 연결하지 않고 axhub 것으로 바꾸기(추천)·그대로 쓰기를 물어요");
    expect(compare).toContain("로컬에서 5분 안에 한 번 돌려 봐요");
    expect(compare).toContain("바로 고칠 것, 물을 것, axhub 에서 안 되는 것, 사용자가 직접 할 일");
  });

  test("3: asks once, never takes values over chat, and treats an empty answer as no approval", () => {
    const confirm = sectionOf("### 3. 안내와 확인", "### 4. 고치기");
    expect(confirm).toContain("한 화면에 보여줘요");
    expect(confirm).toContain("값을 채팅으로 받지 않고 `.env` 에 넣어 달라고 해요");
    expect(confirm).toContain("물을 것을 모아 한 번에 묻고 \"지금 이대로 고쳐서 axhub 에 올릴까요?\"(올리기 / 그만두기)를 물어요");
    expect(confirm).toContain("\"바로 해줘\"라는 말은 승인이 아니에요");
    expect(confirm).toContain("선택 카드에 빈 답변이 돌아오면 답하지 않은 거예요");
    expect(confirm).toContain("카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요");
    // 옛 통합 승인의 4옵션은 올리기/그만두기 둘로 줄었어요.
    expect(IMPORT).not.toContain("설정은 나중에 올리기");
    expect(IMPORT).not.toContain("먼저 고칠게요");
  });

  test("4: fixes by the reference doc and commits each chunk so a rerun resumes", () => {
    const fix = sectionOf("### 4. 고치기", "### 5. 다시 비교");
    expect(fix).toContain("git 저장소가 아니면 `git init` 부터 해요");
    expect(fix).toContain("한 덩어리를 고칠 때마다 `배포 전 맞추기` 로 커밋해서, 끊겨도 다시 부르면 이어가요");
    expect(fix).toContain("`.env*`·키 파일·에이전트 폴더는 커밋하지 않아요");
    expect(fix).toContain("`axhub.yaml` 을 만들고(`name` 은 앱 주소 이름) `axhub deploy --explain --json` 으로 확인해요");
    expect(fix).toContain("못 바꾸는 건 지어내지 않고 알려요");
    expect(IMPORT).not.toContain("import-resume");
  });

  test("5: re-compares and only uploads after it passes", () => {
    const recheck = sectionOf("### 5. 다시 비교", "### 6. 올리기");
    expect(recheck).toContain("`axhub deploy check --json` 과 로컬 빌드를 다시 돌려요");
    expect(recheck).toContain("통과해야 올려요");
    expect(recheck).toContain("두 번 고쳐도 안 되면 고친 데까지 커밋하고 원인을 알린 뒤 멈춰요");
  });

  test("6: executes once, waits with one bounded verify, and exit 6·10 is not a failure", () => {
    const upload = sectionOf("### 6. 올리기", "## 끝");
    expect(upload).toContain("`--mode execute --approved --commit-manifest --verify-wait none` 을 붙여 한 번만 실행해요");
    expect(upload).toContain("`--env-from-dotenv .env --env-only <이름,…>`");
    expect(upload).toContain("`axhub deploy verify <deployment_id> --app <앱> --wait --wait-timeout 9m --json` 으로 확인해요(명령 시간 제한 10분)");
    expect(upload).toContain("기다리다 끊기면 다시 실행하지 말고 같은 확인 명령으로 이어가요");
    expect(upload).toContain("시간 안에 안 끝난 건(exit 6·10) 실패가 아니에요");
    expect(upload).toContain("배포가 실패하면 diagnosis 스킬로 원인을 봐요");
    expect(IMPORT).not.toContain("--wait-timeout 10m");
  });

  test("ends with the result table including what the user must do", () => {
    const end = IMPORT.slice(IMPORT.indexOf("## 끝"), IMPORT.indexOf("## 알아둘 것"));
    expect(end).toContain("| 항목 | 상태 |");
    for (const row of ["| 앱 |", "| 주소 |", "| 저장 위치 |", "| 맞춘 것 |", "| 배포 |"]) {
      expect(end, row).toContain(row);
    }
    expect(end).toContain("| 직접 할 일 | 외부 로그인 허용 주소에 새 주소 넣기 등, 없으면 없음 |");
    expect(end).toContain("(스테이징이면 운영 반영은 심사 뒤)");
    expect(end).toContain("확인하지 못한 항목은 됐다고 표시하지 않아요");
    expect(IMPORT).not.toContain("다음에 할 일은 추천하지 않아요");
    expect(IMPORT).toContain("찾은 것을 넷으로 나눠요: 바로 고칠 것, 물을 것, axhub 에서 안 되는 것, 사용자가 직접 할 일");
    expect(IMPORT).toContain("\"axhub 에서는 안 돼요\"라고 먼저 분명히 말하고 기준 문서의 대안을 보여줘요");
  });

  test("headless stops at the preview without changing anything", () => {
    expect(IMPORT).toContain("질문에 답할 수 없는 실행(headless)이면 `--headless` 미리보기와 요약까지만 하고 바꾸지 않고 멈춰요");
  });

  test("sandbox, CLI errors, and visible copy stay plain", () => {
    expect(IMPORT).toContain("처음부터 권한 상승으로 실행해요");
    expect(IMPORT).toContain("`local.write_denied`");
    expect(IMPORT).toContain("`message_ko`·`recovery_action` 을 쉬운 말로 설명하고 어떻게 할지 물어요");
    expect(IMPORT).toContain("사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요");
  });

  test("codex bundle keeps the approval gate summary and the whole body", () => {
    expect(CODEX_IMPORT).toContain("미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요");
    expect(CODEX_IMPORT).toContain("지금 이대로 고쳐서 axhub 에 올릴까요?");
    expect(CODEX_IMPORT).toContain("| 직접 할 일 |");
    expect(CODEX_IMPORT).not.toContain("examples:");
  });
});

// spec 681 FR-P03 (AP-30): 앱 기능을 만드는 development 도 같은 기준 문서로 6가지를 처음부터 피해요.
describe("배포 기준 문서 (AP-30)", () => {
  const development = readRepo("skills/development/SKILL.md");

  test("development 는 코드를 쓰기 전에 같은 기준 문서로 6가지를 피해요", () => {
    const rule = development.indexOf("7.5. **AXHub 배포 기준 (AP-30).**");
    expect(rule).toBeGreaterThan(-1);
    expect(development.indexOf("9. **코드 생성")).toBeGreaterThan(rule);
    expect(development).toContain("`curl -fsSL` 로 원문 그대로 한 번만 읽고");
    expect(development).toContain("SQLite·파일 대신 AXHub DB");
  });
});
