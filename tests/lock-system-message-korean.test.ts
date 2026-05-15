import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");

describe("Korean systemMessage lock", () => {
  test("deploy artifact verifier still keeps Korean user-facing warning", () => {
    const raw = readFileSync(join(root, "hooks/post-tool-verify-deploy-artifacts.ts"), "utf8");
    expect(raw).toContain("배포 artifact 검증에서 의심 신호를 발견했어요");
    expect(raw).toContain("hookSpecificOutput");
  });
});
