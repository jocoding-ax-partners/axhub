import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// End-to-end enforcement lock for the discover()-verify hard-stop (§2.7).
//
// The migrate SKILL + the 6 SDK expert agents instruct the conversion to run
//   "$HELPER" migrate-data-verify --refs refs.json --schemas schemas.json --json
// and hard-stop on `ok=false` (exit 2). `migrate-skill-contract.test.ts` only
// proves that STRING is present in the docs — it never proves the documented
// invocation actually enforces. The unit tests in migrate_data_verify.rs prove
// the pure set-diff logic, but not the CLI/exit-code contract the agent prose
// leans on.
//
// This test closes that gap: it runs the REAL built binary with the EXACT flag
// signature the SKILL documents, against data_patch_plan-shaped fixtures, and
// asserts the enforcing behavior end-to-end — exit 0 + ok:true on a clean
// conversion, exit 2 + ok:false + a Korean preview that names the silently-wrong
// table/column on a drifted one. If the binary's arg parsing, exit codes, JSON
// shape, or the documented command drift apart, this fails — so the agent prose
// can never quietly stop matching the lever it depends on.

const REPO_ROOT = join(import.meta.dir, "..");
const HELPER_BINARY = join(REPO_ROOT, "target", "debug", "axhub-helpers");

// The canonical invocation the SKILL (§2.7) and every SDK expert agent name.
// Kept here as the single source the e2e exercises, so a doc edit that changes
// the flags without changing the binary (or vice-versa) trips the parity test.
const DOCUMENTED_CMD =
  "migrate-data-verify --refs refs.json --schemas schemas.json --json";

function ensureHelperBuilt(): void {
  const build = spawnSync("cargo", ["build", "-p", "axhub-helpers"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    timeout: 180_000,
  });
  expect(build.status).toBe(0);
}

type RefMap = Record<string, string[]>;

interface Violation {
  table: string;
  kind: "missing_table" | "missing_column";
  column?: string;
}

interface VerifyResult {
  status: number;
  verdict: {
    ok: boolean;
    violations: Violation[];
    tables_checked: number;
    columns_checked: number;
    preview_kr: string;
  };
}

// Runs the real binary with the EXACT flag tokens the SKILL documents, only
// substituting the fixture file paths for the `refs.json` / `schemas.json`
// placeholders. Anything other than the documented flags would not exercise the
// contract the agents actually follow.
function runVerify(refs: RefMap, schemas: RefMap): VerifyResult {
  ensureHelperBuilt();
  const dir = mkdtempSync(join(tmpdir(), "axhub-data-verify-e2e-"));
  const refsPath = join(dir, "refs.json");
  const schemasPath = join(dir, "schemas.json");
  writeFileSync(refsPath, JSON.stringify(refs));
  writeFileSync(schemasPath, JSON.stringify(schemas));

  const args = documentedArgs(refsPath, schemasPath);
  const out = spawnSync(HELPER_BINARY, args, { encoding: "utf8", timeout: 15_000 });
  // The subcommand always emits a JSON verdict on stdout; status carries the
  // hard-stop signal (0 clean / 2 violation).
  expect(out.stdout.trim().length).toBeGreaterThan(0);
  return { status: out.status ?? -1, verdict: JSON.parse(out.stdout) };
}

// Derive argv from the documented command string, swapping only the two file
// placeholders. This is what makes the test a *parity* lock rather than a
// hand-written re-statement of the flags.
function documentedArgs(refsPath: string, schemasPath: string): string[] {
  return DOCUMENTED_CMD.split(/\s+/).map((token) => {
    if (token === "refs.json") return refsPath;
    if (token === "schemas.json") return schemasPath;
    return token;
  });
}

describe("migrate-data-verify enforces the discover()-verify hard-stop end-to-end", () => {
  test("the binary accepts the exact command the SKILL §2.7 documents", () => {
    const skill = readFileSync(join(REPO_ROOT, "skills/migrate/SKILL.md"), "utf8");
    // doc names the canonical invocation (also covered by the contract test, but
    // re-asserted here so this e2e fails loudly if the lever string ever drifts)
    expect(skill).toContain(DOCUMENTED_CMD);
    // and every SDK expert agent names the same subcommand
    for (const lang of ["go", "java", "kotlin", "node", "python", "ruby"]) {
      const agent = readFileSync(join(REPO_ROOT, "agents", `axhub-sdk-${lang}-expert.md`), "utf8");
      expect(agent).toContain("migrate-data-verify");
    }
  });

  test("clean conversion → exit 0, ok:true, no violations", () => {
    const { status, verdict } = runVerify(
      { orders: ["id", "total"], customers: ["id", "email"] },
      { orders: ["id", "total", "status"], customers: ["id", "email", "name"] },
    );
    expect(status).toBe(0);
    expect(verdict.ok).toBe(true);
    expect(verdict.violations).toEqual([]);
    expect(verdict.tables_checked).toBe(2);
    expect(verdict.columns_checked).toBe(4);
    expect(verdict.preview_kr).toContain("✅ data-verify");
  });

  test("silently-wrong column (the headline hazard) → exit 2, ok:false, preview names table.column", () => {
    // The conversion references orders.status, but the real discover()'d schema
    // has no `status` — compiles fine, queries the wrong thing. This is exactly
    // the failure generic build-verify cannot catch.
    const { status, verdict } = runVerify(
      { orders: ["id", "total", "status"] },
      { orders: ["id", "total"] },
    );
    expect(status).toBe(2);
    expect(verdict.ok).toBe(false);
    expect(verdict.violations).toEqual([
      { table: "orders", kind: "missing_column", column: "status" },
    ]);
    expect(verdict.preview_kr).toContain("🛑 data-verify 실패");
    expect(verdict.preview_kr).toContain("`orders.status` column 이 실제 schema 에 없어요");
  });

  test("misspelled table → exit 2, ok:false, preview names the missing table", () => {
    const { status, verdict } = runVerify(
      { ordrs: ["id"] }, // typo'd table name
      { orders: ["id"] },
    );
    expect(status).toBe(2);
    expect(verdict.ok).toBe(false);
    expect(verdict.violations).toEqual([{ table: "ordrs", kind: "missing_table" }]);
    expect(verdict.preview_kr).toContain("table `ordrs` 가 앱에 없어요");
  });

  test("mixed plan → only the drifted ref is flagged, valid refs pass", () => {
    const { status, verdict } = runVerify(
      { orders: ["id", "total"], customers: ["id", "phone"] },
      { orders: ["id", "total"], customers: ["id", "email"] },
    );
    expect(status).toBe(2);
    expect(verdict.ok).toBe(false);
    // orders is fully valid; only customers.phone drifts
    expect(verdict.violations).toEqual([
      { table: "customers", kind: "missing_column", column: "phone" },
    ]);
    expect(verdict.tables_checked).toBe(2);
    expect(verdict.columns_checked).toBe(4);
  });

  test("exit code is the hard-stop signal the SKILL relies on (0 vs 2, not just JSON)", () => {
    // The SKILL prose hard-stops on exit 2; a verify that returned 0 on a bad
    // ref would let a vibe-coder ship the silently-wrong query. Lock both arms.
    expect(runVerify({ t: ["a"] }, { t: ["a"] }).status).toBe(0);
    expect(runVerify({ t: ["a"] }, { t: ["b"] }).status).toBe(2);
  });
});
