#!/usr/bin/env bun
/**
 * Release pre-flight — guard against version drift for the Rust-primary helper.
 *
 * Default local mode:
 *   1. codegen:version syncs generated version files + Cargo workspace version
 *   2. bun run build builds the host Rust helper asset and preserves the shim
 *   3. host runnable helper path reports package.json version
 *   4. workflow/package wiring for the 5 release assets is present
 *
 * Full matrix mode (`AXHUB_RELEASE_CHECK_FULL=1`) also runs `bun run build:all`
 * and requires all 5 release asset names to exist. This is intended for hosts
 * with the required Rust targets/linkers installed; the tag release workflow
 * performs the authoritative 5-platform build in matrix jobs.
 */
import { execSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

import packageJson from "../package.json" with { type: "json" };
import { RELEASE_ASSET_NAMES, RUST_TARGETS, hostPrimaryBinaryName, hostRustTarget } from "./rust-targets.ts";

const REPO_ROOT = join(import.meta.dir, "..");
const BIN_DIR = join(REPO_ROOT, "bin");
const EXPECTED = packageJson.version;
const FULL_MATRIX = process.env.AXHUB_RELEASE_CHECK_FULL === "1";

const hostAssetName = (): string | null => hostRustTarget()?.assetName ?? null;
const hostPrimaryName = (): string => hostPrimaryBinaryName();

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

// ── Track H: AST validator + mcp-serve binary size guard (측정-우선, plan §H1.7/§D) ──
//
// `build-rust-helper.ts` 가 만드는 artifact = `cargo build --release` 산출물
// (UNSTRIPPED — release.yml 가 배포 전 별도 strip). 그래서 한도도 unstripped 기준.
//
// 측정 이력 (host darwin-arm64):
//   T4: ast off(grammar-free)=7,405,936 B → ast on(grammar 8종)=18,018,208 B,
//       host ceiling = post × 1.15 = 20,720,939 B (toolchain 1.96)
//   T6 재산정: mcp default-ON 추가(rmcp transport-io + schemars/darling/tokio-util,
//       toolchain 1.88) → 20,604,672 B (+2.6 MiB). 구 ceiling 의 99.4% 라 헤드룸
//       부족 → 새 host ceiling = mcp-on post × 1.15 = 23,695,373 B.
//   (배포물은 stripped 라 실제 ship size 는 더 작음.)
//
// cross-arch 한도 = host post × (arch_stripped_baseline / host_stripped_baseline)
//                   × 1.15  (v0.9.44 배포 자산 크기 비율로 도출 — host 만 실측,
//                   나머지는 release.yml 실측치로 후속 tighten 가능).
const BINARY_SIZE_CEILING_BYTES: Record<string, number> = {
  "axhub-helpers-darwin-arm64": 23_695_373,
  "axhub-helpers-darwin-amd64": 25_582_210,
  "axhub-helpers-linux-amd64": 28_306_988,
  "axhub-helpers-linux-arm64": 33_311_146,
  "axhub-helpers-windows-amd64.exe": 28_680_784,
};

const assertBinarySize = (relativePath: string, assetName: string): void => {
  const ceiling = BINARY_SIZE_CEILING_BYTES[assetName];
  if (ceiling === undefined) return; // 알 수 없는 asset — gate 걸지 않음
  const binPath = join(REPO_ROOT, relativePath);
  if (!existsSync(binPath)) fail(`${relativePath} missing for size assert`);
  const size = statSync(binPath).size;
  if (size > ceiling) {
    fail(
      `${relativePath} size ${size} B exceeds ceiling ${ceiling} B (measured baseline +15%). ` +
        `grammar 추가로 바이너리 비대 — Cargo \`ast\` feature/strip/codegen 검토.`,
    );
  }
  log(`  ✓ ${assetName} size ${size} B ≤ ${ceiling} B`);
};

const assertWorkflowMatrix = (): void => {
  const releaseYml = readFileSync(join(REPO_ROOT, ".github/workflows/release.yml"), "utf8");
  for (const { target } of RUST_TARGETS) {
    if (!releaseYml.includes(target)) fail(`release.yml missing Rust target ${target}`);
  }
  for (const name of RELEASE_ASSET_NAMES) {
    if (!releaseYml.includes(name)) fail(`release.yml missing asset name ${name}`);
  }
  log("  ✓ release.yml declares 5 Rust target assets");
};

const main = (): void => {
  log(`package.json version = ${EXPECTED}`);

  log("step 1/4: codegen:version (sync version constants)");
  process.stdout.write(exec("bun run codegen:version"));

  log("step 2/4: bun run build (Cargo host build → host asset, shim preserved)");
  process.stdout.write(exec("bun run build"));

  log("step 3/4: assert host runnable Rust binary version");
  assertVersion(`bin/${hostPrimaryName()}`);
  const hostName = hostAssetName();
  if (hostName && hostName !== hostPrimaryName()) assertVersion(`bin/${hostName}`);

  log("step 3b: host binary size assert (Track H AST validator — measured +15%)");
  if (hostName) {
    assertBinarySize(`bin/${hostName}`, hostName);
  } else {
    log("  · host arch not in release matrix — size assert delegated to full matrix");
  }

  log("step 4/4: verify 5-asset release wiring");
  assertWorkflowMatrix();

  if (FULL_MATRIX) {
    log("full matrix mode: bun run build:all");
    process.stdout.write(exec("bun run build:all"));
    for (const bin of RELEASE_ASSET_NAMES) {
      if (!existsSync(join(BIN_DIR, bin))) fail(`bin/${bin} missing after build:all`);
    }
    if (hostName) assertVersion(`bin/${hostName}`);
    log("full matrix: per-arch binary size assert (Track H)");
    for (const { assetName } of RUST_TARGETS) assertBinarySize(`bin/${assetName}`, assetName);
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
