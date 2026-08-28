// 라우팅 fixture 계약 — fixture 를 읽는 코드가 하나도 없으면 fixture 는
// 실행되지 않는 문서예요. 라우팅 판정 자체는 LLM 이라 assert 할 수 없지만,
// fixture 의 형식과 경계 skill 이름이 실재하는지는 잠글 수 있어요.

import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const ROUTING_DIR = join(REPO_ROOT, "tests", "routing");
const SKILLS_DIR = join(REPO_ROOT, "skills");

const skillNames = (): Set<string> =>
  new Set(
    readdirSync(SKILLS_DIR, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name),
  );

// `none` 은 스킬이 아니라 AP-11 의 "어느 스킬도 진입하지 않음" 이에요.
const NON_SKILL_BOUNDARIES = new Set(["none"]);

interface RoutingFixture {
  _doc?: unknown;
  boundary?: Record<string, unknown>;
  cases?: Array<{ utterance?: unknown; expected?: unknown; why?: unknown }>;
}

const fixtureFiles = readdirSync(ROUTING_DIR).filter((name) => name.endsWith(".fixture.json"));

describe("routing fixtures", () => {
  test("fixture 디렉터리에 최소 하나의 fixture 가 있어요", () => {
    expect(existsSync(ROUTING_DIR)).toBe(true);
    expect(fixtureFiles.length).toBeGreaterThan(0);
  });

  for (const file of fixtureFiles) {
    const path = join(ROUTING_DIR, file);

    test(`${file}: 유효한 JSON 이고 _doc·boundary·cases 를 갖춰요`, () => {
      const parsed = JSON.parse(readFileSync(path, "utf8")) as RoutingFixture;
      expect(typeof parsed._doc).toBe("string");
      expect((parsed._doc as string).trim().length).toBeGreaterThan(0);
      expect(parsed.boundary && typeof parsed.boundary).toBe("object");
      expect(Object.keys(parsed.boundary ?? {}).length).toBeGreaterThan(0);
      expect(Array.isArray(parsed.cases)).toBe(true);
      expect((parsed.cases ?? []).length).toBeGreaterThan(0);
    });

    test(`${file}: boundary 키가 실재하는 skill 이름이에요`, () => {
      const parsed = JSON.parse(readFileSync(path, "utf8")) as RoutingFixture;
      const known = skillNames();
      for (const key of Object.keys(parsed.boundary ?? {})) {
        if (NON_SKILL_BOUNDARIES.has(key)) continue;
        expect(known.has(key), `${file}: boundary 에 없는 skill — ${key}`).toBe(true);
      }
    });

    // `expected` 는 boundary 키에 갇히지 않아요 — 양보 대상이나 인계 표기
    // (`deploy → diagnosis`) 를 쓰는 fixture 가 이미 있어요. 잠그는 값은
    // "존재하지 않는 skill 이름을 쓰지 않는다" 하나예요.
    test(`${file}: 모든 case 의 expected 가 실재하는 skill 이름이에요`, () => {
      const parsed = JSON.parse(readFileSync(path, "utf8")) as RoutingFixture;
      const known = skillNames();
      for (const routingCase of parsed.cases ?? []) {
        expect(typeof routingCase.utterance).toBe("string");
        expect(typeof routingCase.why).toBe("string");
        expect(typeof routingCase.expected).toBe("string");
        const targets = (routingCase.expected as string)
          .split(/→|->/)
          .map((token) => token.trim())
          .filter((token) => token.length > 0);
        expect(targets.length).toBeGreaterThan(0);
        for (const target of targets) {
          expect(
            known.has(target) || NON_SKILL_BOUNDARIES.has(target),
            `${file}: 실재하지 않는 skill — ${target}`,
          ).toBe(true);
        }
      }
    });
  }
});
