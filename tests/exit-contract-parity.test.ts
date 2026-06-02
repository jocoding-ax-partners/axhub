import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// spec 004 FR-012 / SC-008: the empathy catalog must stay aligned with the
// pinned ax-hub-cli failure contract. Every catalog key's BASE exit code must
// be one the CLI can actually emit. This guard fails on drift (e.g. a stale
// sysexits 65/67/68/70 key) so the routing bug that spec 004 fixed cannot
// silently regress when the CLI contract changes.

const ROOT = join(import.meta.dir, "..");
const catalog = JSON.parse(
  readFileSync(join(ROOT, "crates/axhub-helpers/data/catalog.json"), "utf8"),
) as Record<string, unknown>;
const contract = JSON.parse(
  readFileSync(join(ROOT, "crates/axhub-helpers/data/cli-exit-contract.json"), "utf8"),
) as { exit_codes: Record<string, string>; cli_version: string };

const validBaseCodes = new Set(Object.keys(contract.exit_codes));
const baseOf = (key: string): string => key.split(":")[0]!;

describe("exit-contract parity (spec 004 FR-012)", () => {
  test("every catalog key base is a CLI-emittable exit code", () => {
    const violations = Object.keys(catalog)
      .map((k) => ({ key: k, base: baseOf(k) }))
      .filter(({ base }) => !validBaseCodes.has(base));
    expect(violations).toEqual([]);
  });

  test("no stale sysexits base codes (65/67/68/70) remain", () => {
    const stale = ["65", "67", "68", "70"];
    const offenders = Object.keys(catalog).filter((k) => stale.includes(baseOf(k)));
    expect(offenders).toEqual([]);
  });

  test("the normalize-target base entries exist at the current CLI code", () => {
    // auth=4, not_found=5, rate_limited=6, api/internal=7 — the four CLI codes
    // that helper-output 65/67/68/70 normalize into (catalog.rs::normalize_helper_exit).
    // Each must have a base template so a bare (subcode-less) failure of that
    // class does not fall through to default_entry.
    expect(catalog["4"]).toBeDefined();
    expect(catalog["5"]).toBeDefined();
    expect(catalog["6"]).toBeDefined();
    expect(catalog["7"]).toBeDefined();
  });

  test("every subcoded key has a base entry for its exit code", () => {
    // A `{exit}:{subcode}` row without a base `{exit}` row means a failure of
    // that exit code with no/unknown subcode silently degrades to default_entry.
    const keys = Object.keys(catalog);
    const orphans = keys
      .filter((k) => k.includes(":"))
      .map(baseOf)
      .filter((base) => !(base in catalog));
    expect([...new Set(orphans)]).toEqual([]);
  });
});
