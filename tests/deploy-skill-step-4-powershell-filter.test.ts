import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const skillPath = join(process.cwd(), "skills", "deploy", "SKILL.md");

function deploySkill(): string {
  return readFileSync(skillPath, "utf8");
}

describe("deploy SKILL Step 4 — PowerShell selective stderr filter invariants", () => {
  test("Step 4 PowerShell creates temp file via New-TemporaryFile", () => {
    const body = deploySkill();
    expect(body).toContain("New-TemporaryFile");
  });

  test("Step 4 PowerShell uses Select-String -Path for pattern match", () => {
    const body = deploySkill();
    expect(body).toContain("Select-String -Path");
  });

  test("Step 4 PowerShell filters validation.deployment_in_progress pattern", () => {
    const body = deploySkill();
    expect(body).toContain("validation\\.deployment_in_progress");
  });

  test("Step 4 PowerShell cleans up temp file with Remove-Item -Force", () => {
    const body = deploySkill();
    expect(body).toContain("Remove-Item $AxhubStderrTmp.FullName -Force");
  });
});
