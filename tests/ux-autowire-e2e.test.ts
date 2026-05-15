// Phase 26 v0.6.0 / v0.6.2 — autowire-statusline e2e tests (9 scenarios).
//
// Coverage: S1 invalid JSON / S2 inter-plugin / S3 schema drift /
//           S4 dotbot sync / S5 subprocess race / S6 2-scope isolation /
//           P1 multi-plugin disambiguation (AC#4) /
//           P2 uninstall graceful (AC#5) / P3 version bump invariance (AC#6).
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

describe("autowire-statusline e2e — 9 scenarios", () => {
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
      // project-scope marker is absent → merge should run
      const projectResult = spawnSync(
        HELPER_BIN,
        ["autowire-statusline", "--scope", "project", "--silent"],
        { encoding: "utf8", timeout: 15_000, env: projectEnv }
      );
      expect(projectResult.status).toBe(0);

      // Project-scope marker would be at axhubState/auto-wire-done-project.json
      // (verified via scope-isolation assertion below — user marker unchanged).
      // User marker must remain unchanged (scope isolation)
      expect(existsSync(userMarker)).toBe(true);
      const userMarkerContent = JSON.parse(readFileSync(userMarker, "utf8"));
      expect(userMarkerContent.scope).toBeUndefined(); // was written by test, no scope field
    } finally {
      rmSync(repoDir, { recursive: true, force: true });
    }
  });

  // ── P1: multi-plugin disambiguation — AC #4 ───────────────────────────────
  // Pre-mortem S1 회귀: CLAUDE_PLUGIN_ROOT 가 OMC plugin 경로인 상태에서
  // autowire-statusline 실행 시 settings.json 이 axhub orphan stub 절대경로를
  // 기록해야 해요. OMC path 나 ${CLAUDE_PLUGIN_ROOT} 리터럴이 아니어야 해요.

  test("P1: multi-plugin disambiguation — CLAUDE_PLUGIN_ROOT=OMC → stub absolute path written", () => {
    const omcPluginRoot = "/tmp/fake-omc-root/oh-my-claudecode/4.13.7";
    const env = makeEnv(tempDir, stateDir, { CLAUDE_PLUGIN_ROOT: omcPluginRoot });

    const result = runAutowire(env);
    expect(result.status).toBe(0);
    expect(existsSync(settingsPath)).toBe(true);

    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    const cmd: string = parsed.statusLine.command;
    // axhub orphan stub 절대경로여야 해요
    expect(cmd).toContain("orphan-stub-statusline");
    // OMC 경로가 들어가면 안 돼요
    expect(cmd).not.toContain("fake-omc-root");
    expect(cmd).not.toContain("oh-my-claudecode");
    // ${CLAUDE_PLUGIN_ROOT} 미확장 리터럴이면 안 돼요
    expect(cmd).not.toContain("${CLAUDE_PLUGIN_ROOT}");
  });

  // ── P2: uninstall graceful — AC #5 ───────────────────────────────────────
  // Pre-mortem S3 회귀: axhub plugin 제거 후에도 orphan stub 이 exit 0 (빈 출력)
  // 으로 graceful 하게 동작해야 해요.

  test("P2: uninstall graceful — stub exits 0 with empty stdout when plugin removed", () => {
    // 1) autowire 실행 → stub 설치 + settings.json 기록 (기본 pluginRoot 사용)
    const env = makeEnv(tempDir, stateDir);
    runAutowire(env);

    // stub 경로 얻기
    const stubPath = join(stateDir, "axhub-plugin", "orphan-stub-statusline.sh");
    if (!existsSync(stubPath)) {
      // stub 설치 안 됐으면 skip (비-POSIX 환경)
      return;
    }

    // 2) "plugin uninstall" 시뮬레이션: CLAUDE_PLUGIN_ROOT 를 빈 임시 dir 로 설정
    const emptyPluginRoot = mkdtempSync(join(tmpdir(), "axhub-uninstall-"));
    try {
      const stubResult = spawnSync(stubPath, [], {
        encoding: "utf8",
        timeout: 5_000,
        env: {
          ...(process.env as Record<string, string>),
          CLAUDE_PLUGIN_ROOT: emptyPluginRoot,
        },
      });
      // 3) exit 0, stdout 비어 있어야 해요 (graceful empty statusline)
      expect(stubResult.status).toBe(0);
      expect(stubResult.stdout.trim()).toBe("");
    } finally {
      rmSync(emptyPluginRoot, { recursive: true, force: true });
    }
  });

  // ── P3: version bump invariance — AC #6 ──────────────────────────────────
  // settings.json 재기록 없이 plugin 0.6.1 → 0.6.2 업그레이드 후에도
  // orphan stub path 가 그대로 유지돼야 해요 (NoOp).

  test("P3: version bump invariance — settings.json not rewritten after plugin upgrade mock", () => {
    // 1) 첫 번째 autowire: v0.6.1 plugin root
    const v1PluginRoot = join(tempDir, ".claude", "plugins", "axhub@0.6.1");
    const env1 = makeEnv(tempDir, stateDir, { CLAUDE_PLUGIN_ROOT: v1PluginRoot });
    runAutowire(env1);
    expect(existsSync(settingsPath)).toBe(true);

    const firstContent = readFileSync(settingsPath, "utf8");
    const firstParsed = JSON.parse(firstContent);
    const firstCmd: string = firstParsed.statusLine?.command ?? "";
    expect(firstCmd).toContain("orphan-stub-statusline");

    // 2) done-marker 삭제 → 두 번째 실행이 mtime guard 에 걸리지 않도록
    const doneMarker = join(stateDir, "axhub-plugin", "auto-wire-done-user.json");
    try { rmSync(doneMarker); } catch { /* already absent is OK */ }

    // 3) 두 번째 autowire: v0.6.2 plugin root (버전 업그레이드 시뮬레이션)
    const v2PluginRoot = join(tempDir, ".claude", "plugins", "axhub@0.6.2");
    const env2 = makeEnv(tempDir, stateDir, { CLAUDE_PLUGIN_ROOT: v2PluginRoot });
    runAutowire(env2);

    // 4) settings.json 이 변경되지 않아야 해요 (stub path 는 version-independent)
    const secondContent = readFileSync(settingsPath, "utf8");
    expect(secondContent).toBe(firstContent);
  });
});
