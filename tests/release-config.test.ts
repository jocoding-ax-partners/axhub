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

  test("uses oven-sh/setup-bun action", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("oven-sh/setup-bun");
  });

  test("runs build:all to produce 5 cross-arch binaries", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("bun run build:all");
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

  test("uploads release assets via softprops/action-gh-release", () => {
    content = readFileSync(path, "utf8");
    expect(content).toContain("softprops/action-gh-release");
    expect(content).toContain("axhub-helpers-*");
    expect(content).toContain("*.sig");
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
