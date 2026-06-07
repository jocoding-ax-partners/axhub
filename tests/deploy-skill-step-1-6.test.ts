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

describe("deploy SKILL Step 1.1 — template-required hard stop", () => {
  test("manifest_missing stops instead of spinning on context checks", () => {
    const body = deploySkill();
    expect(body).toContain('state: "template_required"');
    expect(body).toContain('reason: "manifest_missing"');
    expect(body).toContain('do not run more context/file-inspection commands');
    expect(body).toContain('never spin on "컨텍스트 확인" after `manifest_missing`');
  });

  test("template-required branch asks for local manifest initialization, not remote mutation", () => {
    const body = deploySkill();
    expect(body).toContain("React/Vite로 초기화");
    expect(body).toContain("다른 템플릿 선택");
    expect(body).toContain("취소");
    expect(body).toContain("do not call `apps bootstrap`, `apps create`, or `deploy create`");
  });

  test("React/Vite choice writes a local manifest before retrying deploy", () => {
    const body = deploySkill();
    expect(body).toContain("axhub init --framework react-vite --target auto");
    expect(body).toContain("--no-git --json");
    expect(body).toContain("axhub manifest validate --file axhub.yaml --json");
    expect(body).toContain("Do not stop after saying \"프로젝트 확인\"");
    expect(body).toContain("current CLI `axhub init --framework react-vite` is the manifest-only source of truth");
    expect(body).toContain("show the preview approval card");
  });

  test("local manifest writes must be committed before preview", () => {
    const body = deploySkill();
    expect(body).toContain("git status --porcelain --untracked-files=normal -- axhub.yaml apphub.yaml .gitignore");
    expect(body).toContain("do **not** show a preview using the previous commit");
    expect(body).toContain("Route to Step 1.5");
    expect(body).toContain("generated manifest");
    expect(body).toContain("new `commit_sha`");
  });
});
