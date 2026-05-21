// Phase 27.x — codegen-preflight-injection.ts unit tests.
// Verifies:
//   1. getPreflightInjectionLine() is deterministic and contains required elements.
//   2. All 15 targets (14 SKILL + 1 template) contain the codegen output byte-identical.
//   3. Variant taxonomy: 1 deploy + 15 lite.

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

  test("uses stdio pipe for stdout + stderr capture (cli_unavailable detection + ADR-0011 §검증된 가정 #2)", () => {
    // v0.9.3: stdout switched from inherit to pipe so we can detect
    // auth_error_code:"cli_unavailable" in the preflight JSON and emit a friendly
    // systemMessage instead of letting Claude Code surface a generic "Shell command failed"
    // when axhub CLI is missing. stdout is re-emitted via process.stdout.write(stdoutText)
    // so SKILL preprocessing still sees the JSON payload.
    expect(getPreflightInjectionLine()).toContain("'inherit','pipe','pipe'");
    expect(getPreflightInjectionLine()).toContain("process.stdout.write(stdoutText)");
  });

  test("contains cli_unavailable branch with install-cli systemMessage", () => {
    const line = getPreflightInjectionLine();
    expect(line).toContain("cliUnavailableRegex");
    expect(line).toContain("auth_error_code");
    expect(line).toContain("cli_unavailable");
    expect(line).toContain("axhub CLI 가 감지 안 돼요");
    expect(line).toContain("/axhub:install-cli");
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
    // PR #99 security M2: passthrough now goes through secret-token redaction layer
    // (sk-/gho_/axhub_/Bearer) before writing to parent stderr.
    expect(getPreflightInjectionLine()).toContain("process.stderr.write(stderrText.replace(redactRe,'<redacted>'))");
  });

  test("contains secret token redaction regex (PR #99 security M2)", () => {
    const line = getPreflightInjectionLine();
    expect(line).toContain("sk-[A-Za-z0-9_-]{20,}");
    expect(line).toContain("gho_[A-Za-z0-9]{36}");
    expect(line).toContain("github_pat_[A-Za-z0-9_]{20,}");
    expect(line).toContain("axhub_[A-Za-z0-9]{32,}");
    expect(line).toContain("Bearer");
    expect(line).toContain("<redacted>");
  });

  test("contains exit code propagation", () => {
    expect(getPreflightInjectionLine()).toContain("typeof result.status==='number'?result.status:0");
  });

  test("contains result.error check (spawn failure path)", () => {
    expect(getPreflightInjectionLine()).toContain("result.error");
  });
});

describe("TARGETS — variant taxonomy", () => {
  test("exactly 15 targets (14 SKILL + 1 template)", () => {
    expect(TARGETS).toHaveLength(15);
  });

  test("exactly 1 deploy variant — skills/deploy/SKILL.md", () => {
    const deployTargets = TARGETS.filter((t) => t.variant === "deploy");
    expect(deployTargets).toHaveLength(1);
    expect(deployTargets[0].file).toBe("skills/deploy/SKILL.md");
  });

  test("exactly 14 lite variant targets", () => {
    expect(TARGETS.filter((t) => t.variant === "lite")).toHaveLength(14);
  });

  test("template is lite variant", () => {
    const tmpl = TARGETS.find((t) => t.file === "skills/_template/SKILL.md.tmpl");
    expect(tmpl).toBeDefined();
    expect(tmpl?.variant).toBe("lite");
  });
});

describe("15-target byte-identical lock (Phase 27.x variant-aware manifest invariant)", () => {
  for (const target of TARGETS) {
    test(`${target.file} (${target.variant}) contains codegen output byte-identical`, () => {
      const content = readFileSync(join(REPO_ROOT, target.file), "utf8");
      const expectedLine = getInjectionLineForVariant(target.variant);
      expect(content).toContain(expectedLine);
    });
  }
});
