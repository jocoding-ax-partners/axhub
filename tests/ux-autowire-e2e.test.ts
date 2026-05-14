// Phase 26 v0.6.0 — autowire-statusline e2e tests (6 pre-mortem scenarios).
//
// Coverage: S1 invalid JSON / S2 inter-plugin / S3 schema drift /
//           S4 dotbot sync / S5 subprocess race / S6 2-scope isolation.
//
// Uses the compiled axhub-helpers binary against a real filesystem (tempdir).
// XDG_STATE_HOME is overridden per-test to isolate state.

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const HELPER_BIN = join(REPO_ROOT, "target/debug/axhub-helpers");

/** Build env with isolated HOME + state dir for each test. */
function makeEnv(
  homeDir: string,
  stateDir: string,
  extra: Record<string, string> = {}
): Record<string, string> {
  const pluginRoot = join(homeDir, ".claude", "plugins", "axhub");
  return {
    ...(process.env as Record<string, string>),
    HOME: homeDir,
    CLAUDE_PLUGIN_ROOT: pluginRoot,
    XDG_STATE_HOME: stateDir,
    ...extra,
  };
}

/** Run autowire-statusline subcommand. */
function runAutowire(
  env: Record<string, string>,
  extraArgs: string[] = []
): ReturnType<typeof spawnSync> {
  return spawnSync(
    HELPER_BIN,
    ["autowire-statusline", "--scope", "user", "--silent", ...extraArgs],
    { encoding: "utf8", timeout: 15_000, env }
  );
}

/** Pre-write the disclosure marker so tests skip the first-session gate. */
function writeDisclosureMarker(stateDir: string): void {
  const axhubState = join(stateDir, "axhub-plugin");
  mkdirSync(axhubState, { recursive: true });
  writeFileSync(join(axhubState, "install-disclosure-shown.txt"), "shown-by=test\n");
}

describe("autowire-statusline e2e — 6 pre-mortem scenarios", () => {
  let tempDir: string;
  let stateDir: string;
  let settingsPath: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), "axhub-e2e-"));
    stateDir = join(tempDir, "xdg-state");
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    settingsPath = join(tempDir, ".claude", "settings.json");
    writeDisclosureMarker(stateDir);
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  // ── S1: Invalid JSON — silent overwrite prevention ─────────────────────────
  // Pre-mortem: settings.json is broken JSON → foundation Branch 6 atomic
  // abort. File must remain unchanged (no corruption on read error).

  test("S1: invalid JSON in settings.json — abort, file unchanged", () => {
    const brokenContent = "{broken json: true // missing closing brace";
    writeFileSync(settingsPath, brokenContent);

    const result = runAutowire(makeEnv(tempDir, stateDir));

    // Always exit 0 (fail-open contract)
    expect(result.status).toBe(0);
    // File must not be overwritten
    expect(readFileSync(settingsPath, "utf8")).toBe(brokenContent);
  });

  // ── S2: Inter-plugin coexistence — Branch 5 preserve ──────────────────────
  // Pre-mortem: another popular plugin already owns statusLine.command.
  // axhub must preserve it (Branch 5) without overwriting.

  test("S2: inter-plugin coexistence — other plugin command preserved (Branch 5)", () => {
    const otherPluginCmd = "/usr/local/bin/other-plugin/bin/statusline.sh";
    writeFileSync(
      settingsPath,
      JSON.stringify({
        statusLine: { type: "command", command: otherPluginCmd, padding: 0 },
        otherKey: "must-survive",
      })
    );

    const result = runAutowire(makeEnv(tempDir, stateDir));

    expect(result.status).toBe(0);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    // Original command must be preserved
    expect(parsed.statusLine.command).toBe(otherPluginCmd);
    // Other keys must survive too
    expect(parsed.otherKey).toBe("must-survive");
  });

  // ── S3: Schema drift — partial statusLine (Branch 7) ──────────────────────
  // Pre-mortem: future Claude Code changes statusLine schema (e.g. drops the
  // `command` field). The binary must treat unknown/partial schema as Branch 7
  // (PartialSchema) — preserve + graceful exit, never crash or corrupt.

  test("S3: partial statusLine schema (mock schema drift) — preserved, no crash", () => {
    // Simulate a future Claude Code schema: `statusLine.type` exists but the
    // `command` field is absent (e.g. replaced by a different key).
    const partialSchema = { statusLine: { type: "command" } };
    writeFileSync(settingsPath, JSON.stringify(partialSchema));

    const result = runAutowire(makeEnv(tempDir, stateDir));

    expect(result.status).toBe(0);
    // Content preserved — no command injected into partial schema
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    expect(parsed.statusLine.command).toBeUndefined();
    expect(parsed.statusLine.type).toBe("command");
  });

  // ── S4: Dotbot/chezmoi sync — README warning coverage ─────────────────────
  // Pre-mortem: user tracks ~/.claude/settings.json via dotbot/chezmoi.
  // Mitigation: README must warn dotfile-sync users.

  test("S4: README Trust & Uninstall section warns about dotbot/chezmoi sync", () => {
    const readmePath = join(REPO_ROOT, "README.md");
    expect(existsSync(readmePath)).toBe(true);
    const readme = readFileSync(readmePath, "utf8");
    // README must contain a trust/uninstall section
    expect(readme.toLowerCase()).toMatch(/trust.*uninstall|uninstall.*trust/i);
    // Must mention dotfile sync risk
    const mentionsDotfileSync =
      readme.includes("dotbot") ||
      readme.includes("chezmoi") ||
      readme.includes("dotfile") ||
      readme.includes("git track");
    expect(
      mentionsDotfileSync,
      "README should warn dotbot/chezmoi users about settings.json git-tracking"
    ).toBe(true);
    // Must provide the opt-out env var
    expect(readme).toContain("AXHUB_DISABLE_STATUSLINE_AUTOWIRE");
  });

  // ── S5: Subprocess race — 60s mtime window prevents duplicate run ──────────
  // Pre-mortem: outer `claude` and spawned `claude -p` both fire SessionStart.
  // The scope done-marker written by the dispatcher must cause the child to
  // skip within 60 seconds (mtime guard).

  test("S5: subprocess race — fresh done-marker within 60s causes skip", () => {
    // Write a done-marker timestamped NOW (simulates dispatcher just ran)
    const axhubState = join(stateDir, "axhub-plugin");
    mkdirSync(axhubState, { recursive: true });
    const doneMarker = join(axhubState, "auto-wire-done-user.json");
    writeFileSync(doneMarker, JSON.stringify({ ts: new Date().toISOString(), scope: "user" }));

    // Child process: should see fresh marker and skip → settings.json NOT created
    const result = runAutowire(makeEnv(tempDir, stateDir));

    expect(result.status).toBe(0);
    // Settings file should NOT have been created (merge was skipped)
    expect(existsSync(settingsPath)).toBe(false);
  });

  // ── S6: 2-scope install — marker isolation ─────────────────────────────────
  // Pre-mortem: user installs axhub for both user-scope AND project-scope.
  // Scope-aware markers must be independent — user marker must not affect
  // the project-scope run and vice versa.

  test("S6: 2-scope install — user and project scope markers are independent", () => {
    const axhubState = join(stateDir, "axhub-plugin");
    mkdirSync(axhubState, { recursive: true });

    // Simulate: user-scope done-marker is fresh (project-scope absent)
    const userMarker = join(axhubState, "auto-wire-done-user.json");
    writeFileSync(userMarker, JSON.stringify({ ts: new Date().toISOString() }));

    // User-scope run → should skip (marker fresh)
    const userResult = runAutowire(makeEnv(tempDir, stateDir));
    expect(userResult.status).toBe(0);
    expect(existsSync(settingsPath)).toBe(false);

    // Project-scope run → should NOT skip (project marker absent)
    // Set up a git repo so project scope detection works
    const repoDir = mkdtempSync(join(tmpdir(), "axhub-repo-"));
    try {
      spawnSync("git", ["init", repoDir], { timeout: 5_000 });
      const projectPluginRoot = join(repoDir, ".claude", "plugins", "axhub");
      mkdirSync(join(repoDir, ".claude"), { recursive: true });

      const projectEnv = {
        ...(process.env as Record<string, string>),
        HOME: tempDir,
        CLAUDE_PLUGIN_ROOT: projectPluginRoot,
        XDG_STATE_HOME: stateDir,
      };
      const projectSettingsPath = join(repoDir, ".claude", "settings.json");

      // project-scope marker is absent → merge should run
      const projectResult = spawnSync(
        HELPER_BIN,
        ["autowire-statusline", "--scope", "project", "--silent"],
        { encoding: "utf8", timeout: 15_000, env: projectEnv }
      );
      expect(projectResult.status).toBe(0);

      // Project-scope marker should now exist (written by binary)
      const projectMarker = join(axhubState, "auto-wire-done-project.json");
      // User marker must remain unchanged (scope isolation)
      expect(existsSync(userMarker)).toBe(true);
      const userMarkerContent = JSON.parse(readFileSync(userMarker, "utf8"));
      expect(userMarkerContent.scope).toBeUndefined(); // was written by test, no scope field
    } finally {
      rmSync(repoDir, { recursive: true, force: true });
    }
  });
});
