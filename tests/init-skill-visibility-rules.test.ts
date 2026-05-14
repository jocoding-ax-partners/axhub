import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INIT_SKILL = join(REPO_ROOT, "skills/init/SKILL.md");

describe("init SKILL — Vibe Coder Visibility Rules block (FU-2)", () => {
  const content = readFileSync(INIT_SKILL, "utf8");

  test("has explicit '## Vibe Coder Visibility Rules' section before Workflow", () => {
    const visibilityIdx = content.indexOf("## Vibe Coder Visibility Rules");
    const workflowIdx = content.indexOf("## Workflow");
    expect(visibilityIdx).toBeGreaterThan(-1);
    expect(workflowIdx).toBeGreaterThan(visibilityIdx);
  });

  test("enumerates internal verification primitives that must not be echoed raw", () => {
    const visibilityIdx = content.indexOf("## Vibe Coder Visibility Rules");
    const workflowIdx = content.indexOf("## Workflow");
    const block = content.slice(visibilityIdx, workflowIdx);

    expect(block).toContain("internal verification primitives");
    expect(block).toContain("schema_version");
    expect(block).toContain("templates[].id");
    expect(block).toContain("consent_required_apps_create");
    expect(block).toContain("git_init_required");
    expect(block).toContain("first_commit_required");
    expect(block).toContain("template_required");
    expect(block).toContain("conflict_existing_files");
    expect(block).toContain("next_action");
    expect(block).toContain("recommended_command");
    expect(block).toContain("pending_action_id");
    expect(block).toContain("consent_binding");
    expect(block).toContain("idempotency_key");
  });

  test("provides humanized Korean one-liner template table for each step", () => {
    const visibilityIdx = content.indexOf("## Vibe Coder Visibility Rules");
    const workflowIdx = content.indexOf("## Workflow");
    const block = content.slice(visibilityIdx, workflowIdx);

    expect(block).toMatch(/Step 1 CLI 존재 확인/);
    expect(block).toMatch(/Step 2 template 목록/);
    expect(block).toMatch(/Step 3 template 선택/);
    expect(block).toMatch(/Step 4.*init 실행/);
    expect(block).toMatch(/Step 5.*plan-only/);
    expect(block).toMatch(/Step 6.*next_steps/);
  });

  test("declares AXHUB_INIT_VERBOSE escape hatch for debugging", () => {
    expect(content).toContain("AXHUB_INIT_VERBOSE=1");
  });

  test("references deploy SKILL Visibility Rules as the canonical pattern", () => {
    const visibilityIdx = content.indexOf("## Vibe Coder Visibility Rules");
    const workflowIdx = content.indexOf("## Workflow");
    const block = content.slice(visibilityIdx, workflowIdx);
    expect(block).toContain("deploy SKILL");
  });
});
