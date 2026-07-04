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

    expect(bootstrap).toContain("일반 장르 단어는 exact template 선택이 아니에요");
    expect(bootstrap).toContain("템플릿 질문을 보여줘요");
    expect(templateReference).toContain("Generic category words");
    expect(templateReference).toContain("are not exact template choices");
  });

  test("passes repo name and subdomain explicitly from the app slug", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const reference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");

    expect(bootstrap).toContain("repo name 과 subdomain 은 명시 입력이 없으면 `$APP_SLUG` 로 맞춰요");
    expect(bootstrap).toContain('--repo-name "$REPO_NAME" --subdomain "$SUBDOMAIN"');
    expect(reference).toContain('REPO_NAME="${REPO_NAME:-$APP_SLUG}"');
    expect(reference).toContain('--repo-name "$REPO_NAME" --subdomain "$SUBDOMAIN"');
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
