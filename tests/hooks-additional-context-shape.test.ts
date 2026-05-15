import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { join } from "node:path";

const root = join(import.meta.dir, "..");

describe("hook additionalContext shape", () => {
  test("shape linter passes", () => {
    const result = spawnSync("bun", ["scripts/lint-hook-inject-shape.ts"], { cwd: root, encoding: "utf8" });
    expect(result.status).toBe(0);
    expect(result.stdout).toContain("OK");
  });
});
