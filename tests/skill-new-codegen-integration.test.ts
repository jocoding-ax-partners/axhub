// Phase 27.x — `bun run skill:new` scaffold codegen integration.
// Verifies scaffold output contains getPreflightInjectionLine() byte-identical (lite variant).
// Verifies --no-preflight omits the injection line.

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { getPreflightInjectionLine } from "../scripts/codegen-preflight-injection";

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

describe("skill:new scaffold — codegen preflight injection", () => {
  test("needs-preflight: true (default) scaffold contains getPreflightInjectionLine() byte-identical", () => {
    const result = spawnSync(
      "bun",
      ["run", "scripts/skill-new.ts", TEST_SLUG, "--action", "test codegen integration"],
      { cwd: REPO_ROOT, timeout: 30_000 },
    );
    expect(result.status).toBe(0);
    const skillPath = join(REPO_ROOT, "skills", TEST_SLUG, "SKILL.md");
    expect(existsSync(skillPath)).toBe(true);
    const content = readFileSync(skillPath, "utf8");
    expect(content).toContain(getPreflightInjectionLine());
  });

  test("--no-preflight scaffold omits injection line entirely", () => {
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
    expect(content).not.toContain(getPreflightInjectionLine());
    expect(content).not.toContain("axhub-helpers preflight");
  });

  test("bun run skill:doctor --strict passes after scaffold cleanup (existing SKILLs unbroken)", () => {
    // Clean up test fixtures first — scaffolded SKILLs with TODO placeholders would fail
    // quality checks in skill:doctor --strict. The purpose of this test is to verify that
    // our codegen changes don't break the doctor check for the 19 production SKILLs.
    // The byte-identical codegen lock is already verified in test 1.
    cleanupSlug(TEST_SLUG);
    cleanupSlug(NOPRE_SLUG);
    const result = spawnSync("bun", ["run", "scripts/skill-doctor.ts", "--strict"], {
      cwd: REPO_ROOT,
      timeout: 30_000,
    });
    expect(result.status).toBe(0);
  });
});
