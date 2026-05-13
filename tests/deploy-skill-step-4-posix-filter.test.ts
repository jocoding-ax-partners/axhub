import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const skillPath = join(process.cwd(), "skills", "deploy", "SKILL.md");

function deploySkill(): string {
  return readFileSync(skillPath, "utf8");
}

describe("deploy SKILL Step 4 — POSIX selective stderr filter invariants", () => {
  test("Step 4 POSIX creates temp file via mktemp", () => {
    const body = deploySkill();
    expect(body).toContain("AXHUB_STDERR_TMP=$(mktemp)");
  });

  test("Step 4 POSIX filters validation.deployment_in_progress pattern", () => {
    const body = deploySkill();
    expect(body).toContain("validation\\.deployment_in_progress");
  });

  test("Step 4 POSIX forwards other stderr via cat", () => {
    const body = deploySkill();
    expect(body).toContain('cat "$AXHUB_STDERR_TMP" >&2');
  });

  test("Step 4 POSIX cleans up temp file with rm -f", () => {
    const body = deploySkill();
    expect(body).toContain('rm -f "$AXHUB_STDERR_TMP"');
  });
});
