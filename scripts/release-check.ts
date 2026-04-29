#!/usr/bin/env bun
/**
 * Release pre-flight — guard against version drift for the Rust-primary helper.
 *
 * Default local mode:
 *   1. codegen:version syncs generated version files + Cargo workspace version
 *   2. bun run build builds the host Rust helper into bin/axhub-helpers
 *   3. host runnable binary reports package.json version
 *   4. workflow/package wiring for the 5 release assets is present
 *
 * Full matrix mode (`AXHUB_RELEASE_CHECK_FULL=1`) also runs `bun run build:all`
 * and requires all 5 release asset names to exist. This is intended for hosts
 * with the required Rust targets/linkers installed; the tag release workflow
 * performs the authoritative 5-platform build in matrix jobs.
 */
import { execSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

import packageJson from "../package.json" with { type: "json" };

const REPO_ROOT = join(import.meta.dir, "..");
const BIN_DIR = join(REPO_ROOT, "bin");
const EXPECTED = packageJson.version;
const FULL_MATRIX = process.env.AXHUB_RELEASE_CHECK_FULL === "1";

const ALL_BINARIES = [
  "axhub-helpers-darwin-arm64",
  "axhub-helpers-darwin-amd64",
  "axhub-helpers-linux-arm64",
  "axhub-helpers-linux-amd64",
  "axhub-helpers-windows-amd64.exe",
];

const hostAssetName = (): string | null => {
  if (process.platform === "darwin" && process.arch === "arm64") return "axhub-helpers-darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "axhub-helpers-darwin-amd64";
  if (process.platform === "linux" && process.arch === "arm64") return "axhub-helpers-linux-arm64";
  if (process.platform === "linux" && process.arch === "x64") return "axhub-helpers-linux-amd64";
  if (process.platform === "win32" && process.arch === "x64") return "axhub-helpers-windows-amd64.exe";
  return null;
};

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

const assertVersion = (relativePath: string): void => {
  const binPath = join(REPO_ROOT, relativePath);
  if (!existsSync(binPath)) fail(`${relativePath} missing`);
  const output = exec(`"${binPath}" --version`);
  const version = extractVersion(output);
  if (version !== EXPECTED) fail(`${relativePath} reports ${version ?? "unknown"} but package.json is ${EXPECTED}`);
  log(`  ✓ ${relativePath} = ${version}`);
};

const assertWorkflowMatrix = (): void => {
  const releaseYml = readFileSync(join(REPO_ROOT, ".github/workflows/release.yml"), "utf8");
  for (const target of [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
  ]) {
    if (!releaseYml.includes(target)) fail(`release.yml missing Rust target ${target}`);
  }
  for (const name of ALL_BINARIES) {
    if (!releaseYml.includes(name)) fail(`release.yml missing asset name ${name}`);
  }
  log("  ✓ release.yml declares 5 Rust target assets");
};

const main = (): void => {
  log(`package.json version = ${EXPECTED}`);

  log("step 1/4: codegen:version (sync version constants)");
  process.stdout.write(exec("bun run codegen:version"));

  log("step 2/4: bun run build (Cargo host build → bin/axhub-helpers)");
  process.stdout.write(exec("bun run build"));

  log("step 3/4: assert host runnable Rust binary version");
  assertVersion("bin/axhub-helpers");
  const hostName = hostAssetName();
  if (hostName) assertVersion(`bin/${hostName}`);

  log("step 4/4: verify 5-asset release wiring");
  assertWorkflowMatrix();

  if (FULL_MATRIX) {
    log("full matrix mode: bun run build:all");
    process.stdout.write(exec("bun run build:all"));
    for (const bin of ALL_BINARIES) {
      if (!existsSync(join(BIN_DIR, bin))) fail(`bin/${bin} missing after build:all`);
    }
    if (hostName) assertVersion(`bin/${hostName}`);
  } else {
    log("full matrix build delegated to release.yml; set AXHUB_RELEASE_CHECK_FULL=1 to force local build:all");
  }

  log(`OK — Rust helper host artifact at ${EXPECTED}, release matrix wired for tag v${EXPECTED}`);
};

if (import.meta.main) {
  try {
    main();
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    fail(message);
  }
}
