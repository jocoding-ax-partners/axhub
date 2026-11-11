import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const read = (path: string): string => readFileSync(join(ROOT, path), "utf8");

describe("Claude Desktop live QA regressions", () => {
  // development 의 Desktop 단일 명령 계약·도구 제목·`get_recipe` 인자 계약·`.axhub/import-resume.json` 앱 바인딩은
  // 2026-11 development 개편 때 MCP 도구 체인과 함께 뺐어요 (tests/development-skill-contract.test.ts 가 새 본문을 잠가요).

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
