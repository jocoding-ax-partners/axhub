// Phase 2 US-102: Frozen fixture suite for parseAxhubCommand.
//
// Loads tests/fixtures/*.json and asserts that parseAxhubCommand returns the
// frozen expected output for each. Any change to expected output requires
// editing the .json file (and ideally _curated.ts) explicitly — forces a
// conscious "we are changing parser semantics" moment.

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { parseAxhubCommand } from "../src/axhub-helpers/consent";

interface Fixture {
  description: string;
  input: { command: string };
  expected: {
    is_destructive: boolean;
    action?: "deploy_create" | "update_apply" | "deploy_logs_kill" | "auth_login";
    app_id?: string;
    branch?: string;
    commit_sha?: string;
    profile?: string;
  };
}

const FIXTURE_DIR = join(import.meta.dir, "fixtures");

const fixtureFiles = readdirSync(FIXTURE_DIR)
  .filter((f) => f.endsWith(".json"))
  .sort();

const loadFixture = (filename: string): Fixture =>
  JSON.parse(readFileSync(join(FIXTURE_DIR, filename), "utf8"));

describe("Frozen fixture suite (38 hand-curated parseAxhubCommand cases)", () => {
  test("exactly 38 fixtures present", () => {
    expect(fixtureFiles.length).toBe(38);
  });

  test("each fixture has required schema fields", () => {
    for (const file of fixtureFiles) {
      const fix = loadFixture(file);
      expect(fix.description).toBeTypeOf("string");
      expect(fix.input?.command).toBeTypeOf("string");
      expect(fix.expected?.is_destructive).toBeTypeOf("boolean");
    }
  });

  for (const file of fixtureFiles) {
    const fix = loadFixture(file);
    test(`fixture ${file}: ${fix.description}`, () => {
      const result = parseAxhubCommand(fix.input.command);
      expect(result.is_destructive).toBe(fix.expected.is_destructive);
      if (fix.expected.action !== undefined) {
        expect(result.action).toBe(fix.expected.action);
      }
      // For positive cases with explicit field expectations, assert each.
      // Adversarial fixtures (env-prefix, sub-shell, etc.) only require
      // is_destructive + action; we don't pin app/branch/commit there because
      // the parser may correctly skip extraction inside wrappers.
      if (fix.expected.app_id !== undefined && !file.startsWith("adv-") && !file.startsWith("uni-")) {
        expect(result.app_id).toBe(fix.expected.app_id);
      }
      if (fix.expected.branch !== undefined && !file.startsWith("adv-") && !file.startsWith("uni-")) {
        expect(result.branch).toBe(fix.expected.branch);
      }
      if (fix.expected.commit_sha !== undefined && !file.startsWith("adv-") && !file.startsWith("uni-")) {
        expect(result.commit_sha).toBe(fix.expected.commit_sha);
      }
    });
  }
});

describe("Fixture distribution (curation invariants)", () => {
  const counts = fixtureFiles.reduce<Record<string, number>>((acc, f) => {
    const cat = f.split("-")[0];
    acc[cat] = (acc[cat] ?? 0) + 1;
    return acc;
  }, {});

  test("10 destructive fixtures", () => {
    expect(counts.destructive).toBe(10);
  });

  test("8 read-only fixtures (ro-*)", () => {
    expect(counts.ro).toBe(8);
  });

  test("8 adversarial fixtures (adv-*)", () => {
    expect(counts.adv).toBe(8);
  });

  test("4 unicode fixtures (uni-*)", () => {
    expect(counts.uni).toBe(4);
  });

  test("4 profile/headless fixtures (prf-*)", () => {
    expect(counts.prf).toBe(4);
  });

  test("4 negative fixtures (neg-*)", () => {
    expect(counts.neg).toBe(4);
  });
});
