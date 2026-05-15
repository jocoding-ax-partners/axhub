// v0.5.13 / v0.6.2 — settings-merge integration tests (15 cases)
// Calls the compiled axhub-helpers binary with HOME + CLAUDE_PLUGIN_ROOT override
// to exercise the 7-branch decision table via real filesystem I/O.
//
// Exit codes:
//   0 = NoOp  2 = Created  3 = Merged  4 = PreservedOther
//   5 = InvalidJson  6 = PartialSchema  7 = PermissionDenied

import { beforeEach, describe, expect, test } from "bun:test";
import {
  chmodSync,
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

const FORBIDDEN_TONE = /합니다|입니다|시겠어요|드립니다|당신|아이고/;

function makeEnv(tempDir: string, pluginRoot: string): Record<string, string> {
  return {
    ...(process.env as Record<string, string>),
    HOME: tempDir,
    CLAUDE_PLUGIN_ROOT: pluginRoot,
    // XDG_STATE_HOME 명시 → orphan stub 경로 격리 (HOME/.local/state 와 동일하게 고정)
    XDG_STATE_HOME: join(tempDir, ".local", "state"),
  };
}

/** v0.6.2 — --apply 가 기록하는 orphan stub 절대경로. */
function expectedStubPath(tempDir: string): string {
  return join(tempDir, ".local", "state", "axhub-plugin", "orphan-stub-statusline.sh");
}

function runMerge(args: string[], tempDir: string, pluginRoot: string) {
  return spawnSync(HELPER_BIN, ["settings-merge", ...args], {
    encoding: "utf8",
    timeout: 10_000,
    env: makeEnv(tempDir, pluginRoot),
  });
}

describe("v0.5.13/v0.6.2 settings-merge integration — 15 cases", () => {
  let tempDir: string;
  let pluginRoot: string;
  let settingsPath: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), "axhub-merge-"));
    // pluginRoot inside HOME/.claude/plugins/ so --scope auto resolves to User
    pluginRoot = join(tempDir, ".claude", "plugins", "axhub");
    settingsPath = join(tempDir, ".claude", "settings.json");
  });

  // ── Case 1: Branch 1 — file absent ─────────────────────────────────────────

  test("case 1 — branch 1: file absent → exit 2, settings.json created with stub absolute path (v0.6.2)", () => {
    const result = runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);
    expect(result.status).toBe(2);
    expect(existsSync(settingsPath)).toBe(true);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    expect(parsed.statusLine).toBeDefined();
    expect(parsed.statusLine.type).toBe("command");
    // v0.6.2: stub absolute path, NOT ${CLAUDE_PLUGIN_ROOT} literal
    expect(parsed.statusLine.command).toBe(expectedStubPath(tempDir));
    expect(parsed.statusLine.command).not.toContain("${CLAUDE_PLUGIN_ROOT}");
  });

  // ── Case 2: Branch 2 — empty file ──────────────────────────────────────────

  test("case 2 — branch 2: empty file → exit 2, stub absolute path written (v0.6.2)", () => {
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    writeFileSync(settingsPath, "");
    const result = runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);
    expect(result.status).toBe(2);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    expect(parsed.statusLine).toBeDefined();
    // v0.6.2: stub absolute path, NOT ${CLAUDE_PLUGIN_ROOT} literal
    expect(parsed.statusLine.command).toBe(expectedStubPath(tempDir));
    expect(parsed.statusLine.command).not.toContain("${CLAUDE_PLUGIN_ROOT}");
  });

  // ── Case 3: Branch 3 — valid JSON, no statusLine ───────────────────────────

  test("case 3 — branch 3: valid JSON no statusLine → exit 3, statusLine added, otherKey preserved", () => {
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    const original = { otherKey: "preserve-me" };
    writeFileSync(settingsPath, JSON.stringify(original));
    const result = runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);
    expect(result.status).toBe(3);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    expect(parsed.statusLine).toBeDefined();
    expect(parsed.otherKey).toBe("preserve-me");
  });

  // ── Case 4: Branch 4 — axhub-managed statusLine, no-op ────────────────────

  test("case 4 — branch 4: already stub-managed → exit 0, file mtime unchanged (v0.6.2)", () => {
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    // v0.6.2: "managed" means stub absolute path, not the old ${CLAUDE_PLUGIN_ROOT} literal
    const stubCmd = expectedStubPath(tempDir);
    const existing = { statusLine: { type: "command", command: stubCmd, padding: 0 } };
    writeFileSync(settingsPath, JSON.stringify(existing));
    const before = statSync(settingsPath).mtimeMs;
    const result = runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);
    expect(result.status).toBe(0);
    const after = statSync(settingsPath).mtimeMs;
    expect(after).toBe(before);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    expect(parsed.statusLine.command).toBe(stubCmd);
  });

  // ── Case 5: Branch 5 — other plugin command, preserve ──────────────────────

  test("case 5 — branch 5: other plugin command → exit 4, original preserved, stderr warning emitted", () => {
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    const otherCmd = "/usr/local/bin/vscode-plugin/statusline.sh";
    writeFileSync(
      settingsPath,
      JSON.stringify({ statusLine: { type: "command", command: otherCmd, padding: 0 } })
    );
    const result = runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);
    expect(result.status).toBe(4);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    expect(parsed.statusLine.command).toBe(otherCmd);
    expect(result.stderr).toContain("다른 statusLine");
  });

  // ── Case 6: Branch 6 — invalid JSON, no write ──────────────────────────────

  test("case 6 — branch 6: invalid JSON → exit 5, file unchanged, recovery hint in stderr", () => {
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    const brokenContent = "{broken json: true";
    writeFileSync(settingsPath, brokenContent);
    const result = runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);
    expect(result.status).toBe(5);
    // file must NOT be modified
    expect(readFileSync(settingsPath, "utf8")).toBe(brokenContent);
    expect(result.stderr).toContain("JSON syntax error");
  });

  // ── Case 7: Branch 7 — partial statusLine schema ───────────────────────────

  test("case 7 — branch 7: partial statusLine (missing command) → exit 6, preserved", () => {
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    const partial = { statusLine: { type: "command" } }; // no 'command' field
    writeFileSync(settingsPath, JSON.stringify(partial));
    const result = runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);
    expect(result.status).toBe(6);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    // original preserved — no command injected
    expect(parsed.statusLine.command).toBeUndefined();
    expect(result.stderr).toContain("incomplete");
  });

  // ── Case 8: Branch 8 — readonly parent directory (POSIX only) ──────────────

  test("case 8 — readonly parent → permission error (skipIf win32)", () => {
    if (process.platform === "win32") return;

    const roBase = mkdtempSync(join(tmpdir(), "axhub-ro-"));
    const claudeDir = join(roBase, ".claude");
    mkdirSync(claudeDir, { recursive: true });
    chmodSync(claudeDir, 0o444); // no write bit
    const roPluginRoot = join(roBase, ".claude", "plugins", "axhub");

    try {
      const result = runMerge(
        ["--apply", "--scope", "user"],
        roBase,
        roPluginRoot
      );
      // PermissionDenied (exit 7) or lock-acquisition error (exit 1) are both acceptable —
      // neither is a successful merge.
      expect(result.status).not.toBeNull();
      expect([1, 7]).toContain(result.status as number);
      // No settings.json must have been created/modified
      expect(existsSync(join(claudeDir, "settings.json"))).toBe(false);
    } finally {
      chmodSync(claudeDir, 0o755);
      rmSync(roBase, { recursive: true, force: true });
    }
  });

  // ── Case 9: --dry-run — no write ───────────────────────────────────────────

  test("case 9 — --dry-run: branch computed but no file written", () => {
    // No settings file → would be Branch 1 (Created)
    const result = runMerge(["--dry-run", "--scope", "user"], tempDir, pluginRoot);
    // dry-run still returns the branch exit code
    expect(result.status).toBe(2);
    // Must NOT have written the settings file
    expect(existsSync(settingsPath)).toBe(false);
    expect(result.stderr).toContain("dry-run");
  });

  // ── Case 10: --scope auto detects User from HOME prefix ────────────────────

  test("case 10 — --scope auto: resolves user scope from CLAUDE_PLUGIN_ROOT prefix", () => {
    // pluginRoot = tempDir/.claude/plugins/axhub → starts with HOME/.claude/plugins → User
    const result = runMerge(["--apply", "--scope", "auto"], tempDir, pluginRoot);
    expect(result.status).toBe(2);
    expect(existsSync(settingsPath)).toBe(true);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    expect(parsed.statusLine).toBeDefined();
  });

  // ── Case 11: --scope user explicit override ─────────────────────────────────

  test("case 11 — --scope user explicit: writes to HOME/.claude/settings.json", () => {
    const result = runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);
    expect(result.status).toBe(2);
    expect(existsSync(settingsPath)).toBe(true);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    expect(parsed.statusLine.type).toBe("command");
  });

  // ── Case 12: .bak created before mutation ──────────────────────────────────

  test("case 12 — .bak created before mutation (branch 3)", () => {
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    const original = JSON.stringify({ keepThis: "yes" });
    writeFileSync(settingsPath, original);

    runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);

    const bakPath = join(tempDir, ".claude", "settings.json.bak");
    expect(existsSync(bakPath)).toBe(true);
    // .bak must equal original pre-mutation content
    expect(readFileSync(bakPath, "utf8")).toBe(original);
  });

  // ── Case 13: .bak content equals pre-mutation state ────────────────────────

  test("case 13 — .bak preserves pre-mutation content exactly (branch 3)", () => {
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    const original = JSON.stringify({ nested: { value: 42 }, arr: [1, 2, 3] });
    writeFileSync(settingsPath, original);

    runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot);

    const bakPath = join(tempDir, ".claude", "settings.json.bak");
    const bakParsed = JSON.parse(readFileSync(bakPath, "utf8"));
    // .bak must NOT contain statusLine — it was the pre-mutation snapshot
    expect(bakParsed.statusLine).toBeUndefined();
    expect(bakParsed.nested).toEqual({ value: 42 });
    expect(bakParsed.arr).toEqual([1, 2, 3]);
  });

  // ── Case 14: 해요체 tone check across all warning branches ─────────────────

  test("case 14 — 해요체 tone: stderr never uses 합니다/입니다/시겠어요/드립니다/당신/아이고", () => {
    const stderrSamples: string[] = [];

    // Branch 5 — PreservedOther warning
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    writeFileSync(
      settingsPath,
      JSON.stringify({
        statusLine: { type: "command", command: "/other/plugin.sh", padding: 0 },
      })
    );
    stderrSamples.push(runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot).stderr);

    // Branch 6 — InvalidJson recovery hint
    writeFileSync(settingsPath, "{invalid");
    stderrSamples.push(runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot).stderr);

    // Branch 7 — PartialSchema warning
    writeFileSync(settingsPath, JSON.stringify({ statusLine: { type: "command" } }));
    stderrSamples.push(runMerge(["--apply", "--scope", "user"], tempDir, pluginRoot).stderr);

    for (const stderr of stderrSamples) {
      if (stderr.length > 0) {
        expect(stderr).not.toMatch(FORBIDDEN_TONE);
      }
    }
  });

  // ── Case 15: AC #4 — multi-plugin disambiguation ───────────────────────────
  // Pre-mortem S1 regression: CLAUDE_PLUGIN_ROOT 가 OMC plugin 경로로 설정된
  // 환경에서 --apply 실행 시 settings.json 이 axhub 의 orphan stub 절대경로를
  // 기록해야 해요. OMC 경로나 ${CLAUDE_PLUGIN_ROOT} 리터럴이 아니어야 해요.

  test("case 15 — AC#4: CLAUDE_PLUGIN_ROOT=OMC path → stub absolute path written, not OMC path", () => {
    const omcPluginRoot = "/tmp/fake-omc-root/oh-my-claudecode/4.13.7";
    const isolatedXdgState = join(tempDir, "xdg-state");
    const result = spawnSync(
      join(REPO_ROOT, "target/debug/axhub-helpers"),
      ["settings-merge", "--apply", "--scope", "user"],
      {
        encoding: "utf8",
        timeout: 10_000,
        env: {
          ...(process.env as Record<string, string>),
          HOME: tempDir,
          CLAUDE_PLUGIN_ROOT: omcPluginRoot,
          XDG_STATE_HOME: isolatedXdgState,
        },
      }
    );
    // exit 2 (Created) 또는 3 (Merged)
    expect(result.status).not.toBeNull();
    expect([2, 3]).toContain(result.status as number);
    expect(existsSync(settingsPath)).toBe(true);
    const parsed = JSON.parse(readFileSync(settingsPath, "utf8"));
    const cmd: string = parsed.statusLine.command;
    // stub absolute path 여야 해요
    expect(cmd).toContain("orphan-stub-statusline");
    // OMC 경로가 들어가면 안 돼요
    expect(cmd).not.toContain("fake-omc-root");
    expect(cmd).not.toContain("oh-my-claudecode");
    // ${CLAUDE_PLUGIN_ROOT} 미확장 리터럴이면 안 돼요
    expect(cmd).not.toContain("${CLAUDE_PLUGIN_ROOT}");
    // 절대경로여야 해요
    expect(cmd.startsWith("/")).toBe(true);
  });
});
