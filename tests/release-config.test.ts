// Phase 3 US-204: tests for release infrastructure (workflow + scripts + manifest).

import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

describe("release.yml workflow shape (US-204)", () => {
  const path = join(REPO_ROOT, ".github/workflows/release.yml");
  let content: string;

  test(".github/workflows/release.yml exists", () => {
    expect(existsSync(path)).toBe(true);
    content = readFileSync(path, "utf8");
  });

  test("triggers on tag push (v*.*.*)", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain('"v*.*.*"');
    expect(content).toMatch(/on:\s*\n\s*push:\s*\n\s*tags:/);
  });

  test("declares id-token: write for sigstore OIDC", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("id-token: write");
  });

  test("declares contents: write for release upload", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("contents: write");
  });

  test("build matrix uses Rust toolchain for all 5 release targets", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("dtolnay/rust-toolchain");
    for (const target of [
      "x86_64-unknown-linux-gnu",
      "aarch64-unknown-linux-gnu",
      "x86_64-apple-darwin",
      "aarch64-apple-darwin",
      "x86_64-pc-windows-msvc",
    ]) {
      expect(content).toContain(target);
    }
  });

  test("builds Rust helper via cargo/cross and uploads per-target artifacts", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("cargo build --release -p axhub-helpers");
    expect(content).toContain("cross build --release -p axhub-helpers");
    expect(content).toContain("actions/upload-artifact");
  });

  test("uses Bun only for manifest script in sign-and-upload job", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("Install Bun for release manifest script");
    expect(content).toContain("bun scripts/release/manifest.ts");
  });

  test("generates manifest.json via scripts/release/manifest.ts", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("scripts/release/manifest.ts");
  });

  test("installs cosign via sigstore/cosign-installer action", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("sigstore/cosign-installer");
  });

  test("signs each binary with cosign sign-blob (keyless)", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("cosign sign-blob");
    expect(content).toContain("--yes");
  });

  test("uploads release assets via gh CLI sequential loop (avoids race)", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("gh release upload");
    expect(content).toContain("--clobber");
    expect(content).toContain("axhub-helpers-*");
    expect(content).toContain("*.sig");
  });

  test("verifies uploaded release assets after upload completes", () => {
    content = readFileSync(path, "utf8");
    const uploadIdx = content.indexOf("gh release upload");
    const verifyIdx = content.indexOf("scripts/release/verify-release.sh");
    expect(uploadIdx).toBeGreaterThan(0);
    expect(verifyIdx).toBeGreaterThan(uploadIdx);
    expect(content).toContain('bash scripts/release/verify-release.sh "$TAG"');
  });

  test("manual workflow_dispatch requires an explicit semver tag input", () => {
    content = readFileSync(path, "utf8");
    expect(content).toMatch(/workflow_dispatch:\s*\n\s*inputs:\s*\n\s*tag:/);
    expect(content).toMatch(/TAG=.*github\.event\.inputs\.tag/);
    expect(content).toContain("refs/tags/$TAG");
    expect(content).toMatch(/ref: \$\{\{ github\.event_name == 'workflow_dispatch' && github\.event\.inputs\.tag \|\| github\.ref \}\}/);
  });

  test("release workflow keeps vibe bootstrap SLA measurement advisory after upload", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("vibe-bootstrap-measurement-advisory");
    expect(content).toContain("continue-on-error: true");
    expect(content).toContain("VIBE_BOOTSTRAP_MEASUREMENT_ENABLED");
    expect(content).toContain("bun run measure:vibe-bootstrap");
    expect(content).toContain("bun run check:vibe-sla");
    expect(content).toContain("release-vibe-bootstrap-measurement-summary");
  });
});

describe("rust-staging-gates.yml workflow shape", () => {
  const path = join(REPO_ROOT, ".github/workflows/rust-staging-gates.yml");
  let content: string;

  test(".github/workflows/rust-staging-gates.yml exists", () => {
    expect(existsSync(path)).toBe(true);
    content = readFileSync(path, "utf8");
  });

  test("manual dispatch exposes staging, credential, fuzz, and Windows gates", () => {
    content = readFileSync(path, "utf8");
    expect(content).toMatch(/workflow_dispatch:\s*\n\s*inputs:/);
    expect(content).toContain("run_staging");
    expect(content).toContain("require_credentials");
    expect(content).toContain("fuzz_minutes");
    expect(content).toContain("run_windows_smoke");
    expect(content).toContain("run_measurement");
  });

  test("local gate rebuilds the Rust helper before any staging probe", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("bun run codegen:version");
    expect(content).toContain("bun run build");
    expect(content).toContain("bin/axhub-helpers version");
    expect(content).toContain("bun run release:check");
  });

  test("local gate installs rustfmt and clippy on an edition-2024-capable toolchain", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("RUST_TOOLCHAIN: 1.94.1");
    expect(content).toContain("components: rustfmt, clippy");
    expect(content).not.toContain("RUST_TOOLCHAIN: 1.83.0");
  });

  test("staging job requires explicit credentials and runs read-only E2E", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("AXHUB_E2E_STAGING_TOKEN");
    expect(content).toContain("AXHUB_E2E_STAGING_ENDPOINT");
    expect(content).toContain("AXHUB_E2E_STAGING_APP_ID");
    expect(content).toContain("AXHUB_CLI_INSTALL_COMMAND");
    expect(content).toContain("AXHUB_E2E_REQUIRE_RUST_HELPER: \"1\"");
    expect(content).toContain("bun run test:e2e");
  });

  test("vibe bootstrap measurement is explicit, destructive-gated, and advisory", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("vibe-bootstrap-measurement");
    expect(content).toContain("VIBE_BOOTSTRAP_MEASUREMENT_ENABLED");
    expect(content).toContain("AXHUB_E2E_DESTRUCTIVE");
    expect(content).toContain("AXHUB_E2E_COST_BUDGET_USD");
    expect(content).toContain("AXHUB_E2E_CLEANUP_MODE");
    expect(content).toContain("AXHUB_E2E_TTL_CONFIRMED");
    expect(content).toContain("AXHUB_E2E_PREPROVISIONED_APP_ID");
    expect(content).toContain("AXHUB_E2E_COMMAND_TIMEOUT_MS");
    expect(content).toContain("bun run measure:vibe-bootstrap");
    expect(content).toContain("bun run check:vibe-sla");
    expect(content).toContain("continue-on-error: true");
    expect(content).toContain("actions/upload-artifact");
  });

  test("external security gates include cargo-fuzz and Windows smoke", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("cargo +nightly fuzz run parser");
    expect(content).toContain("windows-latest");
    expect(content).toContain("bin\\axhub-helpers-windows-amd64.exe");
    expect(content).toContain("CredReadW");
  });
});

describe("plugin helper bootstrap", () => {
  test("ships executable bin/axhub-helpers bootstrap shim for plugin PATH", () => {
    const path = join(REPO_ROOT, "bin/axhub-helpers");
    expect(existsSync(path)).toBe(true);
    expect(statSync(path).mode & 0o111).not.toBe(0);

    const content = readFileSync(path, "utf8");
    expect(content).toContain("AXHUB_HELPER_BOOTSTRAP_SHIM=1");
    expect(content).toContain("install.sh");
    expect(content).toContain('exec "$TARGET_PATH" "$@"');
  });

  test("host build preserves the bootstrap shim while writing host release asset", () => {
    const script = readFileSync(join(REPO_ROOT, "scripts/build-rust-helper.ts"), "utf8");
    expect(script).toContain("AXHUB_HELPER_BOOTSTRAP_SHIM=1");
    expect(script).toContain("preserved bin/${outputName} bootstrap shim");
    expect(script).toContain("const hostDest = join(BIN_DIR, spec.assetName)");
  });
});

describe("Rust CI workflow toolchain compatibility", () => {
  const workflowPaths = [
    ".github/workflows/rust-ci.yml",
    ".github/workflows/claude-cli-e2e.yml",
    ".github/workflows/release.yml",
    ".github/workflows/rust-staging-gates.yml",
  ].map((relativePath) => join(REPO_ROOT, relativePath));

  test("all Rust workflows pin the CI toolchain to the same edition-2024-capable version", () => {
    for (const path of workflowPaths) {
      const content = readFileSync(path, "utf8");
      expect(content).toContain("toolchain:");
      expect(content).toContain("1.94.1");
      expect(content).not.toContain("1.83.0");
    }
  });
});

describe("Rust helper workflow path and coverage gates", () => {
  test("claude-cli-e2e path filters track the Rust helper source tree", () => {
    const workflow = readFileSync(join(REPO_ROOT, ".github/workflows/claude-cli-e2e.yml"), "utf8");
    expect(workflow).toContain("crates/axhub-helpers/**");
    expect(workflow).toContain("Cargo.toml");
    expect(workflow).toContain("Cargo.lock");
    expect(workflow).not.toContain("src/axhub-helpers/**");
  });

  test("rust-ci enforces the cargo coverage script instead of leaving it advisory-only", () => {
    const workflow = readFileSync(join(REPO_ROOT, ".github/workflows/rust-ci.yml"), "utf8");
    expect(workflow).toContain("bun run cargo:coverage");
  });
});

describe("manifest.ts generator (US-204)", () => {
  const path = join(REPO_ROOT, "scripts/release/manifest.ts");

  test("scripts/release/manifest.ts exists", () => {
    expect(existsSync(path)).toBe(true);
  });

  test("exports a Manifest schema with required fields", () => {
    const content = readFileSync(path, "utf8");
    expect(content).toContain("schema_version");
    expect(content).toContain("plugin_version");
    expect(content).toContain("helper_version");
    expect(content).toContain("binaries");
    expect(content).toContain("sha256");
    expect(content).toContain("size_bytes");
  });

  test("uses sha256 from node:crypto", () => {
    const content = readFileSync(path, "utf8");
    expect(content).toContain("createHash");
    expect(content).toContain('"sha256"');
  });

  test("excludes cosign signature and certificate sidecars from binary manifest entries", () => {
    const content = readFileSync(path, "utf8");
    expect(content).toContain('!f.endsWith(".sig")');
    expect(content).toContain('!f.endsWith(".pem")');
  });
});

describe(".versionrc.json release lifecycle", () => {
  const path = join(REPO_ROOT, ".versionrc.json");

  test("postbump stages all generated Rust-only version files after release-check rewrites", () => {
    const config = JSON.parse(readFileSync(path, "utf8")) as {
      scripts: { postbump?: string; posttag?: string };
    };
    const postbump = config.scripts.postbump ?? "";
    for (const generatedPath of [
      "bin/install.sh",
      "bin/install.ps1",
      "Cargo.toml",
      "Cargo.lock",
    ]) {
      expect(postbump).toContain(generatedPath);
    }
    expect(postbump).not.toContain("src/axhub-helpers");
    expect(postbump).toContain("git add");
    expect(postbump.indexOf("git add")).toBeGreaterThan(postbump.indexOf("bun run release:check"));
  });

  test("posttag no longer asks maintainers to amend the already-created release tag", () => {
    const config = JSON.parse(readFileSync(path, "utf8")) as {
      scripts: { posttag?: string };
    };
    expect(config.scripts.posttag ?? "").not.toMatch(/commit --amend|amend/);
  });
});

describe("verify-release.sh user-side script (US-204)", () => {
  const path = join(REPO_ROOT, "scripts/release/verify-release.sh");

  test("scripts/release/verify-release.sh exists and is executable", () => {
    expect(existsSync(path)).toBe(true);
    const stats = statSync(path);
    expect((stats.mode & 0o100) !== 0).toBe(true);
  });

  test("verifies manifest.json signature first (trust anchor)", () => {
    const content = readFileSync(path, "utf8");
    expect(content).toContain("manifest.json");
    expect(content).toContain("cosign verify-blob");
  });

  test("uses certificate-identity-regexp + OIDC issuer", () => {
    const content = readFileSync(path, "utf8");
    expect(content).toContain("certificate-identity-regexp");
    expect(content).toContain("token.actions.githubusercontent.com");
  });

  test("cross-checks sha256 against manifest entries", () => {
    const content = readFileSync(path, "utf8");
    expect(content).toContain("sha256");
    expect(content).toContain("expected_sha");
  });
});

describe("docs/RELEASE.md (US-204)", () => {
  test("exists and documents maintainer + user verification", () => {
    const path = join(REPO_ROOT, "docs/RELEASE.md");
    expect(existsSync(path)).toBe(true);
    const content = readFileSync(path, "utf8");
    expect(content).toContain("For maintainers");
    expect(content).toContain("For users (verifying a release)");
    expect(content).toContain("AXHUB_REQUIRE_COSIGN");
    expect(content).toContain("AXHUB_ALLOW_UNSIGNED");
  });
});
