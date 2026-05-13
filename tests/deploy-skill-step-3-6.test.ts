import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const skillPath = join(process.cwd(), "skills", "deploy", "SKILL.md");

function deploySkill(): string {
  return readFileSync(skillPath, "utf8");
}

describe("deploy SKILL Step 3.6 — refresh-in-flight invariants", () => {
  test("Step 3.6 section is present", () => {
    const body = deploySkill();
    expect(body).toContain("Step 3.6");
  });

  test("Step 3.6 uses AXHUB_REFRESH_IN_FLIGHT env var", () => {
    const body = deploySkill();
    expect(body).toContain("AXHUB_REFRESH_IN_FLIGHT");
  });

  test("Step 3.6 calls deploy-prep for in-flight refresh", () => {
    const body = deploySkill();
    expect(body).toContain(
      "axhub-helpers deploy-prep --intent deploy --user-utterance"
    );
  });
});
