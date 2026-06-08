import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");

describe("Korean systemMessage lock", () => {
  test("deploy artifact verifier still keeps Korean user-facing warning", () => {
    const mainRaw = readFileSync(join(root, "crates/axhub-helpers/src/main.rs"), "utf8");
    const hookOutputRaw = readFileSync(join(root, "crates/axhub-helpers/src/hook_output.rs"), "utf8");
    expect(mainRaw).toContain("배포 artifact 검증에서 의심 신호를 발견했어요");
    expect(mainRaw).toContain("post_tool_use_context_with_system");
    expect(mainRaw).toContain("Skip: AXHUB_DISABLE_HOOK=verify-deploy-artifact");
    expect(hookOutputRaw).toContain("hookSpecificOutput");
  });
});
