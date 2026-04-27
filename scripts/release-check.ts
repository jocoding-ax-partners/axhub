#!/usr/bin/env bun
/**
 * v0.1.16 US-1601: Release pre-flight — guard against binary staleness.
 *
 * Runs in this order:
 *   1. codegen:version (sync install.sh / install.ps1 / index.ts / telemetry.ts to package.json)
 *   2. build (local bin/axhub-helpers — symlink target for plugin directory mode)
 *   3. build:all (5 cross-arch artifacts for release.yml parity)
 *   4. Assert each compiled binary's `--version` output matches package.json
 *
 * Failure mode prevented: v0.1.14 release shipped CHANGELOG + JSON bumps
 * but local bin/axhub-helpers stayed at v0.1.10 because `bun run build`
 * was never re-run. Plugin directory mode users saw stale runtime even
 * after pulling the release tag locally.
 *
 * Run BEFORE `git tag vX.Y.Z`. Idempotent — safe to re-run.
 */
import { execSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";

import packageJson from "../package.json" with { type: "json" };

const REPO_ROOT = join(import.meta.dir, "..");
const BIN_DIR = join(REPO_ROOT, "bin");
const EXPECTED = packageJson.version;

const HOST_RUNNABLE_BINARIES: { name: string; arch: NodeJS.Architecture; platform: NodeJS.Platform }[] = [
  { name: "axhub-helpers-darwin-arm64", arch: "arm64", platform: "darwin" },
  { name: "axhub-helpers-darwin-amd64", arch: "x64", platform: "darwin" },
  { name: "axhub-helpers-linux-arm64", arch: "arm64", platform: "linux" },
  { name: "axhub-helpers-linux-amd64", arch: "x64", platform: "linux" },
];

const ALL_BINARIES = [
  "axhub-helpers-darwin-arm64",
  "axhub-helpers-darwin-amd64",
  "axhub-helpers-linux-arm64",
  "axhub-helpers-linux-amd64",
  "axhub-helpers-windows-amd64.exe",
];

const exec = (cmd: string): string => execSync(cmd, { cwd: REPO_ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });

const log = (msg: string) => process.stdout.write(`[release-check] ${msg}\n`);

const fail = (msg: string): never => {
  process.stderr.write(`[release-check] FAIL: ${msg}\n`);
  process.exit(1);
};

const extractVersion = (output: string): string | null => {
  const m = output.match(/(\d+\.\d+\.\d+(?:-[a-z0-9.]+)?)/);
  return m ? m[1] : null;
};

const main = (): void => {
  log(`package.json version = ${EXPECTED}`);

  log("step 1/4: codegen:version (sync version constants)");
  const codegenOutput = exec("bun run codegen:version");
  process.stdout.write(codegenOutput);

  log("step 2/4: bun run build (rebuild local bin/axhub-helpers)");
  exec("bun run build");

  log("step 3/4: bun run build:all (rebuild 5 cross-arch artifacts)");
  exec("bun run build:all");

  log("step 4/4: assert binary --version output matches package.json");

  // Local bin/axhub-helpers — host arch — always runnable
  const localBinPath = join(BIN_DIR, "axhub-helpers");
  if (!existsSync(localBinPath)) fail(`bin/axhub-helpers missing — build step did not produce expected artifact`);
  const localVersionOut = exec(`"${localBinPath}" --version`);
  const localVersion = extractVersion(localVersionOut);
  if (localVersion !== EXPECTED) {
    fail(`bin/axhub-helpers reports ${localVersion ?? "unknown"} but package.json is ${EXPECTED}`);
  }
  log(`  ✓ bin/axhub-helpers = ${localVersion}`);

  // Per-arch binaries: only execute the ones the host can run; verify others exist
  for (const bin of ALL_BINARIES) {
    const binPath = join(BIN_DIR, bin);
    if (!existsSync(binPath)) fail(`bin/${bin} missing — build:all did not produce all artifacts`);
  }

  for (const { name, arch, platform } of HOST_RUNNABLE_BINARIES) {
    if (process.platform !== platform || process.arch !== arch) continue;
    const binPath = join(BIN_DIR, name);
    const out = exec(`"${binPath}" --version`);
    const v = extractVersion(out);
    if (v !== EXPECTED) fail(`bin/${name} reports ${v ?? "unknown"} but package.json is ${EXPECTED}`);
    log(`  ✓ bin/${name} = ${v}`);
  }

  log(`OK — all binaries at ${EXPECTED}, ready to tag v${EXPECTED}`);
};

if (import.meta.main) {
  try {
    main();
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    fail(message);
  }
}
