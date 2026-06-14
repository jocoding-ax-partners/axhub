import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const repoRoot = join(import.meta.dir, "..");

function skillBody(slug: string): string {
  return readFileSync(join(repoRoot, "skills", slug, "SKILL.md"), "utf8");
}

test("onboarding MCP step verifies auth health before claiming the tool is usable", () => {
  const body = skillBody("onboarding");

  expect(body).toContain("claude mcp add --transport http --scope user axhub");
  expect(body).toContain("claude mcp get axhub");
  expect(body).toContain("Needs authentication");
  expect(body).toContain("READY_WITH_USER_ACTION");
});

test("init MCP step reports whether repo-local MCP config was actually written", () => {
  const body = skillBody("init");

  expect(body).toContain("axhub plugin-support mcp-install");
  expect(body).toContain(".mcp.json");
  expect(body).toContain("MCP 도구 설치 결과");
  expect(body).toContain("`9` MCP 설치(선택) → `8` 결과 안내");
});
