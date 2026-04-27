/**
 * Phase 2 US-105: Opt-in observability envelope.
 *
 * Default OFF. Activated only when AXHUB_TELEMETRY=1 is set in the environment.
 * Writes one JSON line per event to ${XDG_STATE_HOME}/axhub-plugin/usage.jsonl
 * (mode 0600, append). Failures swallowed — telemetry MUST NOT block the hot path.
 *
 * Privacy contract: NEVER record raw command args, secrets, or PII. Only event
 * type, decision class, exit code, version metadata, and session id.
 */

import { homedir } from "node:os";
import { join } from "node:path";
import { appendFile, mkdir } from "node:fs/promises";
import { spawnSync } from "node:child_process";

const PLUGIN_VERSION = "0.1.11";
const HELPER_VERSION = "0.1.11";

let cachedCliVersion: string | null = null;

const resolveCliVersion = (): string => {
  if (cachedCliVersion !== null) return cachedCliVersion;
  try {
    const result = spawnSync("axhub", ["--version"], { encoding: "utf8", timeout: 1000 });
    if (result.status === 0 && result.stdout) {
      const match = result.stdout.match(/(\d+\.\d+\.\d+(?:-[a-z0-9.]+)?)/);
      cachedCliVersion = match ? match[1] : "unknown";
    } else {
      cachedCliVersion = "unknown";
    }
  } catch {
    cachedCliVersion = "unknown";
  }
  return cachedCliVersion;
};

const isEnabled = (): boolean => process.env["AXHUB_TELEMETRY"] === "1";

const stateDir = (): string => {
  const xdg = process.env["XDG_STATE_HOME"];
  if (xdg && xdg.length > 0) return join(xdg, "axhub-plugin");
  return join(homedir(), ".local", "state", "axhub-plugin");
};

export interface MetaEnvelope {
  ts: string;
  session_id: string;
  plugin_version: string;
  cli_version: string;
  helper_version: string;
  event: string;
  [key: string]: unknown;
}

export const emitMetaEnvelope = async (
  fields: { event: string } & Record<string, unknown>
): Promise<void> => {
  if (!isEnabled()) return;
  try {
    const dir = stateDir();
    await mkdir(dir, { recursive: true, mode: 0o700 });
    const envelope: MetaEnvelope = {
      ts: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
      session_id: process.env["CLAUDE_SESSION_ID"] ?? process.env["CLAUDECODE_SESSION_ID"] ?? "unknown",
      plugin_version: PLUGIN_VERSION,
      cli_version: resolveCliVersion(),
      helper_version: HELPER_VERSION,
      ...fields,
    };
    const file = join(dir, "usage.jsonl");
    const oldMask = process.umask(0o077);
    try {
      await appendFile(file, JSON.stringify(envelope) + "\n", { mode: 0o600 });
    } finally {
      process.umask(oldMask);
    }
  } catch {
    // Swallow all errors — telemetry must never break the hot path.
  }
};

// Test-only helper: reset cached cli version (used by telemetry.test.ts to
// isolate runs that mutate process.env between assertions).
export const _resetCliVersionCache = (): void => {
  cachedCliVersion = null;
};
