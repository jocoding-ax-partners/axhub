/**
 * spec 006 (AC 7) — SessionStart eager-infra marker gate.
 *
 * The hook calls `$HELPER session-eager-gate` (exit 0 = run, 1 = skip) and gates
 * the THREE eager-infra targets on the result: token-init + Gatekeeper warmup
 * (this shell) and quality-context (inside the helper). Non-axhub projects (no
 * `axhub.yaml` marker) get a zero-footprint session — no token-init, no warmup.
 *
 * Fail-open: any gate exit that is not a clean 0/1 (spawn error / timeout) falls
 * back auth-conditionally on a token-file `.exists()` — authed → run (preserve
 * existing axhub.yaml users), unauthed → skip (stay zero-footprint).
 *
 * The helper binary is stubbed with a bash script whose `session-eager-gate`
 * exit code is controllable, so we can drive every branch deterministically.
 * The download path is NEVER gated (helper is the prerequisite for the gate
 * call) and is short-circuited here via AXHUB_SKIP_AUTODOWNLOAD=1.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { spawn } from "node:child_process";
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const HOOK_PATH = join(process.cwd(), "hooks", "session-start.sh");

interface RunArgs {
  /** Exit code the stub helper returns for `session-eager-gate`. */
  gateExit: number;
  /** When set, a token file is created under XDG_CONFIG_HOME for the fail-open path. */
  tokenPresent?: boolean;
  /** axhub CLI auth json; defaults to an authenticated user so token-init can fire. */
  authJson?: string;
  workdir: string;
}

interface RunResult {
  exitCode: number;
  recordedArgs: string[][];
}

async function runHook(args: RunArgs): Promise<RunResult> {
  const helperLog = join(args.workdir, "helper-args.log");
  const helperBin = join(args.workdir, "bin", "axhub-helpers");
  const xdgConfig = join(args.workdir, "xdg-config");
  await mkdir(join(args.workdir, "bin"), { recursive: true });

  // Stub helper: records argv, returns the configured gate exit, and otherwise
  // mimics the real subcommands the hook touches. `path token-file` returns a
  // path that does NOT exist so the token-init branch is reachable when gated in.
  const stub = `#!/usr/bin/env bash
printf '%s\\n' "$*" >> "${helperLog}"
case "$1" in
  session-eager-gate) exit ${args.gateExit} ;;
  path) echo "${args.workdir}/dummy-token-file" ;;
  session-start) echo '{"systemMessage":"stub session-start"}' ;;
  token-init) ;;
  auth-refresh-bg) ;;
  *) ;;
esac
exit 0
`;
  await writeFile(helperBin, stub);
  await chmod(helperBin, 0o755);

  // uname → Darwin so the warmup branch (macOS-only) is exercised.
  const unameStub = join(args.workdir, "bin", "uname");
  await writeFile(unameStub, `#!/usr/bin/env bash\nprintf 'Darwin\\n'\n`);
  await chmod(unameStub, 0o755);

  // timeout stub (base macOS lacks it) → ignore the seconds arg, exec the rest.
  const timeoutStub = join(args.workdir, "bin", "timeout");
  await writeFile(timeoutStub, `#!/usr/bin/env bash\nshift\nexec "$@"\n`);
  await chmod(timeoutStub, 0o755);

  // axhub CLI stub → authenticated by default so token-init fires when gated in.
  const authJson =
    args.authJson ??
    '{"user_email":"dev@jocodingax.ai","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["deploy:write"]}';
  const axhubStub = join(args.workdir, "bin", "axhub");
  const escaped = authJson.replace(/'/g, "'\"'\"'");
  await writeFile(axhubStub, `#!/usr/bin/env bash\nprintf '%s\\n' '${escaped}'\nexit 0\n`);
  await chmod(axhubStub, 0o755);

  // Optional token file under XDG_CONFIG_HOME for the fail-open auth fallback.
  if (args.tokenPresent) {
    await mkdir(join(xdgConfig, "axhub-plugin"), { recursive: true });
    await writeFile(join(xdgConfig, "axhub-plugin", "token"), "axhub_pat_stub\n");
  }

  const env: NodeJS.ProcessEnv = {
    ...process.env,
    CLAUDE_PLUGIN_ROOT: args.workdir,
    PATH: `${join(args.workdir, "bin")}:/usr/bin:/bin`,
    XDG_CONFIG_HOME: xdgConfig,
    // Keep auth-refresh-bg from adding noise; it is not a gated target.
    AXHUB_AUTH_BG_REFRESH: "0",
  };
  // NOT setting AXHUB_SKIP_AUTODOWNLOAD: the stub helper already exists so the
  // (ungated) download block is skipped anyway, and token-init is itself tied to
  // that opt-out switch — leaving it unset lets the gated token-init path run.
  delete env.AXHUB_SKIP_AUTODOWNLOAD;

  const exitCode = await new Promise<number>((resolve) => {
    const child = spawn("bash", [HOOK_PATH], { env, stdio: ["ignore", "pipe", "pipe"] });
    child.on("exit", (code) => resolve(code ?? -1));
    child.on("error", () => resolve(127));
  });

  await new Promise((r) => setTimeout(r, 100));

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
  return { exitCode, recordedArgs: recorded };
}

const sawWarmup = (calls: string[][]) =>
  calls.some((c) => c.includes("--version") && c.includes("--quiet"));
const sawTokenInit = (calls: string[][]) => calls.some((c) => c.includes("token-init"));
const sawSessionStart = (calls: string[][]) => calls.some((c) => c.includes("session-start"));

describe("SessionStart eager-infra marker gate (spec 006 AC 7)", () => {
  let workdir: string;

  beforeEach(async () => {
    workdir = await mkdtemp(join(tmpdir(), "axhub-marker-gate-"));
  });

  afterEach(async () => {
    await rm(workdir, { recursive: true, force: true });
  });

  test("no marker (gate exit 1) → zero eager footprint: no warmup, no token-init", async () => {
    const { exitCode, recordedArgs } = await runHook({ gateExit: 1, workdir });
    expect(exitCode).toBe(0); // fail-open: hook always exits 0
    expect(sawWarmup(recordedArgs)).toBe(false);
    expect(sawTokenInit(recordedArgs)).toBe(false);
    // Base session-start still runs (helper-runtime notice is NOT a gated target).
    expect(sawSessionStart(recordedArgs)).toBe(true);
  });

  test("marker present (gate exit 0) → eager infra runs: warmup + token-init fire", async () => {
    const { exitCode, recordedArgs } = await runHook({ gateExit: 0, workdir });
    expect(exitCode).toBe(0);
    expect(sawWarmup(recordedArgs)).toBe(true);
    expect(sawTokenInit(recordedArgs)).toBe(true);
    expect(sawSessionStart(recordedArgs)).toBe(true);
  });

  test("gate spawn error (exit 2) + token present → fail-open run", async () => {
    const { exitCode, recordedArgs } = await runHook({
      gateExit: 2,
      tokenPresent: true,
      workdir,
    });
    expect(exitCode).toBe(0);
    expect(sawWarmup(recordedArgs)).toBe(true);
    expect(sawTokenInit(recordedArgs)).toBe(true);
  });

  test("gate spawn error (exit 2) + no token → fail-open skip (zero footprint)", async () => {
    const { exitCode, recordedArgs } = await runHook({
      gateExit: 2,
      tokenPresent: false,
      workdir,
    });
    expect(exitCode).toBe(0);
    expect(sawWarmup(recordedArgs)).toBe(false);
    expect(sawTokenInit(recordedArgs)).toBe(false);
    expect(sawSessionStart(recordedArgs)).toBe(true);
  });
});
