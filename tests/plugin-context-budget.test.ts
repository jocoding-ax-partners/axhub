import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { checkPluginContextBudget, formatBudgetReport, parseClaudePluginDetails } from "../scripts/check-plugin-context-budget.ts";

const REPO_ROOT = join(import.meta.dir, "..");

const CLAUDE_PLUGIN_DETAILS_OUTPUT = `
Claude plugin details: axhub

Context budget
Always-on tokens: 3,963

Component      Always-on tokens  On-invoke tokens
init           0                 21.8k
deploy         0                 28.2k
onboarding     0                 18.5k
clarity        0                 8.1k
`;

const REAL_CLAUDE_PLUGIN_DETAILS_OUTPUT = [
  "$ claude --plugin-dir dist/axhub-plugin plugin details axhub",
  "axhub 1.5.1",
  "  Korean-first Claude Code plugin for axhub. onboarding/init/deploy/import/development/diagnosis/clarity/update 8개 스킬이 ax-hub-cli v0.20.0+ 표면(hidden plugin-support 그룹 + 공개 deploy verify·diagnose·update)을 직접 호출해요. import 는 기존 로컬 앱 가져오기를, diagnosis 는 배포 실패 원인 요약을 담당하고, preview-confirm 배포와 verify 기반 성공 선언을 제공해요.",
  "  Source: axhub@inline\n",
  "Component inventory\n  Skills (8)  clarity, deploy, development, diagnosis, import, init, onboarding, update\n  Agents (0)\n  Hooks (0)\n  MCP servers (0)\n  LSP servers (0)\n",
  "Projected token cost\n  Always-on:   ~2,166 tok   added to every session\n",
  "Per-component (rounded)\n  component    always-on  on-invoke\n  init              ~430      ~8.7k\n  update            ~190      ~4.1k\n  development       ~260      ~6.2k\n  deploy            ~290      ~7.6k\n  clarity           ~290        ~5k\n  import            ~230      ~7.1k\n  onboarding        ~280      ~5.8k\n  diagnosis         ~200        ~4k\n",
  "  On-invoke cost is paid each time a skill or agent fires.\n  Token counts are estimates and may differ from actual usage.\n[exit:0]\n",
].join("\n");

const SHORT_SKILLS = {
  deploy: "short deploy skill",
  init: "short init skill",
  onboarding: "short onboarding skill",
} as const;

const MALFORMED_COMPONENT_ROW =
  "Claude plugin details: axhub\n\nContext budget\nAlways-on tokens: 1,000\n\nComponent      Always-on tokens  On-invoke tokens\ndeploy         0                 nope";

const createFixture = (skills: Record<string, string>): string => {
  const root = mkdtempSync(join(tmpdir(), "axhub-plugin-budget-"));
  for (const [slug, body] of Object.entries(skills)) {
    const skillDir = join(root, "skills", slug);
    mkdirSync(skillDir, { recursive: true });
    writeFileSync(join(skillDir, "SKILL.md"), body);
  }
  return root;
};

const cleanup = (root: string): void => {
  rmSync(root, { recursive: true, force: true });
};

const runBudgetCli = (root: string, ...args: string[]) =>
  Bun.spawnSync({ cmd: ["bun", "scripts/check-plugin-context-budget.ts", "--root", root, ...args], cwd: REPO_ROOT, stdout: "pipe", stderr: "pipe" });

describe("plugin context budget checker", () => {
  test("passes fixture skills below per-skill and total budgets", () => {
    const root = createFixture(SHORT_SKILLS);

    try {
      const result = checkPluginContextBudget({
        root,
        maxSkillBytes: 128,
        maxTotalBytes: 512,
      });

      expect(result.ok).toBe(true);
      expect(result.errors).toEqual([]);
      expect(result.overBudgetSkills).toEqual([]);
      expect(result.skills.map((skill) => skill.slug)).toEqual(["deploy", "init", "onboarding"]);
      expect(formatBudgetReport(result)).toContain("Plugin context budget: PASS");
    } finally {
      cleanup(root);
    }
  });

  test("fails fixtures that exceed per-skill and total budgets", () => {
    const root = createFixture({
      deploy: "x".repeat(32),
      init: "short",
      onboarding: "also short",
    });

    try {
      const result = checkPluginContextBudget({
        root,
        maxSkillBytes: 16,
        maxTotalBytes: 40,
      });
      const report = formatBudgetReport(result);

      expect(result.ok).toBe(false);
      expect(result.overBudgetSkills.map((skill) => skill.slug)).toEqual(["deploy"]);
      expect(result.totalOverBy).toBeGreaterThan(0);
      expect(report).toContain("Plugin context budget: FAIL");
      expect(report).toContain("- deploy:");
      expect(report).toContain("Total budget exceeded");
    } finally {
      cleanup(root);
    }
  });

  test("CLI exits non-zero and names the over-budget fixture skill", () => {
    const root = createFixture({ deploy: "x".repeat(32), init: "short" });

    try {
      const result = runBudgetCli(root, "--max-skill-bytes", "16", "--max-total-bytes", "256");

      const stdout = result.stdout.toString();
      expect(result.exitCode).toBe(1);
      expect(result.stderr.toString()).toBe("");
      expect(stdout).toContain("Plugin context budget: FAIL");
      expect(stdout).toContain("- deploy:");
    } finally {
      cleanup(root);
    }
  });

  test("CLI accepts Claude plugin-details output path flag", () => {
    const root = createFixture(SHORT_SKILLS);
    const detailsPath = join(root, "plugin-details.txt");
    writeFileSync(detailsPath, CLAUDE_PLUGIN_DETAILS_OUTPUT);

    try {
      const result = runBudgetCli(root, "--max-skill-bytes", "128", "--max-total-bytes", "512", "--plugin-details-output", detailsPath);

      const stdout = result.stdout.toString();
      expect(result.exitCode).toBe(1);
      expect(result.stderr.toString()).toBe("");
      expect(stdout).toContain("Always-on tokens: 3963");
      expect(stdout).toContain("- deploy: 28200 on-invoke tokens");
    } finally {
      cleanup(root);
    }
  });

  test("rejects invalid threshold input", () => {
    const root = createFixture({ deploy: "short" });

    try {
      expect(() => checkPluginContextBudget({ root, maxSkillBytes: 0 })).toThrow("maxSkillBytes must be a positive integer");
      expect(() => checkPluginContextBudget({ root, maxTotalBytes: -1 })).toThrow("maxTotalBytes must be a positive integer");
    } finally {
      cleanup(root);
    }
  });

  test("uses source skills root instead of stale dist output", () => {
    const root = mkdtempSync(join(tmpdir(), "axhub-plugin-budget-"));
    const staleSkillDir = join(root, "dist", "axhub-plugin", "skills", "deploy");
    mkdirSync(staleSkillDir, { recursive: true });
    writeFileSync(join(staleSkillDir, "SKILL.md"), "stale bundled skill");

    try {
      const result = checkPluginContextBudget({ root });

      expect(result.ok).toBe(false);
      expect(result.skills).toEqual([]);
      expect(result.errors).toEqual(["skills directory not found: skills"]);
    } finally {
      cleanup(root);
    }
  });

  test("parses Claude plugin-details token output with k-suffix component rows", () => {
    const details = parseClaudePluginDetails(CLAUDE_PLUGIN_DETAILS_OUTPUT);

    expect(details.alwaysOnTokens).toBe(3_963);
    expect(details.components).toEqual([
      { component: "init", alwaysOnTokens: 0, onInvokeTokens: 21_800 },
      { component: "deploy", alwaysOnTokens: 0, onInvokeTokens: 28_200 },
      { component: "onboarding", alwaysOnTokens: 0, onInvokeTokens: 18_500 },
      { component: "clarity", alwaysOnTokens: 0, onInvokeTokens: 8_100 },
    ]);
  });

  test("parses real Claude plugin details output while ignoring inventory summary rows", () => {
    const details = parseClaudePluginDetails(REAL_CLAUDE_PLUGIN_DETAILS_OUTPUT);

    expect(details.alwaysOnTokens).toBe(2_166);
    expect(details.components).toEqual([
      { component: "init", alwaysOnTokens: 430, onInvokeTokens: 8_700 },
      { component: "update", alwaysOnTokens: 190, onInvokeTokens: 4_100 },
      { component: "development", alwaysOnTokens: 260, onInvokeTokens: 6_200 },
      { component: "deploy", alwaysOnTokens: 290, onInvokeTokens: 7_600 },
      { component: "clarity", alwaysOnTokens: 290, onInvokeTokens: 5_000 },
      { component: "import", alwaysOnTokens: 230, onInvokeTokens: 7_100 },
      { component: "onboarding", alwaysOnTokens: 280, onInvokeTokens: 5_800 },
      { component: "diagnosis", alwaysOnTokens: 200, onInvokeTokens: 4_000 },
    ]);
  });

  test("rejects Claude plugin-details output with always-on tokens but no component rows", () => {
    const root = createFixture({
      deploy: "short deploy skill",
      init: "short init skill",
    });

    try {
      const result = checkPluginContextBudget({
        root,
        maxSkillBytes: 128,
        maxTotalBytes: 512,
        pluginDetailsText: "Claude plugin details: axhub\n\nProjected token cost\n  Always-on:   ~1,000 tok   added to every session\n",
      });
      const report = formatBudgetReport(result);

      expect(result.ok).toBe(false);
      expect(result.tokenBudget.measured).toBe(true);
      expect(result.tokenBudget.components).toEqual([]);
      expect(result.tokenBudget.errors).toEqual([
        "token budget parse error: token component data not found in Claude plugin details output",
      ]);
      expect(report).toContain("Plugin context budget: FAIL");
    } finally {
      cleanup(root);
    }
  });

  test("rejects malformed component token rows in Claude plugin-details output", () => {
    expect(() => parseClaudePluginDetails(MALFORMED_COMPONENT_ROW)).toThrow("invalid token component row at line 7 in component token table");
    expect(() => parseClaudePluginDetails(MALFORMED_COMPONENT_ROW)).not.toThrow(/deploy|nope/);
  });

  test("checks Claude plugin-details tokens against always-on and per-component thresholds", () => {
    const root = createFixture({
      clarity: "short clarity skill",
      deploy: "short deploy skill",
      init: "short init skill",
      onboarding: "short onboarding skill",
    });

    try {
      const result = checkPluginContextBudget({
        root,
        maxSkillBytes: 128,
        maxTotalBytes: 512,
        pluginDetailsText: CLAUDE_PLUGIN_DETAILS_OUTPUT,
        maxAlwaysOnTokens: 2_500,
        maxOtherOnInvokeTokens: 8_000,
        maxOnInvokeTokensByComponent: {
          deploy: 12_000,
          init: 12_000,
          onboarding: 10_000,
        },
      });

      expect(result.ok).toBe(false);
      expect(result.tokenBudget.measured).toBe(true);
      expect(result.tokenBudget.errors).toEqual([]);
      expect(result.tokenBudget.alwaysOnTokens).toBe(3_963);
      expect(result.tokenBudget.alwaysOnOverBy).toBe(1_463);

      const componentBudgets = Object.fromEntries(
        result.tokenBudget.components.map((component) => [
          component.component,
          {
            maxOnInvokeTokens: component.maxOnInvokeTokens,
            onInvokeTokens: component.onInvokeTokens,
            overBy: component.overBy,
          },
        ]),
      );

      expect(componentBudgets).toMatchObject({
        init: { maxOnInvokeTokens: 12_000, onInvokeTokens: 21_800, overBy: 9_800 },
        deploy: { maxOnInvokeTokens: 12_000, onInvokeTokens: 28_200, overBy: 16_200 },
        onboarding: { maxOnInvokeTokens: 10_000, onInvokeTokens: 18_500, overBy: 8_500 },
        clarity: { maxOnInvokeTokens: 8_000, onInvokeTokens: 8_100, overBy: 100 },
      });
      expect(result.tokenBudget.overBudgetComponents.map((component) => component.component).sort()).toEqual(["clarity", "deploy", "init", "onboarding"]);
    } finally {
      cleanup(root);
    }
  });

  test("rejects malformed Claude plugin-details token output", () => {
    expect(() => parseClaudePluginDetails("Claude plugin details\nNo token budget rows here")).toThrow(/token/i);
  });
});
