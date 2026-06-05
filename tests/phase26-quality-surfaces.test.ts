import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");

describe("Phase 26 quality surfaces", () => {
  test("hooks.json wires quality gates and tdd injectors", () => {
    const hooks = JSON.parse(readFileSync(join(root, "hooks/hooks.json"), "utf8"));
    const preBash = hooks.hooks.PreToolUse.find((entry: any) => entry.matcher === "Bash");
    // preauth-check consent gate intentionally unwired from hooks.json (plugin-wide
    // opt-out): bootstrap/deploy --execute no longer blocks on a consent token.
    expect(JSON.stringify(preBash)).not.toContain("preauth-check");
    expect(JSON.stringify(preBash)).toContain("commit-gate");
    const preEdit = hooks.hooks.PreToolUse.find((entry: any) =>
      String(entry.matcher).includes("Edit"),
    );
    expect(JSON.stringify(preEdit)).toContain("tdd-inject");
    const postBash = hooks.hooks.PostToolUse.find((entry: any) => entry.matcher === "Bash");
    expect(JSON.stringify(postBash)).toContain("test-classifier");
  });

  test("quality skills and agents are present", () => {
    for (const slug of [
      "using-axhub-quality",
      "axhub-review",
      "axhub-debug",
      "axhub-ship",
      "axhub-tdd",
      "axhub-plan",
      "karpathy-guidelines",
    ]) {
      expect(existsSync(join(root, "skills", slug, "SKILL.md"))).toBe(true);
    }
    for (const agent of ["axhub-reviewer", "axhub-debugger", "axhub-shipper"]) {
      expect(existsSync(join(root, "agents", `${agent}.md`))).toBe(true);
    }
  });

  test("final eval has at least 100 fixtures and package script", () => {
    const fixturePath = join(root, "tests/eval/megaskill-final.yaml");
    expect(existsSync(fixturePath)).toBe(true);
    const raw = readFileSync(fixturePath, "utf8");
    const fixtureCount = (raw.match(/\n\s*- id:/g) ?? []).length;
    expect(fixtureCount).toBeGreaterThanOrEqual(100);
    const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
    expect(pkg.scripts["eval:megaskill-final"]).toBe("bun scripts/eval-megaskill-final.ts");
    expect(pkg.scripts["lint:hook-inject"]).toBe("bun scripts/lint-hook-inject-shape.ts");
  });
});
