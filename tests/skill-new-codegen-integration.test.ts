// Phase 27 — `bun run skill:new` scaffold integration (ADR-0013).
// Verifies the scaffold output contains the canonical in-body preflight block
// (needs-preflight: true) and that --no-preflight omits it. The load-time
// `!command` injection + its byte-identical codegen lock are retired.

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { CANONICAL_PREFLIGHT_BLOCK } from "../scripts/preflight-block";

const REPO_ROOT = join(import.meta.dir, "..");
const REGISTRY = join(REPO_ROOT, "tests/fixtures/ask-defaults/registry.json");
const TEST_SLUG = "test-fixture-codegen-int";
const NOPRE_SLUG = "test-fixture-codegen-nopre";

function cleanupSlug(slug: string): void {
  const dir = join(REPO_ROOT, "skills", slug);
  if (existsSync(dir)) rmSync(dir, { recursive: true });
  // remove registry stub
  try {
    const reg = JSON.parse(readFileSync(REGISTRY, "utf8")) as Record<string, unknown>;
    if (slug in reg) {
      delete reg[slug];
      writeFileSync(REGISTRY, JSON.stringify(reg, null, 2) + "\n");
    }
  } catch {
    // ignore
  }
}

beforeAll(() => {
  cleanupSlug(TEST_SLUG);
  cleanupSlug(NOPRE_SLUG);
});

afterAll(() => {
  cleanupSlug(TEST_SLUG);
  cleanupSlug(NOPRE_SLUG);
});

describe("skill:new scaffold — in-body preflight block", () => {
  test("needs-preflight: true (default) scaffold contains the canonical preflight block", () => {
    const result = spawnSync(
      "bun",
      ["run", "scripts/skill-new.ts", TEST_SLUG, "--action", "test codegen integration"],
      { cwd: REPO_ROOT, timeout: 30_000 },
    );
    expect(result.status).toBe(0);
    const skillPath = join(REPO_ROOT, "skills", TEST_SLUG, "SKILL.md");
    expect(existsSync(skillPath)).toBe(true);
    const content = readFileSync(skillPath, "utf8");
    expect(content).toContain(CANONICAL_PREFLIGHT_BLOCK);
  });

  test("--no-preflight scaffold omits the preflight block entirely", () => {
    const result = spawnSync(
      "bun",
      [
        "run",
        "scripts/skill-new.ts",
        NOPRE_SLUG,
        "--no-preflight",
        "--action",
        "test no-preflight",
      ],
      { cwd: REPO_ROOT, timeout: 30_000 },
    );
    expect(result.status).toBe(0);
    const skillPath = join(REPO_ROOT, "skills", NOPRE_SLUG, "SKILL.md");
    expect(existsSync(skillPath)).toBe(true);
    const content = readFileSync(skillPath, "utf8");
    expect(content).not.toContain(CANONICAL_PREFLIGHT_BLOCK);
    expect(content).not.toContain("PREFLIGHT_JSON=$(");
  });

  test("bun run skill:doctor --strict passes after scaffold cleanup (existing SKILLs unbroken)", () => {
    // Clean up test fixtures first — scaffolded SKILLs with TODO placeholders would fail
    // quality checks in skill:doctor --strict. The purpose of this test is to verify that
    // our scaffold changes don't break the doctor check for the production SKILLs.
    cleanupSlug(TEST_SLUG);
    cleanupSlug(NOPRE_SLUG);
    const result = spawnSync("bun", ["run", "scripts/skill-doctor.ts", "--strict"], {
      cwd: REPO_ROOT,
      timeout: 30_000,
    });
    expect(result.status).toBe(0);
  });
});
