/**
 * SessionStart hook → CLAUDE_ENV_FILE propagation.
 *
 * Claude Code exposes CLAUDE_PLUGIN_ROOT to SessionStart hooks but does not
 * reliably set it in the agent's later tool-call shells, so skills that build
 * `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers` (or the Windows
 * `$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe`) fail to locate the helper.
 *
 * The hook appends an `export CLAUDE_PLUGIN_ROOT=…` line to $CLAUDE_ENV_FILE,
 * which Claude Code loads into subsequent tool calls. These tests lock the
 * bash-side contract: the line is written when CLAUDE_ENV_FILE is set, and the
 * hook stays fail-open when it is absent.
 *
 * NOTE: cross-shell propagation to PowerShell tool calls is a Claude Code
 * runtime behavior verified manually on Windows, not in this unit test.
 */

import { describe, expect, test } from "bun:test";
import { spawn } from "node:child_process";
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const HOOK_PATH = join(process.cwd(), "hooks", "session-start.sh");

async function runHook(workdir: string, envFile: string | undefined): Promise<number> {
  // Stub helper: skip eager infra (gate exit 1 → zero-footprint), satisfy the
  // final `exec session-start`.
  const helperBin = join(workdir, "bin", "axhub-helpers");
  await mkdir(join(workdir, "bin"), { recursive: true });
  await writeFile(
    helperBin,
    `#!/usr/bin/env bash
case "$1" in
  session-eager-gate) exit 1 ;;
  session-start) echo '{}' ; exit 0 ;;
  *) exit 0 ;;
esac
`,
  );
  await chmod(helperBin, 0o755);
  // Force the non-Darwin path so the macOS Gatekeeper branch is skipped.
  const unameStub = join(workdir, "bin", "uname");
  await writeFile(unameStub, `#!/usr/bin/env bash\nprintf 'Linux\\n'\n`);
  await chmod(unameStub, 0o755);

  const env: NodeJS.ProcessEnv = {
    ...process.env,
    CLAUDE_PLUGIN_ROOT: workdir,
    PATH: `${join(workdir, "bin")}:/usr/bin:/bin`,
    AXHUB_SKIP_AUTODOWNLOAD: "1",
    CI: "1",
  };
  if (envFile !== undefined) {
    env.CLAUDE_ENV_FILE = envFile;
  } else {
    delete env.CLAUDE_ENV_FILE;
  }

  return new Promise<number>((resolve) => {
    const child = spawn("bash", [HOOK_PATH], { env, stdio: ["ignore", "ignore", "ignore"] });
    child.on("exit", (code) => resolve(code ?? -1));
    child.on("error", () => resolve(127));
  });
}

describe("SessionStart hook CLAUDE_ENV_FILE propagation", () => {
  test("writes export CLAUDE_PLUGIN_ROOT when CLAUDE_ENV_FILE is set", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "axhub-envfile-"));
    try {
      const envFile = join(workdir, "claude-env");
      await writeFile(envFile, "");
      await runHook(workdir, envFile);
      const contents = await readFile(envFile, "utf8");
      expect(contents).toContain(`export CLAUDE_PLUGIN_ROOT='${workdir}'`);
    } finally {
      await rm(workdir, { recursive: true, force: true });
    }
  });

  test("appends without clobbering existing CLAUDE_ENV_FILE lines", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "axhub-envfile-"));
    try {
      const envFile = join(workdir, "claude-env");
      await writeFile(envFile, "export PRESET=keepme\n");
      await runHook(workdir, envFile);
      const contents = await readFile(envFile, "utf8");
      expect(contents).toContain("export PRESET=keepme");
      expect(contents).toContain("export CLAUDE_PLUGIN_ROOT=");
    } finally {
      await rm(workdir, { recursive: true, force: true });
    }
  });

  test("stays fail-open (exit 0) when CLAUDE_ENV_FILE is absent", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "axhub-envfile-"));
    try {
      const code = await runHook(workdir, undefined);
      expect(code).toBe(0);
      // No stray env file was created in the workdir.
      expect(existsSync(join(workdir, "claude-env"))).toBe(false);
    } finally {
      await rm(workdir, { recursive: true, force: true });
    }
  });
});
