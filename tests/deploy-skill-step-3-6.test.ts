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
      '"$HELPER" deploy-prep --intent deploy --user-utterance'
    );
  });

  // Issue #85: Step 3.6 must use --refresh-in-flight flag so PR B1 selective
  // refresh (preflight cache reuse + resolve fresh fetch) is actually engaged.
  // Without the flag, Step 3.6 was doing full preflight+resolve re-fetch every
  // poll — silent latency regression vs documented intent.
  test("Step 3.6 deploy-prep invocation uses --refresh-in-flight flag", () => {
    const body = deploySkill();
    expect(body).toContain(
      `deploy-prep --intent deploy --user-utterance "$ARGS" --refresh-in-flight --json`
    );
  });
});
