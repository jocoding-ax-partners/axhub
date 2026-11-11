import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

// deploy 는 0 대상 → 1 숙지 → 2 점검 → 3 맞추기 → 4 올리기 → 5 실패하면 → 결과 표 순서의 단일 SKILL.md 예요.
// 2026-11 개편 때 옛 up(저장소 없이 지금 폴더 그대로 올리기)·diagnosis(배포 실패 원인 진단)를 합쳤고, 저장 위치별
// 길 고르기·배포 번호 찾기·기다리기는 CLI `axhub deploy push`(0.62.0, preflight `capabilities.deploy.push`)가 맡아요.
// 옛 판(Desktop 첫 명령·도구 제목 규칙, token-gate, selfhosted push·GitHub·static upload lane, verify 반복 폴링,
// `어디에 배포할까요?` 카드(AP-27), `.axhub/deploy-answers.json`, 진단 6분류, references/)을 잠그던 계약은 그 개편 때
// 의도적으로 뺐어요.

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");
const DEPLOY = readRepo("skills/deploy/SKILL.md");
const CODEX_DEPLOY = readRepo("plugins/axhub-codex/skills/deploy/SKILL.md");
const FRONTMATTER = DEPLOY.slice(0, DEPLOY.indexOf("\n---\n", 4));

const sectionOf = (start: string, end: string): string => {
  const from = DEPLOY.indexOf(start);
  const to = DEPLOY.indexOf(end, from + start.length);
  expect(from, `missing section start: ${start}`).toBeGreaterThanOrEqual(0);
  expect(to, `missing section end: ${end}`).toBeGreaterThan(from);
  return DEPLOY.slice(from, to);
};

// 순서 계약: 앞 문자열이 뒤 문자열보다 먼저 나와야 해요.
const expectOrder = (...needles: string[]): void => {
  const offsets = needles.map((needle) => {
    const offset = DEPLOY.indexOf(needle);
    expect(offset, `missing: ${needle}`).toBeGreaterThanOrEqual(0);
    return offset;
  });
  for (let i = 1; i < offsets.length; i += 1) {
    expect(offsets[i - 1], `${needles[i - 1]} → ${needles[i]}`).toBeLessThan(offsets[i]);
  }
};

describe("deploy skill contract (2026-11 개편판)", () => {
  test("stays one compact SKILL.md that fits the codex 8,000B window", () => {
    // codex 는 SKILL.md 를 파일 앞에서부터 8,000B 만 읽어요 — 통째로 들어가야 절차가 잘리지 않아요.
    expect(Buffer.byteLength(CODEX_DEPLOY, "utf8")).toBeLessThanOrEqual(8_000);
    expect(DEPLOY).not.toContain("references/");
    for (const root of ["skills", "plugins/axhub/skills", "plugins/axhub-codex/skills"]) {
      expect(existsSync(join(REPO_ROOT, root, "deploy", "references")), root).toBe(false);
      // 옛 up·diagnosis 는 deploy 에 합쳐졌어요.
      expect(existsSync(join(REPO_ROOT, root, "up")), root).toBe(false);
      expect(existsSync(join(REPO_ROOT, root, "diagnosis")), root).toBe(false);
    }
  });

  test("frontmatter takes redeploys, as-is folder uploads, and failure questions, and yields first uploads", () => {
    expect(FRONTMATTER).toContain("이미 axhub 에 있는 앱의 바뀐 코드를 배포하고, 실패하면 원인을 봐요");
    expect(FRONTMATTER).toContain("\"이 폴더 그대로 올려줘\"");
    expect(FRONTMATTER).toContain("\"왜 배포 실패했어\"");
    expect(FRONTMATTER).toContain("처음 올리는 폴더는 import, 빈 폴더 새 앱은 bootstrap 이에요");
    expect(FRONTMATTER).toContain('intent: "commit the current folder and deploy"');
    expect(FRONTMATTER).toContain('intent: "diagnose a failed deployment"');
    expect(FRONTMATTER).toContain('intent: "yield to import"');
  });

  test("runs the steps in order 0 → 5 and ends with the result table", () => {
    expectOrder(
      "### 0. 대상",
      "### 1. 숙지",
      "### 2. 점검",
      "### 3. 맞추기",
      "### 4. 올리기",
      "### 5. 실패하면",
      "## 끝",
      "## 알아둘 것",
    );
    expectOrder(
      "axhub plugin-support preflight --json",
      "axhub plugin-support deploy-prep --intent deploy",
      "curl -fsSL <주소>",
      "`axhub deploy check --json`",
      "axhub deploy push --app <앱> --json",
      "`--execute --wait`",
      "axhub deploy verify <deployment_id> --app <앱> --wait --wait-timeout 9m --json",
      "axhub deploy status <deployment_id> --app <앱> --json",
    );
  });

  test("0: gates on deploy.push, picks the app with deploy-prep, and sends first uploads away", () => {
    const target = sectionOf("### 0. 대상", "### 1. 숙지");
    expect(target).toContain("다른 배포 서비스(Vercel 등)를 말했으면 하지 않고 멈춰요");
    expect(target).toContain("안 되면 setup 스킬로 보내고 멈춰요");
    expect(target).toContain("`capabilities.deploy.push` 가 true 가 아니면 CLI 를 업데이트해 달라고 하고 멈춰요");
    expect(target).toContain('axhub plugin-support deploy-prep --intent deploy --user-utterance "<사용자 문장>" --json');
    expect(target).toContain("`bootstrap_plan` 이 있으면 처음 올리는 폴더예요. 코드가 있으면 import, 비어 있으면 bootstrap 으로 보내고 멈춰요");
    expect(target).toContain("\"어느 앱을 배포할까요?\"로 이름 중에서 골라요");
    expect(target).toContain("앱 이름만 말했고 폴더가 비어 있으면 `axhub repo clone <앱> . --json` 으로 받아요");
    // 원인만 물으면 진단 단계만 하고 아무것도 바꾸지 않아요(옛 diagnosis 의 읽기 전용 계약).
    expect(target).toContain("\"왜 실패했어\"처럼 원인만 물으면");
    expect(target).toContain("하고 아무것도 바꾸지 않아요");
  });

  test("1: reads the reference doc verbatim with curl and finds feature docs via llms.txt", () => {
    const learn = sectionOf("### 1. 숙지", "### 2. 점검");
    expect(learn).toContain("`capabilities.readiness.context_url`");
    expect(learn).toContain("`curl -fsSL <주소>` 로 한 번 원문 그대로 받아 읽어요");
    expect(learn).toContain("요약해 주는 웹 읽기 도구는 쓰지 않아요");
    expect(learn).toContain("`https://docs.axhub.ai/llms.txt` 목차에서 그 기능 문서(`.md`)를 찾아");
    expect(DEPLOY).not.toContain("WebFetch");
  });

  test("2: checks changed code with deploy check and a local build, then sorts findings into four", () => {
    const check = sectionOf("### 2. 점검", "### 3. 맞추기");
    expect(check).toContain("`axhub deploy check --json` 과 기준 문서로 지난 배포 뒤 바뀐 코드를 봐요");
    expect(check).toContain("빌드가 있으면 의존성을 설치하고 로컬에서 5분 안에 한 번 돌려 봐요");
    expect(check).toContain("찾은 것을 넷으로 나눠요: 바로 고칠 것, 물을 것, axhub 에서 안 되는 것, 사용자가 직접 할 일");
  });

  test("3: no approval card, irreversible switch is asked, commits per item, env from .env without example values", () => {
    const align = sectionOf("### 3. 맞추기", "### 4. 올리기");
    expect(align).toContain("이미 있는 앱의 배포라 승인 카드는 띄우지 않아요. 물을 것이 있을 때만 모아서 한 번에 물어요");
    expect(align).toContain("\"axhub 에서는 안 돼요\"라고 먼저 말하고 기준 문서의 대안을 보여줘요");
    // 배포 방식 전환은 되돌릴 수 없어서 묻고, 그러자고 할 때만 실행해요.
    expect(align).toContain("\"서버 앱으로 바꾸면 되돌릴 수 없어요\"라고 알리고 물어요");
    expect(align).toContain("바꾸기로 하면 `axhub apps deploy-method --app <앱> --to docker --execute --yes`");
    expect(align.indexOf("되돌릴 수 없어요")).toBeLessThan(align.indexOf("apps deploy-method"));
    expect(align).toContain("고칠 것을 한 항목씩 끝낼 때마다 `배포 전 맞추기: <항목>` 으로 커밋해요");
    expect(align).toContain("`.env*`·키 파일·에이전트 폴더는 커밋하지 않아요");
    expect(align).toContain("`axhub env set --app <앱> --from-dotenv .env --only <이름,…> --execute`");
    expect(align).toContain("값은 채팅으로 받지 않고, 예시 값(`your-`·`xxx` 등)은 등록하지 않고 직접 할 일에 적어요");
    expect(align).toContain("`.env` 의 기존 값은 지우지 않아요");
    expect(align).toContain("두 번 고쳐도 안 되면 고친 데까지 커밋하고 원인을 알린 뒤 멈춰요");
    // 옛 "그대로" 답 저장과 맞춤 알림 문구는 2026-11 개편 때 결과 표로 대신했어요.
    expect(DEPLOY).not.toContain(".axhub/deploy-answers.json");
  });

  test("4: preview first, follow blocked, execute once with --wait, resume with verify instead of re-pushing", () => {
    const push = sectionOf("### 4. 올리기", "### 5. 실패하면");
    expect(push).toContain("`axhub deploy push --app <앱> --json` 으로 미리보기를 받아요. `blocked` 가 있으면 `next` 대로 해요");
    expect(push).toContain("\"지금 폴더 그대로\"라고 해도 커밋해서 올리고 커밋했다고 알려요");
    // CLI 0.63: 저장소 있는 앱은 업로드하지 않아요. 저장소 없는 옛 앱은 Axhub 저장소로 옮기도록 유도해요.
    expect(push).not.toContain("--working-tree");
    expect(push).toContain("저장소 없는 앱이면 Axhub 저장소로 옮길지 묻고");
    expect(push).toContain("이 폴더가 앱 저장소에 안 붙어 있다면 import 로 보내고 멈춰요");
    expect(push).toContain("막힌 게 없으면 같은 명령에 `--execute --wait` 를 붙여 한 번만 실행해요(명령 시간 제한 10분)");
    expect(push).toContain("기다리다 끊기면 실패가 아니에요. 다시 올리지 말고 `axhub deploy verify <deployment_id> --app <앱> --wait --wait-timeout 9m --json` 으로 이어가요");
    expect(push).toContain("(`status` 가 `started`·`running`, exit 6·10)");
    expect(push).toContain("`environment` 가 `staging` 이면 스테이징에만 반영됐고 운영은 심사 뒤라고 알려요");
    expect(DEPLOY).not.toContain("--wait-timeout 10m");
    expect(DEPLOY).not.toContain("axhub deploy create");
  });

  test("5: diagnoses with status, logs, and diagnose, masks secrets, asks before fixing, rolls back only on request", () => {
    const failure = sectionOf("### 5. 실패하면", "## 끝");
    expect(failure).toContain("`axhub deploy status <deployment_id> --app <앱> --json` 으로 실패 단계와 시각을 보고");
    expect(failure).toContain("`axhub deploy logs --app <앱> --since <시작> --until <끝> --json --limit 100`");
    expect(failure).toContain("`axhub --json deploy diagnose <앱>`");
    expect(failure).toContain("원인을 쉬운 말로 설명해요");
    // 11-11 교육(gbgh): AI 가 deploy push 밖에서 git clone·fetch 를 바로 해 403 이 났어요. 직접 git 을 쓸 땐 인증부터 해요.
    expect(DEPLOY).toContain("Axhub 저장소에 git 명령(clone·pull 등)을 직접 쓰려면 `axhub git setup --json` 부터 해요");
    expect(CODEX_DEPLOY).toContain("`axhub git setup --json` 부터 해요");
    expect(failure).toContain("로그의 비밀값·토큰은 가려요");
    expect(failure).toContain("코드로 고칠 수 있으면 고칠지 묻고, 그러자고 하면 2번부터 한 번 더 해요");
    expect(failure).toContain("롤백·이전 배포 다시 올리기는 사용자가 말할 때만 해요");
    expect(DEPLOY).not.toContain("deployment_diagnosis");
  });

  test("ends with the result table and never marks unchecked items done", () => {
    const end = sectionOf("## 끝", "## 알아둘 것");
    expect(end).toContain("| 항목 | 상태 |");
    for (const row of ["| 앱 |", "| 주소 |", "| 맞춘 것 |", "| 배포 |", "| 직접 할 일 |"]) {
      expect(end, row).toContain(row);
    }
    expect(end).toContain("| 주소 | `url` (스테이징이면 운영 반영은 심사 뒤) |");
    expect(end).toContain("| 배포 | 성공 · 진행 중 · 실패와 원인 |");
    expect(end).toContain("확인하지 못한 항목은 됐다고 표시하지 않아요");
  });

  test("headless, sandbox, CLI errors, and visible copy stay plain", () => {
    const notes = DEPLOY.slice(DEPLOY.indexOf("## 알아둘 것"));
    expect(notes).toContain("질문에 답할 수 없는 실행(headless)이면 미리보기와 요약까지만 하고 바꾸지 않고 멈춰요");
    expect(notes).toContain("에이전트 샌드박스 안이면 Axhub 저장소로 올리는 실행은 처음부터 권한 상승으로 해요");
    expect(notes).toContain("`local.write_denied` 면 `recovery_action` 대로 해요");
    expect(notes).toContain("`message_ko`·`recovery_action`·`next` 를 쉬운 말로 설명하고 어떻게 할지 물어요");
    expect(notes).toContain("같은 배포를 새로 만들어 다시 시도하지 않아요");
    expect(notes).toContain("사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요");
  });

  test("codex bundle keeps the question contract and the whole body", () => {
    expect(CODEX_DEPLOY).toContain("미리 넣어 둔 문구·유사 표현·무응답은 답이 아니에요");
    expect(CODEX_DEPLOY).toContain("카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요");
    expect(CODEX_DEPLOY).toContain("`deploy push --execute --wait`·`deploy verify --wait` 가 yield 되면 같은 명령을 다시 실행하지 말고");
    // 재배포라 새로 만드는 일의 승인 게이트 요약은 받지 않아요.
    expect(CODEX_DEPLOY).not.toContain("## 승인 게이트 계약 (요약)");
    expect(CODEX_DEPLOY).toContain("| 직접 할 일 |");
    expect(CODEX_DEPLOY).toContain("사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요");
    expect(CODEX_DEPLOY).not.toContain("examples:");
  });
});

// 옛 up·diagnosis 를 부르던 스킬들은 이제 deploy 로 보내요.
describe("deploy 가 옛 up·diagnosis 자리를 맡아요", () => {
  test("다른 스킬이 실패 원인 진단을 deploy 로 보내요", () => {
    expect(readRepo("skills/bootstrap/SKILL.md")).toContain("배포가 실패하면 deploy 스킬로 원인을 봐요");
    expect(readRepo("skills/import/SKILL.md")).toContain("배포가 실패하면 deploy 스킬로 원인을 봐요");
    expect(readRepo("skills/operate/SKILL.md")).toContain("배포·배포 실패 원인은 deploy");
    for (const skill of ["setup", "bootstrap", "plugins", "import", "development", "operate"]) {
      const body = readRepo(`skills/${skill}/SKILL.md`);
      expect(body, skill).not.toMatch(/\bdiagnosis\b/);
      expect(body, skill).not.toContain("`up`");
      expect(body, skill).not.toContain("deploy/references/");
    }
  });

  test("라우팅 fixture 가 옛 up·diagnosis 발화를 deploy 로 보내요", () => {
    const fixture = JSON.parse(readRepo("tests/routing/deploy-routing.fixture.json")) as {
      cases: Array<{ utterance: string; expected: string; context?: string }>;
    };
    const expectedFor = (utterance: string): string[] =>
      fixture.cases.filter((entry) => entry.utterance === utterance).map((entry) => entry.expected);
    for (const utterance of [
      "이 폴더 그대로 올려줘",
      "저장소 없이 이 폴더 그대로 올려줘",
      "왜 배포 실패했어",
      "배포 실패 원인 진단해줘",
      "why did my deploy fail?",
      "diagnose deployment failure",
    ]) {
      expect(expectedFor(utterance), utterance).toContain("deploy");
    }
    expect(existsSync(join(REPO_ROOT, "tests", "routing", "up-routing.fixture.json"))).toBe(false);
    expect(existsSync(join(REPO_ROOT, "tests", "routing", "diagnosis-routing.fixture.json"))).toBe(false);
  });
});
