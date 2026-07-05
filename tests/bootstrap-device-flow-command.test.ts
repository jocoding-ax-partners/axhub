import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("bootstrap device-flow command contract", () => {
  test("keeps the first desktop execute and resume out of watch mode", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const localReference = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const resumeReference = readRepo("skills/bootstrap/references/resume-and-tenant.md");
    const preCloneReference = localReference.slice(
      localReference.indexOf("## Execute And Watch"),
      localReference.indexOf("## Clone Current Directory"),
    );

    expect(bootstrap).toContain("첫 execute/resume 에 `--watch`/`--watch-timeout` 을 붙이지 않아요");
    expect(localReference).toContain("Do not attach `--watch` or `--watch-timeout` to the first execute");
    expect(localReference).toContain("strip `--watch --watch-timeout <value>` from the first Desktop resume");
    expect(resumeReference).toContain("strip `--watch --watch-timeout <value>` from the first Desktop resume");
    expect(preCloneReference).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap");
    expect(preCloneReference).not.toContain("--execute --watch");
    expect(preCloneReference).not.toContain("--resume-last --watch");
  });
});
