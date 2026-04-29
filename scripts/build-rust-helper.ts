#!/usr/bin/env bun
/**
 * Build the Rust axhub-helpers binary and copy it into bin/ using the release
 * asset naming convention. Bun remains the repo scripting runtime; the helper
 * artifact itself is produced by Cargo.
 */
import { spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const BIN_DIR = join(REPO_ROOT, "bin");

interface TargetSpec {
  target: string;
  name: string;
  platform: NodeJS.Platform;
  arch: NodeJS.Architecture;
  exe: boolean;
}

const TARGETS: TargetSpec[] = [
  { target: "aarch64-apple-darwin", name: "axhub-helpers-darwin-arm64", platform: "darwin", arch: "arm64", exe: false },
  { target: "x86_64-apple-darwin", name: "axhub-helpers-darwin-amd64", platform: "darwin", arch: "x64", exe: false },
  { target: "aarch64-unknown-linux-gnu", name: "axhub-helpers-linux-arm64", platform: "linux", arch: "arm64", exe: false },
  { target: "x86_64-unknown-linux-gnu", name: "axhub-helpers-linux-amd64", platform: "linux", arch: "x64", exe: false },
  { target: "x86_64-pc-windows-msvc", name: "axhub-helpers-windows-amd64.exe", platform: "win32", arch: "x64", exe: true },
];

const argValue = (flag: string): string | null => {
  const idx = process.argv.indexOf(flag);
  return idx >= 0 ? process.argv[idx + 1] ?? null : null;
};

const fail = (message: string): never => {
  process.stderr.write(`[build-rust-helper] FAIL: ${message}\n`);
  process.exit(1);
};

const run = (cmd: string, args: string[]): void => {
  process.stdout.write(`[build-rust-helper] $ ${cmd} ${args.join(" ")}\n`);
  const result = spawnSync(cmd, args, { cwd: REPO_ROOT, stdio: "inherit" });
  if (result.status !== 0) fail(`${cmd} exited with ${result.status ?? "signal"}`);
};

const hostSpec = (): TargetSpec => {
  const spec = TARGETS.find((t) => t.platform === process.platform && t.arch === process.arch);
  if (!spec) fail(`unsupported host ${process.platform}/${process.arch}`);
  return spec;
};

const target = argValue("--target");
const requestedName = argValue("--name");
const spec = target ? TARGETS.find((t) => t.target === target) : hostSpec();
if (!spec) fail(`unknown target ${target}`);
const outputName = requestedName ?? (target ? spec.name : "axhub-helpers");

const cargoArgs = ["build", "--release", "-p", "axhub-helpers"];
if (target) cargoArgs.push("--target", target);
run("cargo", cargoArgs);

const binaryName = `axhub-helpers${spec.exe ? ".exe" : ""}`;
const source = target
  ? join(REPO_ROOT, "target", spec.target, "release", binaryName)
  : join(REPO_ROOT, "target", "release", binaryName);
if (!existsSync(source)) fail(`cargo build did not produce ${source}`);

mkdirSync(BIN_DIR, { recursive: true });
const primaryDest = join(BIN_DIR, outputName);
copyFileSync(source, primaryDest);
chmodSync(primaryDest, 0o755);
process.stdout.write(`[build-rust-helper] wrote bin/${outputName}\n`);

// A host build also refreshes the host-specific release asset so smoke/release
// checks catch stale version drift before tag creation.
if (!target) {
  const hostDest = join(BIN_DIR, spec.name);
  copyFileSync(source, hostDest);
  chmodSync(hostDest, 0o755);
  process.stdout.write(`[build-rust-helper] wrote bin/${spec.name}\n`);
}
