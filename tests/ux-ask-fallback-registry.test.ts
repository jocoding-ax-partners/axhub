// Phase 17 US-1705 — per-question registry lookup. For each AskUserQuestion
// JSON block in any SKILL.md, the question text MUST have a registered
// safe_default in tests/fixtures/ask-defaults/registry.json. Drift catch:
// adding a new AskUserQuestion without registering a default fails this test.
// Critic round 2 BLOCKER 3 fix.

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS_DIR = join(REPO_ROOT, "skills");
const REGISTRY_PATH = join(REPO_ROOT, "tests/fixtures/ask-defaults/registry.json");

interface RegistryEntry {
  safe_default?: string;
  rationale?: string;
  allowed_safe_defaults?: string[];
  _note?: string;
  default_source?: string;
  cold_cache_default?: string;
  exit_65_default?: string;
}

const registry: Record<string, Record<string, RegistryEntry>> = JSON.parse(
  readFileSync(REGISTRY_PATH, "utf8")
);

const skillSlugs = readdirSync(SKILLS_DIR).filter((d) => {
  try {
    readFileSync(join(SKILLS_DIR, d, "SKILL.md"), "utf8");
    return true;
  } catch {
    return false;
  }
});

const extractQuestions = (content: string): string[] => {
  const matches = content.match(/"question":\s*"([^"]+)"/g) ?? [];
  return matches.map((m) => m.match(/"question":\s*"([^"]+)"/)?.[1] ?? "").filter(Boolean);
};

describe("Phase 17 C5/US-1705 — per-question fallback registry coverage", () => {
  test("registry file exists and parses", () => {
    expect(registry).toBeTypeOf("object");
  });

  for (const slug of skillSlugs) {
    test(`skills/${slug}/SKILL.md questions all have registered safe_default`, () => {
      const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
      const questions = extractQuestions(content);
      const skillRegistry = registry[slug] ?? {};
      for (const q of questions) {
        const entry = skillRegistry[q];
        expect(entry, `missing default for ${slug}:${q}`).toBeDefined();
        expect(entry?.safe_default, `empty safe_default for ${slug}:${q}`).toBeTruthy();
      }
    });
  }

  test("registry has no stale entries (every key matches a SKILL question)", () => {
    for (const [slug, questions] of Object.entries(registry)) {
      if (slug.startsWith("_")) continue;
      const skillPath = join(SKILLS_DIR, slug, "SKILL.md");
      let content = "";
      try {
        content = readFileSync(skillPath, "utf8");
      } catch {
        continue;
      }
      for (const qKey of Object.keys(questions)) {
        if (qKey.startsWith("_") || qKey.startsWith("default_") || qKey.startsWith("cold_") || qKey.startsWith("exit_")) {
          continue;
        }
        expect(content, `stale registry key ${slug}:${qKey} not found in SKILL.md`).toContain(qKey);
      }
    }
  });

  test("registry entries declare allowed_safe_defaults enum (Plan v3 새 schema)", () => {
    for (const [slug, questions] of Object.entries(registry)) {
      if (slug.startsWith("_")) continue;
      for (const [qKey, entry] of Object.entries(questions)) {
        if (qKey.startsWith("_") || qKey.startsWith("default_") || qKey.startsWith("cold_") || qKey.startsWith("exit_")) continue;
        const e = entry as RegistryEntry;
        expect(
          e.allowed_safe_defaults,
          `${slug}:${qKey} missing allowed_safe_defaults enum`
        ).toBeInstanceOf(Array);
        expect(e.allowed_safe_defaults!.length).toBeGreaterThan(0);
      }
    }
  });

  test("safe_default 값이 allowed_safe_defaults enum 안에 있음", () => {
    for (const [slug, questions] of Object.entries(registry)) {
      if (slug.startsWith("_")) continue;
      for (const [qKey, entry] of Object.entries(questions)) {
        if (qKey.startsWith("_") || qKey.startsWith("default_") || qKey.startsWith("cold_") || qKey.startsWith("exit_")) continue;
        const e = entry as RegistryEntry;
        if (!e.safe_default || !e.allowed_safe_defaults) continue;
        expect(
          e.allowed_safe_defaults,
          `${slug}:${qKey} safe_default "${e.safe_default}" not in enum`
        ).toContain(e.safe_default);
      }
    }
  });

  test("init SKILL 의 dependency_install_strategy enum 은 정확히 ['skip','manual_terminal']", () => {
    const initEntry = registry["init"]?.["dependency_install_strategy"];
    expect(initEntry).toBeDefined();
    expect((initEntry as RegistryEntry).allowed_safe_defaults).toEqual([
      "skip",
      "manual_terminal",
    ]);
    expect((initEntry as RegistryEntry).safe_default).toBe("manual_terminal");
    expect((initEntry as RegistryEntry).allowed_safe_defaults).not.toContain("inline_session");
  });
});

describe("enable-statusline registry entry (Phase 0.5.11 ralplan + 0.5.12 Windows extension)", () => {
  test("registry has enable-statusline key", () => {
    expect((registry as any)["enable-statusline"]).toBeDefined();
  });
  test("entry has correct safe_default and allowed_safe_defaults (4 items incl project-scope option)", () => {
    const entry = (registry as any)["enable-statusline"]["statusLine 어떻게 켤래요?"];
    expect(entry.safe_default).toBe("나중에 할래요");
    expect(entry.allowed_safe_defaults).toEqual(["나중에 할래요", "어떻게 하는지 보여줘요", "Windows PowerShell snippet 보여줘요", "이 repo 만 켤래요 (project scope, dotfiles 비추천)"]);
  });
  test("rationale literal text locked (extended with project-scope disclaimer in v0.6.3)", () => {
    const entry = (registry as any)["enable-statusline"]["statusLine 어떻게 켤래요?"];
    expect(entry.rationale).toBe("Wiring snippet 표시는 idempotent read-only 라 user explicit consent 없는 비대화형 환경에서도 stdout 출력 안전해요. 다만 clipboard mutation 은 interactive 선택 후에만 진행해요. Windows native 4번째 옵션도 stdout 만 출력해요 (clipboard 미사용). v0.5.13 부터 `복사해서 붙여 넣을래요` 옵션은 axhub-helpers settings-merge --apply 자동 wire 호출, 7-branch atomic + .bak rollback 으로 safe. v0.6.3 부터 `이 repo 만 켤래요 (project scope, dotfiles 비추천)` 옵션은 project `.claude/settings.json` 에 paste 할 snippet 만 stdout 출력해요 — autowire 안 함, $HOME 절대경로 commit 위험 SKILL 본문에 warn.");
  });
});
