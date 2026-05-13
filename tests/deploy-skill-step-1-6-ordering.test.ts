import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const skillPath = join(process.cwd(), "skills", "deploy", "SKILL.md");

function deploySkill(): string {
  return readFileSync(skillPath, "utf8");
}

describe("deploy SKILL step ordering invariants (1.6 / 3.6)", () => {
  test("Step 1.6 appears before Render preview card (Step 3)", () => {
    const body = deploySkill();
    const step1_6Index = body.indexOf("Step 1.6");
    const previewCardIndex = body.indexOf("Render preview card");
    expect(step1_6Index).toBeGreaterThan(-1);
    expect(previewCardIndex).toBeGreaterThan(-1);
    expect(step1_6Index).toBeLessThan(previewCardIndex);
  });

  test("Step 3.6 appears before Step 4 consent-deploy", () => {
    const body = deploySkill();
    const step3_6Index = body.indexOf("Step 3.6");
    const step4Index = body.indexOf("Step 4 consent-deploy");
    expect(step3_6Index).toBeGreaterThan(-1);
    expect(step4Index).toBeGreaterThan(-1);
    expect(step3_6Index).toBeLessThan(step4Index);
  });
});
