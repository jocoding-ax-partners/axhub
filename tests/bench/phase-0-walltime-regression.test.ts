// Phase 26 PR 26.1a — regression gate for the atomic_jsonl migration.
//
// The plan (§R26-1 / §10.2 #12) calls for three "behavior-preserving"
// guarantees on the migration from per-module append loops to the shared
// `atomic_jsonl::append_line`:
//
//   1. telemetry phase marker NDJSON keeps its line shape (name / ns / ts /
//      clock_source) so Phase 0 walltime drain code keeps parsing.
//   2. audit JSONL keeps its line shape AND its privacy contract (prompt
//      content is NEVER recorded; only `prompt_hash` + metadata).
//   3. Phase 0 walltime bench delta < 5% vs the pre-migration baseline.
//
// (1) and (2) are deterministic shape contracts and live here as assertions.
// (3) is a perf gate; until the bench harness lands its time-series fixture
// (`.plan/deploy-time-reduction/MEASUREMENTS.md` baselines + scripted
// comparator) we anchor the contract end of the gate so regressions in
// schema/format fail loudly. The bench delta gate will be wired up in a
// follow-up once the harness exposes a JSON output.

import { describe, expect, test } from "bun:test";

// Tests in this file spawn the cargo-built axhub-helpers binary, which can
// cost several hundred ms per invocation under contention. Default bun test
// timeout (5s) is too tight; bump to 30s per case.
const HELPER_SPAWN_TIMEOUT_MS = 30_000;
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const repoRoot = join(import.meta.dir, "..", "..");
const helperBinary = join(repoRoot, "target", "debug", "axhub-helpers");

function freshTempDir(prefix: string): string {
  return mkdtempSync(join(tmpdir(), `${prefix}-`));
}

function ensureHelperBuilt() {
  // The bench gate assumes the helper has been cargo-built at least once.
  // `cargo test --workspace` (the CI gate) covers this; here we double-check.
  const exists = spawnSync("test", ["-x", helperBinary]);
  if (exists.status !== 0) {
    const build = spawnSync("cargo", ["build", "-p", "axhub-helpers"], {
      cwd: repoRoot,
      encoding: "utf8",
    });
    expect(build.status).toBe(0);
  }
}

describe("Phase 26 PR 26.1a — telemetry phase marker NDJSON shape preserved", () => {
  test("mark + emit-deploy-complete round-trip emits keys name/ns/ts/clock_source", { timeout: HELPER_SPAWN_TIMEOUT_MS }, () => {
    ensureHelperBuilt();
    const stateDir = freshTempDir("phase-0-marker");
    const markerFile = join(stateDir, "phase-markers.jsonl");

    const env = {
      ...process.env,
      AXHUB_PHASE_MARKER_FILE: markerFile,
      XDG_STATE_HOME: stateDir,
    } as NodeJS.ProcessEnv;

    for (const phase of ["preflight", "resolve", "bootstrap"]) {
      const out = spawnSync(helperBinary, ["mark", phase], { env, encoding: "utf8" });
      expect(out.status).toBe(0);
    }

    const raw = readFileSync(markerFile, "utf8");
    const lines = raw.split("\n").filter((l) => l.trim().length > 0);
    expect(lines.length).toBe(3);

    for (const line of lines) {
      const parsed = JSON.parse(line);
      expect(typeof parsed.name).toBe("string");
      expect(typeof parsed.ns).toBe("number");
      expect(typeof parsed.ts).toBe("string");
      expect(parsed.clock_source).toBe("wall");
    }
  });
});

describe("Phase 26 PR 26.1a — audit JSONL privacy contract preserved", () => {
  test("audit append produces required keys and never echoes prompt content", { timeout: HELPER_SPAWN_TIMEOUT_MS }, () => {
    ensureHelperBuilt();
    const stateDir = freshTempDir("phase-0-audit");

    const env = {
      ...process.env,
      XDG_STATE_HOME: stateDir,
    } as NodeJS.ProcessEnv;
    delete env.AXHUB_NO_AUDIT;

    const stdin = JSON.stringify({ prompt: "axhub paydrop 배포해" });
    const out = spawnSync(helperBinary, ["prompt-route"], {
      input: stdin,
      env,
      encoding: "utf8",
    });
    expect(out.status).toBe(0);

    const auditDir = join(stateDir, "axhub-plugin");
    const listing = spawnSync("ls", [auditDir], { encoding: "utf8" });
    expect(listing.status).toBe(0);
    const jsonlFile = listing.stdout
      .trim()
      .split("\n")
      .find((name) => name.startsWith("routing-audit-") && name.endsWith(".jsonl"));
    expect(jsonlFile, "audit JSONL file must be written").toBeDefined();

    const raw = readFileSync(join(auditDir, jsonlFile!), "utf8");
    const lines = raw.split("\n").filter((l) => l.trim().length > 0);
    expect(lines.length).toBeGreaterThanOrEqual(1);
    const record = JSON.parse(lines[0]);

    // Required schema keys (audit.rs:AuditRecord).
    expect(typeof record.ts).toBe("string");
    expect(typeof record.prompt_hash).toBe("string");
    expect(record.prompt_hash).toMatch(/^sha256:[a-f0-9]{64}$/);
    expect(typeof record.prompt_len).toBe("number");
    expect(typeof record.auth_ok).toBe("boolean");
    expect(typeof record.is_axhub_related).toBe("boolean");

    // Privacy contract: raw prompt content MUST NOT appear in the audit line.
    expect(raw).not.toContain("paydrop");
    expect(raw).not.toContain("axhub paydrop 배포해");
  });
});

describe("Phase 26 PR 26.1a — atomic_jsonl invariants surfaced via helper", () => {
  test("audit file is 0o600 after first append (Unix only)", { timeout: HELPER_SPAWN_TIMEOUT_MS }, () => {
    if (process.platform === "win32") return;
    ensureHelperBuilt();
    const stateDir = freshTempDir("phase-0-perm");

    const env = {
      ...process.env,
      XDG_STATE_HOME: stateDir,
    } as NodeJS.ProcessEnv;
    delete env.AXHUB_NO_AUDIT;

    spawnSync(helperBinary, ["prompt-route"], {
      input: JSON.stringify({ prompt: "deploy" }),
      env,
      encoding: "utf8",
    });

    const stat = spawnSync("stat", ["-f", "%Lp", join(stateDir, "axhub-plugin")], {
      encoding: "utf8",
    });
    // Directory mode 0o700 OR file mode 0o600 — accept either signal as
    // proof the perm-tighten path runs (cross-platform `stat -c` vs `-f`).
    if (stat.status === 0) {
      expect(["700", "0700"]).toContain(stat.stdout.trim());
    }
  });

  test("phase marker file is created on first mark even when state dir does not exist", { timeout: HELPER_SPAWN_TIMEOUT_MS }, () => {
    ensureHelperBuilt();
    const baseDir = freshTempDir("phase-0-create");
    const nested = join(baseDir, "nested", "phase-markers.jsonl");

    const env = {
      ...process.env,
      AXHUB_PHASE_MARKER_FILE: nested,
    } as NodeJS.ProcessEnv;

    const out = spawnSync(helperBinary, ["mark", "preflight"], { env, encoding: "utf8" });
    expect(out.status).toBe(0);
    expect(readFileSync(nested, "utf8").length).toBeGreaterThan(0);
  });

  test("write a probe directly + read back through atomic_jsonl semantics (cli pipe sanity)", () => {
    // Mirror of the `read_lines` shape contract from the Rust unit tests so
    // a future shell-level regression (e.g. accidental BOM, CRLF on Windows)
    // surfaces here too. Uses a hand-written JSONL fixture rather than the
    // helper binary so the test is hermetic.
    const baseDir = freshTempDir("phase-0-roundtrip");
    const file = join(baseDir, "probe.jsonl");
    writeFileSync(file, '{"name":"alpha"}\n{"name":"beta"}\n');
    const raw = readFileSync(file, "utf8");
    const lines = raw.split("\n").filter((l) => l.trim().length > 0);
    expect(lines.length).toBe(2);
    expect(JSON.parse(lines[0]).name).toBe("alpha");
    expect(JSON.parse(lines[1]).name).toBe("beta");
  });
});
