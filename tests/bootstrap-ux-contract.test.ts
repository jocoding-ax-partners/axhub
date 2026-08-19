import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { HOST_EXPECTATIONS } from "./fixtures/host-expectations";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");
// host 결합 기대값은 Claude lane fixture 를 참조해요 — codex 열은 후속 유닛이
// tests/fixtures/host-expectations.ts 에 추가해요.
const CLAUDE = HOST_EXPECTATIONS.claude;
const readBootstrap = (): string => readRepo("skills/bootstrap/SKILL.md");
const readBootstrapContract = (): string =>
  [
    readBootstrap(),
    readRepo("skills/bootstrap/references/templates-and-github.md"),
    readRepo("skills/bootstrap/references/bootstrap-and-local.md"),
    readRepo("skills/bootstrap/references/resume-and-tenant.md"),
    readRepo("skills/bootstrap/references/errors-and-followups.md"),
  ].join("\n");

describe("bootstrap desktop UX contract", () => {
  test("keeps the invoked bootstrap entrypoint compact and action-first", () => {
    const bootstrap = readBootstrap();

    // AP-16 폴링 예산 문구 추가로 19_500 → 20_000 상향.
    // AP-17 CLI 경로 계약 + AP-18 device flow 코드 선노출로 20_000 → 22_400 상향 —
    // 둘 다 fresh path 에서 reference 를 읽지 않기 때문에 본문에 있어야 도달해요.
    // AP-16 대기 수단 우선(`deploy verify --wait` 단일 호출)로 22_400 → 23_000 상향 —
    // 연타 폴링 UX 실패를 막는 계약이라 fresh path 본문에 있어야 해요.
    // linked-account-first 서사(6단계 "인증 단계 없음" + 9단계 device flow fallback 전용)로
    // 23_000 → 23_500 상향 — 연동을 마친 사용자에겐 이게 fresh path 의 기본 서사이고,
    // reference 로 빼면 "정상 fresh path 에서는 reference 파일을 읽지 않아요" 규칙 때문에 도달하지 않아요.
    // GitHub 차단 시 로컬 소스 배포(12단계) 트리거로 23_500 → 24_000 상향 —
    // 절차·명령·NEVER 는 전부 references/github-blocked-local-deploy.md 로 뺐고
    // 본문에는 "언제 그 파일을 여는가" 만 남겼어요. 이 트리거까지 reference 로
    // 빼면 분기 자체가 도달하지 않아요.
    // 24_000 → 26_500: 그 절차 분리가 실제로 실패했어요. reference 는 plugin cache
    // 라 workspace 밖이고, Desktop 은 읽을 때 권한 프롬프트를 띄우는데 9.1 규칙은
    // 그 프롬프트를 생략하라고 해요 — 즉 트리거에 닿아도 절차를 못 읽어요. 실패
    // 지점(6·9.1·10)에 12단계 포인터를 넣고, 절차는 reference 없이도 완결되도록
    // 본문으로 되돌렸어요. 실행에 필요한 지시는 reference 로 빼지 않아요.
    // 26_500 → 27_000: 폴백의 placeholder 치환 단계 추가. 서버 bootstrap 은
    // push 전에 치환하지만 폴백은 clone 그대로라, 빠뜨리면 앱이 '{{API_BASE}}'
    // 리터럴로 API 를 불러 로그인·데이터 연동만 조용히 죽어요. 실행 필수
    // 지시라 본문에 둬요.
    // 27_000 → 27_500: scaffold 양보 라우팅 추가 — Desktop 실측에서 "내 계정에
    // 레포 만들어서 새 앱" 이 bootstrap 으로 라우팅됐어요. bootstrap 의 배타
    // 클레임("creation path 는 saga 하나뿐")이 소유 지목 문장까지 삼켜서,
    // frontmatter 와 Scope 에 scaffold 양보를 명시해요. 라우팅은 본문에 있어야
    // 작동해요.
    // (plugin:budget 의 per-skill 35k · 총합 210k 는 별도 gate)
    expect(Buffer.byteLength(bootstrap, "utf8")).toBeLessThanOrEqual(27_500);
    expect(bootstrap).toContain("## Fast Start");
    expect(bootstrap).toContain("Do not explain the skill match");
    expect(bootstrap).toContain("do not mention axhub:bootstrap in chat");
    expect(bootstrap).toContain("start the visible response with a Korean progress sentence");
    expect(bootstrap).toContain("아래 순서대로 CLI 확인과 템플릿 질문까지 바로 진행");
    expect(bootstrap).toContain("`진행해줘라고 말해` 같은 일반 안내만 남기고 멈추지 않아요");
    expect(bootstrap).toContain("`Using axhub:bootstrap skill`");
    expect(bootstrap).toContain("`matches new app + deploy request`");
    expect(bootstrap).toContain("`axhub의 새 앱 생성 스킬`");
    expect(bootstrap).toContain("`스킬을 사용하겠습니다`");
    expect(bootstrap).toContain(CLAUDE.bootstrapSkill.nativeBadgeNoRepeat);
    expect(bootstrap).toContain("CLI 명령이 하나도 실행되지 않았다면 fresh path 를 그대로 시작해요");
    expect(bootstrap.indexOf("## Fast Start")).toBeLessThan(bootstrap.indexOf("## Reference Loading Policy"));
  });

  test("keeps bootstrap discovery focused on empty-folder creation", () => {
    const bootstrap = readBootstrap();
    const frontmatter = bootstrap.slice(0, bootstrap.indexOf("---", 4));

    expect(bootstrap).toContain("Use this skill only in an empty folder");
    expect(frontmatter).toContain("Use this skill only in an empty folder to create a brand-new axhub template app");
    expect(frontmatter).toContain("Positive triggers only when the user asks to create/start/init a new app");
    expect(frontmatter).toContain("새 앱 만들어줘");
    expect(frontmatter).toContain("처음부터 앱 만들어줘");
    expect(frontmatter).toContain("진행 중이던 axhub 앱 만들기/배포 상태 이어서 확인");
    expect(frontmatter).not.toContain("Please make my first app");
    expect(frontmatter).not.toContain("deploy it to the internet");
    expect(frontmatter).not.toContain("put it online");
    expect(bootstrap).toContain("아래 순서대로 CLI 확인과 템플릿 질문까지 바로 진행");
    expect(bootstrap).toContain("If you need an app name or template, choose a reasonable one yourself");
    expect(bootstrap).toContain("deploy it for real");
    expect(bootstrap).toContain("템플릿 선택 카드, 앱 이름 확인 카드, dry-run 뒤 `진행` 확인은 절대 생략하지 않아요");
  });

  test("refuses existing backend app imports before bootstrap preflight", () => {
    const bootstrap = readBootstrap();
    const scopeSection = bootstrap.slice(bootstrap.indexOf("## Scope"), bootstrap.indexOf("## Reference Loading Policy"));

    expect(bootstrap).toContain("never use for \"기존 앱\", \"기존 Express 서버 앱\", \"이미 만든 앱\", \"작업 폴더는 /path\"");
    expect(scopeSection).toContain("사용자 발화에 `기존`, `이미 만든`, `작업 폴더`, `이 폴더`");
    expect(scopeSection).toContain("`Express`, `Fastify`, `Nest`, `FastAPI`, `Flask`, `Django`, `Rails`, `Go 서버`, `Dockerfile`");
    expect(scopeSection).toContain("기존 소스가 있음을 뜻하는 단서");
    expect(scopeSection).toContain("CLI guard, preflight, 템플릿 목록 확인을 실행하기 전에 즉시 import 경계로 양보");
    expect(scopeSection).toContain("chat 본문에서 `/axhub:bootstrap` 또는 bootstrap 선택 이유를 설명하지 않아요");
  });

  test("hides internal routing labels from users", () => {
    const bootstrap = readBootstrap();
    const contract = readBootstrapContract();

    expect(bootstrap).toContain("내부 라벨 노출 금지");
    expect(bootstrap).toContain("`axhub:bootstrap 스킬 호출한다`");
    expect(contract).toContain("`Folder near empty`");
    expect(contract).toContain("`Tenanting`");
    expect(contract).toContain("`Using axhub:bootstrap skill`");
    expect(contract).toContain("`matches new app + deploy request`");
    expect(contract).toContain("`axhub의 새 앱 생성 스킬`");
    expect(contract).toContain("`스킬을 사용하겠습니다`");
    expect(contract).toContain("`Bootstraping`");
    expect(contract).toContain("`Idempotencying key`");
    expect(contract).toContain("`saga 실행`");
    expect(contract).toContain("`Saga 완료`");
    expect(contract).toContain("`GitHubed repo`");
    expect(contract).toContain("`DB 선언된 템플릿`");
    expect(contract).toContain("`development 단계`");
    expect(bootstrap).toContain("Tool/Bash 제목은 한국어 명사구로 쓰고");
    expect(bootstrap).toContain("제품명·명령어·영어 단어에 `ing`/`ed` 를 붙인 제목");
    expect(bootstrap).toContain("반드시 한글로 시작해요");
    expect(contract).toContain("`tenanting 확인`, `tenant 확인`, `테넌트 확인`");
    expect(bootstrap).toContain("`실행 중 명령`");
    expect(bootstrap).toContain("`저장소 계정 확인`");
    expect(bootstrap).toContain("`앱 이름 확인`");
    expect(bootstrap).toContain("`계정 인증 시작`");
    expect(bootstrap).toContain("`인증 확인`");
    expect(bootstrap).toContain("스킬 선택 이유");
    expect(bootstrap).toContain("route label");
    expect(bootstrap).toContain("첫 visible 응답은 반드시");
  });

  test("keeps bootstrap checks portable without rtk or shell-wrapper preflight", () => {
    const bootstrap = readBootstrap();
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");
    const guardSection = bootstrap.slice(
      bootstrap.indexOf("### 1. CLI Guard"),
      bootstrap.indexOf("### 2. Resume And Workspace"),
    );

    expect(bootstrap).toContain("`rtk` 같은 개발자 전용 CLI 래퍼는 이 skill 에서 절대 쓰지 않아요");
    expect(bootstrap).toContain("`pwd`, `ls`, `find`, `cat`, `curl` 같은 generic shell probe");
    expect(localReference).toContain("Never prefix with `rtk`");
    expect(resumeReference).toContain("no `rtk`, no generic `ls`/`pwd` probes");
    expect(guardSection).toContain("axhub plugin-support preflight --json");
    expect(guardSection).not.toContain("command -v axhub");
    expect(guardSection).not.toContain("PREFLIGHT_JSON=");
    expect(guardSection).not.toContain("PLUGIN_VER=");
    expect(guardSection).not.toContain("find \"$STAMP\"");
  });

  test("hydrates Claude Desktop metadata-only folders without clone probes", () => {
    const bootstrap = readBootstrap();
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const cloneSection = bootstrap.slice(
      bootstrap.indexOf("### 10. Clone And Manifest"),
      bootstrap.indexOf("### 11. Result"),
    );

    expect(cloneSection).toContain("`.omc/` 같은 Desktop 메타데이터");
    expect(cloneSection).toContain("`git clone ... .` 는 쓰지 않아요");
    expect(cloneSection).toContain("git -C <target> init -q -b main");
    expect(cloneSection).toContain("remote set-url origin https://github.com/<repo>.git");
    expect(cloneSection).toContain("fetch origin main --quiet --depth=1");
    expect(cloneSection).toContain("reset --hard FETCH_HEAD");
    expect(cloneSection).toContain("target 채운 뒤 추가 `rtk ls`, `ls`, `find`, `cat`");
    expect(cloneSection).toContain("clone/hydrate 명령 안에서는 raw `git`만 써요");
    expect(cloneSection).toContain("`rtk git`, `grep`, `cut`, `awk`, `sed`");
    expect(cloneSection).not.toContain("cd /absolute/target && git clone");
    expect(localReference).toContain(CLAUDE.bootstrapSkill.desktopMetadataFolders);
    expect(localReference).toContain("Do not run `git clone ... .`");
    expect(localReference).toContain("leads to extra `rtk ls -la` probes");
  });

  test("keeps the common fresh bootstrap path out of plugin-cache reference reads and shell tenant writes", () => {
    const bootstrap = readBootstrap();
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");

    expect(bootstrap).toContain("정상 fresh path 에서는 reference 파일을 읽지 않아요");
    expect(bootstrap).toContain("선택한 값은 `.axhub/state/tenant.json` 같은 로컬 파일로 저장하지 않아요");
    expect(bootstrap).toContain("이 본문만으로 CLI guard, 작업공간 선택, template/app-name 질문");
    expect(bootstrap).toContain("execute/status/verify/result 까지 진행");
    expect(bootstrap).not.toContain("registry 설명과 AskUserQuestion shape 는 `references/templates-and-github.md` 를 읽어요");
    expect(bootstrap).not.toContain("references/bootstrap-and-local.md");
    expect(bootstrap).not.toContain("../deploy/references/session-carryover.md");
    expect(resumeReference).toContain(CLAUDE.bootstrapSkill.noTenantFileWrite);
    expect(resumeReference).not.toContain("TENANT_CACHE=");
    expect(resumeReference).not.toContain("mkdir -p \"$(dirname \"$TENANT_CACHE\")\"");
    expect(resumeReference).not.toContain("date +%s");
  });

  test("keeps desktop failure recovery inline and axhub-only", () => {
    const bootstrap = readBootstrap();

    expect(bootstrap).toContain("Desktop Error Recovery");
    expect(bootstrap).toContain("workspace 밖 plugin cache reference 읽기 권한 프롬프트");
    expect(bootstrap).toContain("읽지 않아요");
    expect(bootstrap).toContain("plugin cache 파일 읽기를 복구 조건으로 삼지 않아요");
    expect(bootstrap).toContain("복구 명령도 `rtk`, `curl`, `pwd`, `ls`, `find`, `cat` 같은 generic probe 로 빠지지 않아요");
    expect(bootstrap).toContain("`axhub` CLI 상태 명령만 써요");
    expect(bootstrap).toContain("axhub apps bootstrap-status");
    expect(bootstrap).toContain("axhub deploy status <deployment-id>");
    // AP-16 대기 수단 우선: status 연타 폴링 대신 CLI 내부 대기 단일 호출.
    expect(bootstrap).toContain("--wait --wait-interval 20s --wait-timeout 10m --json");
    expect(bootstrap).toContain("연타 폴링은 UX 실패예요");
    expect(bootstrap).toContain("axhub deploy verify <deployment-id> --app <app>");
    expect(bootstrap).toContain("재시도는 최대 1회예요");
    expect(bootstrap).toContain("새 앱을 다시 만들지 않아요");
  });

  test("keeps tenant internals out of desktop-visible workspace wording", () => {
    const bootstrap = readBootstrap();
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");

    expect(bootstrap).toContain("Tool 제목은 `작업공간 확인` 또는 `앱 설정 확인`을 써요");
    expect(bootstrap).toContain("사용자에게는 tenant/테넌트라고 말하지 말고 `작업공간`이라고 말해요");
    expect(bootstrap).toContain("`새 앱을 어느 작업공간에 만들까요?`");
    expect(resumeReference).toContain("visible tool title for this command must be `앱 설정 확인`");
    expect(resumeReference).toContain("User-facing text must call these `작업공간`, not `tenant` or `테넌트`");
    expect(resumeReference).toContain("\"question\": \"새 앱을 어느 작업공간에 만들까요?\"");
    expect(resumeReference).toContain("\"header\": \"작업공간\"");
    expect(resumeReference).not.toContain("\"question\": \"어떤 tenant 로 진행할까요?\"");
    expect(resumeReference).not.toContain("\"header\": \"Tenant\"");
  });

  test("requires preview confirmation before execute even for direct deploy requests", () => {
    const bootstrap = readBootstrap();
    const reference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");

    expect(bootstrap).toContain("미리보기 뒤 확인 필수");
    expect(bootstrap).toContain("그 말은 목표이지 execute 승인 토큰이 아니에요");
    expect(bootstrap).toContain("`deploy it for real`");
    expect(bootstrap).toContain("추천 허용일 뿐이에요");
    expect(bootstrap).toContain("정확히 `지금 만들고 배포까지 진행할까요?` 질문과 `진행`/`취소` 선택지");
    expect(bootstrap).toContain("질문·선택지·설명은 의역하거나 새로 만들지 않아요");
    expect(bootstrap).toContain("사용자가 `진행`을 고른 뒤에만 `--execute` 를 호출해요");
    expect(reference).toContain("treat that as the user's goal, not as execute approval");
    expect(reference).toContain('"question": "지금 만들고 배포까지 진행할까요?"');
    expect(reference).toContain('"label": "진행", "value": "execute"');
    expect(reference).toContain('"label": "취소", "value": "cancel"');
    expect(reference).toContain("Use exactly this question, labels, values, and descriptions");
    expect(reference).toContain("Do not paraphrase the question or invent new Korean option labels/descriptions");
  });

  test("asks for template when the user only gives a generic app category", () => {
    const bootstrap = readBootstrap();
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");

    expect(bootstrap).toContain("일반 장르·기능 단어는 exact template 선택이 아니에요");
    expect(bootstrap).toContain("template 선택을 native Question/AskUserQuestion card 로 먼저 물어요");
    expect(bootstrap).toContain("card 가 렌더링되지 않거나 선택지가 보이지 않는 경우에만");
    expect(bootstrap).toContain("`preorder`");
    expect(bootstrap).toContain("추천 순서를 정하는 근거일 뿐 선택 확정이 아니며");
    expect(bootstrap).toContain("`--template ... --dry-run` 은 템플릿 질문 답변을 받은 뒤에만 실행해요");
    expect(templateReference).toContain("use a native Question/AskUserQuestion card first for backend template selection");
    expect(templateReference).toContain("text fallback only when the card does not render choices");
    expect(templateReference).toContain("번호나 템플릿 이름으로 답해 주세요.");
    expect(templateReference).toContain("Generic category or feature words");
    expect(templateReference).toContain("are not exact template choices");
    expect(templateReference).toContain("Those words can make Next.js the recommended first option");
    expect(templateReference).toContain("they never finalize `--template`");
    expect(bootstrap).toContain("`recommend the best option` 처럼 말해도 그 말은 추천을 원한다는 뜻");
    expect(bootstrap).toContain("`choose whatever is reasonable`");
    expect(bootstrap).toContain("`pick the best template/name`");
    expect(bootstrap).toContain("반드시 `어떤 템플릿으로 시작할까요?` 질문을 보여주고 답을 기다려요");
    expect(templateReference).toContain("Recommendation wording such as");
    expect(templateReference).toContain("is not template approval");
    expect(templateReference).not.toContain("Ask in normal chat text instead");
  });

  test("confirms app name before freezing slug for new bootstrap apps", () => {
    const bootstrap = readBootstrap();
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");

    expect(bootstrap).toContain("앱 이름이 발화에서 유추되더라도 새 앱 생성에서는 한 번 확인해요");
    expect(bootstrap).toContain("앱 이름 질문 문구는 반드시 `앱 이름을 무엇으로 할까요?`");
    expect(bootstrap).toContain("`앵 이름` 같은 오타나 줄임말을 쓰지 않아요");
    expect(bootstrap).toContain("표시 제목은 `앱 이름 확인`");
    expect(bootstrap).toContain("앱 이름 확인도 native Question/AskUserQuestion card 로 먼저 물어요");
    expect(bootstrap).toContain("답변 입력이 막힐 때만 일반 채팅 텍스트로 fallback");
    expect(bootstrap).toContain("사용자가 답한 뒤에만 `--name`/`--slug` 를 확정해요");
    expect(bootstrap).toContain("`use the recommended name` 은 앱 이름 질문이 먼저 보인 뒤의 답변일 때만 확정");
    expect(bootstrap).toContain("`choose whatever is reasonable`");
    expect(bootstrap).toContain("선택지 설명은 짧고 검수된 한국어만 써요");
    expect(bootstrap).toContain("기존 앱들과 겹치지 않는 새 콘셉트");
    expect(bootstrap).toContain("예약 폼과 시간 선택에 적합");
    expect(templateReference).toContain("Keep choice descriptions short and proofread");
    expect(templateReference).toContain("정적 페이지 중심이면 가까운 구조");
    expect(templateReference).toContain(CLAUDE.bootstrapSkill.nameConfirmBeforeFinalize);
    expect(templateReference).toContain("Recommendation wording like");
    expect(templateReference).toContain("is approval only after the app-name prompt is visible");
    expect(templateReference).toContain("Ask with a native Question/AskUserQuestion card first");
    expect(templateReference).toContain("Fall back to normal chat text only when the card does not render or the answer UI is unavailable");
    expect(templateReference).toContain("앱 이름을 무엇으로 할까요?");
    expect(templateReference).toContain("번호나 원하는 앱 이름으로 답해 주세요.");
    expect(templateReference).not.toContain("\"앱 이름 뭘로 할래요?\"");
  });

  test("does not accept pre-bootstrap concept or slug prompts as bootstrap confirmation", () => {
    const bootstrap = readBootstrap();
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");
    const clarity = readRepo("skills/clarity/SKILL.md");

    expect(bootstrap).toContain("섞인 요청에서 update/clarity 가 먼저 처리됐어도 template 은 여기서만 확정");
    expect(bootstrap).toContain("콘셉트·slug·이름 질문 답은 추천 힌트일 뿐");
    expect(bootstrap).toContain("추천 후보로만 쓰고");
    expect(templateReference).toContain("Pre-bootstrap answers to concept or slug questions are not confirmation");
    expect(templateReference).toContain("bootstrap must still show `앱 이름 확인`");
    expect(clarity).toContain("clarity 가 앱 콘셉트, 이름, slug, 템플릿을 대신 묻거나 확정하지 않아요");
    expect(clarity).toContain("새 앱 생성은 이어서 진행할게요");
    expect(clarity).toContain("question 제목 `예약 사이트 컨셉`");
    expect(clarity).toContain("질문 `새로 만들 예약 웹사이트, 어떤 컨셉으로 할까요?`");
    expect(clarity).toContain("선택지 `네일샵 예약`, `사진관 예약`, `애견미용 예약`, `피아노 레슨 예약`");
    expect(clarity).toContain("추천 후보는 bootstrap 이 묻고 clarity 는 만들지 않아요");
    expect(clarity).toContain("bootstrap 이 `어떤 템플릿으로 시작할까요?` 와 `앱 이름을 무엇으로 할까요?` 카드를 묻게 해요");
  });

  test("blocks typoed Korean prompt copy observed in desktop bootstrap QA", () => {
    const contract = readBootstrapContract() + "\n" + readRepo("skills/clarity/SKILL.md");

    expect(contract).toContain("`어느 템플릿` 대신 반드시 `어떤 템플릿으로 시작할까요?`");
    expect(contract).toContain("`작업공간`이라고 말해요");
    expect(contract).toContain("`가장 빠른 시작`");
    expect(contract).not.toContain("앱 이륨");
    expect(contract).not.toContain("앱 이맄");
    expect(contract).not.toContain("뱰른");
    expect(contract).not.toContain("작업구역");
    expect(contract).not.toContain("지어버릴게요");
    expect(contract).not.toContain("곹치지");
    expect(contract).not.toContain("컴셈");
    expect(contract).not.toContain("컴셉");
    expect(contract).not.toContain("가깜운");
    expect(contract).not.toContain("가벼고");
    expect(contract).not.toContain("안 격침");
    expect(contract).not.toContain("멈췤요");
    expect(contract).not.toContain("앨 젼");
    expect(contract).not.toContain("낹니다");
    expect(contract).toContain("가볍고 빠른 시작");
    expect(contract).toContain("진행하지 않고 멈춰요");
  });

  test("chooses template and app name before checking the GitHub owner", () => {
    const bootstrap = readBootstrap();
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");

    const workflow = bootstrap.slice(bootstrap.indexOf("실제 순서:"), bootstrap.indexOf("Slash command"));
    expect(workflow.indexOf("Template picker")).toBeGreaterThanOrEqual(0);
    expect(workflow.indexOf("App name")).toBeGreaterThanOrEqual(0);
    expect(workflow.indexOf("GitHub App gate")).toBeGreaterThanOrEqual(0);
    expect(workflow.indexOf("Template picker")).toBeLessThan(workflow.indexOf("GitHub App gate"));
    expect(workflow.indexOf("App name")).toBeLessThan(workflow.indexOf("GitHub App gate"));
    expect(templateReference).toContain("Ask for template and app name before the GitHub App gate");
  });

  test("checks app address availability before bootstrap dry-run preview", () => {
    const bootstrap = readBootstrap();

    const workflow = bootstrap.slice(bootstrap.indexOf("실제 순서:"), bootstrap.indexOf("Slash command"));
    expect(workflow.indexOf("GitHub App gate")).toBeGreaterThanOrEqual(0);
    expect(workflow.indexOf("Availability check")).toBeGreaterThanOrEqual(0);
    expect(workflow.indexOf("Dry-run preview")).toBeGreaterThanOrEqual(0);
    expect(workflow.indexOf("GitHub App gate")).toBeLessThan(workflow.indexOf("Availability check"));
    expect(workflow.indexOf("Availability check")).toBeLessThan(workflow.indexOf("Dry-run preview"));
    expect(bootstrap).toContain("Tool 제목은 `앱 주소 확인`");
    expect(bootstrap).toContain("axhub apps check-availability --tenant <tenant> --slug <app-slug> --subdomain <app-slug> --json");
    expect(bootstrap).toContain("dry-run preview 전 pre-preview guard");
    expect(bootstrap).toContain("slug 또는 subdomain 중 하나라도 unavailable 이면 bootstrap dry-run/execute 금지");
    expect(bootstrap).toContain("availability check 부터 다시 실행해요");
    expect(bootstrap).toContain("execute 실패 뒤 복구로 처리하지 않아요");
  });

  test("keeps desktop-visible bootstrap commands literal and one command at a time", () => {
    const bootstrap = readBootstrap();
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const preCloneReference = localReference.slice(
      localReference.indexOf("## Dry-Run Preview"),
      localReference.indexOf("## Clone Current Directory"),
    );

    expect(bootstrap).toContain("한 tool call 에 하나의 직접 CLI 호출만 넣어요");
    expect(bootstrap).toContain("literal flag 로 넣어요");
    expect(bootstrap).toContain("execute/resume 명령에는 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1` 을 붙이지 않아요");
    expect(templateReference).toContain("replace `test` with the selected tenant literal");
    expect(localReference).toContain("Do not run a Desktop-visible command that contains");
    expect(localReference).toContain("Execute and resume commands carry no env prefix");
    expect(preCloneReference).toContain("axhub --no-input apps bootstrap --template nextjs-axhub --name bakery-preorder");
    expect(preCloneReference).not.toContain('axhub apps bootstrap --template "$TEMPLATE"');
    expect(preCloneReference).not.toContain('AXHUB_TENANT="${AXHUB_TENANT');
    expect(preCloneReference).not.toContain("export ");
  });

  test("uses direct deploy status calls instead of shell polling or JSON text parsing", () => {
    const bootstrap = readBootstrap();
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");

    expect(bootstrap).toContain("배포 상태 대기/확인도 예외가 아니에요");
    expect(bootstrap).toContain("`for`, `while`, `until`, `sleep`, `grep`, `head`, `tail`, `cut`, `awk`, `sed`, `jq`");
    expect(bootstrap).toContain("`Monitor`, `ScheduleWakeup`, background watch");
    expect(bootstrap).toContain("CLI 내부 대기인 `deploy verify --wait` 는 이 금지에 걸리지 않고 오히려 우선이에요");
    expect(bootstrap).toContain("fallback 으로 상태를 다시 볼 때는 별도 tool call 로 `axhub deploy status <deployment-id> --tenant <tenant> --json` 한 명령만 실행");
    expect(bootstrap).toContain("`until axhub ... | grep ...`, `axhub ... | head ...` 같은 권한 요청창이 뜨는 긴 shell watch 는 UX 실패예요");
    expect(bootstrap).toContain("성공/실패 판정은 shell text parsing 이 아니라 tool output JSON 을 읽어서 해요");
    expect(bootstrap).toContain("deployment id 를 알면 terminal/verify 완료 전 응답을 끝내지 않아요");
    expect(localReference).toContain("Never poll deployment status with a shell loop");
    expect(localReference).toContain(CLAUDE.bootstrapSkill.desktopWatcherBan);
    expect(localReference).toContain("Run one direct `axhub deploy status <deployment-id> --tenant <tenant> --json` command per check");
    expect(localReference).toContain("do not express the wait as `sleep`, `until`, `while`, a pipe, or a compound shell block");
    expect(localReference).toContain("A Desktop permission request containing `until axhub deploy status`");
    expect(bootstrap).toContain("terminal 까지 봐요");
    expect(bootstrap).toContain("verify 성공 전 최종 성공 문구 금지");
    expect(bootstrap).toContain("`잠시 후 확인해보세요` 로 끝내기 금지");
    expect(localReference).toContain("the known `deployment_id` becomes an owned watch");
    expect(localReference).toContain("Do not end the response by telling the user to check later");
    expect(localReference).toContain("Do not report final success while the deployment status is building/running/pending");
    expect(localReference).toContain("run one direct `axhub deploy verify <deployment-id> --app <app-slug> --json` command");
    expect(localReference).toContain("Do not claim \"I'll automatically check again in 90 seconds\"");
    expect(localReference).toContain("Do not parse JSON with `grep`, `head`, `tail`, `cut`, `awk`, `sed`, or `jq` in Desktop-visible commands");
    expect(localReference).not.toContain("--watch --watch-timeout 9m");
    expect(localReference).not.toContain("you can check the URL later");
    expect(localReference).not.toContain("for i in $(seq");
    expect(localReference).not.toContain("grep -o '\"status\"");
    expect(localReference).not.toContain("cut -d");
  });

  test("uses axhub to prepare idempotency without exposing OS UUID commands", () => {
    const bootstrap = readBootstrap();
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");
    const executeReference = localReference.slice(
      localReference.indexOf("## Execute And Watch"),
      localReference.indexOf("## Device-Code Event"),
    );
    const preCloneReference = localReference.slice(
      localReference.indexOf("## Execute And Watch"),
      localReference.indexOf("## Clone Current Directory"),
    );

    expect(bootstrap).not.toContain("uuidgen");
    expect(localReference).not.toContain("uuidgen");
    expect(bootstrap).toContain("`axhub plugin-support init-resume put` 에 생성을 맡겨요");
    expect(localReference).toContain("let `axhub plugin-support init-resume put` generate the idempotency key");
    expect(bootstrap).toContain("`device_flow_required_user_action` 에서 멈추거나");
    expect(bootstrap).toContain("사용자에게 승인 완료를 채팅으로 알려 달라고 쓰지 않아요");
    expect(localReference).toContain("따로 `승인했어`라고 말하지 않아도 돼요");
    expect(localReference).toContain("Prefer the emitted `resume_command` literally");
    expect(localReference).toContain("do not end the response asking the user to report approval");
    expect(resumeReference).toContain("Do not ask the user to say an approval phrase in chat");
    expect(executeReference).toContain("axhub plugin-support init-resume put --template nextjs-axhub --app-name bakery-preorder --slug bakery-preorder --subdomain bakery-preorder --json");
    expect(executeReference).not.toContain("init-resume put --template nextjs-axhub --app-name bakery-preorder --slug bakery-preorder --subdomain bakery-preorder --idempotency-key");
    expect(preCloneReference).toContain("axhub --no-input apps bootstrap");
  });

  test("passes repo name and subdomain explicitly from the app slug", () => {
    const bootstrap = readBootstrap();
    const reference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");

    expect(bootstrap).toContain("repo name 과 subdomain 은 명시 입력이 없으면 app slug 로 맞춰요");
    expect(bootstrap).toContain("--repo-name bakery-preorder --subdomain bakery-preorder");
    expect(reference).toContain("--repo-name bakery-preorder --subdomain bakery-preorder");
  });

  test("distinguishes deployed private access from approved public access", () => {
    const bootstrap = readBootstrap();
    const resultReference = readRepo("skills/bootstrap/references/errors-and-followups.md");

    expect(bootstrap).toContain("`visibility=private` 또는 `review_status=pending` 이면 친구에게 바로 공개됐다고 말하지 않아요");
    expect(bootstrap).toContain('axhub publish --app "$APP_SLUG" --visibility public --execute --json');
    expect(bootstrap).not.toContain('axhub publish --app "$APP_SLUG" --visibility public --dry-run');
    expect(bootstrap).toContain("publish dry-run 을 먼저 호출하지 않고");
    expect(bootstrap).toContain("`Dry-run 기본값` 같은 내부 CLI dry-run semantics");
    expect(bootstrap).toContain("승인 전 공개 확대를 `axhub apps update --visibility public` 로 시도하지 않아요");
    expect(bootstrap).toContain("도메인-only target 금지");
    expect(resultReference).toContain("If `PUBLIC_URL` exists and `VISIBILITY=public` and `REVIEW_STATUS=approved`");
    expect(resultReference).toContain("do not call it public");
    expect(resultReference).toContain('axhub publish --app "$APP_SLUG" --visibility public --execute --json');
    expect(resultReference).not.toContain('axhub publish --app "$APP_SLUG" --visibility public --dry-run');
    expect(resultReference).toContain('Never try `axhub apps update "$APP_SLUG" --visibility public` before approval');
  });

  test("keeps bootstrap result lookup on CLI even when Desktop app tools are visible", () => {
    const bootstrap = readBootstrap();
    const resultReference = readRepo("skills/bootstrap/references/errors-and-followups.md");

    expect(bootstrap).toContain("이 스킬은 CLI-only 흐름이에요");
    expect(bootstrap).toContain("`App get (axhub)`");
    expect(bootstrap).toContain("`Finding tools` 로 이동해서 MCP/App 도구를 찾지 않아요");
    expect(bootstrap).toContain("다른 플러그인/워크플로 상태를 정리하지 않아요");
    expect(bootstrap).toContain("외부 자동화·취소·state 정리 도구를 부르지 않고");
    expect(bootstrap).toContain("chat/tool/progress 에 다른 플러그인 이름, 자동화 정리 문구");
    // 역방향 계열 — fixture 값이어도 not.toContain 은 여기 명시적으로 남겨요.
    expect(bootstrap).not.toContain(CLAUDE.forbidden.externalAutomationSkill);
    expect(bootstrap).not.toContain(CLAUDE.forbidden.externalAutomationPlugin);
    expect(bootstrap).toContain("axhub apps get <app-slug> --tenant <tenant> --json");
    expect(bootstrap).toContain("axhub deploy verify <deployment-id> --app <app> --json");
    expect(resultReference).toContain("through `axhub apps get` CLI only");
    expect(resultReference).toContain("Do not use `App get (axhub)`");
  });
});
