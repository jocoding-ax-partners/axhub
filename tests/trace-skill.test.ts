import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const repoRoot = process.cwd();
const skillPath = join(repoRoot, "skills", "trace", "SKILL.md");
const registryPath = join(repoRoot, "tests", "fixtures", "ask-defaults", "registry.json");

function traceSkill(): string {
  return readFileSync(skillPath, "utf8");
}

function registry(): any {
  return JSON.parse(readFileSync(registryPath, "utf8"));
}

describe("Phase 25 PR 25.4 trace SKILL invariants", () => {
  test("frontmatter preserves expensive-risk routing and preflight contract", () => {
    const body = traceSkill();
    expect(body).toContain("multi-step: true");
    expect(body).toContain("needs-preflight: true");
    expect(body).toContain("model: sonnet");
    expect(body).toContain("allows-dependency-execution: false");
  });

  test("workflow uses helper trace command and the 3-source evidence contract", () => {
    const body = traceSkill();
    expect(body).toContain('axhub-helpers trace --deploy-id=$ID --app "$APP" --json');
    expect(body).toContain("event_log + runtime_log + audit");
    expect(body).toContain("references/error-patterns.md");
  });

  test("ambiguous deploy target is registered as abort in non-interactive mode", () => {
    const entry = registry().trace["최근 Failed 배포가 여러 개예요. 어떤 거 추적할까요?"];
    expect(entry.safe_default).toBe("abort");
    expect(entry.allowed_safe_defaults).toContain("가장 최근");
    expect(entry.rationale).toContain("비대화형");
  });
});
