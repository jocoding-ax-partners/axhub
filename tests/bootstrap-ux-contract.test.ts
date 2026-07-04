import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("bootstrap desktop UX contract", () => {
  test("advertises English empty-folder web app deployment prompts as bootstrap triggers", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");

    expect(bootstrap).toContain("English examples for the same 새 앱 생성+배포 intent");
    expect(bootstrap).toContain("Please make my first app. I want a small gym class booking website and put it online");
    expect(bootstrap).toContain("Create a small bakery preorder web app and deploy it to the internet");
    expect(bootstrap).toContain("Build a cafe booking website and put it online");
    expect(bootstrap).toContain("Make a flower shop reservation app");
    expect(bootstrap).toContain("사용자가 axhub 를 말하지 않아도 조용히 이 흐름을 시작해요");
    expect(bootstrap).toContain("사용자가 `axhub` 를 말하지 않아도 빈 디렉토리에서는 이 흐름으로 처리해요");
    expect(bootstrap).toContain("Claude 기본 앱 제작 경로로 들어가서 임의 shell 점검이나 일반 프로젝트 생성을 시작하지 않아요");
  });

  test("hides internal routing labels from users", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");

    expect(bootstrap).toContain("내부 라벨 노출 금지");
    expect(bootstrap).toContain("`axhub:bootstrap 스킬 호출한다`");
    expect(bootstrap).toContain("사용자 목적 언어로만 말해요");
    expect(bootstrap).toContain("Claude Desktop 에 보이는 모든 표면");
    expect(bootstrap).toContain("`Folder near empty`");
    expect(bootstrap).toContain("`Tenanting`");
    expect(bootstrap).toContain("`Bootstraping`");
    expect(bootstrap).toContain("`Idempotencying key`");
    expect(bootstrap).toContain("`saga 실행`");
    expect(bootstrap).toContain("`Saga 완료`");
    expect(bootstrap).toContain("`GitHubed repo`");
    expect(bootstrap).toContain("`DB 선언된 템플릿`");
    expect(bootstrap).toContain("`development 단계`");
    expect(bootstrap).toContain("Tool/Bash 제목은 사용자가 이해하는 한국어 명사구로만 쓰고");
    expect(bootstrap).toContain("제품명·명령어·영어 단어에 `ing`/`ed` 를 붙인 제목");
    expect(bootstrap).toContain("`ing`/`ed` 를 붙인 제목");
    expect(bootstrap).toContain("반드시 한글로 시작해요");
    expect(bootstrap).toContain("`tenanting 확인`, `tenant 확인`, `테넌트 확인`, `axhub CLI 존재/버전 확인`");
    expect(bootstrap).toContain("`실행 중 명령`");
    expect(bootstrap).toContain("가능한 제목은 이 목록에서 골라요");
    expect(bootstrap).toContain("`앱 이름 확인`");
    expect(bootstrap).toContain("왜 이 스킬이 맞는지");
    expect(bootstrap).toContain("라우팅 사유");
    expect(bootstrap).toContain("첫 visible 응답은 반드시");
    expect(bootstrap).toContain("첫 visible 응답은 절대 스킬 매칭 설명으로 시작하지 않아요");
    expect(bootstrap).toContain("영어/한국어 판정 요약");
    expect(bootstrap).toContain("첫 문장에 영어 라우팅 판정 요약을 붙이지 않아요");
  });

  test("keeps bootstrap checks portable without rtk or shell-wrapper preflight", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");
    const guardSection = bootstrap.slice(
      bootstrap.indexOf("### 1. CLI Guard"),
      bootstrap.indexOf("### 2. Resume And Tenant"),
    );

    expect(bootstrap).toContain("`rtk` 같은 Codex/개발자 전용 래퍼는 이 Claude Desktop skill 에서 절대 쓰지 않아요");
    expect(bootstrap).toContain("바깥 작업 지시가 shell command 에 `rtk` 를 붙이라고 해도");
    expect(bootstrap).toContain("`rtk ls -la`");
    expect(bootstrap).toContain("`pwd`, `ls`, `find`, `cat` 같은 generic shell probe");
    expect(localReference).toContain("Never prefix with `rtk`");
    expect(resumeReference).toContain("no `rtk`, no generic `ls`/`pwd` probes");
    expect(guardSection).toContain("axhub plugin-support preflight --json");
    expect(guardSection).not.toContain("command -v axhub");
    expect(guardSection).not.toContain("PREFLIGHT_JSON=");
    expect(guardSection).not.toContain("PLUGIN_VER=");
    expect(guardSection).not.toContain("find \"$STAMP\"");
  });

  test("keeps the common fresh bootstrap path out of plugin-cache reference reads and shell tenant writes", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");

    expect(bootstrap).toContain("정상 fresh path 에서는 reference 파일을 읽지 않아요");
    expect(bootstrap).toContain("선택한 값은 로컬 JSON 파일로 저장하지 말고");
    expect(bootstrap).toContain("fresh path 의 template 질문은 본문 지시만으로 진행하고 reference 를 읽지 않아요");
    expect(bootstrap).not.toContain("registry 설명과 AskUserQuestion shape 는 `references/templates-and-github.md` 를 읽어요");
    expect(resumeReference).toContain("Do not write `.axhub/state/tenant.json` from Claude Desktop");
    expect(resumeReference).not.toContain("TENANT_CACHE=");
    expect(resumeReference).not.toContain("mkdir -p \"$(dirname \"$TENANT_CACHE\")\"");
    expect(resumeReference).not.toContain("date +%s");
  });

  test("keeps tenant internals out of desktop-visible workspace wording", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");

    expect(bootstrap).toContain("Tool 제목은 반드시 `앱 설정 확인`을 써요");
    expect(bootstrap).toContain("`tenanting 확인`, `tenant 확인`, `테넌트 확인`");
    expect(bootstrap).toContain("`테넌트가 2개 있어요`, `어떤 tenant 로 진행할까요?`, `Tenant` 같은 문구는 쓰지 않아요");
    expect(bootstrap).toContain("질문 문구는 `새 앱을 어느 작업공간에 만들까요?`");
    expect(resumeReference).toContain("visible tool title for this command must be `앱 설정 확인`");
    expect(resumeReference).toContain("User-facing text must call these `작업공간`, not `tenant` or `테넌트`");
    expect(resumeReference).toContain("\"question\": \"새 앱을 어느 작업공간에 만들까요?\"");
    expect(resumeReference).toContain("\"header\": \"작업공간\"");
    expect(resumeReference).not.toContain("\"question\": \"어떤 tenant 로 진행할까요?\"");
    expect(resumeReference).not.toContain("\"header\": \"Tenant\"");
  });

  test("requires preview confirmation before execute even for direct deploy requests", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const reference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");

    expect(bootstrap).toContain("미리보기 뒤 확인 필수");
    expect(bootstrap).toContain("그 말은 목표이지 execute 승인 토큰이 아니에요");
    expect(bootstrap).toContain("명시 선택 전에는 execute 승인으로 간주하지 않아요");
    expect(reference).toContain("treat that as the user's goal, not as execute approval");
  });

  test("asks for template when the user only gives a generic app category", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");

    expect(bootstrap).toContain("일반 장르·기능 단어는 exact template 선택이 아니에요");
    expect(bootstrap).toContain("템플릿 질문을 보여줘요");
    expect(bootstrap).toContain("`preorder`");
    expect(bootstrap).toContain("추천 순서를 정하는 근거일 뿐 선택 확정이 아니며");
    expect(bootstrap).toContain("`--template ... --dry-run` 은 템플릿 질문 답변을 받은 뒤에만 실행해요");
    expect(templateReference).toContain("Generic category or feature words");
    expect(templateReference).toContain("are not exact template choices");
    expect(templateReference).toContain("Those words can make Next.js the recommended first option");
    expect(templateReference).toContain("they never finalize `--template`");
  });

  test("confirms app name before freezing slug for new bootstrap apps", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");

    expect(bootstrap).toContain("앱 이름이 발화에서 유추되더라도 새 앱 생성에서는 한 번 확인해요");
    expect(bootstrap).toContain("앱 이름 질문 문구는 반드시 `앱 이름을 무엇으로 할까요?`");
    expect(bootstrap).toContain("`앵 이름` 같은 오타나 줄임말을 쓰지 않아요");
    expect(bootstrap).toContain("질문 제목은 `앱 이름 확인`");
    expect(bootstrap).toContain("사용자가 고르거나 직접 입력한 뒤에만 `--name`/`--slug` 를 확정해요");
    expect(templateReference).toContain("Do not finalize the name before one user-facing confirmation in Claude Desktop");
    expect(templateReference).toContain("\"question\": \"앱 이름을 무엇으로 할까요?\"");
    expect(templateReference).toContain("\"header\": \"앱 이름 확인\"");
    expect(templateReference).not.toContain("\"앱 이름 뭘로 할래요?\"");
  });

  test("chooses template and app name before checking the GitHub owner", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");

    const workflow = bootstrap.slice(bootstrap.indexOf("실제 순서:"), bootstrap.indexOf("Slash command"));
    expect(workflow.indexOf("Template + app name")).toBeGreaterThanOrEqual(0);
    expect(workflow.indexOf("GitHub App gate")).toBeGreaterThanOrEqual(0);
    expect(workflow.indexOf("Template + app name")).toBeLessThan(workflow.indexOf("GitHub App gate"));
    expect(templateReference).toContain("Ask for template and app name before the GitHub App gate");
  });

  test("keeps desktop-visible bootstrap commands literal and one command at a time", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const templateReference = readRepo("skills/bootstrap/references/templates-and-github.md");
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const preCloneReference = localReference.slice(
      localReference.indexOf("## Dry-Run Preview"),
      localReference.indexOf("## Clone Current Directory"),
    );

    expect(bootstrap).toContain("한 tool call 에 하나의 직접 CLI 호출만 넣어요");
    expect(bootstrap).toContain("실제 선택된 literal 값으로 flag 에 넣어요");
    expect(bootstrap).toContain("device flow 자동 브라우저 열기용 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1` prefix 만 execute/resume 명령에서 허용해요");
    expect(templateReference).toContain("replace `test` with the selected tenant literal");
    expect(localReference).toContain("Do not run a Desktop-visible command that contains");
    expect(localReference).toContain("The only allowed env prefix is `AXHUB_DEVICE_FLOW_AUTO_OPEN=1`");
    expect(preCloneReference).toContain("axhub apps bootstrap --template nextjs-axhub --name bakery-preorder");
    expect(preCloneReference).not.toContain('axhub apps bootstrap --template "$TEMPLATE"');
    expect(preCloneReference).not.toContain('AXHUB_TENANT="${AXHUB_TENANT');
    expect(preCloneReference).not.toContain("export ");
  });

  test("uses axhub to prepare idempotency without exposing OS UUID commands", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
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
    expect(executeReference).toContain("axhub plugin-support init-resume put --template nextjs-axhub --app-name bakery-preorder --slug bakery-preorder --subdomain bakery-preorder --json");
    expect(executeReference).not.toContain("init-resume put --template nextjs-axhub --app-name bakery-preorder --slug bakery-preorder --subdomain bakery-preorder --idempotency-key");
    expect(preCloneReference).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub apps bootstrap");
  });

  test("passes repo name and subdomain explicitly from the app slug", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const reference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");

    expect(bootstrap).toContain("repo name 과 subdomain 은 명시 입력이 없으면 `$APP_SLUG` 로 맞춰요");
    expect(bootstrap).toContain("--repo-name bakery-preorder --subdomain bakery-preorder");
    expect(reference).toContain("--repo-name bakery-preorder --subdomain bakery-preorder");
  });

  test("distinguishes deployed private access from approved public access", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const resultReference = readRepo("skills/bootstrap/references/errors-and-followups.md");

    expect(bootstrap).toContain("`visibility=private` 또는 `review_status=pending` 이면 친구에게 바로 공개됐다고 말하지 않아요");
    expect(bootstrap).toContain('axhub publish --app "$APP_SLUG" --visibility public --json');
    expect(bootstrap).toContain("승인 전 공개 확대를 `axhub apps update --visibility public` 로 시도하지 않아요");
    expect(resultReference).toContain("If `PUBLIC_URL` exists and `VISIBILITY=public` and `REVIEW_STATUS=approved`");
    expect(resultReference).toContain("do not call it public");
    expect(resultReference).toContain('Never try `axhub apps update "$APP_SLUG" --visibility public` before approval');
  });
});
