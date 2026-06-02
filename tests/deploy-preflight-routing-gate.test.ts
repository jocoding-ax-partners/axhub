/**
 * spec 006 AC 15 — deploy SKILL preflight Step 0 routing gate.
 *
 * The deploy SKILL is reachable two ways: a `/deploy` slash command and
 * description-driven NL skill-selection. To stop axhub from hijacking non-axhub
 * projects / foreign-target prompts, Step 0 calls the shared routing-decision
 * function (`axhub-helpers route-decision`) BEFORE any auth/resolve, and proceeds
 * only when `decision == axhub`. `yield` (foreign target named) steps aside with
 * no question; `ignore` (no marker, bare NL) / `ask` (axhub + foreign both named)
 * open a single disambiguation AskUserQuestion.
 *
 * This test pins the SKILL-side wiring (the Rust subcommand's decision contract is
 * pinned by `crates/axhub-helpers/tests/routing_preflight_gate.rs`): the gate runs
 * before the canonical preflight + deploy-prep, branches on every decision, and
 * registers its disambiguation question's non-interactive safe default.
 */

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = process.cwd();
const SKILL = readFileSync(join(ROOT, "skills", "deploy", "SKILL.md"), "utf8");
const REGISTRY = JSON.parse(
  readFileSync(join(ROOT, "tests", "fixtures", "ask-defaults", "registry.json"), "utf8"),
);

const DISAMBIG_Q = "axhub 에 배포할까요, 아니면 다른 곳에 배포할까요?";

describe("deploy preflight Step 0 routing gate (spec 006 AC 15)", () => {
  test("Step 0 invokes the shared route-decision function", () => {
    expect(SKILL).toContain("route-decision --user-utterance");
    expect(SKILL).toContain("routing-gate");
  });

  test("gate runs BEFORE auth/resolve (canonical preflight + deploy-prep)", () => {
    const gateIdx = SKILL.indexOf("route-decision --user-utterance");
    const preflightIdx = SKILL.indexOf('"$HELPER" preflight --json');
    const deployPrepIdx = SKILL.indexOf("deploy-prep --intent deploy");
    expect(gateIdx).toBeGreaterThan(-1);
    expect(preflightIdx).toBeGreaterThan(-1);
    expect(deployPrepIdx).toBeGreaterThan(-1);
    // Spec §68: shared function called "auth/resolve 전에".
    expect(gateIdx).toBeLessThan(preflightIdx);
    expect(gateIdx).toBeLessThan(deployPrepIdx);
  });

  test("proceeds only when decision == axhub", () => {
    // The gate must express the "axhub 일 때만 진행" contract.
    expect(SKILL).toMatch(/`axhub`\s*일\s*때만/);
  });

  test("yield steps aside without a disambiguation question", () => {
    const yieldBlock = SKILL.slice(SKILL.indexOf("**`yield`**"));
    expect(yieldBlock).toContain("**`yield`**");
    // Named-target-wins: yield must NOT route into the AskUserQuestion.
    // It explicitly says "disambiguation 질문 없이" (no question, just step aside).
    expect(yieldBlock).toMatch(/disambiguation 질문 없이/);
  });

  test("ignore AND ask both open the disambiguation question", () => {
    expect(SKILL).toContain("**`ignore`**");
    expect(SKILL).toContain("**`ask`**");
    expect(SKILL).toContain(DISAMBIG_Q);
    // The disambiguation offers an explicit non-axhub ("다른 곳") escape.
    expect(SKILL).toContain("여기 말고 다른 곳");
    expect(SKILL).toContain("axhub 에 배포");
  });

  test("blocking decisions must not call consent/deploy create", () => {
    // Anchored to the gate's yield block (not the pre-existing NEVER clauses):
    // yield must run NONE of preflight/deploy-prep/consent-mint/deploy create.
    expect(SKILL).toContain(
      "Preflight·deploy-prep·consent-mint·`axhub deploy create` 를 하나도 호출하지 말아요",
    );
  });

  test("explicit (slash) invocation is carried via EXPLICIT and is fail-open", () => {
    expect(SKILL).toContain("EXPLICIT");
    expect(SKILL).toMatch(/EXPLICIT=1/);
    // Korean-first plugin: the /배포 alias (commands/배포.md) must be enumerated
    // as an explicit invocation, else /배포 in a non-marker repo wrongly blocks.
    expect(SKILL).toContain("/배포");
    // Uncertain → treat as explicit (don't block explicit intent).
    expect(SKILL).toMatch(/확실하지 않으면 `EXPLICIT=1`/);
    // Empty helper output → proceed (axhub), not block.
    expect(SKILL).toMatch(/fail-open/);
    expect(SKILL).toContain('.decision // "axhub"');
  });

  test("disambiguation question is registered with a non-axhub safe default", () => {
    const entry = REGISTRY.deploy?.[DISAMBIG_Q];
    expect(entry).toBeDefined();
    // Non-interactive must NOT auto-route to axhub (zero-footprint preservation).
    expect(entry.safe_default).toBe("여기 말고 다른 곳");
    expect(entry.allowed_safe_defaults).toContain("여기 말고 다른 곳");
    expect(entry.allowed_safe_defaults).toContain("axhub 에 배포");
  });

  test("gate scope is deploy only — does not leak grace ownership", () => {
    // once-per-project grace is the prompt-route hook's job, not the preflight.
    expect(SKILL).toMatch(/grace 경고는 prompt-route hook 소유/);
  });
});
