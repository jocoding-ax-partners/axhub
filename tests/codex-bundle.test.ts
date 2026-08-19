import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";

import { CODEX_SUBSTITUTIONS } from "../scripts/build-plugin-bundle";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS = [
  "onboarding",
  "bootstrap",
  "scaffold",
  "deploy",
  "import",
  "development",
  "diagnosis",
  "clarity",
  "update",
] as const;

// R8 — codex 번들 전 텍스트 파일에서 Claude-호스트 문자열 0건이 계약이에요
// (U6 override 저작 완료로 배제 목록 없음).
const FORBIDDEN_STRINGS = [
  "AskUserQuestion",
  "claude plugin",
  "claude -p",
  "command -v claude",
  "claude mcp",
  "Claude Desktop",
  "Claude Code",
  "oh-my-claudecode",
] as const;
const U6_PENDING_EXCLUDED_PREFIXES = [] as const;

const walk = (dir: string): string[] => {
  const files: string[] = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) files.push(...walk(path));
    else if (stat.isFile()) files.push(path);
  }
  return files;
};

let tempRoot = "";
let outDir = "";

beforeAll(() => {
  tempRoot = mkdtempSync(join(tmpdir(), "axhub-codex-bundle-"));
  outDir = join(tempRoot, "bundle");
  const result = Bun.spawnSync({
    cmd: ["bun", "scripts/build-plugin-bundle.ts", "--host", "codex", "--out", outDir, "--json"],
    cwd: REPO_ROOT,
    stdout: "pipe",
    stderr: "pipe",
  });
  if (result.exitCode !== 0) {
    throw new Error(`codex bundle build failed: ${result.stderr.toString()}`);
  }
});

afterAll(() => {
  if (tempRoot) rmSync(tempRoot, { recursive: true, force: true });
});

describe("codex bundle transform (U5 게이트 골격 — 본체는 U8)", () => {
  test("substitution table is longest-first", () => {
    const lengths = CODEX_SUBSTITUTIONS.map(([from]) => from.length);
    const sorted = [...lengths].sort((a, b) => b - a);
    expect(lengths).toEqual(sorted);
  });

  test("bundle carries codex manifests with rewritten names and synced versions", () => {
    const packageVersion = (
      JSON.parse(readFileSync(join(REPO_ROOT, "package.json"), "utf8")) as { version: string }
    ).version;

    const plugin = JSON.parse(readFileSync(join(outDir, ".claude-plugin", "plugin.json"), "utf8")) as {
      name: string;
      version: string;
      keywords?: string[];
    };
    expect(plugin.name).toBe("axhub-codex");
    expect(plugin.version).toBe(packageVersion);
    expect(plugin.keywords ?? []).not.toContain("claude-code-plugin");

    const marketplace = JSON.parse(
      readFileSync(join(outDir, ".claude-plugin", "marketplace.json"), "utf8"),
    ) as { plugins: Array<{ name?: string; source?: string }> };
    expect(marketplace.plugins[0]?.name).toBe("axhub-codex");
    expect(marketplace.plugins[0]?.source).toBe(".");

    const codexPlugin = JSON.parse(
      readFileSync(join(outDir, ".codex-plugin", "plugin.json"), "utf8"),
    ) as Record<string, unknown>;
    expect(codexPlugin.name).toBe("axhub-codex");
    expect(codexPlugin.version).toBe(packageVersion);
    // KTD11: hooks 필드는 codex 스캐폴드 지침대로 넣지 않아요.
    expect("hooks" in codexPlugin).toBe(false);
  });

  test("hooks.json drops shell keys, adds commandWindows, swaps merged always-on entry", () => {
    const hooksRaw = readFileSync(join(outDir, "hooks", "hooks.json"), "utf8");
    expect(hooksRaw).not.toContain('"shell"');
    const hooksJson = JSON.parse(hooksRaw) as {
      hooks: Record<string, Array<{ hooks: Array<{ command: string; commandWindows?: string }> }>>;
    };
    for (const entries of Object.values(hooksJson.hooks)) {
      for (const entry of entries) {
        for (const hook of entry.hooks) {
          const script = hook.command.match(/hooks\/([A-Za-z0-9._-]+\.sh)/)?.[1];
          expect(script, hook.command).toBeDefined();
          // U1-(p) 확정: PLUGIN_ROOT·CLAUDE_PLUGIN_ROOT 둘 다 주입 — 호환 env 유지.
          expect(hook.command).toContain("${CLAUDE_PLUGIN_ROOT}");
          expect(hook.commandWindows).toBe(
            `where bash >nul 2>nul && bash "\${CLAUDE_PLUGIN_ROOT}/hooks/${script}" || cd .`,
          );
        }
      }
    }
    expect(hooksRaw).toContain("session-always-on-codex.sh");
    expect(hooksRaw).not.toContain("session-update-router-guard.sh");
    expect(hooksRaw).not.toContain("session-feedback-contract.sh");
    expect(existsSync(join(outDir, "hooks", "session-always-on-codex.sh"))).toBe(true);
    expect(existsSync(join(outDir, "hooks", "session-update-router-guard.sh"))).toBe(false);
    expect(existsSync(join(outDir, "hooks", "session-feedback-contract.sh"))).toBe(false);
  });

  test("merged always-on wrapper preserves both kill-switch branches and emits one JSON", () => {
    const script = join(outDir, "hooks", "session-always-on-codex.sh");
    const syntax = Bun.spawnSync({ cmd: ["bash", "-n", script], stdout: "pipe", stderr: "pipe" });
    expect(syntax.exitCode, syntax.stderr.toString()).toBe(0);

    const env = { ...process.env, HOME: tempRoot };
    const bothKilled = Bun.spawnSync({
      cmd: ["bash", script],
      env: { ...env, AXHUB_NO_UPDATE_ROUTER: "1", AXHUB_NO_FEEDBACK_REPORT: "1" },
      stdout: "pipe",
    });
    expect(bothKilled.stdout.toString()).toBe("");

    const routerOnly = Bun.spawnSync({
      cmd: ["bash", script],
      env: { ...env, AXHUB_NO_FEEDBACK_REPORT: "1" },
      stdout: "pipe",
    });
    const routerPayload = JSON.parse(routerOnly.stdout.toString()) as {
      continue: boolean;
      suppressOutput: boolean;
      hookSpecificOutput: { hookEventName: string; additionalContext: string };
    };
    expect(routerPayload.continue).toBe(true);
    expect(routerPayload.suppressOutput).toBe(true);
    expect(routerPayload.hookSpecificOutput.hookEventName).toBe("SessionStart");
    expect(routerPayload.hookSpecificOutput.additionalContext).toContain("update-first routing guard");
    expect(routerPayload.hookSpecificOutput.additionalContext).toContain("codex plugin list");
  });

  test("update-router.sh keeps the gate pipeline and emits the codex context", () => {
    const script = join(outDir, "hooks", "update-router.sh");
    const source = readFileSync(script, "utf8");
    expect(source).toContain("AXHUB_NO_UPDATE_ROUTER");
    expect(source).toContain('*\\"prompt\\":*');

    const run = (stdin: string, env: Record<string, string | undefined> = {}) =>
      Bun.spawnSync({
        cmd: ["bash", script],
        env: { ...process.env, HOME: tempRoot, ...env },
        stdin: Buffer.from(stdin),
        stdout: "pipe",
      }).stdout.toString();

    // fail-closed: prompt 키 부재 → 침묵.
    expect(run('{"cwd":"/tmp/axhub-project 업데이트"}')).toBe("");
    const fired = run('{"prompt":"axhub 최신 버전으로 업데이트해줘"}');
    const payload = JSON.parse(fired) as {
      hookSpecificOutput: { hookEventName: string; additionalContext: string };
    };
    expect(payload.hookSpecificOutput.hookEventName).toBe("UserPromptSubmit");
    expect(payload.hookSpecificOutput.additionalContext).toContain("[axhub update router]");
    expect(payload.hookSpecificOutput.additionalContext).toContain("codex plugin list");
    expect(payload.hookSpecificOutput.additionalContext).not.toContain("claude plugin list");
  });

  test("state markers are host-suffixed (KTD5)", () => {
    const autoUpdate = readFileSync(join(outDir, "hooks", "session-auto-update.sh"), "utf8");
    const restartConfirm = readFileSync(join(outDir, "hooks", "session-restart-confirm.sh"), "utf8");
    expect(autoUpdate).toContain(".plugin-update-check-codex");
    expect(restartConfirm).toContain(".plugin-update-restart-codex");
    expect(/\.plugin-update-check(?!-codex)/.test(autoUpdate)).toBe(false);
    expect(/\.plugin-update-restart(?!-codex)/.test(restartConfirm)).toBe(false);
  });

  test("skills drop frontmatter examples and carry the truncation self-recovery line", () => {
    for (const skill of SKILLS) {
      const path = join(outDir, "skills", skill, "SKILL.md");
      expect(existsSync(path), `missing codex skill: ${skill}`).toBe(true);
      const source = readFileSync(path, "utf8");
      const frontmatterEnd = source.indexOf("\n---\n", 4);
      const frontmatter = source.slice(0, frontmatterEnd);
      const body = source.slice(frontmatterEnd + "\n---\n".length);
      expect(frontmatter).toContain("description:");
      expect(frontmatter).not.toContain("examples:");
      expect(body.startsWith("> 이 본문이 중간에 끊겨 보이면")).toBe(true);
    }
  });

  test("update lane override owns the codex apply flow (R4)", () => {
    const skill = readFileSync(join(outDir, "skills", "update", "SKILL.md"), "utf8");
    expect(skill).toContain("axhub update check --plugin-version");
    expect(skill).toContain("codex plugin list --json");
    expect(skill).toContain("현재 버전을 확인할게요.");
    expect(skill).toContain("받았어요. Codex 를 재시작하면 새 버전이 적용돼요.");
    expect(skill).not.toContain("--scope");

    const reference = readFileSync(
      join(outDir, "skills", "update", "references", "plugin-update.md"),
      "utf8",
    );
    expect(reference).toContain("codex plugin marketplace upgrade axhub");
    expect(reference).toContain("codex plugin add axhub-codex@axhub");
    expect(reference).toContain("marketplaceSource.sourceType");

    const autoUpdatePrompt = readFileSync(join(outDir, "hooks", "auto-update-prompt.md"), "utf8");
    expect(autoUpdatePrompt).toContain(".plugin-update-check-codex");
    expect(autoUpdatePrompt).toContain(".plugin-update-restart-codex");
    const restartPrompt = readFileSync(
      join(outDir, "hooks", "plugin-restart-confirm-prompt.md"),
      "utf8",
    );
    expect(restartPrompt).toContain(".plugin-update-restart-codex");
    expect(/\.plugin-update-restart(?!-codex)/.test(restartPrompt)).toBe(false);
  });

  test("SOURCE_HASHES pins match the current sources (KTD9)", async () => {
    const pins = JSON.parse(
      readFileSync(join(REPO_ROOT, "codex-overrides", "SOURCE_HASHES.json"), "utf8"),
    ) as Record<string, string>;
    expect(Object.keys(pins).length).toBeGreaterThan(0);
    for (const [source, pinned] of Object.entries(pins)) {
      const bytes = readFileSync(join(REPO_ROOT, source));
      const hasher = new Bun.CryptoHasher("sha256");
      hasher.update(bytes);
      expect(hasher.digest("hex"), `stale pin for ${source}`).toBe(pinned);
    }
  });

  test("codex bundle has no forbidden host strings outside U6-pending overrides", () => {
    const files = walk(outDir).map((file) => relative(outDir, file));
    for (const file of files) {
      if (U6_PENDING_EXCLUDED_PREFIXES.some((prefix) => file.startsWith(prefix))) continue;
      if (![".md", ".sh", ".json"].some((ext) => file.endsWith(ext))) continue;
      const content = readFileSync(join(outDir, file), "utf8");
      for (const forbidden of FORBIDDEN_STRINGS) {
        expect(content.includes(forbidden), `${forbidden} in ${file}`).toBe(false);
      }
    }
  });
});
