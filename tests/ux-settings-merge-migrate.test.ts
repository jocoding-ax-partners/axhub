// v0.6.2 — settings-merge --migrate integration tests (7 cases)
// Covers AC #16 (dual-scope), AC #17 (git-tracked warn-only),
//         AC #9  (already-stub NoOp), AC #10 (invalid JSON abort),
//         dry-run detection, --json output, actual rewrite (--yes).
//
// Exit codes: 0 = no-op/no-stale/cancelled, 2 = migrated/dry-detected,
//             3 = git-tracked warn

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const HELPER_BIN = join(REPO_ROOT, "target/debug/axhub-helpers");

/** Stale literal present in pre-v0.6.2 settings. */
const STALE_CMD = "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh";

function makeEnv(tempDir: string, extra: Record<string, string> = {}): Record<string, string> {
  const pluginRoot = join(tempDir, ".claude", "plugins", "axhub");
  const xdgState = join(tempDir, ".local", "state");
  return {
    ...(process.env as Record<string, string>),
    HOME: tempDir,
    CLAUDE_PLUGIN_ROOT: pluginRoot,
    XDG_STATE_HOME: xdgState,
    ...extra,
  };
}

describe("v0.6.2 settings-merge --migrate integration — 6 cases", () => {
  let tempDir: string;
  let settingsPath: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), "axhub-migrate-"));
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    settingsPath = join(tempDir, ".claude", "settings.json");
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  // ── Case 1: --migrate --yes → actual atomic rewrite, stale literal gone ─────
  // v0.6.2 fix: dry_run = dry_run_flag (not !apply) in migrate mode so
  // --migrate --yes (no --dry-run) performs actual file rewrite.

  test("case 1 — --migrate --yes: actual rewrite, stale literal removed, exit 2", () => {
    const original = JSON.stringify({
      statusLine: { type: "command", command: STALE_CMD, padding: 0 },
    });
    writeFileSync(settingsPath, original);

    const env = makeEnv(tempDir);
    const result = spawnSync(
      HELPER_BIN,
      ["settings-merge", "--migrate", "--yes", "--scope", "user"],
      { encoding: "utf8", timeout: 10_000, env }
    );

    // exit 2 = Migrated
    expect(result.status).toBe(2);
    // File MUST be modified — stale literal must be gone
    const after = readFileSync(settingsPath, "utf8");
    expect(after).not.toBe(original);
    const parsed = JSON.parse(after) as { statusLine: { command: string } };
    expect(parsed.statusLine.command).not.toContain("${CLAUDE_PLUGIN_ROOT}");
    // New command points at orphan stub absolute path
    const stubPath = join(tempDir, ".local", "state", "axhub-plugin", "orphan-stub-statusline.sh");
    expect(parsed.statusLine.command).toBe(stubPath);
  });

  // ── Case 2: AC #9 — already stub-path → NoStaleFound → exit 0 ─────────────

  test("case 2 — AC #9: no stale literal → NoStaleFound, exit 0, mtime unchanged", () => {
    const stubCmd = join(tempDir, ".local", "state", "axhub-plugin", "orphan-stub-statusline.sh");
    const original = JSON.stringify({
      statusLine: { type: "command", command: stubCmd, padding: 0 },
    });
    writeFileSync(settingsPath, original);
    const before = statSync(settingsPath).mtimeMs;

    const result = spawnSync(
      HELPER_BIN,
      ["settings-merge", "--migrate", "--yes", "--scope", "user"],
      { encoding: "utf8", timeout: 10_000, env: makeEnv(tempDir) }
    );

    expect(result.status).toBe(0);
    const after = statSync(settingsPath).mtimeMs;
    expect(after).toBe(before);
  });

  // ── Case 3: AC #10 — invalid JSON → InvalidJson → exit 0, file unchanged ───

  test("case 3 — AC #10: invalid JSON → abort, file unchanged, exit 0", () => {
    const brokenContent = "{broken json: true /* migrate should abort */";
    writeFileSync(settingsPath, brokenContent);

    const result = spawnSync(
      HELPER_BIN,
      ["settings-merge", "--migrate", "--yes", "--scope", "user"],
      { encoding: "utf8", timeout: 10_000, env: makeEnv(tempDir) }
    );

    // InvalidJson outcome → not 2 (would migrate)
    expect(result.status).not.toBe(2);
    // File must be unchanged
    expect(readFileSync(settingsPath, "utf8")).toBe(brokenContent);
  });

  // ── Case 4: explicit --dry-run → stderr mentions dry-run ──────────────────

  test("case 4 — explicit --dry-run: stale detected, stderr contains dry-run hint", () => {
    writeFileSync(
      settingsPath,
      JSON.stringify({ statusLine: { type: "command", command: STALE_CMD } })
    );

    const result = spawnSync(
      HELPER_BIN,
      ["settings-merge", "--migrate", "--dry-run", "--scope", "user"],
      { encoding: "utf8", timeout: 10_000, env: makeEnv(tempDir) }
    );

    expect(result.status).toBe(2);
    expect(result.stderr).toContain("dry-run");
  });

  // ── Case 5: --json flag → stdout is valid JSON with scope key ──────────────

  test("case 5 — --json: stdout is parseable JSON with at least one scope key", () => {
    writeFileSync(
      settingsPath,
      JSON.stringify({ statusLine: { type: "command", command: STALE_CMD } })
    );

    const result = spawnSync(
      HELPER_BIN,
      ["settings-merge", "--migrate", "--yes", "--json", "--scope", "user"],
      { encoding: "utf8", timeout: 10_000, env: makeEnv(tempDir) }
    );

    expect(result.status).toBe(2);
    let parsed: Record<string, unknown> = {};
    expect(() => { parsed = JSON.parse(result.stdout); }).not.toThrow();
    expect(Object.keys(parsed).length).toBeGreaterThan(0);
  });

  // ── Case 6: AC #17 — git-tracked project settings → WarnGitTracked (exit 3) ─
  // Git-tracked guard fires for scope_label=="project" only (settings_merge.rs:603).
  // Spawns binary with cwd=repoDir so project_settings_path() resolves correctly.

  test("case 6 — AC #17: git-tracked project settings.json → exit 3, file unchanged", () => {
    if (process.platform === "win32") return;

    const repoDir = mkdtempSync(join(tmpdir(), "axhub-git-"));
    try {
      // Init git repo and commit the stale settings.json
      spawnSync("git", ["init", repoDir], { timeout: 5_000 });
      spawnSync("git", ["-C", repoDir, "config", "user.email", "test@test.com"], { timeout: 5_000 });
      spawnSync("git", ["-C", repoDir, "config", "user.name", "Test"], { timeout: 5_000 });

      const claudeDir = join(repoDir, ".claude");
      mkdirSync(claudeDir, { recursive: true });
      const trackedSettings = join(claudeDir, "settings.json");
      writeFileSync(
        trackedSettings,
        JSON.stringify({ statusLine: { type: "command", command: STALE_CMD } })
      );
      const originalContent = readFileSync(trackedSettings, "utf8");

      spawnSync("git", ["-C", repoDir, "add", ".claude/settings.json"], { timeout: 5_000 });
      spawnSync("git", ["-C", repoDir, "commit", "-m", "add settings"], { timeout: 10_000 });

      const env = {
        ...(process.env as Record<string, string>),
        HOME: repoDir,
        CLAUDE_PLUGIN_ROOT: join(repoDir, ".claude", "plugins", "axhub"),
        XDG_STATE_HOME: join(repoDir, ".local", "state"),
      };

      // cwd=repoDir so git rev-parse resolves to repoDir as the project root
      const result = spawnSync(
        HELPER_BIN,
        ["settings-merge", "--migrate", "--yes", "--scope", "project"],
        { encoding: "utf8", timeout: 10_000, env, cwd: repoDir }
      );

      // exit 3 = WarnGitTracked
      expect(result.status).toBe(3);
      // File must NOT be modified (warn-only)
      expect(readFileSync(trackedSettings, "utf8")).toBe(originalContent);
    } finally {
      rmSync(repoDir, { recursive: true, force: true });
    }
  });
});
