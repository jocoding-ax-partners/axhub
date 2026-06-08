import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INIT_SKILL_PATH = join(REPO_ROOT, "skills", "init", "SKILL.md");
const DEFAULTS_PATH = join(REPO_ROOT, "tests", "fixtures", "ask-defaults", "registry.json");

const initSkill = readFileSync(INIT_SKILL_PATH, "utf8");
const defaults = JSON.parse(readFileSync(DEFAULTS_PATH, "utf8"));

const guidanceSection = (): string => {
  const marker = "## 템플릿 선택 가이드";
  const start = initSkill.indexOf(marker);
  expect(start, "init skill should include a novice-friendly template guide section").toBeGreaterThanOrEqual(0);
  const rest = initSkill.slice(start);
  const nextHeading = rest.slice(marker.length).search(/^##\s/m);
  return nextHeading === -1 ? rest : rest.slice(0, marker.length + nextHeading);
};

describe("init template guidance UX (bootstrap saga workflow)", () => {
  test("uses backend templates list as source of truth and safe fallback as abort", () => {
    expect(initSkill).toContain("axhub apps templates list --json");
    expect(initSkill).toContain("Non-interactive AskUserQuestion guard (D1)");
    expect(initSkill).toContain("어떤 템플릿으로 시작할까요?");
    expect(defaults.init["어떤 템플릿으로 시작할까요?"].safe_default).toBe("abort");
  });

  test("registers all bootstrap-flow AskUserQuestions with safe defaults", () => {
    expect(defaults.init["앱 이름 뭘로 할래요?"].safe_default).toBe("abort");
    expect(defaults.init["지금 만들고 배포까지 진행할까요?"].safe_default).toBe("취소");
  });

  test("routes plain Korean project initialization phrasing without requiring axhub prefix", () => {
    expect(initSkill).toContain('"프로젝트 초기화"');
    expect(initSkill).toContain('"프로젝트 초기화해줘"');
  });

  test("explains each built-in template alias with beginner outcomes", () => {
    const guide = guidanceSection();
    for (const alias of ["nextjs", "astro", "react"]) {
      expect(guide, `${alias} alias should have local guidance`).toContain(alias);
    }
    expect(guide).toMatch(/쇼핑몰|예약|결제|로그인|관리자 화면/);
    expect(guide).toMatch(/회사 소개|랜딩 페이지|블로그|문서|글과 이미지/);
    expect(guide).toMatch(/설정 화면|입력 폼|관리 화면/);
  });

  test("documents overlay semantics and unknown template fallback", () => {
    const guide = guidanceSection();
    expect(guide).toContain("backend 가 반환한 template");
    expect(guide).toMatch(/알 수 없는|새 template|unknown/);
    expect(guide).toMatch(/folder_name|name/);
    expect(guide).toMatch(/숨기지/);
  });

  test("keeps the structured template picker within the AskUserQuestion option limit", () => {
    const questionStart = initSkill.indexOf('"question": "어떤 템플릿으로 시작할까요?"');
    expect(questionStart).toBeGreaterThanOrEqual(0);

    const questionEnd = initSkill.indexOf("```", questionStart);
    expect(questionEnd).toBeGreaterThan(questionStart);

    const questionBlock = initSkill.slice(questionStart, questionEnd);
    const optionCount = [...questionBlock.matchAll(/"label":/g)].length;
    expect(optionCount).toBeLessThanOrEqual(3);
  });

  test("uses only real backend templates in the Desktop template picker", () => {
    const questionStart = initSkill.indexOf('"question": "어떤 템플릿으로 시작할까요?"');
    expect(questionStart).toBeGreaterThanOrEqual(0);

    const questionEnd = initSkill.indexOf("```", questionStart);
    expect(questionEnd).toBeGreaterThan(questionStart);

    const questionBlock = initSkill.slice(questionStart, questionEnd);
    for (const label of ["Next.js 추천", "Vite + React", "Astro"]) {
      expect(questionBlock).toContain(`"label": "${label}"`);
    }
    expect(questionBlock).not.toContain('"value"');
    expect(questionBlock).not.toContain("manual_template_id");
    expect(questionBlock).not.toContain("직접 고르기");
    expect(questionBlock).not.toContain("취소");

    const guide = guidanceSection();
    expect(guide).toContain("Claude Desktop AskUserQuestion");
    expect(guide).toContain("skip/free-text");
    expect(guide).toContain("선택지는 모두 실제 backend template");
  });

  test("avoids unexplained developer jargon in user-facing guidance", () => {
    const guide = guidanceSection();
    expect(guide).not.toMatch(/\bSPA\b/i);
    expect(guide).not.toMatch(/\bedge\b/i);
    expect(guide).not.toContain("라우팅");
    expect(guide).not.toContain("데이터 로딩");
    expect(guide).not.toContain("프로토타입");
    expect(guide).not.toContain("프론트");
  });

  test("forbids legacy stub commands and remote template fetches via NEVER bullets", () => {
    expect(initSkill).toMatch(/NEVER `axhub init`/);
    expect(initSkill).toMatch(/--from-template/);
    expect(initSkill).toMatch(/NEVER `axhub-helpers fetch-template`/);
    expect(initSkill).toMatch(/source-of-truth/);
  });

  test("uses bootstrap saga as the primary workflow (not plan-only)", () => {
    expect(initSkill).toMatch(/axhub apps bootstrap --template/);
    expect(initSkill).toMatch(/--execute/);
    expect(initSkill).toMatch(/--dry-run/);
    expect(initSkill).toMatch(/bootstrap-status/);
    expect(initSkill).toMatch(/repo_full_name/);
    expect(initSkill).toMatch(/git clone/);
  });

  test("preserves vibe coder visibility rules and TodoWrite checklist", () => {
    expect(initSkill).toContain("Vibe Coder Visibility Rules");
    expect(initSkill).toContain("TodoWrite");
    expect(initSkill).toContain("TodoWrite status sync");
  });
});
