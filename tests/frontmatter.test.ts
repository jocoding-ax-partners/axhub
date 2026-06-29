// US-010 survivor: SKILL.md frontmatter validity gate.
//
// Post-diet binding constraint (decision 4B·T4-B): the only structural skill
// test we keep asserts that each surviving SKILL.md has well-formed YAML
// frontmatter with a non-empty Korean-trigger description and NO dead-contract
// keys. No yaml dep — hand-rolled split.
//
// Dead keys are `multi-step:` / `needs-preflight:` only — the in-body preflight
// migration (ADR-0013) retired them. `model:` is NOT dead: the live SKILL.md
// files carry `model: sonnet` and CLAUDE.md (Phase 25 PR 25.5a+) keeps it as a
// supported routing contract, so it is intentionally excluded from the ban.

import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS = ["onboarding", "init", "deploy", "import", "development", "diagnosis", "clarity", "update"] as const;

// Dead-contract keys retired by ADR-0013. Their presence means a stale
// scaffold leaked back in. (`model:` is excluded — it is a live key.)
const FORBIDDEN_KEYS = ["multi-step", "needs-preflight"] as const;

interface Frontmatter {
  raw: string;
  keys: Record<string, string>;
}

// Split `---\n<yaml>\n---\n<body>` and pull top-level `key:` pairs. Values can
// be single-line quoted strings (our descriptions) or block markers we ignore.
const parseSkill = (text: string): { fm: Frontmatter; body: string } => {
  const match = text.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/);
  if (!match) throw new Error("no YAML frontmatter delimiters");
  const raw = match[1];
  const body = match[2] ?? "";
  const keys: Record<string, string> = {};
  for (const line of raw.split("\n")) {
    // Only top-level keys (no leading indentation) like `name:` / `description:`.
    const kv = line.match(/^([A-Za-z][\w-]*):\s?(.*)$/);
    if (kv) keys[kv[1]] = kv[2] ?? "";
  }
  return { fm: { raw, keys }, body };
};

describe("SKILL.md frontmatter validity (US-010)", () => {
  for (const slug of SKILLS) {
    const path = join(REPO_ROOT, "skills", slug, "SKILL.md");

    test(`${slug}: file exists`, () => {
      expect(existsSync(path), `missing ${path}`).toBe(true);
    });

    test(`${slug}: frontmatter parses + required keys non-empty`, () => {
      const text = readFileSync(path, "utf8");
      const { fm } = parseSkill(text);
      expect(typeof fm.keys.name).toBe("string");
      expect(fm.keys.name.trim().length).toBeGreaterThan(0);
      expect(typeof fm.keys.description).toBe("string");
      expect(fm.keys.description.trim().length).toBeGreaterThan(0);
    });

    test(`${slug}: description carries a Korean trigger phrase (non-ASCII)`, () => {
      const text = readFileSync(path, "utf8");
      const { fm } = parseSkill(text);
      // Korean trigger phrases are non-ASCII; assert at least one such char.
      expect(/[^\x00-\x7F]/.test(fm.keys.description)).toBe(true);
    });

    test(`${slug}: no dead-contract keys`, () => {
      const text = readFileSync(path, "utf8");
      const { fm } = parseSkill(text);
      for (const key of FORBIDDEN_KEYS) {
        // Match top-level `key:` in the frontmatter block only.
        const present = new RegExp(`^${key}:`, "m").test(fm.raw);
        expect(present, `forbidden frontmatter key present: ${key}`).toBe(false);
      }
    });

    test(`${slug}: body non-empty`, () => {
      const text = readFileSync(path, "utf8");
      const { body } = parseSkill(text);
      expect(body.trim().length).toBeGreaterThan(0);
    });
  }
});
