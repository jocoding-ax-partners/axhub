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

// The canonical shape the SKILL (§2.7) and every SDK expert agent must name.
const CANONICAL_CMD =
  "migrate-data-verify --refs refs.json --schemas schemas.json --json";

// Parse the ACTUAL command out of SKILL.md §2.7 so the documented text — not a
// hardcoded constant — drives the binary invocation below. If the SKILL renames
// a flag (or drops the command), extraction changes and every execution arm runs
// the binary with the drifted flags → the binary rejects them → tests fail. That
// is the real doc↔binary parity lock; a `toContain` on a self-defined constant
// would only restate the doc, never bind it to the binary.
function extractDocumentedCmd(): {
  command: string;
  refsToken: string;
  schemasToken: string;
} {
  const skill = readFileSync(join(REPO_ROOT, "skills/migrate/SKILL.md"), "utf8");
  const m = skill.match(/migrate-data-verify --refs (\S+) --schemas (\S+) --json/);
  if (!m) {
    throw new Error(
      "SKILL.md §2.7 no longer documents `migrate-data-verify --refs <f> --schemas <f> --json` — doc/binary parity lock broken",
    );
  }
  return { command: m[0], refsToken: m[1], schemasToken: m[2] };
}

const DOCUMENTED = extractDocumentedCmd();

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
  return DOCUMENTED.command.split(/\s+/).map((token) => {
    if (token === DOCUMENTED.refsToken) return refsPath;
    if (token === DOCUMENTED.schemasToken) return schemasPath;
    return token;
  });
}

describe("migrate-data-verify enforces the discover()-verify hard-stop end-to-end", () => {
  test("the §2.7 command parsed from SKILL.md matches the canonical shape the binary parses", () => {
    // DOCUMENTED is extracted from SKILL.md at load (above); assert it still has
    // the exact flag skeleton the binary accepts. A flag rename in the doc would
    // change DOCUMENTED.command and trip both this and every execution arm below,
    // because the execution arms build argv from DOCUMENTED — the doc text, not a
    // hardcoded constant, drives the binary invocation.
    expect(DOCUMENTED.command).toBe(CANONICAL_CMD);
    expect(DOCUMENTED.refsToken).toBe("refs.json");
    expect(DOCUMENTED.schemasToken).toBe("schemas.json");
    // every SDK expert agent the migrate flow dispatches names the same lever
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

  test("fail-closed on broken input — malformed JSON and a missing flag both hard-stop (exit 2, ok:false)", () => {
    // The most safety-critical branch: bad/missing input must NOT fall through to
    // a silent pass. A verify that returned 0 / ok:true on unparseable refs or a
    // dropped flag would let a conversion ship past a gate that did nothing — the
    // exact "passes even though it verified nothing" hazard, just on the
    // input-validation arm rather than the set-diff arm.
    ensureHelperBuilt();
    const dir = mkdtempSync(join(tmpdir(), "axhub-data-verify-e2e-bad-"));
    const refsPath = join(dir, "refs.json");
    const schemasPath = join(dir, "schemas.json");

    // 1) malformed refs JSON
    writeFileSync(refsPath, "{ this is not valid json");
    writeFileSync(schemasPath, JSON.stringify({ orders: ["id"] }));
    const malformed = spawnSync(HELPER_BINARY, documentedArgs(refsPath, schemasPath), {
      encoding: "utf8",
      timeout: 15_000,
    });
    expect(malformed.status).toBe(2);
    expect(JSON.parse(malformed.stdout).ok).toBe(false);

    // 2) missing --schemas flag entirely (no schema to diff against → cannot pass)
    writeFileSync(refsPath, JSON.stringify({ orders: ["id"] }));
    const missingFlag = spawnSync(
      HELPER_BINARY,
      ["migrate-data-verify", "--refs", refsPath, "--json"],
      { encoding: "utf8", timeout: 15_000 },
    );
    expect(missingFlag.status).toBe(2);
    expect(JSON.parse(missingFlag.stdout).ok).toBe(false);
  });
});
