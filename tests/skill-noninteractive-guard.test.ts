// Phase 12 v0.1.12 regression — locks the non-interactive guard pattern in
// skills/{status,logs}/SKILL.md. If a future skill rewrite removes the guard,
// `/axhub:status` and `/axhub:logs` will hang in subprocess (claude -p) again.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const STATUS_SKILL = join(REPO_ROOT, "skills/status/SKILL.md");
const LOGS_SKILL = join(REPO_ROOT, "skills/logs/SKILL.md");
const DEPLOY_SKILL = join(REPO_ROOT, "skills/deploy/SKILL.md");

describe("Phase 12 v0.1.12 + v0.1.15 — non-interactive guard regression lock", () => {
  test("skills/status/SKILL.md has TTY guard literal `[ -t 1 ]`", () => {
    const content = readFileSync(STATUS_SKILL, "utf8");
    expect(content).toContain("[ -t 1 ]");
  });

  test("skills/status/SKILL.md uses WATCH=--watch / WATCH= toggle", () => {
    const content = readFileSync(STATUS_SKILL, "utf8");
    expect(content).toContain("WATCH=--watch");
    expect(content).toContain("WATCH=;");
  });

  test("skills/logs/SKILL.md has TTY guard literal `[ -t 1 ]`", () => {
    const content = readFileSync(LOGS_SKILL, "utf8");
    expect(content).toContain("[ -t 1 ]");
  });

  test("skills/logs/SKILL.md uses FOLLOW=--follow / FOLLOW= toggle", () => {
    const content = readFileSync(LOGS_SKILL, "utf8");
    expect(content).toContain("FOLLOW=--follow");
    expect(content).toContain("FOLLOW=;");
  });

  test("All three skills check $CI env var for headless detection", () => {
    const status = readFileSync(STATUS_SKILL, "utf8");
    const logs = readFileSync(LOGS_SKILL, "utf8");
    const deploy = readFileSync(DEPLOY_SKILL, "utf8");
    expect(status).toContain('-z "$CI"');
    expect(logs).toContain('-z "$CI"');
    expect(deploy).toContain('-z "$CI"');
  });

  test("All three skills check $CLAUDE_NON_INTERACTIVE for explicit override", () => {
    const status = readFileSync(STATUS_SKILL, "utf8");
    const logs = readFileSync(LOGS_SKILL, "utf8");
    const deploy = readFileSync(DEPLOY_SKILL, "utf8");
    expect(status).toContain('CLAUDE_NON_INTERACTIVE');
    expect(logs).toContain('CLAUDE_NON_INTERACTIVE');
    expect(deploy).toContain('CLAUDE_NON_INTERACTIVE');
  });

  test("skills/deploy/SKILL.md post-deploy chain has TTY guard for `--watch`", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");
    expect(content).toContain("[ -t 1 ]");
    expect(content).toContain("WATCH=--watch");
    expect(content).toContain("WATCH=;");
  });

  test("skills/deploy/SKILL.md does NOT call `axhub deploy status` with raw `--watch` flag", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");
    // post-chain MUST use $WATCH variable, never literal --watch on the status line
    expect(content).not.toMatch(/axhub deploy status dep_\$DEPLOY_ID --watch/);
  });
});
