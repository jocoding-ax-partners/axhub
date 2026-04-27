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
});
