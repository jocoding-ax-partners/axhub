import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INIT_SKILL = join(REPO_ROOT, "skills/init/SKILL.md");

describe("init SKILL workflow — top-level step numbering uniqueness (bootstrap saga refactor)", () => {
  const content = readFileSync(INIT_SKILL, "utf8");

  function extractWorkflowBody(): string {
    const startMarker = "## Workflow";
    const endMarker = "## NEVER";
    const startIdx = content.indexOf(startMarker);
    const endIdx = content.indexOf(endMarker);
    if (startIdx < 0 || endIdx < 0 || endIdx <= startIdx) {
      throw new Error("Workflow / NEVER markers not found in init SKILL");
    }
    return content.slice(startIdx, endIdx);
  }

  test("each top-level step header appears exactly once (no number collisions)", () => {
    const body = extractWorkflowBody();

    const stepHeaders: number[] = [];
    const re = /^(\d+)\.\s+\*\*/gm;
    let m: RegExpExecArray | null;
    while ((m = re.exec(body)) !== null) {
      const n = parseInt(m[1], 10);
      if (!Number.isNaN(n)) stepHeaders.push(n);
    }

    expect(stepHeaders.length).toBeGreaterThanOrEqual(7);
    expect(new Set(stepHeaders).size).toBe(stepHeaders.length);
    const sorted = [...stepHeaders].sort((a, b) => a - b);
    expect(sorted[0]).toBe(0);
  });

  test("workflow follows the bootstrap saga (template list → select → name → dry-run → execute → clone)", () => {
    const body = extractWorkflowBody();
    expect(body).toContain("axhub apps templates list --json");
    expect(body).toMatch(/axhub apps bootstrap --template/);
    expect(body).toContain("--dry-run");
    expect(body).toContain("--execute");
    expect(body).toContain("repo_full_name");
    expect(body).toContain("git clone");
  });

  test("workflow has no leftover D-prefix dependency-install subsection", () => {
    expect(content).not.toMatch(/^D\d+\.\s+/m);
    expect(content).not.toContain("dependency-plan");
    expect(content).not.toMatch(/inline_session|manual_terminal|package_manager_choice/);
  });
});
