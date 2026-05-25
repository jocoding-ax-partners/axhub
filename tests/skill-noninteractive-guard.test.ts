// v0.15.3 — watch/follow agent-safety moved from a manual skill-side toggle to
// CLI auto-degrade. axhub-cli 0.15.3+ detects non-TTY/agent context and degrades
// `--watch` / `--follow` to a single snapshot, so skills pass the flag directly
// instead of branching on `WATCH=--watch` / `FOLLOW=--follow`. This lock prevents
// (a) re-introducing the obsolete manual toggle and (b) dropping the documented
// CLI-version dependency note that justifies passing the flag unconditionally.
//
// History: replaces the Phase 12 v0.1.12 / v0.1.15 manual non-interactive guard
// regression lock. The hang protection now lives in the CLI, not the skill body;
// the D1 AskUserQuestion guard (`[ -t 1 ]` + `$CLAUDE_NON_INTERACTIVE`) is a
// separate concern and must still survive (asserted at the end).

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const STATUS_SKILL = join(REPO_ROOT, "skills/status/SKILL.md");
const LOGS_SKILL = join(REPO_ROOT, "skills/logs/SKILL.md");
const DEPLOY_SKILL = join(REPO_ROOT, "skills/deploy/SKILL.md");
const VERIFY_SKILL = join(REPO_ROOT, "skills/verify/SKILL.md");
const INIT_SKILL = join(REPO_ROOT, "skills/init/SKILL.md");

const AUTO_DEGRADE_NOTE = "axhub-cli 0.15.3";

describe("v0.15.3 — watch/follow CLI auto-degrade contract", () => {
  test("status passes --watch directly and documents the 0.15.3 auto-degrade dependency", () => {
    const content = readFileSync(STATUS_SKILL, "utf8");
    expect(content).toContain("--watch --json");
    expect(content).toContain(AUTO_DEGRADE_NOTE);
  });

  test("status no longer uses the obsolete manual WATCH= toggle", () => {
    const content = readFileSync(STATUS_SKILL, "utf8");
    expect(content).not.toContain("WATCH=--watch");
    expect(content).not.toContain("WATCH=;");
  });

  test("logs passes --follow directly and documents the 0.15.3 auto-degrade dependency", () => {
    const content = readFileSync(LOGS_SKILL, "utf8");
    expect(content).toContain("--follow --source");
    expect(content).toContain(AUTO_DEGRADE_NOTE);
  });

  test("logs no longer uses the obsolete manual FOLLOW= toggle", () => {
    const content = readFileSync(LOGS_SKILL, "utf8");
    expect(content).not.toContain("FOLLOW=--follow");
    expect(content).not.toContain("FOLLOW=;");
  });

  test("deploy post-deploy chain passes --watch directly and documents the 0.15.3 auto-degrade dependency", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");
    expect(content).toContain("--watch --json");
    expect(content).toContain(AUTO_DEGRADE_NOTE);
  });

  test("deploy no longer uses the obsolete manual WATCH= toggle", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");
    expect(content).not.toContain("WATCH=--watch");
    expect(content).not.toContain("WATCH=;");
  });

  test("verify documents the 0.15.3 auto-degrade dependency for any watch/follow call", () => {
    const content = readFileSync(VERIFY_SKILL, "utf8");
    expect(content).toContain(AUTO_DEGRADE_NOTE);
  });

  test("init bootstrap passes --watch directly and dropped the obsolete manual WATCH= toggle", () => {
    const content = readFileSync(INIT_SKILL, "utf8");
    expect(content).toContain("--execute --yes --watch --json");
    expect(content).toContain(AUTO_DEGRADE_NOTE);
    expect(content).not.toContain("WATCH=--watch");
    expect(content).not.toContain("WATCH=;");
  });

  test("status/logs/deploy/verify keep the D1 non-interactive AskUserQuestion guard", () => {
    for (const path of [STATUS_SKILL, LOGS_SKILL, DEPLOY_SKILL, VERIFY_SKILL]) {
      const content = readFileSync(path, "utf8");
      expect(content).toContain("[ -t 1 ]");
      expect(content).toContain("CLAUDE_NON_INTERACTIVE");
    }
  });
});
