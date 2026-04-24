// Phase 2 US-105: Tests for opt-in observability envelope (telemetry.ts).

import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { mkdtempSync, existsSync, readFileSync, statSync } from "node:fs";
import { rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { emitMetaEnvelope, _resetCliVersionCache } from "../src/axhub-helpers/telemetry";

let scratchDir: string;
let originalEnv: Record<string, string | undefined>;

beforeEach(() => {
  scratchDir = mkdtempSync(join(tmpdir(), "axhub-telemetry-"));
  originalEnv = {
    AXHUB_TELEMETRY: process.env["AXHUB_TELEMETRY"],
    XDG_STATE_HOME: process.env["XDG_STATE_HOME"],
    CLAUDE_SESSION_ID: process.env["CLAUDE_SESSION_ID"],
  };
  process.env["XDG_STATE_HOME"] = scratchDir;
  delete process.env["AXHUB_TELEMETRY"];
  process.env["CLAUDE_SESSION_ID"] = "test_session_abc123";
  _resetCliVersionCache();
});

afterEach(() => {
  for (const [k, v] of Object.entries(originalEnv)) {
    if (v === undefined) delete process.env[k];
    else process.env[k] = v;
  }
  rmSync(scratchDir, { recursive: true, force: true });
});

describe("telemetry opt-in gate", () => {
  test("AXHUB_TELEMETRY unset → no file written", async () => {
    await emitMetaEnvelope({ event: "test_event" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    expect(existsSync(file)).toBe(false);
  });

  test("AXHUB_TELEMETRY=0 → no file written", async () => {
    process.env["AXHUB_TELEMETRY"] = "0";
    await emitMetaEnvelope({ event: "test_event" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    expect(existsSync(file)).toBe(false);
  });

  test("AXHUB_TELEMETRY=true (not '1') → no file written", async () => {
    process.env["AXHUB_TELEMETRY"] = "true";
    await emitMetaEnvelope({ event: "test_event" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    expect(existsSync(file)).toBe(false);
  });

  test("AXHUB_TELEMETRY=1 → file created with one line", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    await emitMetaEnvelope({ event: "test_event" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    expect(existsSync(file)).toBe(true);
    const content = readFileSync(file, "utf8");
    const lines = content.trim().split("\n");
    expect(lines.length).toBe(1);
  });
});

describe("telemetry envelope shape", () => {
  test("envelope has all required meta fields", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    await emitMetaEnvelope({ event: "session_start" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    const envelope = JSON.parse(readFileSync(file, "utf8").trim());
    expect(envelope.ts).toBeTypeOf("string");
    expect(envelope.session_id).toBeTypeOf("string");
    expect(envelope.plugin_version).toBeTypeOf("string");
    expect(envelope.cli_version).toBeTypeOf("string");
    expect(envelope.helper_version).toBeTypeOf("string");
    expect(envelope.event).toBe("session_start");
  });

  test("ts is ISO 8601 UTC with Z suffix", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    await emitMetaEnvelope({ event: "test" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    const envelope = JSON.parse(readFileSync(file, "utf8").trim());
    expect(envelope.ts).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
  });

  test("session_id pulled from CLAUDE_SESSION_ID", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    process.env["CLAUDE_SESSION_ID"] = "custom_session_xyz";
    await emitMetaEnvelope({ event: "test" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    const envelope = JSON.parse(readFileSync(file, "utf8").trim());
    expect(envelope.session_id).toBe("custom_session_xyz");
  });

  test("session_id falls back to 'unknown' when env unset", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    delete process.env["CLAUDE_SESSION_ID"];
    delete process.env["CLAUDECODE_SESSION_ID"];
    await emitMetaEnvelope({ event: "test" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    const envelope = JSON.parse(readFileSync(file, "utf8").trim());
    expect(envelope.session_id).toBe("unknown");
  });

  test("plugin_version is hardcoded 0.1.0", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    await emitMetaEnvelope({ event: "test" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    const envelope = JSON.parse(readFileSync(file, "utf8").trim());
    expect(envelope.plugin_version).toBe("0.1.0");
  });

  test("custom payload fields preserved", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    await emitMetaEnvelope({ event: "preauth_check_deny", action: "deploy_create", reason: "missing_token" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    const envelope = JSON.parse(readFileSync(file, "utf8").trim());
    expect(envelope.action).toBe("deploy_create");
    expect(envelope.reason).toBe("missing_token");
  });
});

describe("telemetry file behavior", () => {
  test("multiple emits → multiple lines (append, not overwrite)", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    await emitMetaEnvelope({ event: "first" });
    await emitMetaEnvelope({ event: "second" });
    await emitMetaEnvelope({ event: "third" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    const lines = readFileSync(file, "utf8").trim().split("\n");
    expect(lines.length).toBe(3);
    expect(JSON.parse(lines[0]).event).toBe("first");
    expect(JSON.parse(lines[1]).event).toBe("second");
    expect(JSON.parse(lines[2]).event).toBe("third");
  });

  test("each line is valid JSON", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    await emitMetaEnvelope({ event: "a" });
    await emitMetaEnvelope({ event: "b" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    const lines = readFileSync(file, "utf8").trim().split("\n");
    for (const line of lines) {
      expect(() => JSON.parse(line)).not.toThrow();
    }
  });

  test("file mode is 0600 (private)", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    await emitMetaEnvelope({ event: "test" });
    const file = join(scratchDir, "axhub-plugin", "usage.jsonl");
    const stats = statSync(file);
    // Mask off the file type bits, keep permission bits
    const perms = stats.mode & 0o777;
    expect(perms).toBe(0o600);
  });

  test("dir created with mode 0700", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    await emitMetaEnvelope({ event: "test" });
    const dir = join(scratchDir, "axhub-plugin");
    const stats = statSync(dir);
    const perms = stats.mode & 0o777;
    // Some umask configs produce 0700 exactly; others may yield 0600 if dir
    // permissions are tightened by inherited umask. Accept either as private.
    expect(perms === 0o700 || perms === 0o600).toBe(true);
  });
});

describe("telemetry error swallowing", () => {
  test("does NOT throw when XDG path is unwritable (silent fail)", async () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    process.env["XDG_STATE_HOME"] = "/dev/null/cannot-write-here";
    await expect(emitMetaEnvelope({ event: "test" })).resolves.toBeUndefined();
  });

  test("does NOT throw when called without await (fire-and-forget)", () => {
    process.env["AXHUB_TELEMETRY"] = "1";
    expect(() => {
      void emitMetaEnvelope({ event: "fire_forget" });
    }).not.toThrow();
  });
});
