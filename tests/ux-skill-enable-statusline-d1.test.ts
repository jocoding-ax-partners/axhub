import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const SKILL = join(import.meta.dir, "../skills/enable-statusline/SKILL.md");

describe("enable-statusline D1 TTY guard contract", () => {
  test("body documents D1 non-interactive guard (CLAUDE_NO_TTY or non-TTY stdin)", () => {
    const body = readFileSync(SKILL, "utf8");
    expect(body).toMatch(/CLAUDE_NO_TTY|! -t 0|TTY/);
  });
  test("body uses command -v guard before invoking clipboard binaries", () => {
    const body = readFileSync(SKILL, "utf8");
    expect(body).toMatch(/command -v\s+(pbcopy|clip\.exe|xclip)/);
  });
  test("body lists pbcopy/clip.exe/xclip fallback chain", () => {
    const body = readFileSync(SKILL, "utf8");
    expect(body).toContain("pbcopy");
    expect(body).toContain("clip.exe");
    expect(body).toContain("xclip");
  });
});
