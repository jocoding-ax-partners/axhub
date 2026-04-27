// Phase 17 US-1705 — every commands/*.md MUST have argument-hint frontmatter
// for autocomplete UX. Missing hint = vibe coder must guess args.

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const COMMANDS_DIR = join(REPO_ROOT, "commands");

const commandFiles = readdirSync(COMMANDS_DIR).filter((f) => f.endsWith(".md"));

describe("Phase 17 C4/US-1704 — argument-hint frontmatter in 9 commands", () => {
  for (const file of commandFiles) {
    test(`commands/${file} has argument-hint frontmatter`, () => {
      const content = readFileSync(join(COMMANDS_DIR, file), "utf8");
      // Frontmatter is the first --- ... --- block at top of file
      const frontmatter = content.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
      expect(frontmatter).toContain("argument-hint:");
    });
  }

  test("at least 9 commands files exist (current expected count)", () => {
    expect(commandFiles.length).toBeGreaterThanOrEqual(9);
  });
});
