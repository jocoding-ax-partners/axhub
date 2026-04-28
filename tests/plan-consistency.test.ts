// Phase 1 PLAN reconciliation — stale roadmap text must not revive canceled
// plugin-server work after the Phase 6.5 user clarification.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const plan = readFileSync(join(REPO_ROOT, "PLAN.md"), "utf8");
const between = (start: string, end: string): string => plan.slice(plan.indexOf(start), plan.indexOf(end, plan.indexOf(start)));
const milestones = between("## 11. Milestones", "## 12. Risks");
const layout = between("### 16.2 Revised Plugin Layout", "### 16.3 Revised Skill Template");
const scope = between("## 14. NOT in scope", "## 15. What already exists");

describe("Phase 1 PLAN reconciliation", () => {
  test("cancellation decision remains explicit in the audit trail", () => {
    expect(plan).toContain("CANCEL row 7 + 12 + 39 + 40 + 58");
    expect(plan).toContain("§11 milestones 재정렬: M7 삭제");
    expect(plan).toContain("Plugin이 자체 MCP server expose / MCP 호출");
  });

  test("active milestones do not list canceled M7 implementation work", () => {
    expect(milestones).not.toContain("| **M7 (v0.2 scope)**");
    expect(milestones).toContain("M7 removed by Decision Audit Trail row 62");
    expect(milestones).not.toContain("v0.2 (M7) entered");
  });

  test("active scope permanently excludes plugin server work", () => {
    expect(scope).toContain("v0.x 전체에서 영구 제외");
    expect(scope).toContain(".mcp.json");
    expect(scope).toContain("MCP tool naming");
  });

  test("repository layout documents absence of plugin server placeholder", () => {
    expect(layout).toContain("(no .mcp.json)");
    expect(layout).toContain("plugin MCP server placeholder canceled");
    expect(layout).not.toContain("mcp-serve");
  });

  test("current architecture names CLI as the shared surface", () => {
    expect(plan).toContain("항상 ax-hub-cli만 호출");
    expect(plan).toContain("공통 surface는 별도 plugin layer가 아니라 `ax-hub-cli` 자체");
  });
});
