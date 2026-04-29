import { describe, expect, test } from "bun:test";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const REPO_ROOT = join(import.meta.dir, "..");

describe("AXHUB_HELPERS_RUNTIME rust delegation", () => {
  test("runtime=rust delegates to configured Rust helper binary with stdin preserved", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-runtime-rust-"));
    try {
      const fake = join(dir, "axhub-helpers-rs");
      writeFileSync(fake, "#!/usr/bin/env bash\ncat >/tmp/axhub-rust-delegated-stdin.txt\necho rust:$1:$(cat /tmp/axhub-rust-delegated-stdin.txt)\nexit 7\n");
      chmodSync(fake, 0o755);
      const result = spawnSync("bun", ["src/axhub-helpers/index.ts", "redact"], {
        cwd: REPO_ROOT,
        env: {
          ...process.env,
          AXHUB_HELPERS_RUNTIME: "rust",
          AXHUB_HELPERS_RUST_BIN: fake,
        },
        input: "hello",
        encoding: "utf8",
      });
      expect(result.status).toBe(7);
      expect(result.stdout.trim()).toBe("rust:redact:hello");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("runtime=auto delegates supported commands when Rust helper exists", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-runtime-auto-"));
    try {
      const fake = join(dir, "axhub-helpers-rs");
      writeFileSync(fake, "#!/usr/bin/env bash\ncat >/tmp/axhub-rust-auto-stdin.txt\necho auto:$1:$(cat /tmp/axhub-rust-auto-stdin.txt)\nexit 9\n");
      chmodSync(fake, 0o755);
      const result = spawnSync("bun", ["src/axhub-helpers/index.ts", "redact"], {
        cwd: REPO_ROOT,
        env: {
          ...process.env,
          AXHUB_HELPERS_RUNTIME: "auto",
          AXHUB_HELPERS_RUST_BIN: fake,
        },
        input: "hello",
        encoding: "utf8",
      });
      expect(result.status).toBe(9);
      expect(result.stdout.trim()).toBe("auto:redact:hello");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("runtime=auto falls back to TypeScript when Rust helper is missing", () => {
    const result = spawnSync("bun", ["src/axhub-helpers/index.ts", "redact"], {
      cwd: REPO_ROOT,
      env: {
        ...process.env,
        AXHUB_HELPERS_RUNTIME: "auto",
        AXHUB_HELPERS_RUST_BIN: "/definitely/not/axhub-helpers-rs",
      },
      input: "Bearer abcdef1234567890abcdef",
      encoding: "utf8",
    });
    expect(result.status).toBe(0);
    expect(result.stdout.trim()).toBe("Bearer ***");
  });

  test("runtime=ts never delegates even when Rust helper exists", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-runtime-ts-"));
    try {
      const fake = join(dir, "axhub-helpers-rs");
      writeFileSync(fake, "#!/usr/bin/env bash\necho should-not-run\nexit 42\n");
      chmodSync(fake, 0o755);
      const result = spawnSync("bun", ["src/axhub-helpers/index.ts", "redact"], {
        cwd: REPO_ROOT,
        env: {
          ...process.env,
          AXHUB_HELPERS_RUNTIME: "ts",
          AXHUB_HELPERS_RUST_BIN: fake,
        },
        input: "Bearer abcdef1234567890abcdef",
        encoding: "utf8",
      });
      expect(result.status).toBe(0);
      expect(result.stdout.trim()).toBe("Bearer ***");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
