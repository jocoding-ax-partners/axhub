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

describe("init template guidance UX", () => {
  test("keeps CLI registry as source of truth and safe fallback as abort", () => {
    expect(initSkill).toContain("axhub --json init --list-templates");
    expect(initSkill).toContain("Non-interactive AskUserQuestion guard (D1)");
    expect(initSkill).toContain("어떤 템플릿으로 시작할까요?");
    expect(defaults.init["어떤 템플릿으로 시작할까요?"].safe_default).toBe("abort");
  });

  test("explains each current visible template with beginner outcomes", () => {
    const guide = guidanceSection();
    for (const id of [
      "nextjs-axhub",
      "astro-axhub",
      "vite-react-axhub",
      "remix-axhub",
      "express-axhub",
      "hono-axhub",
    ]) {
      expect(guide, `${id} should have local guidance`).toContain(id);
    }

    expect(guide).toMatch(/쇼핑몰|예약|결제|로그인|관리자 화면/);
    expect(guide).toMatch(/회사 소개|랜딩 페이지|블로그|문서|글과 이미지/);
    expect(guide).toMatch(/설정 화면|입력 폼|관리 화면/);
    expect(guide).toMatch(/신청서|설문|주문/);
    expect(guide).toMatch(/주문 처리|데이터 저장/);
    expect(guide).toMatch(/작고 빠른 연결용 서버|외부 서비스/);
  });

  test("documents overlay semantics and unknown template fallback", () => {
    const guide = guidanceSection();
    expect(guide).toContain("CLI가 반환한 template");
    expect(guide).toMatch(/알 수 없는|새 template|unknown/);
    expect(guide).toMatch(/framework.*description|description.*framework/);
    expect(guide).toMatch(/숨기지/);
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

  test("does not introduce bootstrap, remote-template, or dependency-install commands", () => {
    expect(initSkill).not.toMatch(/axhub-helpers\s+fetch-template/);
    expect(initSkill).not.toMatch(/\b(?:npm|pnpm|bun)\s+install\b/);
    expect(initSkill).not.toMatch(/\b(?:yarn|pip|poetry)\s+install\b/);
    expect(initSkill).not.toMatch(/curl\s+.*templates\.json/);
  });
});
