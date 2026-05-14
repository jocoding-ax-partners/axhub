import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const REPO = join(import.meta.dir, "..");
const SKILL = join(REPO, "skills/enable-statusline/SKILL.md");
const STATUSLINE_SH = join(REPO, "bin/statusline.sh");

describe("enable-statusline SKILL — Phase 0.5.11 ralplan", () => {
  test("exists", () => expect(existsSync(SKILL)).toBe(true));
  test("frontmatter declares multi-step:false, needs-preflight:false, model:haiku", () => {
    const body = readFileSync(SKILL, "utf8");
    expect(body).toMatch(/^multi-step:\s*false/m);
    expect(body).toMatch(/^needs-preflight:\s*false/m);
    expect(body).toMatch(/^model:\s*haiku/m);
  });
  test("body uses 해요체 (no 합니다/입니다/시겠어요/드립니다/당신/아이고)", () => {
    const body = readFileSync(SKILL, "utf8");
    expect(body).not.toMatch(/합니다|입니다|시겠어요|드립니다|당신|아이고/);
  });
  test("body contains canonical wiring snippet bytes from bin/statusline.sh", () => {
    const shellSrc = readFileSync(STATUSLINE_SH, "utf8");
    expect(shellSrc).toContain('"statusLine"');
    const skill = readFileSync(SKILL, "utf8");
    expect(skill).toContain('"statusLine"');
    expect(skill).toContain('${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh');
  });
});
