import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const read = (path: string): string => readFileSync(join(ROOT, path), "utf8");

describe("Claude Desktop live QA regressions", () => {
  test("development starts with one analyzable preflight and uses the exact recipe contract", () => {
    const skill = read("skills/development/SKILL.md");

    expect(skill).toContain("Claude Desktop 단일 명령 계약");
    expect(skill).toContain("axhub plugin-support preflight --json");
    expect(skill).toContain("Claude Desktop tool 제목은 정확히 `axhub·앱 점검`");
    expect(skill).not.toContain("PREFLIGHT_JSON=$(axhub plugin-support preflight");
    expect(skill).not.toContain("if ! command -v axhub");
    expect(skill).toContain("development 요청에서 별도 `axhub update check`, `command -v`, `axhub --version`을 실행하지 않아요");
    expect(skill).toContain("`app_id`, `recipe_id`, 선택 `framework`, `preferred_table`");
    expect(skill).toContain("`dynamic-db-crud`");
    expect(skill).toContain("`recipe`, `tenant`, `schema` 키를 만들거나 실패 뒤 키 모양을 바꿔 반복하지 않아요");
    expect(skill).toContain("플러그인 캐시를 탐색하지 않고 reference 링크만 읽어요");
  });

  test("development binds the imported app id instead of guessing axhub.yaml name", () => {
    const skill = read("skills/development/SKILL.md");

    expect(skill).toContain("`axhub.yaml` 의 `name` 은 로컬 프로젝트 이름일 수 있으므로 원격 앱 slug 로 단정하지 않아요");
    expect(skill).toContain("`.axhub/import-resume.json`");
    expect(skill).toContain("bare `axhub apps get <app-id> --json`");
    expect(skill).toContain("실패하면 `apps list`, `--help`, pipe 탐색으로 복구하지 말고");
  });

  test("clarity routes destructive operations before generic Desktop probes", () => {
    const skill = read("skills/clarity/SKILL.md");
    const frontmatter = skill.slice(0, skill.indexOf("---", 4));

    expect(frontmatter).toContain("axhub 앱 삭제해줘");
    expect(frontmatter).toContain("스킬 실행 전 사용자 문장·App/MCP·파일 조회는 0개");
    expect(frontmatter.split(".")[0]).toContain("요청하신 운영 명령을 확인할게요");
    expect(frontmatter).toContain("Start directly with that Korean sentence; no preamble");
    expect(frontmatter).toContain('First tool title/description exactly "명령 찾기"');
    expect(frontmatter).toContain("do not mention the skill or routing");
    expect(skill).toContain("첫 visible 문장을 정확히 `요청하신 운영 명령을 확인할게요.`");
    expect(skill).toContain("제목 `명령 찾기`의 bare `axhub --json-schema --field-expr '<가장 좁은 후보>'`");
    expect(skill).toContain("명사·동사 후보가 하나면 leaf 부터");
    expect(skill).toContain("parent/root 를 더 조회하지 않아요");
    expect(skill).toContain("null/empty/error 일 때만 한 단계씩 넓히며");
    expect(skill).not.toContain("`command -v axhub` 가 실패하면");
  });

  // deploy·diagnosis 의 Desktop 첫 명령(deploy-preview-summary)·도구 제목·`어디에 배포할까요?` 카드 계약은
  // 2026-11 deploy 개편(옛 up·diagnosis 를 합친 판) 때 그 동작과 함께 뺐어요
  // (tests/deploy-skill-contract.test.ts 가 새 본문을 잠가요).
  // import 의 ignore 검사 셸 문법·배포 방식 카드(AP-28)·완료 확인 이어 하기 계약은 2026-11 import
  // 개편 때 그 동작과 함께 뺐어요 (tests/import-skill-contract.test.ts 가 새 본문을 잠가요).
});
