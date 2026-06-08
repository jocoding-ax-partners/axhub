import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INIT_SKILL = join(REPO_ROOT, "skills/init/SKILL.md");

describe("init SKILL — Vibe Coder Visibility Rules block (bootstrap saga refactor)", () => {
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
    expect(block).toContain("items[].id");
    expect(block).toContain("bootstrap_id");
    expect(block).toContain("status_url");
    expect(block).toContain("app_id");
    expect(block).toContain("deployment_id");
    expect(block).toContain("repo_full_name");
    expect(block).toContain("error_code");
    expect(block).toContain("idempotency_key");
  });

  test("provides humanized Korean one-liner table for each bootstrap saga step", () => {
    const visibilityIdx = content.indexOf("## Vibe Coder Visibility Rules");
    const workflowIdx = content.indexOf("## Workflow");
    const block = content.slice(visibilityIdx, workflowIdx);

    expect(block).toMatch(/Step 1 CLI 존재 확인/);
    expect(block).toMatch(/Step 2 template 목록/);
    expect(block).toMatch(/Step 3 template 선택/);
    expect(block).toMatch(/Step 4.*앱 이름/);
    expect(block).toMatch(/Step 5.*dry-run/);
    expect(block).toMatch(/Step 6.*실행 확인/);
    expect(block).toMatch(/Step 7.*execute/);
    expect(block).toMatch(/Step 8.*clone/);
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
