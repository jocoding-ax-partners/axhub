// Phase 0.5.12 US-1707 — statusline.sh ↔ statusline.ps1 cross-platform parity.
// Asserts both files contain the same contract tokens (output strings, env refs, cache key).

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO = join(import.meta.dir, "..");
const SH = join(REPO, "bin/statusline.sh");
const PS1 = join(REPO, "bin/statusline.ps1");

describe("Phase 0.5.12 — statusline.sh ↔ statusline.ps1 parity", () => {
  test("both files exist and are non-empty", () => {
    expect(readFileSync(SH, "utf8").length).toBeGreaterThan(0);
    expect(readFileSync(PS1, "utf8").length).toBeGreaterThan(0);
  });

  test("both reference axhub-helpers binary fast path", () => {
    expect(readFileSync(SH, "utf8")).toMatch(/axhub-helpers/);
    expect(readFileSync(PS1, "utf8")).toMatch(/axhub-helpers/);
  });

  test("both emit '로그인 안 됐어요' on auth miss", () => {
    expect(readFileSync(SH, "utf8")).toContain("로그인 안 됐어요");
    expect(readFileSync(PS1, "utf8")).toContain("로그인 안 됐어요");
  });

  test("both emit '배포 기록 없어요' on empty cache", () => {
    expect(readFileSync(SH, "utf8")).toContain("배포 기록 없어요");
    expect(readFileSync(PS1, "utf8")).toContain("배포 기록 없어요");
  });

  test("both emit '최근 배포' on full hit path", () => {
    expect(readFileSync(SH, "utf8")).toContain("최근 배포");
    expect(readFileSync(PS1, "utf8")).toContain("최근 배포");
  });

  test("both reference AXHUB_TOKEN env", () => {
    expect(readFileSync(SH, "utf8")).toContain("AXHUB_TOKEN");
    expect(readFileSync(PS1, "utf8")).toContain("AXHUB_TOKEN");
  });

  test("both reference AXHUB_PROFILE env", () => {
    expect(readFileSync(SH, "utf8")).toContain("AXHUB_PROFILE");
    expect(readFileSync(PS1, "utf8")).toContain("AXHUB_PROFILE");
  });

  test("both reference last-deploy.json cache", () => {
    expect(readFileSync(SH, "utf8")).toContain("last-deploy.json");
    expect(readFileSync(PS1, "utf8")).toContain("last-deploy.json");
  });
});
