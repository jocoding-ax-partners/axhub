// Phase 19 — dep-execution rule: skill-doctor 4th pattern regression lock.
// Verifies that skills declaring `allows-dependency-execution: false` cannot
// embed any of the 4 dep-exec patterns (P1_DIRECT / P2_SHELL_WRAP /
// P3_CHAIN / P4_INDIRECT), and that the allowlist schema is enforced.

import { describe, expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS_DIR = join(REPO_ROOT, "skills");
const FIXTURE_SRC = join(REPO_ROOT, "tests", "fixtures", "skill-doctor");

const runDoctor = (extraArgs: string[] = []) =>
  spawnSync("bun", ["scripts/skill-doctor.ts", "--strict", ...extraArgs], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    timeout: 30_000,
  });

const withTempSkill = (slug: string, content: string, fn: () => void) => {
  const dir = join(SKILLS_DIR, slug);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "SKILL.md"), content, "utf8");
  try {
    fn();
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
};

const fixtureContent = (name: string) =>
  readFileSync(join(FIXTURE_SRC, name), "utf8");

// ── 8 positive fixtures: each should make doctor exit 1 ──────────────────────

describe("dep-execution positive fixtures — doctor must fail", () => {
  const cases: { letter: string; pattern: string }[] = [
    { letter: "a", pattern: "P2_SHELL_WRAP (sh -c)" },
    { letter: "b", pattern: "P2_SHELL_WRAP (eval)" },
    { letter: "c", pattern: "P3_CHAIN (corepack && npm install)" },
    { letter: "d", pattern: "P4_INDIRECT (CMD=npm; $CMD install)" },
    { letter: "e", pattern: "P3_CHAIN (setup; npm i)" },
    { letter: "f", pattern: 'P1_DIRECT ("npm" install)' },
    { letter: "g", pattern: "P1_DIRECT (npm i)" },
    { letter: "h", pattern: "P4_INDIRECT (npx --package npm)" },
  ];

  for (const { letter, pattern } of cases) {
    test(`fixture dep-exec-${letter} (${pattern}) → exit 1`, () => {
      const content = fixtureContent(`dep-exec-${letter}.md`);
      const slug = `_dep-exec-fixture-${letter}`;
      withTempSkill(slug, content, () => {
        const result = runDoctor();
        expect(result.status).toBe(1);
        expect(result.stdout).toContain(`skills/${slug}/SKILL.md: missing dep-execution`);
      });
    });
  }
});

// ── negative cases: should NOT trigger dep-execution failure ──────────────────

describe("dep-execution negative cases — doctor must pass", () => {
  test("echo with npm install text inside string is benign", () => {
    const content = [
      "---",
      "name: _neg-echo",
      "description: negative fixture — echo string contains npm install",
      "multi-step: false",
      "needs-preflight: false",
      "allows-dependency-execution: false",
      "---",
      "",
      "Run the following manually:",
      "",
      "```",
      "npm install foo",
      "```",
      "",
    ].join("\n");
    const slug = "_dep-exec-neg-echo";
    withTempSkill(slug, content, () => {
      const result = runDoctor();
      // code fence (no leading !) should not match — doctor passes for this skill
      const lines = result.stdout.split("\n").filter(Boolean);
      const thisFail = lines.filter((l) => l.includes(slug));
      expect(thisFail.length).toBe(0);
    });
  });

  test("plain prose mentioning npm install (no ! prefix) is benign", () => {
    const content = [
      "---",
      "name: _neg-prose",
      "description: negative fixture — prose only",
      "multi-step: false",
      "needs-preflight: false",
      "allows-dependency-execution: false",
      "---",
      "",
      "After cloning, run npm install in your terminal.",
      "",
    ].join("\n");
    const slug = "_dep-exec-neg-prose";
    withTempSkill(slug, content, () => {
      const result = runDoctor();
      const lines = result.stdout.split("\n").filter(Boolean);
      expect(lines.filter((l) => l.includes(slug)).length).toBe(0);
    });
  });

  test("init SKILL with allows-dependency-execution: false is not flagged by doctor", () => {
    // After the bootstrap-saga refactor, init no longer claims dep-execution.
    // Doctor must not flag init for allowlist concerns since the SKILL does
    // not require an allowlist entry.
    const result = runDoctor();
    const lines = result.stdout.split("\n").filter(Boolean);
    expect(lines.filter((l) => l.includes("skills/init/SKILL.md")).length).toBe(0);
  });

  test("allows-dependency-execution field missing → exit 1 with helpful message", () => {
    const content = [
      "---",
      "name: _neg-missing-field",
      "description: negative fixture — no allows-dependency-execution field",
      "multi-step: false",
      "needs-preflight: false",
      "---",
      "",
      "No dep-exec commands here.",
      "",
    ].join("\n");
    const slug = "_dep-exec-neg-missing";
    withTempSkill(slug, content, () => {
      const result = runDoctor();
      expect(result.status).toBe(1);
      expect(result.stdout).toContain(`skills/${slug}/SKILL.md: missing dep-execution`);
    });
  });
});

// ── allowlist enforcement ─────────────────────────────────────────────────────

describe("dep-execution allowlist enforcement", () => {
  test("allows-dependency-execution: true in non-allowlisted skill → exit 1", () => {
    const content = [
      "---",
      "name: _allowlist-test",
      "description: allowlist test fixture — true but not in allowlist",
      "multi-step: false",
      "needs-preflight: false",
      "allows-dependency-execution: true",
      "---",
      "",
      "This skill claims to allow dep-exec but is not in the allowlist.",
      "",
    ].join("\n");
    const slug = "_dep-exec-allowlist-not-listed";
    withTempSkill(slug, content, () => {
      const result = runDoctor();
      expect(result.status).toBe(1);
      expect(result.stdout).toContain(`skills/${slug}/SKILL.md: missing dep-execution`);
    });
  });

  test("allowlist entry with rationale shorter than 50 chars → exit 1", () => {
    // Create a temp SKILL that claims dep-execution, then patch the allowlist
    // to give that SKILL a too-short rationale. Doctor must reject.
    const slug = "_dep-exec-short-rationale";
    const content = [
      "---",
      `name: ${slug}`,
      "description: short-rationale fixture — true with too-short allowlist rationale",
      "multi-step: false",
      "needs-preflight: false",
      "allows-dependency-execution: true",
      "---",
      "",
      "This skill claims to allow dep-exec; allowlist will inject a short rationale.",
      "",
    ].join("\n");
    const allowlistPath = join(REPO_ROOT, "scripts", "skill-doctor-allowlist.json");
    const original = readFileSync(allowlistPath, "utf8");
    const patched = JSON.stringify({
      allows_dependency_execution: [
        { skill: slug, rationale: "too short" },
      ],
    });
    writeFileSync(allowlistPath, patched, "utf8");
    try {
      withTempSkill(slug, content, () => {
        const result = runDoctor();
        expect(result.status).toBe(1);
        expect(result.stdout).toContain(`skills/${slug}/SKILL.md: missing dep-execution`);
      });
    } finally {
      writeFileSync(allowlistPath, original, "utf8");
    }
  });
});

// ── full doctor --strict passes with clean codebase ──────────────────────────

describe("dep-execution — full doctor gate", () => {
  test("bun run skill:doctor --strict exits 0 on clean codebase", () => {
    const result = runDoctor();
    if (result.status !== 0) {
      process.stderr.write(`skill-doctor --strict output:\n${result.stdout}\n${result.stderr}\n`);
    }
    expect(result.status).toBe(0);
  });
});
