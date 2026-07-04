import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("bootstrap desktop UX contract", () => {
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
    expect(bootstrap).toContain("Tool/Bash 제목은 사용자가 이해하는 한국어 명사구로만 써요");
    expect(bootstrap).toContain("제품명·명령어·영어 단어에 `ing` 를 붙인 제목");
    expect(bootstrap).toContain("`실행 중 명령`");
    expect(bootstrap).toContain("가능한 제목은 이 목록에서 골라요");
    expect(bootstrap).toContain("`앱 이름 확인`");
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
    expect(bootstrap).toContain("사용자가 고르거나 직접 입력한 뒤에만 `--name`/`--slug` 를 확정해요");
    expect(templateReference).toContain("Do not finalize the name before one user-facing confirmation in Claude Desktop");
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
