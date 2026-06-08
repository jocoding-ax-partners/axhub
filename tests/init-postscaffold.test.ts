import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INIT_SKILL_PATH = join(REPO_ROOT, "skills", "init", "SKILL.md");
const DEFAULTS_PATH = join(REPO_ROOT, "tests", "fixtures", "ask-defaults", "registry.json");
const ALLOWLIST_PATH = join(REPO_ROOT, "scripts", "skill-doctor-allowlist.json");

const initSkill = readFileSync(INIT_SKILL_PATH, "utf8");
const defaults = JSON.parse(readFileSync(DEFAULTS_PATH, "utf8"));
const allowlist = JSON.parse(readFileSync(ALLOWLIST_PATH, "utf8"));

describe("init post-scaffold auto-connect contract", () => {
  test("declares dependency execution explicitly and is allowlisted", () => {
    expect(initSkill).toContain("allows-dependency-execution: true");
    expect(allowlist.allows_dependency_execution).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          skill: "init",
        }),
      ]),
    );
  });

  test("adds Step 8.5 helper-driven auto-connect with lifecycle scripts blocked", () => {
    expect(initSkill).toContain("Step 8.5");
    expect(initSkill).toContain("axhub-helpers scaffold-detect --json");
    expect(initSkill).toContain("axhub-helpers scaffold-dev start --json");
    expect(initSkill).toContain("--ignore-scripts");
    expect(initSkill).toContain("로컬 미리보기는 잠깐 준비가 더 필요해요");
  });

  test("does not leave manual npm install/npm run dev instructions in the result card", () => {
    expect(initSkill).not.toContain("의존성 설치 — package manager 자유");
    expect(initSkill).not.toContain("`npm i` / `pnpm i` / `bun install`");
    expect(initSkill).not.toContain("로컬 실행 — README 의 dev 스크립트");
  });

  test("uses confirmed deployment URL as hero and forbids subdomain synthesis", () => {
    expect(initSkill).toContain("배포 공개 URL");
    expect(initSkill).toContain("URL 은 절대 합성하지 않아요");
    expect(initSkill).toContain("dry-run 의 subdomain");
    expect(initSkill).not.toContain("https://<subdomain>.<domain>");
  });

  test("registers auto-connect and resume AskUserQuestion safe defaults", () => {
    expect(defaults.init["앱을 바로 실행해 볼까요?"].safe_default).toBe("아니요");
    expect(defaults.init["저번에 만들던 앱을 이어서 할까요?"].safe_default).toBe("새로 시작");
  });
});

describe("init cross-session resume contract", () => {
  test("proactively checks repo-local init resume state before fresh scaffold", () => {
    expect(initSkill).toContain("Step 0.5");
    expect(initSkill).toContain("axhub-helpers init-resume route --json");
    expect(initSkill).toContain(".axhub/init-resume.json");
    expect(initSkill).not.toContain("state-show");
    expect(initSkill).not.toContain("state-update");
  });

  test("resume-last includes template, name, slug, and idempotency key", () => {
    expect(initSkill).toMatch(
      /axhub apps bootstrap --template "\$TEMPLATE" --name "\$APP_NAME" --slug "\$APP_SLUG" --execute --resume-last --watch --watch-timeout 9m --idempotency-key "\$IDEMPOTENCY_KEY" --json/,
    );
  });

  test("keeps consent and preauth internals out of the init user-flow body", () => {
    expect(initSkill).not.toMatch(/\bpreauth\b/i);
    expect(initSkill).not.toMatch(/\bHMAC\b/);
    expect(initSkill).not.toMatch(/\bconsent\b/i);
  });
});
