#!/usr/bin/env bun
/** Cross-platform smoke runner for host helper builds. */
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const full = process.argv.includes("--full");
const helperName = process.platform === "win32" ? "axhub-helpers.exe" : "axhub-helpers";
const helperPath = join(REPO_ROOT, "bin", helperName);

const run = (cmd: string, args: string[], opts: { input?: string } = {}): void => {
  process.stdout.write(`[smoke] $ ${[cmd, ...args].join(" ")}\n`);
  const result = spawnSync(cmd, args, {
    cwd: REPO_ROOT,
    encoding: "utf8",
    input: opts.input,
    stdio: opts.input === undefined ? "inherit" : ["pipe", "inherit", "inherit"],
  });
  if (result.status !== 0) {
    process.stderr.write(`[smoke] FAIL: ${cmd} exited with ${result.status ?? "signal"}\n`);
    process.exit(result.status ?? 1);
  }
};

run(process.execPath, ["run", "build"]);

if (!existsSync(helperPath)) {
  process.stderr.write(`[smoke] FAIL: helper not found at ${helperPath}\n`);
  process.exit(1);
}

run(helperPath, ["version"]);
run(helperPath, ["help"]);

if (full) {
  run(helperPath, ["session-start"], { input: "" });
  run(process.execPath, ["tests/docs-link-audit.ts"]);
  run(process.execPath, ["run", "codegen:catalog"]);
  run(process.execPath, ["run", "codegen:version"]);
}
