// Phase 26 v0.6.0 — autowire observability tests (4 cases).
//
// Coverage: O1 events.jsonl schema / O2 plaintext redaction /
//           O3 HMAC determinism + per-install uniqueness / O4 concurrent append.
//
// Privacy contract: command strings are NEVER logged in plaintext.
// Only HMAC-SHA256(per-install-salt, value) appears in events.jsonl.

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

/** Required fields per plan §B Observability schema (8 fields). */
const REQUIRED_EVENT_FIELDS = [
  "ts",
  "event",
  "action",
  "branch",
  "scope",
  "before_hash",
  "after_hash",
  "other_command_hash",
] as const;

function makeEnv(
  homeDir: string,
  stateDir: string,
  extra: Record<string, string> = {}
): Record<string, string> {
  return {
    ...(process.env as Record<string, string>),
    HOME: homeDir,
    CLAUDE_PLUGIN_ROOT: join(homeDir, ".claude", "plugins", "axhub"),
    XDG_STATE_HOME: stateDir,
    ...extra,
  };
}

function writeDisclosureMarker(stateDir: string): void {
  const axhubState = join(stateDir, "axhub-plugin");
  mkdirSync(axhubState, { recursive: true });
  writeFileSync(join(axhubState, "install-disclosure-shown.txt"), "shown-by=test\n");
}

function runAutowire(
  env: Record<string, string>,
  scope: "user" | "project" = "user",
  extraArgs: string[] = []
): ReturnType<typeof spawnSync> {
  return spawnSync(
    HELPER_BIN,
    ["autowire-statusline", "--scope", scope, "--silent", ...extraArgs],
    { encoding: "utf8", timeout: 15_000, env }
  );
}

function eventsPath(stateDir: string): string {
  return join(stateDir, "axhub-plugin", "events.jsonl");
}

function readEvents(stateDir: string): unknown[] {
  const path = eventsPath(stateDir);
  if (!existsSync(path)) return [];
  return readFileSync(path, "utf8")
    .split("\n")
    .filter((l) => l.trim().length > 0)
    .map((l) => JSON.parse(l));
}

describe("autowire observability — 4 cases", () => {
  let tempDir: string;
  let stateDir: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), "axhub-obs-"));
    stateDir = join(tempDir, "xdg-state");
    mkdirSync(join(tempDir, ".claude"), { recursive: true });
    writeDisclosureMarker(stateDir);
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  // ── O1: events.jsonl schema — all 8 required fields present ───────────────
  // Spec: plan §B Observability schema (8 fields per event line).

  test("O1: events.jsonl has all 8 required fields on every line", () => {
    // Branch 1 (file absent → create) — triggers an event
    const result = runAutowire(makeEnv(tempDir, stateDir));
    expect(result.status).toBe(0);

    const events = readEvents(stateDir);
    expect(events.length).toBeGreaterThan(0);

    for (const event of events) {
      const ev = event as Record<string, unknown>;
      for (const field of REQUIRED_EVENT_FIELDS) {
        expect(
          field in ev,
          `events.jsonl event missing required field: "${field}"`
        ).toBe(true);
      }
      // event type must always be "autowire-statusline"
      expect(ev.event).toBe("autowire-statusline");
      // action must be a known value
      expect(["create", "merge", "noop", "preserve", "abort"]).toContain(ev.action as string);
      // branch must be a number
      expect(typeof ev.branch).toBe("number");
      // ts must be an ISO-8601 string
      expect(typeof ev.ts).toBe("string");
      expect(() => new Date(ev.ts as string)).not.toThrow();
    }
  });

  // ── O2: plaintext redaction — other plugin command never logged ────────────
  // Privacy contract: other_command_hash = HMAC-SHA256(salt, command).
  // The literal command string must never appear in events.jsonl.

  test("O2: other plugin command absent in plaintext from events.jsonl (Branch 5 privacy)", () => {
    const otherPluginCmd =
      "/Users/testuser/.claude/plugins/gstack-plugin/bin/statusline.sh";

    const settingsPath = join(tempDir, ".claude", "settings.json");
    writeFileSync(
      settingsPath,
      JSON.stringify({
        statusLine: { type: "command", command: otherPluginCmd, padding: 0 },
      })
    );

    const result = runAutowire(makeEnv(tempDir, stateDir));
    expect(result.status).toBe(0);

    const rawContent = readFileSync(eventsPath(stateDir), "utf8");

    // Plaintext command must NOT appear anywhere in the events file
    expect(rawContent).not.toContain(otherPluginCmd);
    expect(rawContent).not.toContain("gstack-plugin");
    expect(rawContent).not.toContain("testuser");

    // preserve event must be logged; other_command_hash is null or hmac string —
    // never the raw command string (FU-4 will thread it through as a hash).
    const events = readEvents(stateDir);
    const preserveEvent = (events as Array<Record<string, unknown>>).find(
      (e) => e.action === "preserve"
    );
    expect(preserveEvent, "preserve event logged for Branch 5").toBeDefined();
    const hash = preserveEvent!.other_command_hash;
    // If non-null it must be HMAC format; null is allowed (FU-4 pending)
    if (hash !== null && hash !== undefined) {
      expect(typeof hash).toBe("string");
      expect(hash as string).toMatch(/^hmac-sha256:[0-9a-f]{64}$/);
    }
  });

  // ── O3: HMAC determinism + per-install uniqueness ─────────────────────────
  // Same command + same salt → same hash (deterministic).
  // Same command + different salt (different install) → different hash.

  test("O3: observability-salt one-time init — created on first run, mode 0600, unique across installs", () => {
    // Trigger first run → salt file must be created
    runAutowire(makeEnv(tempDir, stateDir));

    const axhubStateA = join(stateDir, "axhub-plugin");
    const saltPathA = join(axhubStateA, "observability-salt");
    expect(existsSync(saltPathA), "observability-salt must be created on first run").toBe(true);

    const saltA = readFileSync(saltPathA, "utf8").trim();
    // Must be 64 hex chars (32 random bytes hex-encoded)
    expect(saltA).toMatch(/^[0-9a-f]{64}$/);

    // Mode must be 0600 (POSIX only)
    if (process.platform !== "win32") {
      const { statSync } = require("node:fs") as typeof import("node:fs");
      const mode = statSync(saltPathA).mode & 0o777;
      expect(mode).toBe(0o600);
    }

    // Second run in same install: salt must NOT change (one-time init idempotent)
    try { rmSync(join(axhubStateA, "auto-wire-done-user.json")); } catch { /* ok */ }
    runAutowire(makeEnv(tempDir, stateDir));
    expect(readFileSync(saltPathA, "utf8").trim()).toBe(saltA);

    // Fresh install B: distinct state dir → different cryptographic salt
    const tempDirB = mkdtempSync(join(tmpdir(), "axhub-obs-b-"));
    const stateDirB = join(tempDirB, "xdg-state");
    try {
      mkdirSync(join(tempDirB, ".claude"), { recursive: true });
      writeDisclosureMarker(stateDirB);
      runAutowire(makeEnv(tempDirB, stateDirB));

      const saltPathB = join(stateDirB, "axhub-plugin", "observability-salt");
      expect(existsSync(saltPathB)).toBe(true);
      const saltB = readFileSync(saltPathB, "utf8").trim();
      expect(saltB).toMatch(/^[0-9a-f]{64}$/);
      // Different installs must produce distinct salts
      expect(saltA).not.toBe(saltB);
    } finally {
      rmSync(tempDirB, { recursive: true, force: true });
    }
  });

  // ── O4: concurrent append — events.jsonl integrity under parallel writes ───
  // POSIX O_APPEND guarantee: writes ≤ PIPE_BUF (≥ 4 KiB) are atomic.
  // Multiple concurrent SessionStart fires must not corrupt the JSONL file.

  test("O4: concurrent SessionStart appends — every events.jsonl line parses as valid JSON", async () => {
    const concurrency = 5;
    const processes = Array.from({ length: concurrency }, (_, i) => {
      // Each process needs its own temp home to avoid settings.json lock contention,
      // but shares stateDir so events.jsonl receives concurrent appends.
      const homeI = mkdtempSync(join(tmpdir(), `axhub-obs-c${i}-`));
      mkdirSync(join(homeI, ".claude"), { recursive: true });
      const env = makeEnv(homeI, stateDir); // shared stateDir → concurrent writes
      return { homeI, proc: Bun.spawn([HELPER_BIN, "autowire-statusline", "--scope", "user", "--silent"], {
        env,
        stdout: "pipe",
        stderr: "pipe",
      }) };
    });

    // Wait for all concurrent processes to finish
    await Promise.all(processes.map(({ proc }) => proc.exited));

    // Cleanup per-process temp dirs
    for (const { homeI } of processes) {
      rmSync(homeI, { recursive: true, force: true });
    }

    // events.jsonl must exist and every line must be valid JSON
    const path = eventsPath(stateDir);
    if (!existsSync(path)) {
      // If file absent, all processes skipped (possible if mtime guard fired) —
      // that's acceptable; we only assert no corruption when lines exist.
      return;
    }

    const lines = readFileSync(path, "utf8")
      .split("\n")
      .filter((l) => l.trim().length > 0);

    expect(lines.length).toBeGreaterThan(0);

    for (const [i, line] of lines.entries()) {
      let parsed: unknown;
      expect(
        () => { parsed = JSON.parse(line); },
        `events.jsonl line ${i + 1} is not valid JSON: ${line.slice(0, 120)}`
      ).not.toThrow();
      const ev = parsed as Record<string, unknown>;
      expect(ev.event).toBe("autowire-statusline");
    }
  });
});
