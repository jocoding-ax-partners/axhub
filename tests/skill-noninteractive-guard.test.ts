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
const AUTH_SKILL = join(REPO_ROOT, "skills/auth/SKILL.md");
const GITHUB_SKILL = join(REPO_ROOT, "skills/github/SKILL.md");

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

  test("device-flow fast-exit guidance does not promise stale approval resume", () => {
    const init = readFileSync(INIT_SKILL, "utf8");
    const github = readFileSync(GITHUB_SKILL, "utf8");

    for (const content of [init, github]) {
      expect(content).toContain("device_code_issued");
      expect(content).toContain("새 device flow");
      expect(content).toContain("internal `device_code` 를 노출하지 않기 때문");
      expect(content).not.toContain("승인한 뒤 터미널에서 직접");
    }
  });

  test("init/github do not reintroduce pre-0.15.3 detach wrappers", () => {
    for (const path of [INIT_SKILL, GITHUB_SKILL]) {
      const content = readFileSync(path, "utf8");
      expect(content).not.toContain("nohup axhub apps");
      expect(content).not.toContain("disown 2>/dev/null");
      expect(content).not.toContain("BOOT_LOG=");
      expect(content).not.toContain("GIT_LOG=");
    }
  });

  test("auth fresh device-flow lane matches the CLI's stdout gate", () => {
    const content = readFileSync(AUTH_SKILL, "utf8");
    expect(content).toContain("stdout non-TTY");
    expect(content).toContain("`--no-input` / `--non-interactive`");
    expect(content).toContain("PTY harness");
    expect(content).toContain("fresh device flow");
    expect(content).not.toContain("fresh device flow 시작 전에 exit 65");
    expect(content).toContain("axhub auth login --force --no-browser --json $AUTH_EXTRA");
  });

  test("device-flow follow-up references point at committed docs", () => {
    for (const path of [AUTH_SKILL, INIT_SKILL, GITHUB_SKILL]) {
      const content = readFileSync(path, "utf8");
      expect(content).not.toContain(".omc/plans/device-flow-agent-completion-gap.md");
      expect(content).toContain("docs/superpowers/specs/2026-05-25-github-device-flow-surface-design.md");
    }
  });

  test("status/logs/deploy/verify keep the D1 non-interactive AskUserQuestion guard", () => {
    for (const path of [STATUS_SKILL, LOGS_SKILL, DEPLOY_SKILL, VERIFY_SKILL]) {
      const content = readFileSync(path, "utf8");
      expect(content).toContain("[ -t 1 ]");
      expect(content).toContain("CLAUDE_NON_INTERACTIVE");
    }
  });
});
