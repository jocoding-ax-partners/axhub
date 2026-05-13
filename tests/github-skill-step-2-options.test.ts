import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const GITHUB_SKILL = join(REPO_ROOT, "skills/github/SKILL.md");

describe("github SKILL Step 2 — option set membership lock (Plan F′ regression guard)", () => {
  const content = readFileSync(GITHUB_SKILL, "utf8");

  test("Step 2 fenced JSON block has exactly 3 options (list_only / connect / disconnect)", () => {
    const startMarker = "2. **작업을 고르게 해요.**";
    const endMarker = "3. **repo 목록은 read-only 로 실행해요.**";
    const startIdx = content.indexOf(startMarker);
    const endIdx = content.indexOf(endMarker);
    expect(startIdx).toBeGreaterThan(-1);
    expect(endIdx).toBeGreaterThan(startIdx);

    const slice = content.slice(startIdx, endIdx);
    const valueMatches = slice.match(/"value":\s*"[^"]+"/g) ?? [];
    expect(valueMatches.length).toBe(3);

    const values = valueMatches.map((m) => m.match(/"value":\s*"([^"]+)"/)?.[1]);
    expect(values).toEqual(["list_only", "connect", "disconnect"]);
  });

  test("NEVER section forbids 4th option with HTTP 422 reason", () => {
    expect(content).toContain('NEVER add a 4th option');
    expect(content).toContain("git_connection_required");
    expect(content).toContain("HTTP 422");
  });

  test("no '지금은 스킵' or 'skip' option appears as an actual JSON option in Step 2", () => {
    const startMarker = "2. **작업을 고르게 해요.**";
    const endMarker = "3. **repo 목록은 read-only 로 실행해요.**";
    const slice = content.slice(content.indexOf(startMarker), content.indexOf(endMarker));
    expect(slice).not.toMatch(/"label":\s*"지금은 스킵"/);
    expect(slice).not.toMatch(/"value":\s*"skip"/);
  });
});
