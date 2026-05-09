/**
 * Phase 3.5 — SessionStart hook auth-refresh-bg detached spawn.
 *
 * Spec: .plan/deploy-time-reduction/phase-3-client-cascade-reduced.md §3.3.
 *
 * Hook expected behavior:
 *   - axhub CLI reports UNAUTHORIZED → fire `nohup helper auth-refresh-bg & disown`.
 *   - axhub CLI reports authenticated → no spawn.
 *   - axhub CLI absent on PATH → no spawn (best-effort skip).
 *   - AXHUB_AUTH_BG_REFRESH=0 → no spawn (kill switch).
 *   - exec session-start always runs at the end.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { spawn } from "node:child_process";
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const HOOK_PATH = join(process.cwd(), "hooks", "session-start.sh");

interface RunArgs {
  authJson: string | null; // null = axhub CLI absent on PATH
  warmupEnv?: string; // "0" disables Gatekeeper warmup
  refreshEnv?: string; // "0" disables auth-refresh-bg
  workdir: string;
}

interface RunResult {
  exitCode: number;
  helperLog: string[][];
}

async function runHook(args: RunArgs): Promise<RunResult> {
  const helperLog = join(args.workdir, "helper-args.log");
  const helperBin = join(args.workdir, "bin", "axhub-helpers");
  await mkdir(join(args.workdir, "bin"), { recursive: true });

  const helperStub = `#!/usr/bin/env bash
printf '%s\\n' "$*" >> "${helperLog}"
case "$1" in
  path) echo "${args.workdir}/dummy-token-file" ;;
  session-start) echo '{"systemMessage":"stub session-start"}' ;;
  auth-refresh-bg) sleep 0.1 ;;
  *) ;;
esac
exit 0
`;
  await writeFile(helperBin, helperStub);
  await chmod(helperBin, 0o755);

  const unameStub = join(args.workdir, "bin", "uname");
  await writeFile(unameStub, `#!/usr/bin/env bash\nprintf 'Linux\\n'\n`);
  await chmod(unameStub, 0o755);

  if (args.authJson !== null) {
    const axhubStub = join(args.workdir, "bin", "axhub");
    const escaped = args.authJson.replace(/'/g, "'\"'\"'");
    await writeFile(
      axhubStub,
      `#!/usr/bin/env bash\nprintf '%s\\n' '${escaped}'\nexit 0\n`,
    );
    await chmod(axhubStub, 0o755);
  }

  const env: NodeJS.ProcessEnv = {
    ...process.env,
    CLAUDE_PLUGIN_ROOT: args.workdir,
    // Isolate PATH so the host's real axhub binary cannot leak into the
    // hook's `command -v axhub` probe. Keep `/usr/bin:/bin` for `mktemp`,
    // `cat`, `bash`, etc. Test stubs (axhub, uname, axhub-helpers) live in
    // workdir/bin and shadow nothing else.
    PATH: `${join(args.workdir, "bin")}:/usr/bin:/bin`,
    AXHUB_SKIP_AUTODOWNLOAD: "1",
  };
  if (args.warmupEnv !== undefined) env.AXHUB_GATEKEEPER_WARMUP = args.warmupEnv;
  else delete env.AXHUB_GATEKEEPER_WARMUP;
  if (args.refreshEnv !== undefined) env.AXHUB_AUTH_BG_REFRESH = args.refreshEnv;
  else delete env.AXHUB_AUTH_BG_REFRESH;

  const exitCode = await new Promise<number>((resolve) => {
    const child = spawn("bash", [HOOK_PATH], { env, stdio: ["ignore", "pipe", "pipe"] });
    child.on("exit", (code) => resolve(code ?? -1));
    child.on("error", () => resolve(127));
  });

  // Helper bg spawn is detached; give it a tick to flush the argv line.
  await new Promise((r) => setTimeout(r, 250));

  let recorded: string[][] = [];
  try {
    const raw = await readFile(helperLog, "utf8");
    recorded = raw
      .split("\n")
      .filter((line) => line.length > 0)
      .map((line) => line.split(" ").filter((s) => s.length > 0));
  } catch {
    recorded = [];
  }
  return { exitCode, helperLog: recorded };
}

describe("SessionStart hook auth-refresh-bg detached spawn", () => {
  let workdir: string;

  beforeEach(async () => {
    workdir = await mkdtemp(join(tmpdir(), "axhub-bg-refresh-"));
  });

  afterEach(async () => {
    await rm(workdir, { recursive: true, force: true });
  });

  test("UNAUTHORIZED triggers detached auth-refresh-bg spawn", async () => {
    const { exitCode, helperLog } = await runHook({
      authJson: '{"code":"auth.token_missing","detail":"no token"}',
      warmupEnv: "0",
      workdir,
    });
    expect(exitCode).toBe(0);
    const sawRefresh = helperLog.some((c) => c.includes("auth-refresh-bg"));
    expect(sawRefresh).toBe(true);
  });

  test("Authenticated session does NOT trigger refresh", async () => {
    const { exitCode, helperLog } = await runHook({
      authJson:
        '{"user_email":"dev@jocodingax.ai","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["deploy:write"]}',
      warmupEnv: "0",
      workdir,
    });
    expect(exitCode).toBe(0);
    const sawRefresh = helperLog.some((c) => c.includes("auth-refresh-bg"));
    expect(sawRefresh).toBe(false);
  });

  test("axhub CLI absent on PATH skips refresh trigger entirely", async () => {
    const { exitCode, helperLog } = await runHook({
      authJson: null,
      warmupEnv: "0",
      workdir,
    });
    expect(exitCode).toBe(0);
    const sawRefresh = helperLog.some((c) => c.includes("auth-refresh-bg"));
    expect(sawRefresh).toBe(false);
  });

  test("AXHUB_AUTH_BG_REFRESH=0 kills the trigger even when UNAUTHORIZED", async () => {
    const { exitCode, helperLog } = await runHook({
      authJson: '{"code":"auth.token_missing","detail":"no token"}',
      warmupEnv: "0",
      refreshEnv: "0",
      workdir,
    });
    expect(exitCode).toBe(0);
    const sawRefresh = helperLog.some((c) => c.includes("auth-refresh-bg"));
    expect(sawRefresh).toBe(false);
  });

  test("session-start exec still runs after refresh trigger", async () => {
    const { helperLog } = await runHook({
      authJson: '{"code":"auth.token_missing","detail":"no token"}',
      warmupEnv: "0",
      workdir,
    });
    const sawSessionStart = helperLog.some((c) => c.includes("session-start"));
    expect(sawSessionStart).toBe(true);
  });
});
