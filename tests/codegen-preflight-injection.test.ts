// Phase 27.x — codegen-preflight-injection.ts unit tests.
// Verifies:
//   1. getPreflightInjectionLine() is deterministic and contains required elements.
//   2. All 10 targets (9 SKILL + 1 template) contain the codegen output byte-identical.
//   3. Variant taxonomy: 1 deploy + 9 lite.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  getPreflightInjectionLine,
  getInjectionLineForVariant,
  TARGETS,
} from "../scripts/codegen-preflight-injection";

const REPO_ROOT = join(import.meta.dir, "..");

describe("getPreflightInjectionLine — deterministic + structure", () => {
  test("returns same value on repeated calls (idempotent)", () => {
    expect(getPreflightInjectionLine()).toBe(getPreflightInjectionLine());
  });

  test("starts with !`node -e \"", () => {
    expect(getPreflightInjectionLine()).toMatch(/^!`node -e "/);
  });

  test("uses stdio pipe for stderr capture (ADR-0011 §검증된 가정 #2)", () => {
    expect(getPreflightInjectionLine()).toContain("'inherit','inherit','pipe'");
  });

  test("contains strict-anchor denialRegex (ADR-0010 §42 Pattern relaxation 비채택)", () => {
    // M1 review (PR #99): expanded to (?:Shell|Bash) to cover both Claude Code prefixes.
    expect(getPreflightInjectionLine()).toContain("(?:Shell|Bash) command permission check failed.*requires approval");
  });

  test("contains Korean systemMessage (해요체)", () => {
    const line = getPreflightInjectionLine();
    expect(line).toContain("첫 실행이라 권한이 필요해요");
    expect(line).toContain("'허용' 을 누르면 다음부터 자동으로 진행돼요");
  });

  test("contains stderr passthrough branch (ADR-0010 §42 — raw stderr 가 chat 으로 흘러요)", () => {
    expect(getPreflightInjectionLine()).toContain("process.stderr.write(stderrText)");
  });

  test("contains exit code propagation", () => {
    expect(getPreflightInjectionLine()).toContain("typeof result.status==='number'?result.status:0");
  });

  test("contains result.error check (spawn failure path)", () => {
    expect(getPreflightInjectionLine()).toContain("result.error");
  });
});

describe("TARGETS — variant taxonomy", () => {
  test("exactly 10 targets (9 SKILL + 1 template)", () => {
    expect(TARGETS).toHaveLength(10);
  });

  test("exactly 1 deploy variant — skills/deploy/SKILL.md", () => {
    const deployTargets = TARGETS.filter((t) => t.variant === "deploy");
    expect(deployTargets).toHaveLength(1);
    expect(deployTargets[0].file).toBe("skills/deploy/SKILL.md");
  });

  test("exactly 9 lite variant targets", () => {
    expect(TARGETS.filter((t) => t.variant === "lite")).toHaveLength(9);
  });

  test("template is lite variant", () => {
    const tmpl = TARGETS.find((t) => t.file === "skills/_template/SKILL.md.tmpl");
    expect(tmpl).toBeDefined();
    expect(tmpl?.variant).toBe("lite");
  });
});

describe("10-target byte-identical lock (Phase 27.x variant-aware manifest invariant)", () => {
  for (const target of TARGETS) {
    test(`${target.file} (${target.variant}) contains codegen output byte-identical`, () => {
      const content = readFileSync(join(REPO_ROOT, target.file), "utf8");
      const expectedLine = getInjectionLineForVariant(target.variant);
      expect(content).toContain(expectedLine);
    });
  }
});
