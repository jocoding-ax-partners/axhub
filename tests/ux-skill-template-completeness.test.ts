// Phase 18 R5/US-1805 — meta-test: every SKILL.md carries all required Phase
// 17/18 patterns per its own frontmatter declaration. Wraps `bun run skill:doctor
// --strict` so authors get the same machine-parseable diagnostic that CI sees.

import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

describe("Phase 18 C3/US-1805 — skill:doctor --strict gate", () => {
  test("bun run skill:doctor --strict exits 0 (all 11 SKILLs complete)", () => {
    const result = spawnSync("bun", ["scripts/skill-doctor.ts", "--strict"], {
      cwd: REPO_ROOT,
      encoding: "utf8",
      timeout: 30000,
    });
    if (result.status !== 0) {
      process.stderr.write(`skill-doctor --strict output:\n${result.stdout}\n${result.stderr}\n`);
    }
    expect(result.status).toBe(0);
  });

  test("skill-doctor diagnostic format: machine-parseable line per finding", () => {
    // Sanity check on the strict mode output shape: every line that's NOT empty
    // matches `skills/<slug>/SKILL.md: missing <pattern>`.
    const result = spawnSync("bun", ["scripts/skill-doctor.ts", "--strict"], {
      cwd: REPO_ROOT,
      encoding: "utf8",
      timeout: 30000,
    });
    const lines = result.stdout.split("\n").filter((l) => l.trim().length > 0);
    for (const line of lines) {
      expect(line).toMatch(/^skills\/[a-z][a-z0-9-]*\/SKILL\.md: missing /);
    }
  });

  test("skill-doctor diagnostic format is exercised with a controlled missing-pattern fixture", () => {
    const fixtureDir = join(REPO_ROOT, "skills", "_doctor-format-fixture");
    try {
      mkdirSync(fixtureDir, { recursive: true });
      writeFileSync(
        join(fixtureDir, "SKILL.md"),
        [
          "---",
          "name: _doctor-format-fixture",
          "description: 이 스킬은 테스트용 AskUserQuestion 누락 fixture 입니다.",
          "---",
          "",
          "AskUserQuestion",
          "",
        ].join("\n"),
      );

      const result = spawnSync("bun", ["scripts/skill-doctor.ts", "--strict"], {
        cwd: REPO_ROOT,
        encoding: "utf8",
        timeout: 30000,
      });

      expect(result.status).toBe(1);
      expect(result.stdout.split("\n").filter(Boolean)).toContain(
        "skills/_doctor-format-fixture/SKILL.md: missing D1 sentinel",
      );
    } finally {
      rmSync(fixtureDir, { recursive: true, force: true });
    }
  });
});
