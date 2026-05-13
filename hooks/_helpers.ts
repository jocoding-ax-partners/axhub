// Phase 25 PR 25.3 — shared hook helpers for the TypeScript PostToolUse
// surface. Mirrors `crates/axhub-helpers/src/hook_safety.rs` so the kill
// switch behavior is identical across language boundaries.
//
// Public surface:
//   isHookDisabled(hookName)         — canonical kill switch precedence
//                                       (AXHUB_DISABLE_HOOKS > AXHUB_DISABLE_HOOK > DISABLE_AXHUB)
//   isAxhubDeployCommand(toolInput)  — recognizes `axhub deploy create [...]`
//   emitSystemMessage(message)       — write {"systemMessage":...} to stdout
//   readStdin()                      — read full stdin (Bun + Node fallback)
//
// All helpers are dependency-free so the hook entry stays a single-file ship.

const TRUTHY = new Set(["1", "true", "yes", "on"]);

const LEGACY_WARNING_EMITTED = { value: false };

export function isHookDisabled(hookName: string): boolean {
  if (envTruthy("AXHUB_DISABLE_HOOKS")) return true;
  const list = process.env.AXHUB_DISABLE_HOOK;
  if (list && list.split(",").some((entry) => entry.trim() === hookName)) {
    return true;
  }
  if (envTruthy("DISABLE_AXHUB")) {
    emitLegacyWarningOnce();
    return true;
  }
  return false;
}

function envTruthy(name: string): boolean {
  const value = process.env[name];
  return value !== undefined && TRUTHY.has(value);
}

function emitLegacyWarningOnce(): void {
  if (LEGACY_WARNING_EMITTED.value) return;
  LEGACY_WARNING_EMITTED.value = true;
  process.stderr.write(
    "[axhub] warning: `DISABLE_AXHUB` 는 deprecated 됐어요. v0.8.0 에서 제거 예정 — " +
      "canonical 한 `AXHUB_DISABLE_HOOKS` 또는 `AXHUB_DISABLE_HOOK=<csv>` 로 옮겨주세요.\n",
  );
}

export function isAxhubDeployCommand(toolInput: unknown): boolean {
  if (!toolInput || typeof toolInput !== "object") return false;
  const command = (toolInput as { command?: unknown }).command;
  if (typeof command !== "string") return false;
  return /^\s*axhub\s+deploy\s+create\b/.test(command);
}

export function emitSystemMessage(message: string): void {
  process.stdout.write(JSON.stringify({ systemMessage: message }) + "\n");
}

export async function readStdin(): Promise<string> {
  // Bun runtime: Bun.stdin.text() returns full input as a string.
  const maybeBun = (globalThis as Record<string, unknown>).Bun as
    | { stdin?: { text(): Promise<string> } }
    | undefined;
  if (maybeBun?.stdin?.text) {
    return maybeBun.stdin.text();
  }
  // Node fallback: collect chunks from process.stdin.
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    process.stdin.on("data", (chunk: Buffer) => chunks.push(chunk));
    process.stdin.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    process.stdin.on("error", reject);
  });
}

// Exposed for unit tests — lets the test suite reset the legacy warning flag
// between cases without forcing a process reload.
export function __resetLegacyWarningForTest(): void {
  LEGACY_WARNING_EMITTED.value = false;
}
