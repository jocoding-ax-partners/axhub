// Phase 2 US-104: Manifest spec assertions to prevent regression of bug class A
// (manifest/JSON shape) discovered during Phase 6 actual loader testing.
//
// Each describe block has a Reason header citing the historical incident.
// Total assertion count target: ≥88.

import { describe, expect, test, beforeAll } from "bun:test";
import { readFile, readdir, stat } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

interface PluginJson {
  name: string;
  version: string;
  description: string;
  author: { name: string; url?: string };
  homepage?: string;
  repository: string;
  license: string;
  keywords?: string[];
}

interface MarketplaceJson {
  name: string;
  owner: { name: string; url?: string };
  plugins: Array<{ name: string; source: string; description: string; version: string }>;
}

interface PackageJson {
  name: string;
  version: string;
  scripts: Record<string, string>;
  engines?: Record<string, string>;
  dependencies?: Record<string, string>;
}

interface HookConfig {
  type: string;
  command: string;
  timeout?: number;
  shell?: string;
}

interface HookGroup {
  matcher?: string;
  hooks: HookConfig[];
}

interface HooksJson {
  description?: string;
  hooks: Record<string, HookGroup[]>;
}

let pluginJson: PluginJson;
let marketplaceJson: MarketplaceJson;
let packageJson: PackageJson;
let hooksJson: HooksJson;
let upgradeSkill: string;
// helperSource removed v0.2.0 — TS shadow박멸.

beforeAll(async () => {
  pluginJson = JSON.parse(await readFile(join(REPO_ROOT, ".claude-plugin/plugin.json"), "utf8"));
  marketplaceJson = JSON.parse(await readFile(join(REPO_ROOT, ".claude-plugin/marketplace.json"), "utf8"));
  packageJson = JSON.parse(await readFile(join(REPO_ROOT, "package.json"), "utf8"));
  hooksJson = JSON.parse(await readFile(join(REPO_ROOT, "hooks/hooks.json"), "utf8"));
  upgradeSkill = await readFile(join(REPO_ROOT, "skills/upgrade/SKILL.md"), "utf8");
  // helperSource (TS shadow) removed v0.2.0 — equivalent validation now in
  // crates/axhub-helpers/tests/phase_parity.rs (cargo test).
});

// ---------------------------------------------------------------------------
// Reason: Phase 6 incident — Claude Code loader rejected `repository: {type, url}` object
// (must be string). Persist hard assertions on plugin.json shape.
// ---------------------------------------------------------------------------
describe("plugin.json schema", () => {
  test("name field present and matches kebab-case", () => {
    expect(pluginJson.name).toBeTypeOf("string");
    expect(pluginJson.name).toMatch(/^[a-z][a-z0-9-]*$/);
  });

  test("name is exactly 'axhub'", () => {
    expect(pluginJson.name).toBe("axhub");
  });

  test("version is semver", () => {
    expect(pluginJson.version).toMatch(/^\d+\.\d+\.\d+(-[a-z0-9.]+)?$/);
  });

  test("description present and non-empty", () => {
    expect(pluginJson.description).toBeTypeOf("string");
    expect(pluginJson.description.length).toBeGreaterThan(20);
  });

  test("author is object with name", () => {
    expect(pluginJson.author).toBeTypeOf("object");
    expect(pluginJson.author.name).toBeTypeOf("string");
  });

  test("author.url is HTTPS URL", () => {
    expect(pluginJson.author.url).toMatch(/^https:\/\//);
  });

  test("homepage is HTTPS URL", () => {
    expect(pluginJson.homepage).toMatch(/^https:\/\//);
  });

  test("repository is STRING (not object) — Phase 6 incident #1", () => {
    expect(pluginJson.repository).toBeTypeOf("string");
    expect(typeof pluginJson.repository === "object").toBe(false);
  });

  test("repository ends in .git", () => {
    expect(pluginJson.repository).toMatch(/\.git$/);
  });

  test("license is recognized SPDX identifier", () => {
    expect(pluginJson.license).toMatch(/^(MIT|Apache-2\.0|BSD-3-Clause|ISC|GPL-3\.0(-only|-or-later)?)$/);
  });

  test("keywords is array if present", () => {
    if (pluginJson.keywords) {
      expect(Array.isArray(pluginJson.keywords)).toBe(true);
      expect(pluginJson.keywords.length).toBeGreaterThan(0);
    }
  });

  test("keywords contain 'axhub'", () => {
    expect(pluginJson.keywords).toContain("axhub");
  });

  test("no unknown top-level keys", () => {
    const allowed = new Set(["name", "version", "description", "author", "homepage", "repository", "license", "keywords"]);
    for (const key of Object.keys(pluginJson)) {
      expect(allowed.has(key)).toBe(true);
    }
  });

  test("version matches package.json version", () => {
    expect(pluginJson.version).toBe(packageJson.version);
  });

  test("description mentions axhub", () => {
    expect(pluginJson.description.toLowerCase()).toContain("axhub");
  });
});

// ---------------------------------------------------------------------------
// Reason: marketplace.json must be loadable by Claude Code marketplace add.
// ---------------------------------------------------------------------------
describe("marketplace.json schema", () => {
  test("name field present", () => {
    expect(marketplaceJson.name).toBeTypeOf("string");
    expect(marketplaceJson.name.length).toBeGreaterThan(0);
  });

  test("owner is object with name", () => {
    expect(marketplaceJson.owner).toBeTypeOf("object");
    expect(marketplaceJson.owner.name).toBeTypeOf("string");
  });

  test("owner.url is HTTPS URL", () => {
    expect(marketplaceJson.owner.url).toMatch(/^https:\/\//);
  });

  test("plugins is non-empty array", () => {
    expect(Array.isArray(marketplaceJson.plugins)).toBe(true);
    expect(marketplaceJson.plugins.length).toBeGreaterThan(0);
  });

  test("upgrade skill reads marketplace version fallback", () => {
    const marketplaceAxhub = marketplaceJson.plugins.find((p) => p.name === "axhub")!;
    expect(marketplaceAxhub.version).toBeTypeOf("string");
    expect(upgradeSkill).toContain("(.latest_version // .version // empty)");
  });

  test("upgrade helper fallback is cache-name agnostic", () => {
    expect(upgradeSkill).toContain(".claude/plugins/cache");
    expect(upgradeSkill).toContain("plugins/cache/*/*/*/bin/axhub-helpers");
    expect(upgradeSkill).not.toContain("cache/axhub/axhub/*/bin/axhub-helpers");
    expect(upgradeSkill).not.toContain("find \"$HOME/.claude/plugins/cache\"");
  });

  test("upgrade skill covers human plugin-latest phrasing", () => {
    expect(upgradeSkill).toContain("Claude에 설치된 axhub 플러그인도 최신인지 봐줘");
    expect(upgradeSkill).toContain("플러그인 최신인지 확인해줘");
    expect(upgradeSkill).toContain("axhub plugin latest");
  });

  test("each plugin has name", () => {
    for (const p of marketplaceJson.plugins) {
      expect(p.name).toBeTypeOf("string");
    }
  });

  test("each plugin has source path", () => {
    for (const p of marketplaceJson.plugins) {
      expect(p.source).toBeTypeOf("string");
    }
  });

  test("each plugin has description", () => {
    for (const p of marketplaceJson.plugins) {
      expect(p.description).toBeTypeOf("string");
      expect(p.description.length).toBeGreaterThan(10);
    }
  });

  test("each plugin has semver version", () => {
    for (const p of marketplaceJson.plugins) {
      expect(p.version).toMatch(/^\d+\.\d+\.\d+(-[a-z0-9.]+)?$/);
    }
  });

  test("plugin name in marketplace matches plugin.json name", () => {
    const axhub = marketplaceJson.plugins.find((p) => p.name === "axhub");
    expect(axhub).toBeDefined();
  });

  test("plugin version in marketplace matches plugin.json version", () => {
    const axhub = marketplaceJson.plugins.find((p) => p.name === "axhub")!;
    expect(axhub.version).toBe(pluginJson.version);
  });
});

// ---------------------------------------------------------------------------
// Reason: hooks.json must wrap event arrays in `hooks` outer key (Claude Code
// loader convention — bare event-keyed object fails to load).
// ---------------------------------------------------------------------------
describe("hooks.json structure", () => {
  test("outer wrapper has 'hooks' key", () => {
    expect(hooksJson.hooks).toBeTypeOf("object");
  });

  test("description present and non-empty", () => {
    expect(hooksJson.description).toBeTypeOf("string");
    expect(hooksJson.description!.length).toBeGreaterThan(10);
  });

  test("contains SessionStart event", () => {
    expect(hooksJson.hooks.SessionStart).toBeDefined();
  });

  test("contains PreToolUse event", () => {
    expect(hooksJson.hooks.PreToolUse).toBeDefined();
  });

  test("contains PostToolUse event", () => {
    expect(hooksJson.hooks.PostToolUse).toBeDefined();
  });

  test("contains UserPromptSubmit event", () => {
    expect(hooksJson.hooks.UserPromptSubmit).toBeDefined();
  });

  test("each event value is an array", () => {
    for (const [, group] of Object.entries(hooksJson.hooks)) {
      expect(Array.isArray(group)).toBe(true);
    }
  });

  test("each hook group has hooks array", () => {
    for (const [, group] of Object.entries(hooksJson.hooks)) {
      for (const g of group) {
        expect(Array.isArray(g.hooks)).toBe(true);
      }
    }
  });

  test("each hook config has type 'command'", () => {
    for (const [, group] of Object.entries(hooksJson.hooks)) {
      for (const g of group) {
        for (const h of g.hooks) {
          expect(h.type).toBe("command");
        }
      }
    }
  });

  test("each hook command references CLAUDE_PLUGIN_ROOT", () => {
    for (const [, group] of Object.entries(hooksJson.hooks)) {
      for (const g of group) {
        for (const h of g.hooks) {
          expect(h.command).toContain("${CLAUDE_PLUGIN_ROOT}");
        }
      }
    }
  });

  test("each hook command references axhub-helpers binary or session-start shim", () => {
    // SessionStart uses the universal bash shim; most hooks call the binary
    // directly; Phase 25 PR 25.3 introduces a TS PostToolUse hook executed
    // via the host bun runtime (hooks/*.ts).
    for (const [, group] of Object.entries(hooksJson.hooks)) {
      for (const g of group) {
        for (const h of g.hooks) {
          const refsBinary = h.command.includes("axhub-helpers");
          const refsShim = h.command.includes("hooks/session-start.sh");
          const refsAutowireShim = h.command.includes("hooks/session-start-autowire.sh");
          const refsTsHook = /hooks\/[a-z0-9_-]+\.ts\b/.test(h.command);
          expect(refsBinary || refsShim || refsAutowireShim || refsTsHook).toBe(true);
        }
      }
    }
  });

  test("each hook timeout is positive integer if set", () => {
    for (const [, group] of Object.entries(hooksJson.hooks)) {
      for (const g of group) {
        for (const h of g.hooks) {
          if (h.timeout !== undefined) {
            expect(h.timeout).toBeGreaterThan(0);
            expect(Number.isInteger(h.timeout)).toBe(true);
          }
        }
      }
    }
  });

  test("PreToolUse + PostToolUse have Bash matcher", () => {
    expect(hooksJson.hooks.PreToolUse[0].matcher).toBe("Bash");
    expect(hooksJson.hooks.PostToolUse[0].matcher).toBe("Bash");
  });

  test("UserPromptSubmit routes through axhub-helpers prompt-route", () => {
    const hook = hooksJson.hooks.UserPromptSubmit[0].hooks[0];
    expect(hook.command).toBe("bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh prompt-route");
    expect(hook.timeout).toBe(5);
  });

  // Regression: Claude Code evaluates every SessionStart sibling on the current host.
  // Do not register a universal shell:powershell sibling; macOS/Linux hosts without
  // pwsh/powershell surface a startup hook error before the Unix shim can help.
  test("SessionStart registers only the portable Unix shim in universal hooks.json", () => {
    expect(hooksJson.hooks.SessionStart.length).toBe(1);
  });

  test("SessionStart entry [0] is bash (Unix) — preserved byte-identical from v0.1.6", () => {
    const bashEntry = hooksJson.hooks.SessionStart[0].hooks[0];
    expect(bashEntry.command).toBe("bash ${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh");
    expect(bashEntry.timeout).toBe(30);
    expect(bashEntry.shell).toBeUndefined(); // bash is the implicit default
  });

  test("universal SessionStart hook does not require PowerShell on non-Windows hosts", () => {
    const commands = hooksJson.hooks.SessionStart.flatMap((entry) => entry.hooks.map((h) => h.command));
    const shells = hooksJson.hooks.SessionStart.flatMap((entry) => entry.hooks.map((h) => h.shell).filter(Boolean));
    expect(commands.some((command) => command.includes("session-start.ps1"))).toBe(false);
    expect(shells).not.toContain("powershell");
  });

  test("PreToolUse + PostToolUse register one Bash gate plus quality source-file gates", () => {
    expect(hooksJson.hooks.PreToolUse.map((g) => g.matcher).sort()).toEqual([
      "Bash",
      "Edit|Write|MultiEdit|NotebookEdit",
    ]);
    expect(hooksJson.hooks.PostToolUse.map((g) => g.matcher).sort()).toEqual([
      "Bash",
      "Edit|Write|MultiEdit|NotebookEdit",
    ]);
  });
});

// Phase 11 US-1103/US-1104: deferred-doc + executable-scaffold artifacts
// must exist on disk so next pilot session can run them without rediscovery.
describe("Phase 11 deferred-doc artifacts", () => {
  test("docs/pilot/windows-vm-smoke-checklist.md exists with 14 numbered steps", async () => {
    const checklist = await readFile(join(REPO_ROOT, "docs/pilot/windows-vm-smoke-checklist.md"), "utf8");
    // Top-level numbered steps "1." through "14." in the "## 14 manual steps" section
    const stepHeadings = checklist.match(/^\d+\.\s+\*\*[^*]+\*\*/gm);
    expect(stepHeadings).not.toBeNull();
    expect(stepHeadings!.length).toBe(14);
  });

  test("tests/smoke-windows-vm-checklist.ps1 exists with $env:AXHUB_VM_SMOKE guard", async () => {
    const ps1 = await readFile(join(REPO_ROOT, "tests/smoke-windows-vm-checklist.ps1"), "utf8");
    expect(ps1).toContain("$env:AXHUB_VM_SMOKE");
    expect(ps1).toContain("if ($env:AXHUB_VM_SMOKE -ne '1')");
    // 15 Run-Step calls (sh/ps1-absorption Phase 3.2 T2 added Step 14
    // auth-refresh-bg trigger check; original Summary moved to Step 15)
    const runSteps = ps1.match(/^Run-Step \d+/gm);
    expect(runSteps).not.toBeNull();
    expect(runSteps!.length).toBe(15);
  });

  test("docs/pilot/authenticode-signing-runbook.md exists", async () => {
    const runbook = await readFile(join(REPO_ROOT, "docs/pilot/authenticode-signing-runbook.md"), "utf8");
    expect(runbook).toContain("Sectigo");
    expect(runbook).toContain("AXHUB_SIGNING_STUB");
  });

  test(".github/workflows/sign-windows.yml.template exists with workflow_dispatch + continue-on-error", async () => {
    const wf = await readFile(join(REPO_ROOT, ".github/workflows/sign-windows.yml.template"), "utf8");
    expect(wf).toContain("workflow_dispatch:");
    expect(wf).toContain("continue-on-error: true");
    expect(wf).toContain("signtool verify");
  });

  test(".gitattributes contains *.yml.template linguist exemption", async () => {
    const gitattributes = await readFile(join(REPO_ROOT, ".gitattributes"), "utf8");
    expect(gitattributes).toContain("*.yml.template");
    expect(gitattributes).toContain("linguist-detectable=false");
  });
});

// ---------------------------------------------------------------------------
// Reason: Phase 6 incident #2 — hookSpecificOutput in helper code missing
// hookEventName field caused "Hook JSON output validation failed". Every
// emission MUST include hookEventName.
// ---------------------------------------------------------------------------
// hookSpecificOutput field validation moved to cargo test (Rust binary
// emissions). v0.2.0 TS shadow removal — see crates/axhub-helpers/tests/.

// ---------------------------------------------------------------------------
// Reason: commands/*.md frontmatter shape — Claude Code loader requires
// explicit command metadata. Phase 1 PLAN reconciliation makes
// description / allowed-tools / argument-hint / model mandatory for all
// command files so missing metadata cannot silently regress marketplace UX.
// ---------------------------------------------------------------------------
describe("commands/*.md frontmatter", () => {
  let cmdFiles: string[] = [];
  const cmdContents = new Map<string, string>();
  const expectedCommands = [
    "apps.md",
    "deploy.md",
    "doctor.md",
    "help.md",
    "login.md",
    "logs.md",
    "status.md",
    "update.md",
    "배포.md",
  ].sort();

  const frontmatterOf = (content: string): string => content.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
  const frontmatterValue = (content: string, key: string): string | undefined =>
    frontmatterOf(content).match(new RegExp(`^${key}:\\s*(.+)$`, "m"))?.[1]?.trim();

  beforeAll(async () => {
    const dir = join(REPO_ROOT, "commands");
    cmdFiles = (await readdir(dir)).filter((f) => f.endsWith(".md")).sort();
    for (const f of cmdFiles) {
      cmdContents.set(f, await readFile(join(dir, f), "utf8"));
    }
  });

  test("exactly 9 command files exist, including the Korean deploy alias", () => {
    expect(cmdFiles).toEqual(expectedCommands);
  });

  test("each command file has YAML frontmatter (--- delimited)", () => {
    for (const [, content] of cmdContents) {
      expect(content.startsWith("---\n")).toBe(true);
      const closeIdx = content.indexOf("\n---\n", 4);
      expect(closeIdx).toBeGreaterThan(0);
    }
  });

  test("each command frontmatter has all required metadata fields", () => {
    for (const [file, content] of cmdContents) {
      const fm = frontmatterOf(content);
      for (const key of ["description", "allowed-tools", "argument-hint", "model"]) {
        expect(fm, `${file} missing ${key}`).toMatch(new RegExp(`^${key}:\\s*.+`, "m"));
      }
    }
  });

  test("each command description is non-empty string", () => {
    for (const [, content] of cmdContents) {
      const description = frontmatterValue(content, "description");
      expect(description).toBeDefined();
      expect(description!.length).toBeGreaterThan(5);
    }
  });

  test("each command description ≤200 chars", () => {
    for (const [, content] of cmdContents) {
      const description = frontmatterValue(content, "description")!;
      expect(description.length).toBeLessThanOrEqual(200);
    }
  });

  test("commands without name in frontmatter (auto-derived from filename)", () => {
    for (const [, content] of cmdContents) {
      const fm = frontmatterOf(content);
      expect(fm).not.toMatch(/^name:\s/m);
    }
  });

  test("model field is present and valid Claude model", () => {
    const validModels = new Set(["sonnet", "opus", "haiku", "claude-sonnet-4-6", "claude-opus-4-7", "claude-haiku-4-5"]);
    for (const [, content] of cmdContents) {
      const model = frontmatterValue(content, "model");
      expect(model).toBeDefined();
      expect(validModels.has(model!)).toBe(true);
    }
  });

  test("CLI-wrapper commands use Haiku while risky mutation/recovery commands stay on Sonnet", () => {
    const haikuCommands = new Set(["apps.md", "help.md", "logs.md", "status.md"]);
    const sonnetCommands = new Set(["deploy.md", "배포.md", "doctor.md", "login.md", "update.md"]);

    for (const [file, content] of cmdContents) {
      const model = frontmatterValue(content, "model");
      if (haikuCommands.has(file)) {
        expect(model, `${file} should prefer fast Haiku for simple CLI wrapping`).toBe("haiku");
      } else if (sonnetCommands.has(file)) {
        expect(model, `${file} should keep Sonnet for auth, recovery, or deploy risk`).toBe("sonnet");
      } else {
        throw new Error(`Unclassified command model policy for ${file}`);
      }
    }
  });

  test("argument-hint is present and non-empty", () => {
    for (const [, content] of cmdContents) {
      const hint = frontmatterValue(content, "argument-hint");
      expect(hint).toBeDefined();
      expect(hint!.replace(/^"|"$/g, "").trim().length).toBeGreaterThan(0);
    }
  });

  test("body section exists after frontmatter", () => {
    for (const [, content] of cmdContents) {
      const closeIdx = content.indexOf("\n---\n", 4);
      const body = content.slice(closeIdx + 5).trim();
      expect(body.length).toBeGreaterThan(0);
    }
  });

  test("help command remains least-privilege and tool-free", () => {
    const help = cmdContents.get("help.md")!;
    expect(frontmatterValue(help, "allowed-tools")).toBe("[]");
    expect(help).not.toContain("Bash(");
    expect(help).not.toContain("AskUserQuestion");
  });

  test("help command refuses operational routing drift", () => {
    const help = cmdContents.get("help.md")!;
    expect(help).toContain("only for explicit help/menu requests");
    expect(help).toContain("dynamic table creation");
    expect(help).toContain("route to the matching axhub skill");
  });

  test("Korean deploy alias delegates to deploy skill without forking deploy logic", () => {
    const alias = cmdContents.get("배포.md")!;
    const deploy = cmdContents.get("deploy.md")!;
    expect(frontmatterValue(alias, "allowed-tools")).toBe(frontmatterValue(deploy, "allowed-tools"));
    expect(frontmatterValue(alias, "argument-hint")).toBe(frontmatterValue(deploy, "argument-hint"));
    expect(alias).toContain("skills/deploy/SKILL.md");
    expect(alias).toContain("Korean alias");
    expect(alias).not.toMatch(/^\s*axhub deploy create\b/m);
  });

  test("login command does not advertise unsupported token-file auth flags", () => {
    expect(cmdContents.get("login.md")!).not.toContain("--token-file");
  });

  test("help command exists", () => {
    expect(cmdFiles).toContain("help.md");
  });

  test("deploy command exists", () => {
    expect(cmdFiles).toContain("deploy.md");
  });

  test("Korean deploy alias exists", () => {
    expect(cmdFiles).toContain("배포.md");
  });

  test("login command exists (auth entrypoint)", () => {
    expect(cmdFiles).toContain("login.md");
  });

  test("commands allow axhub-helpers when their target skill invokes the helper binary", async () => {
    const skillByCommand = new Map([
      ["apps.md", "apps"],
      ["login.md", "auth"],
      ["deploy.md", "deploy"],
      ["배포.md", "deploy"],
    ]);
    for (const [commandFile, skillSlug] of skillByCommand) {
      const command = cmdContents.get(commandFile)!;
      const skill = await readFile(join(REPO_ROOT, "skills", skillSlug, "SKILL.md"), "utf8");
      if (skill.includes("axhub-helpers")) {
        expect(command, `${commandFile} delegates to skills/${skillSlug}/SKILL.md`).toContain(
          "Bash(axhub-helpers:*)",
        );
      }
    }
  });
});

// ---------------------------------------------------------------------------
// Reason: Phase 6 finding Q1 — skills/deploy/SKILL.md had `allowed-tools` in
// frontmatter (over-spec; not part of skill spec). All 11 skills must use only
// `name` + `description`.
// ---------------------------------------------------------------------------
describe("skills/*/SKILL.md frontmatter", () => {
  let skillDirs: string[] = [];
  const skillContents = new Map<string, string>();

  beforeAll(async () => {
    const dir = join(REPO_ROOT, "skills");
    skillDirs = (await readdir(dir, { withFileTypes: true }))
      .filter((d) => d.isDirectory())
      // Phase 18 R2 — exclude scaffold _template (sibling dir without SKILL.md
      // by design). Any leading-underscore dir is a scaffold helper, not a skill.
      .filter((d) => !d.name.startsWith("_"))
      .map((d) => d.name);
    for (const d of skillDirs) {
      const path = join(dir, d, "SKILL.md");
      if (existsSync(path)) {
        skillContents.set(d, await readFile(path, "utf8"));
      }
    }
  });

  test("at least 11 skills exist", () => {
    expect(skillDirs.length).toBeGreaterThanOrEqual(11);
  });

  test("all 44 shipped skills are present, including v0.17.3 CLI gap-fill skills + infer-tables-env", () => {
    expect(skillDirs.sort()).toEqual([
      "apis",
      "app-lifecycle",
      "apps",
      "auth",
      "axhub-debug",
      "axhub-diagnose",
      "axhub-plan",
      "axhub-review",
      "axhub-ship",
      "axhub-tdd",
      "browse",
      "clarify",
      "connectors",
      "data",
      "deploy",
      "doctor",
      "enable-statusline",
      "env",
      "github",
      "infer-tables-env",
      "init",
      "inspect",
      "install-cli",
      "karpathy-guidelines",
      "logs",
      "migrate",
      "my-resources",
      "open",
      "profile",
      "publish",
      "recover",
      "resources",
      "rollback",
      "routing-stats",
      "setup",
      "status",
      "tables",
      "team",
      "trace",
      "update",
      "upgrade",
      "using-axhub-quality",
      "verify",
      "workspace",
    ]);
  });

  test("each skill dir has SKILL.md", () => {
    for (const d of skillDirs) {
      expect(skillContents.has(d)).toBe(true);
    }
  });

  test("each SKILL.md starts with --- frontmatter", () => {
    for (const [, content] of skillContents) {
      expect(content.startsWith("---\n")).toBe(true);
    }
  });

  test("each SKILL.md frontmatter has name field", () => {
    for (const [, content] of skillContents) {
      const fm = content.split("\n---\n")[0];
      expect(fm).toMatch(/^name:\s*.+/m);
    }
  });

  test("each skill name matches its directory name", () => {
    for (const [d, content] of skillContents) {
      const m = content.match(/^name:\s*(.+)/m);
      expect(m![1].trim()).toBe(d);
    }
  });

  test("each SKILL.md frontmatter has description field", () => {
    for (const [, content] of skillContents) {
      const fm = content.split("\n---\n")[0];
      expect(fm).toMatch(/^description:\s*.+/m);
    }
  });

  test("setup skill covers human first-run phrasing", () => {
    const setup = skillContents.get("setup")!;
    expect(setup).toContain("처음 쓰는데");
    expect(setup).toContain("뭐부터 하면 돼");
    expect(setup).toContain("axhub 처음 쓰는데 뭐부터 하면 돼?");
  });

  test("auth skill covers human login-status phrasing", () => {
    const auth = skillContents.get("auth")!;
    expect(auth).toContain("나 로그인 돼 있어?");
    expect(auth).toContain("로그인 상태 좀 봐줘");
    expect(auth).toContain("로그인 상태 확인해줘");
    expect(auth).toContain("지금 로그인 필요한 상태인지 봐줘");
    expect(auth).toContain("axhub 로그인 상태 좀 봐줘");
  });

  test("auth Desktop contract keeps plain login-status prompts safe", () => {
    const auth = skillContents.get("auth")!;
    expect(auth).toContain("Claude Desktop natural-language contract");
    expect(auth).toContain("로그인 상태를 확인할게요");
    expect(auth).toContain("axhub-helpers auth-summary --user-utterance");
    expect(auth).toContain("설치 상태 점검, 환경 진단, 업데이트 확인, 새 로그인, 로그아웃, 계정 상세 표시 같은 다른 작업으로 넘어가지 않아요");
    expect(auth).not.toContain("setup, doctor, update, install, login, logout 로 우회하지");
    expect(auth).toContain("계정 상세 표시");
    expect(auth).toContain("계정 이메일, raw user id, tenant/workspace 이름, profile 이름, scope, 정확한 만료 시각");
    expect(auth).toContain("사용자가 `어떤 계정이야`, `whoami`, `권한 보여줘`, `scope 보여줘`처럼");
    expect(auth).toContain("Do not show scopes for plain login-status questions");
  });

  test("auth status card does not default to full user_id disclosure", () => {
    const auth = skillContents.get("auth")!;
    expect(auth).not.toContain("계정: <user_email>  (user_id: <user_id>)");
    expect(auth).toContain("기본 상태 카드에서는 `user_id` UUID 를 표시하지 마세요");
    expect(auth).toContain("user_id: ...<last8>");
  });

  test("workspace skill covers human team/workspace membership phrasing", () => {
    const workspace = skillContents.get("workspace")!;
    expect(workspace).toContain("내가 속한 팀이랑 워크스페이스 보여줘");
    expect(workspace).toContain("내가 속한 팀");
    expect(workspace).toContain("자신의 axhub 팀/워크스페이스/테넌트 membership");
    expect(workspace).toContain("my teams and workspaces");
    expect(workspace).toContain("팀 멤버 초대/권한 변경 같은 관리는 team 스킬");
  });

  test("team skill handles pure Desktop invite phrasing without agent-team ambiguity", () => {
    const team = skillContents.get("team")!;
    expect(team).toContain("팀원 초대해");
    expect(team).toContain("팀 작업을 확인할게요.");
    expect(team).toContain("팀 작업 확인");
    expect(team).toContain("axhub-helpers team-summary --user-utterance");
    expect(team).toContain("do not ask whether the user means a Claude/OMC multi-agent team");
    expect(team).toContain("Do not mention or display route labels");
    expect(team).toContain("current_team_id");
    expect(team).toContain("raw emails that the user did not type");
  });

  test("browse skill covers pure template discovery phrasing", () => {
    const browse = skillContents.get("browse")!;
    expect(browse).toContain("템플릿 뭐 있어");
    expect(browse).toContain('utterance: "템플릿 뭐 있어?"');
    expect(browse).toContain("내 앱 목록은 apps 스킬");
  });

  test("init skill forbids visible internal routing label narration", () => {
    const init = skillContents.get("init")!;
    expect(init).toContain('the first visible chat sentence must be exactly "새 앱을 만들 수 있는 템플릿을 확인할게요."');
    expect(init).toContain("do not restate badge metadata in prose");
    expect(init).toContain("새 앱을 만들 수 있는 템플릿을 확인할게요");
  });

  test("inspect skill covers human manifest/config review phrasing", () => {
    const inspect = skillContents.get("inspect")!;
    expect(inspect).toContain("매니페스트랑 설정 괜찮은지 봐줘");
    expect(inspect).toContain("the first visible chat sentence must be exactly `매니페스트와 설정을 확인할게요.`");
    expect(inspect).toContain("with no planning sentence before or after it");
    expect(inspect).toContain("copy the Korean stdout as the answer and do not reinterpret it");
    expect(inspect).toContain("Do not narrate internal routing labels");
    expect(inspect).toContain("Do not start with repository file discovery");
    expect(inspect).toContain("do not run raw `axhub manifest validate`");
    expect(inspect).toContain("plugin package inspection");
    expect(inspect).toContain("Read axhub.yaml");
    expect(inspect).toContain("For Bash tool calls, set the tool `description` or title exactly");
    expect(inspect).toContain("매니페스트와 설정 확인");
    expect(inspect).toContain("inspect-config-summary");
    expect(inspect).toContain("Natural-language summary contract");
    expect(inspect).toContain("Do not use markdown tables");
    expect(inspect).toContain("Do not show raw command names");
    expect(inspect).toContain("raw JSON field names");
    expect(inspect).toContain("hook labels");
    expect(inspect).toContain("workflow labels");
    expect(inspect).toContain("Use at most four concise bullets");
    expect(inspect).toContain("로그인 정보가 서로 다르게 보여서 배포 전에 다시 로그인 확인이 필요할 수 있어요");
    expect(inspect).not.toContain("AXHub inspect summary helper");
    expect(inspect).not.toContain("summary helper command");
  });

  test("status skill uses a single human Desktop summary path", () => {
    const status = skillContents.get("status")!;
    expect(status).toContain("배포 상태를 확인할게요.");
    expect(status).toContain("배포 상태 확인");
    expect(status).toContain("status-summary --user-utterance");
    expect(status).toContain("Do not show intermediate resolver failures");
    expect(status).toContain("For ordinary Claude Desktop status questions, stop after this step");
    expect(status).not.toContain("APP 미해석");
    expect(status).not.toContain("items[0]");
    expect(status).not.toContain("Resolve app and list deployments");
  });

  test("logs skill uses a single human Desktop summary path", () => {
    const logs = skillContents.get("logs")!;
    expect(logs).toContain("로그를 확인할게요.");
    expect(logs).toContain("로그 확인");
    expect(logs).toContain("logs-summary --user-utterance");
    expect(logs).toContain("For ordinary Claude Desktop log questions, stop after this step");
    expect(logs).toContain("Do not show intermediate resolver text");
    expect(logs).not.toContain("Resolve deployment first");
    expect(logs).not.toContain("No deploy id cached");
    expect(logs).not.toContain("List deployments for app");
    expect(logs).not.toContain("Fetch build logs snapshot");
  });

  test("open skill uses a single human Desktop summary path", () => {
    const open = skillContents.get("open")!;
    expect(open).toContain("라이브 페이지 열어봐");
    expect(open).toContain("앱 페이지를 확인할게요.");
    expect(open).toContain("앱 페이지 확인");
    expect(open).toContain("open-summary --user-utterance");
    expect(open).toContain("For ordinary Claude Desktop open/browser questions, stop after this step");
    expect(open).toContain("Do not show QA-result-file reads");
    expect(open).toContain("ToolSearch narration");
  });

  test("verify skill uses a single human Desktop summary path", () => {
    const verify = skillContents.get("verify")!;
    expect(verify).toContain("방금 배포 진짜 열리는지 확인해줘");
    expect(verify).toContain("배포가 실제로 열리는지 확인할게요.");
    expect(verify).toContain("배포 검증");
    expect(verify).toContain("verify-summary --user-utterance");
    expect(verify).toContain("For ordinary Claude Desktop verify questions, stop after this step");
    expect(verify).toContain("Do not show routing labels");
    expect(verify).toContain("stale cache IDs");
    expect(verify).not.toContain("→ verify skill 호출");
  });

  test("trace skill uses a single human Desktop summary path", () => {
    const trace = skillContents.get("trace")!;
    expect(trace).toContain("배포 실패 원인 알려줘");
    expect(trace).toContain("배포 기록을 확인할게요.");
    expect(trace).toContain("배포 기록 확인");
    expect(trace).toContain("trace-summary --user-utterance");
    expect(trace).toContain("For ordinary Claude Desktop failure-cause questions, stop after this step");
    expect(trace).toContain("Do not show routing labels");
    expect(trace).toContain("failure_reason");
    expect(trace).toContain("matched_patterns");
    expect(trace).toContain("build_log_errors");
    expect(trace).not.toContain("→ trace skill 호출");
  });

  test("routing-stats skill uses a single human Desktop summary path", () => {
    const routingStats = skillContents.get("routing-stats")!;
    expect(routingStats).toContain("이번 주 axhub 라우팅 어땠어?");
    expect(routingStats).toContain("라우팅 통계를 확인할게요.");
    expect(routingStats).toContain("라우팅 통계 확인");
    expect(routingStats).toContain("routing-stats --since 7d");
    expect(routingStats).toContain("do not read QA result files");
    expect(routingStats).toContain("Do not show raw command names");
  });

  test("env skill uses a single masked Desktop summary path for read-only queries", () => {
    const env = skillContents.get("env")!;
    expect(env).toContain("환경변수 뭐 있어?");
    expect(env).toContain("환경변수를 확인할게요.");
    expect(env).toContain("환경변수 확인");
    expect(env).toContain("env-summary --user-utterance");
    expect(env).toContain("For ordinary Claude Desktop env questions, stop after this step");
    expect(env).toContain("Do not show raw values or secret values");
    expect(env).toContain("셸 환경변수");
    expect(env).toContain(".env");
  });

  test("doctor skill uses a single safe Desktop install-status summary path", () => {
    const doctor = skillContents.get("doctor")!;
    expect(doctor).toContain("axhub CLI 설치 상태 괜찮아?");
    expect(doctor).toContain("설치 상태를 확인할게요.");
    expect(doctor).toContain("설치 상태 확인");
    expect(doctor).toContain("doctor-summary --user-utterance");
    expect(doctor).toContain("For ordinary Claude Desktop install/setup/status questions, stop after this step");
    expect(doctor).toContain("Do not run installers, updates, login, logout");
    expect(doctor).toContain("raw user emails");
    expect(doctor).toContain("preflight narration");
  });

  test("install-cli skill checks existing install before installer flow", () => {
    const install = skillContents.get("install-cli")!;
    expect(install).toContain("axhub CLI 설치해줘");
    expect(install).toContain("설치 상태를 확인할게요.");
    expect(install).toContain("설치 상태 확인");
    expect(install).toContain("install-summary --user-utterance");
    expect(install).toContain("If stdout says the CLI is already installed");
    expect(install).toContain("Do not run installer commands");
    expect(install).toContain("preflight");
    expect(install).toContain("English tool-title fragments");
  });

  test("update skill uses a single human Desktop summary path for check-only prompts", () => {
    const update = skillContents.get("update")!;
    expect(update).toContain("업데이트 필요한지 봐줘");
    expect(update).toContain("업데이트를 확인할게요.");
    expect(update).toContain("업데이트 확인");
    expect(update).toContain("update-summary --user-utterance");
    expect(update).toContain("raw JSON field names such as `has_update`");
    expect(update).toContain("do not run `axhub update apply` yet");
  });

  test("rollback and recover use a safe Desktop restore summary before mutation", () => {
    for (const slug of ["rollback", "recover"]) {
      const content = skillContents.get(slug)!;
      expect(content).toContain("Claude Desktop natural-language path");
      expect(content).toContain("방금 배포 되돌려줘");
      expect(content).toContain("되돌릴 수 있는 배포를 확인할게요.");
      expect(content).toContain("배포 되돌리기 확인");
      expect(content).toContain("rollback-summary --user-utterance");
      expect(content).toContain("Copy the Korean stdout as the answer, then stop");
      expect(content).toContain("do not run preflight/list/rollback/recover/create directly");
      expect(content).toContain("raw deploy IDs");
      expect(content).toContain("commit_not_found");
      expect(content).toContain("no-op");
      expect(content).toContain("explicitly approves in a later turn");
    }
  });

  test("enable-statusline skill uses a single safe Desktop statusbar path", () => {
    const statusline = skillContents.get("enable-statusline")!;
    expect(statusline).toContain("상태바 켜줘");
    expect(statusline).toContain("상태바 설정을 확인할게요.");
    expect(statusline).toContain("상태바 설정");
    expect(statusline).toContain("statusline-summary --user-utterance");
    expect(statusline).toContain("For ordinary Claude Desktop prompts");
    expect(statusline).toContain("Preserve an existing third-party status bar");
    expect(statusline).toContain("Do not run `settings-merge`");
    expect(statusline).toContain("existing command strings");
    expect(statusline).toContain("statusLine");
    expect(statusline).toContain("wire");
  });

  test("workspace helper fallback is cache-name agnostic", () => {
    const workspace = skillContents.get("workspace")!;
    expect(workspace).toContain("plugins/cache/*/*/*/bin/axhub-helpers");
    expect(workspace).not.toContain("cache/axhub/axhub/*/bin/axhub-helpers");
  });

  test("all skill helper fallbacks are local-plugin cache-name agnostic", () => {
    for (const [slug, content] of skillContents) {
      expect(content, slug).not.toContain("cache/axhub/axhub/*/bin/axhub-helpers");
      if (content.includes(".claude/plugins/cache") && content.includes("axhub-helpers")) {
        expect(content, slug).toContain("plugins/cache/*/*/*/bin/axhub-helpers");
      }
    }
  });

  test("skills prefer natural-language handoffs for Desktop users", () => {
    for (const [slug, content] of skillContents) {
      expect(content, slug).toContain("User-facing handoff language");
      expect(content, slug).toContain("prefer natural phrases the user can say");
      expect(content, slug).toContain("do not tell a Desktop user to type `/axhub:*`");
    }
  });

  test("preflight and failure handoffs do not surface slash commands to Desktop users", () => {
    const forbidden = [
      "auth_ok` 가 false 면 `/axhub:auth`",
      "→ `/axhub:install-cli`",
      "→ `/axhub:auth`",
      "→ `/axhub:upgrade`",
      "먼저 /axhub:deploy",
      "/axhub:doctor 로 진단",
      "/axhub:status <IN_FLIGHT_DEPLOY_ID>",
    ];

    for (const [slug, content] of skillContents) {
      for (const needle of forbidden) {
        expect(content, `${slug} should not contain ${needle}`).not.toContain(needle);
      }
    }
  });

  test("my-resources uses a single normalized summary flow in Desktop", () => {
    const myResources = skillContents.get("my-resources")!;
    expect(myResources).toContain("한 번에 요약을 만들어요");
    expect(myResources).toContain("위 Bash 출력은 이미 최종 Markdown");
    expect(myResources).toContain("추가 jq probing");
    expect(myResources).toContain("다시 로그인해줘라고 말하면");
    expect(myResources).not.toContain("로그인이 필요해요. /axhub:auth");
  });

  test("connectors Desktop contract treats database connection as AXHub connector work", () => {
    const connectors = skillContents.get("connectors")!;
    expect(connectors).toContain("Postgres 데이터베이스 연결하고 싶어");
    expect(connectors).toContain("데이터베이스 연결을 준비할게요");
    expect(connectors).toContain("로컬 앱 코드 수정으로 우회하지 않고 AXHub 외부 데이터베이스 연결 설정");
    expect(connectors).toContain("workflow`, `워크플로`, skill 이름");
    expect(connectors).toContain("계정 이메일, raw user id 를 쓰지 말고");
    expect(connectors).toContain("SAFE_PREFLIGHT_JSON");
    expect(connectors).toContain("server.js");
    expect(connectors).toContain("DATABASE_URL");
    expect(connectors).toContain("A/B");
    expect(connectors).toContain("비밀값은 채팅 평문으로 받지 않아요");
  });

  test("data Desktop contract keeps natural data-read prompts human-readable", () => {
    const data = skillContents.get("data")!;
    expect(data).toContain("orders 데이터 조회해줘");
    expect(data).toContain("데이터 리소스를 확인할게요");
    expect(data).toContain("workflow`, `워크플로`, skill 이름");
    expect(data).toContain("preflight`, `catalog 조회`, `catalog 비어있음`");
    expect(data).toContain("계정 이메일, raw user id, scope 를 쓰지 말고");
    expect(data).toContain("SAFE_PREFLIGHT_JSON");
    expect(data).toContain("현재 연결된 데이터 리소스를 찾지 못했어요");
    expect(data).toContain("명시적 승인 전에는 실행하지 않아요");
  });

  test("resources Desktop contract treats cleanup as AXHub resource organization", () => {
    const resources = skillContents.get("resources")!;
    expect(resources).toContain("리소스 정리하고 싶어");
    expect(resources).toContain("리소스 정리 방식을 확인할게요");
    expect(resources).toContain("axhub-helpers resources-summary --user-utterance");
    expect(resources).toContain("리소스 현황 확인");
    expect(resources).toContain("AXHub gateway resource organization");
    expect(resources).toContain("not local file cleanup");
    expect(resources).toContain("Do not say the prompt is `모호`");
    expect(resources).toContain(".shim");
    expect(resources).toContain(".omc");
    expect(resources).toContain("QA result files");
    expect(resources).toContain("git status");
    expect(resources).toContain("어떤 정리를 할까요?");
    expect(resources).toContain("AskUserQuestion, Question, or a question-card tool");
    expect(resources).toContain("raw question JSON");
    expect(resources).toContain("local artifact names");
    expect(resources).toContain("Do not say resource changes are impossible");
    expect(resources).toContain("`catalog kinds`, `connector/resource`");
  });

  test("tables Desktop contract keeps natural table mutations human-readable", () => {
    const tables = skillContents.get("tables")!;
    expect(tables).toContain("orders 테이블 만들고 title 컬럼도 넣어줘");
    expect(tables).toContain("테이블 변경 내용을 확인할게요");
    expect(tables).toContain("로컬 앱 코드, local database, SQL migration, ORM");
    expect(tables).toContain("raw CLI command line 은 사용자 답변에 쓰지 말고");
    expect(tables).toContain("Claude Desktop 에서는 AskUserQuestion, Question, 질문 카드 도구를 쓰지 않아요");
    expect(tables).toContain("raw question JSON");
    expect(tables).toContain("workflow`, `워크플로`, skill 이름");
    expect(tables).toContain("preflight`, `consent-mint`, consent 내부값");
    expect(tables).toContain("계정 이메일, raw user id, scope 를 쓰지 말고");
    expect(tables).toContain("SAFE_PREFLIGHT_JSON");
    expect(tables).toContain("표시한 변경만 한 번 실행해요");
  });

  test("apis empty catalog response stays human-readable in Desktop", () => {
    const apis = skillContents.get("apis")!;
    expect(apis).toContain("현재 권한에서 바로 사용할 수 있는 API 카탈로그는 없어요");
    expect(apis).toContain("Do not say `preflight`, `catalog 조회`, `items 비어`, `items.length`, or `--search`");
    expect(apis).toContain("the first visible chat sentence must be exactly `사용 가능한 API를 확인할게요.`");
    expect(apis).toContain("Do not include raw email addresses, app slugs, profile IDs, team IDs, or `current_app` values");
    expect(apis).toContain("use `로그인 상태 확인`");
    expect(apis).toContain("use `사용 가능한 API 확인`");
    expect(apis).toContain("검색어를 알려주면 더 좁혀볼게요");
  });

  test("each description starts with 'This skill' or '이 스킬' (Korean equivalent — Phase 5 한국어 전환)", () => {
    for (const [, content] of skillContents) {
      const m = content.match(/^description:\s*(.+)/m);
      const description = m![1].replace(/^['"]|['"]$/g, "");
      expect(description).toMatch(/^(This skill|이 스킬)/);
    }
  });

  test("NO skill has allowed-tools in frontmatter — Phase 6 Q1 finding", () => {
    for (const [, content] of skillContents) {
      const fm = content.split("\n---\n")[0];
      expect(fm).not.toMatch(/^allowed-tools:/m);
    }
  });

  test("model field (Phase 25 PR 25.5a+) is haiku|sonnet|opus when declared", () => {
    const validModels = new Set(["haiku", "sonnet", "opus"]);
    for (const [, content] of skillContents) {
      const fm = content.split("\n---\n")[0];
      const match = fm.match(/^model:\s*([a-z]+)\s*$/m);
      if (match) {
        expect(validModels.has(match[1])).toBe(true);
      }
    }
  });

  test("frontmatter contains ONLY allowed keys (Phase 18: + multi-step / needs-preflight; Phase 9: + examples; Phase 25 PR 25.5a+: + model)", () => {
    for (const [, content] of skillContents) {
      const fm = content.split("\n---\n")[0].slice(4);
      const keys = fm.match(/^[a-z-]+:/gm) ?? [];
      const allowed = new Set([
        "name:",
        "description:",
        "multi-step:",
        "needs-preflight:",
        "allows-dependency-execution:",
        "examples:",
        "model:",
      ]);
      for (const k of keys) {
        expect(allowed.has(k)).toBe(true);
      }
    }
  });

  test("description includes Korean trigger phrases (per skill convention)", () => {
    for (const [, content] of skillContents) {
      const m = content.match(/^description:\s*(.+)/m);
      // At least one Hangul char in description
      expect(m![1]).toMatch(/[ㄱ-ㆎ가-힣]/);
    }
  });

  test("description stays YAML-plain-safe for Claude plugin validation", () => {
    for (const [, content] of skillContents) {
      const m = content.match(/^description:\s*(.+)/m);
      // Claude Code's plugin validator parses SKILL frontmatter as YAML.
      // Plain scalars cannot contain ": " because that starts a mapping token,
      // so long natural-language descriptions with trigger labels must be
      // quoted.
      const description = m![1];
      const isQuoted = /^'.*'$|^".*"$/.test(description);
      expect(isQuoted || !/:\s/.test(description)).toBe(true);
    }
  });

  test("body section exists after frontmatter", () => {
    for (const [, content] of skillContents) {
      const closeIdx = content.indexOf("\n---\n", 4);
      const body = content.slice(closeIdx + 5).trim();
      expect(body.length).toBeGreaterThan(50);
    }
  });

  test("description length reasonable (≤2000 chars — skill activation dispatcher)", () => {
    for (const [, content] of skillContents) {
      const m = content.match(/^description:\s*(.+)/m);
      expect(m![1].length).toBeLessThanOrEqual(2000);
    }
  });

  test("expected 10 specific skills present", () => {
    const expected = ["apps", "auth", "clarify", "deploy", "doctor", "logs", "recover", "status", "update", "upgrade"];
    for (const e of expected) {
      expect(skillDirs).toContain(e);
    }
  });

  test("doctor skill distinguishes Windows helper install states", () => {
    const doctorContent = skillContents.get("doctor")!;
    expect(doctorContent).toContain("Get-Command axhub-helpers");
    expect(doctorContent).toContain("axhub-helpers.exe");
    expect(doctorContent).toContain("axhub-helpers-windows-amd64.exe");
    expect(doctorContent).toContain("install.ps1");
    expect(doctorContent).toContain("powershell -NoProfile -ExecutionPolicy Bypass -File");
  });

  test("deploy skill has body referencing axhub-helpers binary", () => {
    const deployContent = skillContents.get("deploy")!;
    expect(deployContent).toContain("axhub-helpers");
  });

  test("deploy skill forbids visible internal routing narration on Desktop", () => {
    const deployContent = skillContents.get("deploy")!;
    expect(deployContent).toContain("Claude Desktop Natural-Language Path");
    expect(deployContent).toContain("The first visible chat sentence must be exactly `배포 준비를 확인할게요.`");
    expect(deployContent).toContain("stop reading this skill after this section");
    expect(deployContent).toContain("axhub-helpers deploy-preview-summary --user-utterance");
    expect(deployContent).toContain("axhub-helpers deploy-approved-run --user-utterance");
    expect(deployContent).toContain("Do not call this skill again after approval");
    expect(deployContent).toContain("Do not echo the user's phrase as a route conversion");
    expect(deployContent).toContain("Invoke deploy skill");
    expect(deployContent).toContain("Read rest of SKILL");
    expect(deployContent).toContain("Route=axhub");
    expect(deployContent).toContain("consent token");
    expect(deployContent).toContain("Korean titles only");
    expect(deployContent).toContain("Before any destructive deploy, show only the Korean preview card");
    expect(deployContent).not.toContain("Observed: axhub deploy/create prompt");
    expect(deployContent).not.toContain("Suggested: use the AXHub deploy workflow");
  });

  test("clarify skill keeps Desktop question card natural", () => {
    const clarifyContent = skillContents.get("clarify")!;
    expect(clarifyContent).toContain("Claude Desktop Natural-Language Path");
    expect(clarifyContent).toContain("First visible chat sentence must be exactly `어떤 걸 도와드릴까요?`");
    expect(clarifyContent).toContain("환경 점검");
    expect(clarifyContent).toContain("앱 배포");
    expect(clarifyContent).toContain("앱과 리소스 조회");
    expect(clarifyContent).toContain("문제 원인 보기");
    expect(clarifyContent).toContain("처음부터 안내");
    expect(clarifyContent).toContain("set each `value` to exactly the same Korean text as its visible `label`");
    expect(clarifyContent).toContain("Selected Option Handoff");
    expect(clarifyContent).toContain("Do not call another skill from this skill");
    expect(clarifyContent).toContain("설치 상태를 확인할게요.");
    expect(clarifyContent).toContain("axhub-helpers doctor-summary --user-utterance");
    expect(clarifyContent).toContain('"value": "환경 점검"');
    expect(clarifyContent).toContain('"value": "앱 배포"');
    expect(clarifyContent).toContain("NEVER include parenthesized internal labels");
    expect(clarifyContent).not.toContain('"value": "doctor"');
    expect(clarifyContent).not.toContain('"value": "deploy"');
    expect(clarifyContent).not.toContain("invoke the matching sibling skill");
    expect(clarifyContent).not.toContain("Route to chosen skill");
    expect(clarifyContent).not.toContain("CLI 설치/인증/버전 점검 (doctor)");
    expect(clarifyContent).not.toContain("현재 브랜치 axhub 라이브 배포 (deploy)");
    expect(clarifyContent).not.toContain("너무 막연");
    expect(clarifyContent).not.toContain("axhub doctor 스킬 실행");
    expect(clarifyContent).not.toContain("읽는 중 SKILL.md");
  });

  test("app-lifecycle skill keeps Desktop mutation path human-readable", () => {
    const appLifecycleContent = skillContents.get("app-lifecycle")!;
    expect(appLifecycleContent).toContain("Claude Desktop Natural-Language Path");
    expect(appLifecycleContent).toContain("Claude Desktop 일반 자연어 요청은 UserPromptSubmit hook의 inline flow가 처리");
    expect(appLifecycleContent).toContain("prompt-route inline flow in Claude Desktop");
    expect(appLifecycleContent).toContain("앱 잠깐 멈춰");
    expect(appLifecycleContent).toContain("앱 다시 올려");
    expect(appLifecycleContent).toContain("testnextjs 다시 켜줘");
    expect(appLifecycleContent).toContain("testnextjs 멈춰줘");
    expect(appLifecycleContent).toContain("앱을 잠깐 멈출 준비를 할게요.");
    expect(appLifecycleContent).toContain("앱을 다시 켤 준비를 할게요.");
    expect(appLifecycleContent).toContain("앱 상태 확인");
    expect(appLifecycleContent).toContain("앱 찾기");
    expect(appLifecycleContent).toContain("앱 변경 준비");
    expect(appLifecycleContent).toContain("앱 변경 실행");
    expect(appLifecycleContent).toContain("앱 변경을 실행할까요?");
    expect(appLifecycleContent).toContain('"label":"취소"');
    expect(appLifecycleContent).toContain('"value":"취소"');
    expect(appLifecycleContent).toContain('"label":"진행"');
    expect(appLifecycleContent).toContain('"value":"진행"');
    expect(appLifecycleContent).toContain("Pick exactly one branch");
    expect(appLifecycleContent).toContain("Do not combine the preparation command and the app-changing command");
    expect(appLifecycleContent).toContain("Between the user's `진행` answer and the first Bash tool call, do not write a visible chat sentence");
    expect(appLifecycleContent).toContain("Never say `User chose`, `Mint consent`, `execute suspend`");
    expect(appLifecycleContent).toContain("literal 앱 인자와 정확히 같아야 해요");
    expect(appLifecycleContent).toContain("not a resolved UUID");
    expect(appLifecycleContent).toContain("trailing success echo");
    expect(appLifecycleContent).toContain("consent-mint-app-lifecycle");
    expect(appLifecycleContent).toContain("--action suspend --app");
    expect(appLifecycleContent).toContain("--action resume --app");
    expect(appLifecycleContent).toContain("--action fork --app");
    expect(appLifecycleContent).toContain('axhub apps suspend "$APP_ARG" --execute --json >/dev/null');
    expect(appLifecycleContent).toContain('axhub apps resume "$APP_ARG" --execute --json >/dev/null');
    expect(appLifecycleContent).toContain('axhub apps fork "$SOURCE_APP" --slug "$NEW_SLUG" --subdomain "$NEW_SUBDOMAIN" --name "$NAME" --tenant "$TENANT" --execute --json >/dev/null');
    expect(appLifecycleContent).toContain("raw JSON stdout");
    expect(appLifecycleContent).toContain("비공개 (private)");
    expect(appLifecycleContent).toContain("[DESTRUCTIVE] about to run");
    expect(appLifecycleContent).toContain("do not run another preparation/execution pair");
    expect(appLifecycleContent).toContain("같은 변경을 다시 준비하거나 다시 실행하지 않아요");
    expect(appLifecycleContent).toContain("APP_ARG");
    expect(appLifecycleContent).toContain("account emails");
    expect(appLifecycleContent).toContain("JSON, schema, fixture, helper source");
    expect(appLifecycleContent).toContain("NEVER include parenthesized internal labels");
    expect(appLifecycleContent).toContain("NEVER mention internal authorization primitives");
    expect(appLifecycleContent).not.toContain("echo done");
    expect(appLifecycleContent).not.toContain("auth OK, current_app");
    expect(appLifecycleContent).not.toContain("app_id resolve");
    expect(appLifecycleContent).not.toContain("ID를 확인할게요");
    expect(appLifecycleContent).not.toContain("승인 토큰");
    expect(appLifecycleContent).not.toContain("앱 resolve");
    expect(appLifecycleContent).not.toContain("caveat 안내");
    expect(appLifecycleContent).not.toContain("preview 준비");
    expect(appLifecycleContent).not.toContain("実行");
  });

  test("auth skill has body referencing consent-mint (US-004 outcome)", () => {
    const authContent = skillContents.get("auth")!;
    expect(authContent).toContain("consent-mint");
  });

  test("Phase 5 US-505: update skill does NOT force-set AXHUB_DISABLE_AUTOUPDATE=1", () => {
    const updateContent = skillContents.get("update")!;
    // Plugin must respect company policy — disable should only happen when
    // user/admin explicitly sets the env var, not when the plugin forces it.
    expect(updateContent).not.toMatch(/AXHUB_DISABLE_AUTOUPDATE=1\s+axhub/);
  });

  test("skills use stdin JSON consent-mint instead of unsupported flags", () => {
    for (const [slug, content] of skillContents) {
      expect(content, slug).not.toMatch(/consent-mint\s+--/);
    }
    for (const slug of ["deploy", "recover", "auth"]) {
      const content = skillContents.get(slug)!;
      // consent-mint pipes stdin JSON into the resolved "$HELPER" (no flags), and
      // "$HELPER" is resolved robustly (plugin-root → PATH → versioned cache scan)
      // so an empty CLAUDE_PLUGIN_ROOT in the Bash tool no longer breaks the gate.
      expect(content, slug).toMatch(/\|\s*"\$HELPER" consent-mint/);
      expect(content, slug).toMatch(/HELPER="\$\{CLAUDE_PLUGIN_ROOT:\+\$CLAUDE_PLUGIN_ROOT\/bin\/axhub-helpers\}"/);
    }
    const appLifecycleContent = skillContents.get("app-lifecycle")!;
    expect(appLifecycleContent).toContain('"$HELPER" consent-mint-app-lifecycle');
    expect(appLifecycleContent).not.toMatch(/\|\s*"\$HELPER" consent-mint/);
  });

  test("destructive skill consent examples do not require POSIX-only session unsetting", () => {
    for (const slug of ["deploy", "recover", "auth"]) {
      const content = skillContents.get(slug)!;
      expect(content, slug).not.toContain("unset CLAUDE_SESSION_ID");
    }
  });

  test("auth headless token-paste docs use token-import and the plugin token path", () => {
    const files = [
      skillContents.get("auth")!,
      skillContents.get("deploy")!,
      skillContents.get("recover")!,
    ];
    for (const content of files) {
      expect(content).not.toContain("token-install");
      expect(content).not.toContain("~/.config/axhub/token");
    }
    expect(skillContents.get("auth")!).toContain("token-import");
    expect(skillContents.get("auth")!).toContain("~/.config/axhub-plugin/token");
  });

  test("auth logout path prompts with AskUserQuestion before running axhub auth logout", () => {
    const authContent = skillContents.get("auth")!;
    const logoutIdx = authContent.indexOf("axhub auth logout");
    expect(logoutIdx).toBeGreaterThan(0);
    const logoutSectionPrefix = authContent.slice(0, logoutIdx);
    expect(logoutSectionPrefix).toContain('"question": "로그아웃할래요?"');
    expect(logoutSectionPrefix).toContain('"value": "abort"');
  });

  test("skills do not instruct unavailable helper-schedule command", () => {
    for (const [slug, content] of skillContents) {
      expect(content, slug).not.toContain("axhub-helpers schedule");
    }
  });

  test("github skill mints consent before connect/disconnect and avoids manual hook bypass", () => {
    const github = skillContents.get("github")!;
    expect(github).toContain("axhub-helpers");
    expect(github).toContain("axhub-helpers github-summary --user-utterance");
    expect(github).toContain("GitHub 연결 상태를 확인할게요.");
    expect(github).toContain("GitHub 연결 상태 확인");
    expect(github).toContain("Do not run `git remote`");
    expect(github).toContain('"action":"github_connect"');
    expect(github).toContain('"branch":"${BRANCH}"');
    expect(github).toContain('"context":{"repo":"${OWNER_REPO}","branch":"${BRANCH}","account":"${ACCOUNT}"}');
    expect(github).toContain('axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json');
    expect(github).toContain('"action":"github_disconnect"');
    expect(github).toContain('"context":{"slug":"${APP_ID_OR_SLUG}"}');
    expect(github).toContain('axhub apps git disconnect --app "$APP_ID" --execute --json');
    expect(github).toContain("PATH 의 `axhub-helpers`");
    expect(github).toContain("GitHub 연결 링크: <install_url>");
    expect(github).toContain('axhub apps git status --app "$APP_ID" --json');
    expect(github).toContain("NEVER `CLAUDE_PLUGIN_ROOT` 누락");
    expect(github).not.toContain("! axhub apps git connect");
  });

  test("migrate skill handles pure Desktop import-readiness phrasing without local detours", () => {
    const migrate = skillContents.get("migrate")!;
    expect(migrate).toContain("이 프로젝트 axhub로 옮길 수 있어?");
    expect(migrate).toContain("가져오기 상태를 확인할게요.");
    expect(migrate).toContain("가져오기 상태 확인");
    expect(migrate).toContain("axhub-helpers migrate-summary --user-utterance");
    expect(migrate).toContain("Copy the Korean stdout as the answer");
    expect(migrate).toContain("Do not inspect local server state");
    expect(migrate).toContain("package scripts");
    expect(migrate).toContain("previous deployment failures");
    expect(migrate).toContain("English tool-title fragments");
    expect(migrate).toContain("App registration, GitHub connection, env writes, and deployment require");
    expect(migrate).not.toContain("Check server and axhub apps");
    expect(migrate).not.toContain("last_deployment_status");
  });

  test("publish skill handles pure Desktop review phrasing without route leakage", () => {
    const publish = skillContents.get("publish")!;
    expect(publish).toContain("이 앱 공개 심사 넣고 싶어");
    expect(publish).toContain("공개 심사 준비를 확인할게요.");
    expect(publish).toContain("공개 심사 준비 확인");
    expect(publish).toContain("axhub-helpers publish-summary --user-utterance");
    expect(publish).toContain("Copy the Korean stdout as the answer");
    expect(publish).toContain("Do not read `quality.json`");
    expect(publish).toContain("local state files");
    expect(publish).toContain("plugin source");
    expect(publish).toContain("English tool-title fragments");
    expect(publish).toContain("Actual submission requires a Korean preview");
    expect(publish).not.toContain("App store review submission");
  });

  test("quality auto-mode does not advertise direct quality prompts in frontmatter", () => {
    const quality = skillContents.get("using-axhub-quality")!;
    const frontmatter = quality.split("\n---\n")[0];
    expect(frontmatter).toContain("background axhub quality auto-mode");
    expect(frontmatter).not.toContain("리뷰해줘");
    expect(frontmatter).not.toContain("코드 봐줘");
    expect(frontmatter).not.toContain("디버그해");
    expect(frontmatter).not.toContain("ship readiness");
    expect(quality).toContain("DIRECT REQUEST OVERRIDE");
    expect(quality).toContain("Direct explicit quality requests take precedence");
  });

  test("dedicated quality skills document Claude Desktop natural-language contracts", () => {
    const contracts = [
      ["axhub-review", "이 코드 리뷰해줘", "코드 리뷰를 시작할게요.", "리뷰 상태 저장"],
      ["axhub-debug", "왜 테스트가 깨지는지 디버그해줘", "원인을 좁혀볼게요.", "디버그 상태 저장"],
      ["axhub-diagnose", "loop 돌려서 원인 찾아줘", "진단 루프를 준비할게요.", "raw question JSON"],
      ["axhub-plan", "큰 구조 변경 계획 세워줘", "변경 계획을 잡아볼게요.", "영향 범위 확인"],
      ["axhub-ship", "PR 만들기 전에 배포 준비 봐줘", "출시 준비 상태를 확인할게요.", "출시 상태 저장"],
      ["axhub-tdd", "테스트 먼저 TDD로 가자", "테스트부터 잡아볼게요.", "TDD 대상 확인"],
      ["karpathy-guidelines", "작은 diff랑 테스트 우선 원칙 기억해줘", "Claude Desktop natural-language contract", "internal injection details"],
    ] as const;

    for (const [slug, utterance, firstSentence, guard] of contracts) {
      const content = skillContents.get(slug)!;
      expect(content, slug).toContain(utterance);
      expect(content, slug).toContain(firstSentence);
      expect(content, slug).toContain(guard);
      expect(content, slug).not.toContain("Survey repo scope and code files");
      expect(content, slug).not.toContain("Show git status and file listing");
    }

    const review = skillContents.get("axhub-review")!;
    expect(review).toContain("review directly in the current session first");
    expect(review).toContain("주요 변경 직접 검토");
    expect(review).toContain("review-scope-summary");
    expect(review).toContain("변경 범위 확인");
    expect(review).toContain("desktop-pure-routing-results.md");
    expect(review).toContain("QA 산출물");
    expect(review).toContain("변경량이 커서 먼저 범위를 정할게요");
    expect(review).not.toContain("Survey repo scope and code files");
    expect(review).not.toContain("Show git status and file listing");
    expect(review).not.toContain("axhub-reviewer agent 위임");

    const debug = skillContents.get("axhub-debug")!;
    expect(debug).toContain("debug directly in the current session first");
    expect(debug).toContain("원인 가설 직접 검토");
    expect(debug).not.toContain("axhub-debugger agent 위임");

    const ship = skillContents.get("axhub-ship")!;
    expect(ship).toContain("prepare the readiness summary directly in the current session first");
    expect(ship).toContain("PR 또는 release 초안 직접 정리");
    expect(ship).not.toContain("axhub-shipper agent 위임");
  });

  test("dedicated quality skill frontmatter does not auto-advertise plain Desktop prompts", () => {
    const directQualitySkills = [
      "axhub-review",
      "axhub-debug",
      "axhub-diagnose",
      "axhub-plan",
      "axhub-ship",
      "axhub-tdd",
    ];
    const plainDesktopTriggers = [
      "이 코드 리뷰해줘",
      "리뷰해줘",
      "코드 봐줘",
      "왜 테스트가 깨지는지 디버그해줘",
      "디버그해",
      "왜 안 돼",
      "loop 돌려서 원인 찾아줘",
      "큰 구조 변경 계획 세워줘",
      "플랜 짜줘",
      "PR 만들기 전에 배포 준비 봐줘",
      "배포 준비",
      "PR 만들어",
      "테스트 먼저 TDD로 가자",
      "테스트 먼저",
    ];

    for (const slug of directQualitySkills) {
      const content = skillContents.get(slug)!;
      const frontmatter = content.split("\n---\n")[0];
      expect(frontmatter, slug).toContain("Plain Desktop chat must be handled by prompt-route");
      for (const trigger of plainDesktopTriggers) {
        expect(frontmatter, `${slug} should not advertise ${trigger}`).not.toContain(trigger);
      }
    }
  });

  test("dedicated quality skills redact auth preflight before exposing output", () => {
    const directQualitySkills = [
      "axhub-review",
      "axhub-debug",
      "axhub-diagnose",
      "axhub-plan",
      "axhub-ship",
      "axhub-tdd",
    ];

    for (const slug of directQualitySkills) {
      const content = skillContents.get(slug)!;
      expect(content, slug).toContain("SAFE_PREFLIGHT_JSON");
      expect(content, slug).toContain("del(.user_email, .user_id, .email, .account_email, .scope, .scopes)");
      expect(content, slug).toContain("echo \"$SAFE_PREFLIGHT_JSON\"");
      expect(content, slug).toContain("계정 이메일, raw user id, scope 를 쓰지 말고");
      expect(content, slug).not.toContain("echo \"$PREFLIGHT_JSON\"");
    }
  });

  test("github skill locks guided repo setup capability ladder and consent gates", () => {
    const github = skillContents.get("github")!;
    expect(github).toContain("Strict guided capability ladder");
    expect(github).toContain("read-only git inspect");
    expect(github).toContain("parse existing remote");
    expect(github).toContain('axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --json');
    expect(github).toContain("gh repo create");
    expect(github).toContain("gh exists/authenticated");
    expect(github).toContain("owner-repo-visibility confirmed");
    expect(github).toContain('"question": "GitHub repo 를 만들까요?"');
    expect(github).toContain('"question": "git remote 를 추가할까요?"');
    expect(github).toContain('"question": "첫 push 를 실행할까요?"');
    expect(github).toContain('"question": "axhub 앱에 repo 를 연결할까요?"');
    expect(github).toContain("re-list after create/push");
    expect(github).toContain("before connect");
    expect(github).toContain("unsupported gap");
  });

  test("deploy skill documents current deploy list and cancel surfaces", () => {
    const deploy = skillContents.get("deploy")!;
    expect(deploy).toContain('axhub deploy list --app "$APP_ID" --json');
    expect(deploy).toContain('action=deploy_cancel');
    expect(deploy).toContain('axhub deploy cancel "$DEPLOYMENT_ID" --app "$APP_ID" --execute --json');
  });

  test("skill error-catalog cross references are resolvable relative paths", () => {
    for (const [slug, content] of skillContents) {
      if (slug === "deploy") continue; // deploy owns references/error-empathy-catalog.md locally.
      expect(content, slug).not.toMatch(/route to `error-empathy-catalog\.md`/);
    }
  });
});

// ---------------------------------------------------------------------------
// Reason: Cross-file consistency — broken cross-refs break loader silently.
// ---------------------------------------------------------------------------
describe("cross-manifest consistency", () => {
  test("headless auth references use implemented token-import command and plugin token path", async () => {
    for (const relPath of [
      "skills/deploy/references/headless-flow.md",
      "skills/deploy/references/recovery-flows.md",
      "docs/troubleshooting.ko.md",
      "docs/org-admin-rollout.ko.md",
    ]) {
      const content = await readFile(join(REPO_ROOT, relPath), "utf8");
      expect(content, relPath).not.toContain("token-install");
      expect(content, relPath).not.toContain("~/.config/axhub/token");
      expect(content, relPath).toContain("token-import");
      expect(content, relPath).toContain("~/.config/axhub-plugin/token");
    }
  });

  test("current user-facing auth docs do not advertise legacy token-file env or flags", async () => {
    for (const relPath of [
      "commands/login.md",
      "docs/troubleshooting.ko.md",
      "docs/org-admin-rollout.ko.md",
      "skills/auth/SKILL.md",
    ]) {
      const content = await readFile(join(REPO_ROOT, relPath), "utf8");
      expect(content, relPath).not.toContain("AXHUB_TOKEN_FILE");
      expect(content, relPath).not.toContain("--token-file");
      expect(content, relPath).not.toMatch(/consent-mint\s+--/);
    }
  });

  test("recover troubleshooting docs describe the shipped forward-fix skill", async () => {
    const troubleshooting = await readFile(join(REPO_ROOT, "docs/troubleshooting.ko.md"), "utf8");
    const recover = await readFile(join(REPO_ROOT, "skills/recover/SKILL.md"), "utf8");
    expect(troubleshooting).toContain("`recover` skill 이 현재 동작합니다");
    expect(troubleshooting).toContain("forward-fix-as-rollback");
    expect(troubleshooting).not.toContain("앞으로 ship 예정");
    expect(recover).toContain("forward-fix-as-rollback");
  });

  test("plugin.json name matches package.json name suffix", () => {
    expect(packageJson.name).toContain(pluginJson.name);
  });

  test("plugin.json version === package.json version", () => {
    expect(pluginJson.version).toBe(packageJson.version);
  });

  test("plugin.json version === marketplace.json plugin version", () => {
    const marketplaceAxhub = marketplaceJson.plugins.find((p) => p.name === "axhub")!;
    expect(pluginJson.version).toBe(marketplaceAxhub.version);
  });

  test("hooks.json command paths reference existing helper subcommands or shim", () => {
    const knownSubcommands = new Set([
      "session-start", "preauth-check", "consent-mint", "consent-verify",
      "resolve", "preflight", "classify-exit", "redact", "version", "help",
      "list-deployments", "prompt-route", "token-init", "token-import",
      "commit-gate", "test-classifier", "tdd-inject", "state-update",
    ]);
    for (const [, group] of Object.entries(hooksJson.hooks)) {
      for (const g of group) {
        for (const h of g.hooks) {
          // Skip shim paths: universal hooks.json registers bash SessionStart shims (v0.5.x base + v0.6.0 autowire).
          if (h.command.includes("hooks/session-start.sh")) continue;
          if (h.command.includes("hooks/session-start-autowire.sh")) continue;
          // Phase 25 PR 25.3 — bun-launched TS hooks live under hooks/*.ts and
          // are not axhub-helpers subcommands, so they're outside this check.
          if (/hooks\/[a-z0-9_-]+\.ts\b/.test(h.command)) continue;
          const parts = h.command.split(/\s+/);
          const helperIdx = parts.findIndex((part) => part.includes("axhub-helpers"));
          const sub = helperIdx >= 0 ? parts[helperIdx + 1] : parts.at(-1);
          if (sub) {
            expect(knownSubcommands.has(sub)).toBe(true);
          }
        }
      }
    }
  });

  test("session-start shims only call implemented helper subcommands", async () => {
    const helperMain = await readFile(join(REPO_ROOT, "crates/axhub-helpers/src/main.rs"), "utf8");
    const helperCli = await readFile(join(REPO_ROOT, "crates/axhub-helpers/src/cli/mod.rs"), "utf8");

    // kebab-case subcommand → clap PascalCase variant (token-init → TokenInit).
    const toVariant = (name: string) =>
      name
        .split("-")
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join("");

    for (const subcommand of ["session-start", "token-init"]) {
      expect(helperMain, `${subcommand} must be in usage`).toContain(`  ${subcommand}`);
      // "dispatched" = routed to a real handler. The clap migration (PR #151)
      // moved data/auth subcommands like token-init to typed Commands variants,
      // so accept either the legacy match arm (main.rs) or the typed clap
      // dispatch arm (cli/mod.rs `Commands::Variant`).
      const dispatched =
        helperMain.includes(`"${subcommand}" =>`) ||
        helperCli.includes(`Commands::${toVariant(subcommand)}`);
      expect(
        dispatched,
        `${subcommand} must be dispatched (legacy arm or clap variant)`,
      ).toBe(true);
    }

    for (const relPath of ["hooks/session-start.sh", "hooks/session-start.ps1"]) {
      const content = await readFile(join(REPO_ROOT, relPath), "utf8");
      expect(content, `${relPath} must not reference an unimplemented token bootstrap command`)
        .toContain("token-init");
    }
  });

  test("README.md exists and references plugin name", async () => {
    const readme = await readFile(join(REPO_ROOT, "README.md"), "utf8");
    expect(readme).toContain("axhub");
  });

  test("README current-release summary matches package metadata and shipped surfaces", async () => {
    const readme = await readFile(join(REPO_ROOT, "README.md"), "utf8");
    expect(readme).toContain(`**상태**: v${packageJson.version}`);
    expect(readme).toContain("44 SKILL / 9 command");
    expect(readme).not.toContain("AXHUB_HELPERS_RUNTIME=ts");
    expect(readme).not.toContain("TypeScript fallback");
  });

  test("tsconfig covers release and validation scripts, not only tests", async () => {
    const tsconfig = JSON.parse(await readFile(join(REPO_ROOT, "tsconfig.json"), "utf8")) as {
      include?: string[];
    };
    expect(tsconfig.include).toContain("scripts/**/*.ts");
  });

  test("CLAUDE.md exists and is non-empty", async () => {
    const claudeMd = await readFile(join(REPO_ROOT, "CLAUDE.md"), "utf8");
    expect(claudeMd.length).toBeGreaterThan(100);
  });

  test("LICENSE file exists", () => {
    expect(existsSync(join(REPO_ROOT, "LICENSE"))).toBe(true);
  });

  test("CHANGELOG.md exists", () => {
    expect(existsSync(join(REPO_ROOT, "CHANGELOG.md"))).toBe(true);
  });

  test("package.json scripts include build, test, typecheck", () => {
    expect(packageJson.scripts.build).toBeDefined();
    expect(packageJson.scripts.test).toBeDefined();
    expect(packageJson.scripts.typecheck).toBeDefined();
  });

  test("package.json scripts include build:all (cross-arch)", () => {
    expect(packageJson.scripts["build:all"]).toBeDefined();
  });

  test("package.json scripts include smoke and smoke:full", () => {
    expect(packageJson.scripts.smoke).toBeDefined();
    expect(packageJson.scripts["smoke:full"]).toBeDefined();
  });

  test("package.json declares Bun engine", () => {
    expect(packageJson.engines?.bun).toBeDefined();
  });

  test("install.sh exists and is executable", async () => {
    const path = join(REPO_ROOT, "bin/install.sh");
    expect(existsSync(path)).toBe(true);
    const stats = await stat(path);
    expect((stats.mode & 0o100) !== 0).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Phase 27 — preflight is an IN-BODY bash step (ADR-0013, supersedes ADR-0011).
// The load-time `!command` injection + its byte-identical codegen lock are retired:
// the injection hard-failed on first run (Claude Code gates the outer `node -e`
// wrapper) and its inner denialRegex fallback could never catch its own denial.
// The in-body preflight contract is asserted in tests/ux-skill-preflight-injection.test.ts.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// sh/ps1-absorption Phase 4 (F3 / TODO 3) — hooks.json invariant 자동 가드.
// Reason: Phase 4 PR review checklist (sh/ps1-absorption Issue 1.4 결정 (a))
// 가 manual 이어서 reviewer 가 diff 놓치면 helper 부재 환경 (clean install) 에서
// hook silent fail → 사용자 onboarding 차단 (high blast radius). codex outside
// voice finding #15 가 자동화 권장. 본 describe 는 hooks.json 의 hook 진입점 +
// 명령 string + timeout 을 baseline 으로 lock — 의도치 않은 변경은 PR diff
// 에서 즉시 catch 돼요. 의도적 변경 시 이 baseline 도 같이 업데이트해요 (PR
// reviewer 가 update 의도를 확인하는 신호).
// ---------------------------------------------------------------------------
describe("Phase 4 (F3) hooks.json invariant baseline", () => {
  // Baseline = canonical hook entry shape. PR diff 에서 변경 시 reviewer 가
  // 의도적 인지 확인. 의도적이면 본 baseline 도 같이 업데이트.
  interface CanonicalHook {
    matcher?: string;
    commands: Array<{ command: string; timeout?: number }>;
  }
  const expectedBaseline: Record<string, CanonicalHook[]> = {
    SessionStart: [
      {
        commands: [
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh", timeout: 30 },
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/session-start-autowire.sh", timeout: 10 },
        ],
      },
    ],
    UserPromptSubmit: [
      {
        commands: [
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh prompt-route", timeout: 5 },
        ],
      },
    ],
    PreToolUse: [
      {
        matcher: "Bash",
        commands: [
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh preauth-check", timeout: 5 },
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh commit-gate", timeout: 5 },
        ],
      },
      {
        matcher: "Edit|Write|MultiEdit|NotebookEdit",
        commands: [
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh tdd-inject", timeout: 5 },
        ],
      },
    ],
    PostToolUse: [
      {
        matcher: "Bash",
        commands: [
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh classify-exit", timeout: 5 },
          { command: "bun ${CLAUDE_PLUGIN_ROOT}/hooks/post-tool-verify-deploy-artifacts.ts", timeout: 7 },
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh test-classifier", timeout: 5 },
        ],
      },
      {
        matcher: "Edit|Write|MultiEdit|NotebookEdit",
        commands: [
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh state-update --edit-event", timeout: 5 },
        ],
      },
    ],
    PostToolUseFailure: [
      {
        matcher: "Bash",
        commands: [
          { command: "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh test-classifier", timeout: 5 },
        ],
      },
    ],
  };

  test("hook event set exactly matches baseline (no surprise additions/removals)", () => {
    const actualEvents = Object.keys(hooksJson.hooks).sort();
    const expectedEvents = Object.keys(expectedBaseline).sort();
    expect(actualEvents).toEqual(expectedEvents);
  });

  for (const [event, expectedGroups] of Object.entries(expectedBaseline)) {
    test(`${event} group count matches baseline`, () => {
      const actual = hooksJson.hooks[event];
      expect(actual).toBeDefined();
      expect(actual.length).toBe(expectedGroups.length);
    });

    expectedGroups.forEach((expectedGroup, groupIdx) => {
      test(`${event}[${groupIdx}] matcher matches baseline${expectedGroup.matcher ? ` (matcher=${expectedGroup.matcher})` : ""}`, () => {
        const actualGroup = hooksJson.hooks[event][groupIdx];
        expect(actualGroup.matcher).toBe(expectedGroup.matcher);
      });

      test(`${event}[${groupIdx}] command count matches baseline`, () => {
        const actualGroup = hooksJson.hooks[event][groupIdx];
        expect(actualGroup.hooks.length).toBe(expectedGroup.commands.length);
      });

      expectedGroup.commands.forEach((expectedCmd, cmdIdx) => {
        test(`${event}[${groupIdx}].hooks[${cmdIdx}] command byte-identical to baseline`, () => {
          const actualGroup = hooksJson.hooks[event][groupIdx];
          const actualCmd = actualGroup.hooks[cmdIdx];
          expect(actualCmd.command).toBe(expectedCmd.command);
        });

        test(`${event}[${groupIdx}].hooks[${cmdIdx}] timeout = ${expectedCmd.timeout}`, () => {
          const actualGroup = hooksJson.hooks[event][groupIdx];
          const actualCmd = actualGroup.hooks[cmdIdx];
          expect(actualCmd.timeout).toBe(expectedCmd.timeout);
        });
      });
    });
  }
});
