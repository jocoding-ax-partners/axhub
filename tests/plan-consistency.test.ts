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

  // 2026-06-10 ralplan(Track H) 으로 rows 61–64(plugin MCP 제외)이 철회됐어요 —
  // helpers stdio MCP(mcp-serve) + .mcp.json 도입. 아래 두 테스트는 stale 한
  // "MCP 영구 제외" prose 를 강제하던 drift-enforcer 였어서, §16.6 supersession
  // 본문을 §14 scope / §16.2 layout 에 반영하고 assertion 도 현실(도입)로 갱신해요.
  test("active scope documents the 2026-06-10 MCP re-introduction (rows 61–64 superseded)", () => {
    expect(scope).toContain("철회");
    expect(scope).toContain(".mcp.json");
    expect(scope).toContain("mcp-serve");
    // backend 데이터 op 를 MCP 로 우회하는 것은 여전히 제외 — CLI 경유 유지.
    expect(scope).toContain("backend 호출은 항상");
  });

  test("repository layout documents the .mcp.json install (rows 61–64 superseded)", () => {
    expect(layout).toContain(".mcp.json");
    expect(layout).toContain("mcp-serve");
    expect(layout).not.toContain("(no .mcp.json)");
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
