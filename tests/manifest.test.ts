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
// helperSource removed v0.2.0 — TS shadow박멸.

beforeAll(async () => {
  pluginJson = JSON.parse(await readFile(join(REPO_ROOT, ".claude-plugin/plugin.json"), "utf8"));
  marketplaceJson = JSON.parse(await readFile(join(REPO_ROOT, ".claude-plugin/marketplace.json"), "utf8"));
  packageJson = JSON.parse(await readFile(join(REPO_ROOT, "package.json"), "utf8"));
  hooksJson = JSON.parse(await readFile(join(REPO_ROOT, "hooks/hooks.json"), "utf8"));
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
    // SessionStart uses the universal bash shim; other hooks call the binary directly.
    for (const [, group] of Object.entries(hooksJson.hooks)) {
      for (const g of group) {
        for (const h of g.hooks) {
          const refsBinary = h.command.includes("axhub-helpers");
          const refsShim = h.command.includes("hooks/session-start.sh");
          expect(refsBinary || refsShim).toBe(true);
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
    expect(hook.command).toBe("${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers prompt-route");
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

  test("PreToolUse + PostToolUse remain single-entry (no platform branching needed — direct binary call)", () => {
    expect(hooksJson.hooks.PreToolUse.length).toBe(1);
    expect(hooksJson.hooks.PostToolUse.length).toBe(1);
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
    // 14 Run-Step calls
    const runSteps = ps1.match(/^Run-Step \d+/gm);
    expect(runSteps).not.toBeNull();
    expect(runSteps!.length).toBe(14);
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
    "apis.md",
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

  test("exactly 10 command files exist, including the Korean deploy alias", () => {
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
      ["apis.md", "apis"],
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

  test("NO skill has model field in frontmatter (skills are model-agnostic)", () => {
    for (const [, content] of skillContents) {
      const fm = content.split("\n---\n")[0];
      expect(fm).not.toMatch(/^model:/m);
    }
  });

  test("frontmatter contains ONLY allowed keys (Phase 18: + multi-step / needs-preflight)", () => {
    for (const [, content] of skillContents) {
      const fm = content.split("\n---\n")[0].slice(4);
      const keys = fm.match(/^[a-z-]+:/gm) ?? [];
      const allowed = new Set(["name:", "description:", "multi-step:", "needs-preflight:"]);
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

  test("expected 11 specific skills present", () => {
    const expected = ["apis", "apps", "auth", "clarify", "deploy", "doctor", "logs", "recover", "status", "update", "upgrade"];
    for (const e of expected) {
      expect(skillDirs).toContain(e);
    }
  });

  test("deploy skill has body referencing axhub-helpers binary", () => {
    const deployContent = skillContents.get("deploy")!;
    expect(deployContent).toContain("axhub-helpers");
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
      expect(content).toMatch(/\|\s*\$\{CLAUDE_PLUGIN_ROOT\}\/bin\/axhub-helpers consent-mint/);
    }
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

  test("deploy skill documents current deploy list and cancel surfaces", () => {
    const deploy = skillContents.get("deploy")!;
    expect(deploy).toContain('axhub deploy list --app "$APP_ID" --json');
    expect(deploy).toContain('action=deploy_cancel');
    expect(deploy).toContain('axhub deploy cancel "$DEPLOYMENT_ID" --app "$APP_ID" --yes --json');
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
      "list-deployments", "prompt-route", "token-import",
    ]);
    for (const [, group] of Object.entries(hooksJson.hooks)) {
      for (const g of group) {
        for (const h of g.hooks) {
          // Skip shim paths: universal hooks.json only registers the bash SessionStart shim.
          if (h.command.includes("hooks/session-start.sh")) continue;
          const sub = h.command.split(/\s+/).pop();
          if (sub) {
            expect(knownSubcommands.has(sub)).toBe(true);
          }
        }
      }
    }
  });

  test("README.md exists and references plugin name", async () => {
    const readme = await readFile(join(REPO_ROOT, "README.md"), "utf8");
    expect(readme).toContain("axhub");
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
