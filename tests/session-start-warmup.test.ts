/**
 * Phase 2 — SessionStart hook Gatekeeper warmup behavior.
 *
 * Spec: .plan/deploy-time-reduction/phase-2-helper-batch-telemetry-inprocess.md §4.1.
 *
 * The hook is expected to:
 *   - Spawn `$HELPER --version --quiet` exactly once on macOS when
 *     AXHUB_GATEKEEPER_WARMUP is unset or != "0".
 *   - Skip the spawn when AXHUB_GATEKEEPER_WARMUP=0 (opt-out).
 *   - Skip the spawn on non-Darwin platforms (Linux/Windows runners).
 *   - Always reach `exec "$HELPER" session-start` afterwards (we observe
 *     this by recording the final argv as well).
 *
 * Tests stub the helper binary with a shell script that appends its argv
 * to a log file, so we can assert what the hook called.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { spawn } from "node:child_process";
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const HOOK_PATH = join(process.cwd(), "hooks", "session-start.sh");

interface RunResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

async function runHookWithStubHelper(args: {
  unameOverride: string;
  warmupEnv?: string;
  workdir: string;
}): Promise<{ result: RunResult; recordedArgs: string[][] }> {
  const helperLog = join(args.workdir, "helper-args.log");
  const helperBin = join(args.workdir, "bin", "axhub-helpers");
  await mkdir(join(args.workdir, "bin"), { recursive: true });

  // Stub helper records argv to the log file. `path token-file` returns a
  // dummy path so the hook's token-init branch short-circuits cleanly.
  const stubScript = `#!/usr/bin/env bash
printf '%s\\n' "$*" >> "${helperLog}"
case "$1" in
  path) echo "${args.workdir}/dummy-token-file" ;;
  session-start) echo '{"systemMessage":"stub session-start"}' ;;
  *) ;;
esac
exit 0
`;
  await writeFile(helperBin, stubScript);
  await chmod(helperBin, 0o755);

  // uname stub so the hook's `[ "$(uname -s)" = "Darwin" ]` branch is
  // controllable from the test without forking a kernel call.
  const unameStub = join(args.workdir, "bin", "uname");
  await writeFile(unameStub, `#!/usr/bin/env bash\nprintf '%s\\n' "${args.unameOverride}"\n`);
  await chmod(unameStub, 0o755);

  // timeout stub — base macOS does not ship `timeout`, only Linux/Homebrew.
  // The hook hard-skips warmup when `timeout` is missing (avoids unbounded
  // hang on a wedged Gatekeeper). For deterministic test coverage we provide
  // a stub that ignores the seconds argument and execs the rest.
  const timeoutStub = join(args.workdir, "bin", "timeout");
  await writeFile(
    timeoutStub,
    `#!/usr/bin/env bash\nshift\nexec "$@"\n`,
  );
  await chmod(timeoutStub, 0o755);

  const env: NodeJS.ProcessEnv = {
    ...process.env,
    CLAUDE_PLUGIN_ROOT: args.workdir,
    PATH: `${join(args.workdir, "bin")}:${process.env.PATH ?? ""}`,
    AXHUB_SKIP_AUTODOWNLOAD: "1",
  };
  if (args.warmupEnv !== undefined) {
    env.AXHUB_GATEKEEPER_WARMUP = args.warmupEnv;
  } else {
    delete env.AXHUB_GATEKEEPER_WARMUP;
  }

  const result = await new Promise<RunResult>((resolve) => {
    const child = spawn("bash", [HOOK_PATH], { env, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (b) => (stdout += b.toString()));
    child.stderr.on("data", (b) => (stderr += b.toString()));
    child.on("exit", (code) => resolve({ exitCode: code ?? -1, stdout, stderr }));
    child.on("error", () => resolve({ exitCode: 127, stdout, stderr }));
  });

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
  return { result, recordedArgs: recorded };
}

describe("SessionStart hook Gatekeeper warmup", () => {
  let workdir: string;

  beforeEach(async () => {
    workdir = await mkdtemp(join(tmpdir(), "axhub-warmup-"));
  });

  afterEach(async () => {
    await rm(workdir, { recursive: true, force: true });
  });

  test("macOS without opt-out invokes --version --quiet warmup", async () => {
    const { result, recordedArgs } = await runHookWithStubHelper({
      unameOverride: "Darwin",
      workdir,
    });
    expect(result.exitCode).toBe(0);
    const sawWarmup = recordedArgs.some(
      (call) => call.includes("--version") && call.includes("--quiet"),
    );
    expect(sawWarmup).toBe(true);
  });

  test("macOS with AXHUB_GATEKEEPER_WARMUP=0 skips warmup", async () => {
    const { result, recordedArgs } = await runHookWithStubHelper({
      unameOverride: "Darwin",
      warmupEnv: "0",
      workdir,
    });
    expect(result.exitCode).toBe(0);
    const sawWarmup = recordedArgs.some(
      (call) => call.includes("--version") && call.includes("--quiet"),
    );
    expect(sawWarmup).toBe(false);
  });

  test("Linux skips warmup regardless of opt-out flag", async () => {
    const { result, recordedArgs } = await runHookWithStubHelper({
      unameOverride: "Linux",
      workdir,
    });
    expect(result.exitCode).toBe(0);
    const sawWarmup = recordedArgs.some(
      (call) => call.includes("--version") && call.includes("--quiet"),
    );
    expect(sawWarmup).toBe(false);
  });

  test("hook still reaches session-start exec after warmup", async () => {
    const { recordedArgs } = await runHookWithStubHelper({
      unameOverride: "Darwin",
      workdir,
    });
    const sawSessionStart = recordedArgs.some((call) => call.includes("session-start"));
    expect(sawSessionStart).toBe(true);
  });
});
