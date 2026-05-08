import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

describe("clarify audit feedback helper", () => {
  test("uses packaged axhub-helpers with --prompt and avoids shell hashing", () => {
    const skill = readFileSync(join(REPO_ROOT, "skills/clarify/SKILL.md"), "utf8");

    expect(skill).toContain('"${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers"');
    expect(skill).toContain('audit-clarify --prompt "$ORIGINAL_PROMPT" --chosen "$FINAL_SKILL"');
    expect(skill).not.toContain("shasum");
  });
});
