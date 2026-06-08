#!/usr/bin/env bun
/**
 * Build the Rust axhub-helpers binary and copy it into bin/ using the release
 * asset naming convention. Bun remains the repo scripting runtime; the helper
 * artifact itself is produced by Cargo.
 */
import { spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import {
  cargoBinaryName,
  hostPrimaryBinaryName,
  hostRustTarget,
  rustTargetByAlias,
  rustTargetByTriple,
  type RustTargetSpec,
} from "./rust-targets.ts";

const REPO_ROOT = join(import.meta.dir, "..");
const BIN_DIR = join(REPO_ROOT, "bin");

const argValue = (flag: string): string | null => {
  const idx = process.argv.indexOf(flag);
  return idx >= 0 ? process.argv[idx + 1] ?? null : null;
};

const fail = (message: string): never => {
  process.stderr.write(`[build-rust-helper] FAIL: ${message}\n`);
  process.exit(1);
};

const isBootstrapShim = (path: string): boolean => {
  if (!existsSync(path)) return false;
  try {
    return readFileSync(path, "utf8").includes("AXHUB_HELPER_BOOTSTRAP_SHIM=1");
  } catch {
    return false;
  }
};

const run = (cmd: string, args: string[]): void => {
  process.stdout.write(`[build-rust-helper] $ ${cmd} ${args.join(" ")}\n`);
  const result = spawnSync(cmd, args, { cwd: REPO_ROOT, stdio: "inherit" });
  if (result.status !== 0) fail(`${cmd} exited with ${result.status ?? "signal"}`);
};

const hostSpec = (): RustTargetSpec => {
  const spec = hostRustTarget();
  return spec ?? fail(`unsupported host ${process.platform}/${process.arch}`);
};

const target = argValue("--target");
const targetAlias = argValue("--target-alias");
const requestedName = argValue("--name");
if (target && targetAlias) fail("use either --target or --target-alias, not both");
const spec = target
  ? (rustTargetByTriple(target) ?? fail(`unknown target ${target}`))
  : targetAlias
    ? (rustTargetByAlias(targetAlias) ?? fail(`unknown target alias ${targetAlias}`))
    : hostSpec();
const outputName = requestedName ?? (target || targetAlias ? spec.assetName : hostPrimaryBinaryName(spec.platform));

const cargoArgs = ["build", "--release", "-p", "axhub-helpers"];
if (target || targetAlias) cargoArgs.push("--target", spec.target);
run("cargo", cargoArgs);

const binaryName = cargoBinaryName(spec);
const source = target || targetAlias
  ? join(REPO_ROOT, "target", spec.target, "release", binaryName)
  : join(REPO_ROOT, "target", "release", binaryName);
if (!existsSync(source)) fail(`cargo build did not produce ${source}`);

mkdirSync(BIN_DIR, { recursive: true });
const primaryDest = join(BIN_DIR, outputName);
if (!target && !targetAlias && outputName === "axhub-helpers" && isBootstrapShim(primaryDest)) {
  chmodSync(primaryDest, 0o755);
  process.stdout.write(`[build-rust-helper] preserved bin/${outputName} bootstrap shim\n`);
} else {
  copyFileSync(source, primaryDest);
  chmodSync(primaryDest, 0o755);
  process.stdout.write(`[build-rust-helper] wrote bin/${outputName}\n`);
}

// A host build also refreshes the host-specific release asset so smoke/release
// checks catch stale version drift before tag creation.
if (!target && !targetAlias) {
  const hostDest = join(BIN_DIR, spec.assetName);
  copyFileSync(source, hostDest);
  chmodSync(hostDest, 0o755);
  process.stdout.write(`[build-rust-helper] wrote bin/${spec.assetName}\n`);
}
