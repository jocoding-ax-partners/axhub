import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const skillPath = join(process.cwd(), "skills", "deploy", "SKILL.md");

function deploySkill(): string {
  return readFileSync(skillPath, "utf8");
}

describe("deploy SKILL Step 1.6 — in-flight deploy guard invariants", () => {
  test("Step 1.6 question text present", () => {
    const body = deploySkill();
    expect(body).toContain("이미 배포가 진행 중이에요. 어떻게 할까요?");
  });

  test("Step 1.6 AskUserQuestion has monitor option value", () => {
    const body = deploySkill();
    expect(body).toContain('"value": "monitor"');
  });

  test("Step 1.6 AskUserQuestion has force_new option value", () => {
    const body = deploySkill();
    expect(body).toContain('"value": "force_new"');
  });

  test("Step 1.6 AskUserQuestion has abort option value", () => {
    const body = deploySkill();
    expect(body).toContain('"value": "abort"');
  });

  test("Step 1.6 uses in_flight_deploy.created_at for 60-second window logic", () => {
    const body = deploySkill();
    expect(body).toContain("in_flight_deploy.created_at");
  });
});
