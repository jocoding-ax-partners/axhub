// Phase 1 PLAN reconciliation — stale roadmap text must not revive canceled
// plugin-server work after the Phase 6.5 user clarification.

import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const plan = readFileSync(join(REPO_ROOT, "PLAN.md"), "utf8");
const packageJson = JSON.parse(readFileSync(join(REPO_ROOT, "package.json"), "utf8")) as { version: string };
const between = (start: string, end: string): string => plan.slice(plan.indexOf(start), plan.indexOf(end, plan.indexOf(start)));
const milestones = between("## 11. Milestones", "## 12. Risks");
const layout = between("### 16.2 Revised Plugin Layout", "### 16.3 Revised Skill Template");
const scope = between("## 14. NOT in scope", "## 15. What already exists");
const schema = between("**§16.12 plugin.json + marketplace.json Schemas**", "**§16.13 bin/axhub-helpers");
const bestPractices = between("### 16.7 Best Practices Audit Checklist", "### 16.8 Phase 5 transition summary");

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

  test("repository layout mirrors current Rust helper primary implementation, not stale Go scaffolding", () => {
    expect(layout).toContain("crates/axhub-helpers/");
    expect(layout).toContain("bin/axhub-helpers");
    expect(layout).toContain("Rust helper primary");
    expect(layout).not.toContain("resolve.go");
    expect(layout).not.toContain("Go binary preferred");
    expect(layout).not.toContain("trust-ux-patterns.ko.md");
  });

  test("current architecture names CLI as the shared surface", () => {
    expect(plan).toContain("항상 ax-hub-cli만 호출");
    expect(plan).toContain("공통 surface는 별도 plugin layer가 아니라 `ax-hub-cli` 자체");
  });
});

describe("PLAN release artifact reconciliation", () => {
  const supplyChain = between("**§16.9 Plugin Supply Chain Integrity**", "**§16.10 CLI Cosign Default-On**");

  test("supply-chain section matches current release artifact names", () => {
    expect(supplyChain).toContain("manifest.json");
    expect(supplyChain).toContain("checksums.txt");
    expect(supplyChain).toContain("manifest.json.sig");
    expect(supplyChain).toContain("checksums.txt.sig");
    expect(supplyChain).toContain("scripts/release/verify-release.sh");
    expect(supplyChain).not.toContain("manifest.sha256");
    expect(supplyChain).not.toContain("manifest.sig");
  });

  test("active supply-chain section names the current Rust helper, not a new Go rewrite", () => {
    expect(supplyChain).toContain("Rust helper binary");
    expect(supplyChain).not.toContain("single multi-command Go binary");
  });
});

describe("PLAN plugin schema reconciliation", () => {
  test("active schema snippet matches current release metadata decisions", () => {
    expect(schema).toContain(`"version": "${packageJson.version}"`);
    expect(schema).toContain('"license": "MIT"');
    expect(schema).toContain('"repository": "https://github.com/jocoding-ax-partners/axhub.git"');
    expect(schema).toContain('"claude-code-plugin"');
    expect(schema).not.toContain("TBD");
  });
});

describe("PLAN best-practices checklist reconciliation", () => {
  test("best-practices section is a status ledger, not an unchecked open-work list", () => {
    expect(bestPractices).toContain("Status ledger as of 2026-04-27");
    expect(bestPractices).not.toMatch(/- \[ \]/);
  });

  test("manual review-only rows are marked as evidence/replaced instead of active implementation gaps", () => {
    expect(bestPractices).toContain("MANUAL EVIDENCE");
    expect(bestPractices).toContain("REPLACED BY GATES");
    expect(bestPractices).toContain("bun run skill:doctor --strict");
    expect(bestPractices).toContain("tests/manifest.test.ts");
  });

  test("skill bodies remain free of the explicit 'you should' anti-pattern", () => {
    for (const dir of readdirSync(join(REPO_ROOT, "skills"))) {
      if (dir.startsWith("_")) continue;
      if (!statSync(join(REPO_ROOT, "skills", dir)).isDirectory()) continue;
      const skill = readFileSync(join(REPO_ROOT, "skills", dir, "SKILL.md"), "utf8");
      expect(skill).not.toMatch(/\byou should\b/i);
    }
  });
});
