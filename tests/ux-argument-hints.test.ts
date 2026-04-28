// Phase 17 US-1705 + Phase 1 PLAN reconciliation — every commands/*.md MUST have argument-hint frontmatter
// for autocomplete UX. Missing hint = vibe coder must guess args.

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const COMMANDS_DIR = join(REPO_ROOT, "commands");

const commandFiles = readdirSync(COMMANDS_DIR).filter((f) => f.endsWith(".md")).sort();
const expectedCommandFiles = ["apis.md", "apps.md", "deploy.md", "doctor.md", "help.md", "login.md", "logs.md", "status.md", "update.md", "배포.md"].sort();

describe("Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands", () => {
  for (const file of commandFiles) {
    test(`commands/${file} has argument-hint frontmatter`, () => {
      const content = readFileSync(join(COMMANDS_DIR, file), "utf8");
      // Frontmatter is the first --- ... --- block at top of file
      const frontmatter = content.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
      expect(frontmatter).toContain("argument-hint:");
    });
  }

  test("exactly 10 command files exist, including the Korean deploy alias", () => {
    expect(commandFiles).toEqual(expectedCommandFiles);
  });

  test("Korean deploy alias participates in command metadata checks", () => {
    expect(commandFiles).toContain("배포.md");
  });
});
