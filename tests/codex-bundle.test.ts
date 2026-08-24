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
  // U2 (KTD5): 치환이 제거하고 FORBIDDEN 이 재유입을 막아요. bare "Monitor" 는
  // 일반 영단어와 충돌해 넣지 않고, 명확한 도구 이름 둘만 잠가요.
  "AUQ",
  "TodoWrite",
  "CLAUDE_NON_INTERACTIVE",
  "ScheduleWakeup",
  "TaskOutput",
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

  test("merged always-on wrapper preserves every kill-switch branch and emits one JSON", () => {
    const script = join(outDir, "hooks", "session-always-on-codex.sh");
    const syntax = Bun.spawnSync({ cmd: ["bash", "-n", script], stdout: "pipe", stderr: "pipe" });
    expect(syntax.exitCode, syntax.stderr.toString()).toBe(0);

    const env = { ...process.env, HOME: tempRoot };
    const bothKilled = Bun.spawnSync({
      cmd: ["bash", script],
      env: {
        ...env,
        AXHUB_NO_UPDATE_ROUTER: "1",
        AXHUB_NO_QUESTION_PROTOCOL: "1",
        AXHUB_NO_FEEDBACK_REPORT: "1",
      },
      stdout: "pipe",
    });
    expect(bothKilled.stdout.toString()).toBe("");

    const routerOnly = Bun.spawnSync({
      cmd: ["bash", script],
      env: { ...env, AXHUB_NO_QUESTION_PROTOCOL: "1", AXHUB_NO_FEEDBACK_REPORT: "1" },
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

    // U5/KTD6: 질문 프로토콜은 기본 on 이고, 훅이 꺼진 세션에서도 skill 본문·정책이
    // 같은 fail-closed 규칙을 들고 있어요 (3채널 중복 소유).
    const questionOnly = Bun.spawnSync({
      cmd: ["bash", script],
      env: { ...env, AXHUB_NO_UPDATE_ROUTER: "1", AXHUB_NO_FEEDBACK_REPORT: "1" },
      stdout: "pipe",
    });
    const questionPayload = JSON.parse(questionOnly.stdout.toString()) as {
      hookSpecificOutput: { additionalContext: string };
    };
    expect(questionPayload.hookSpecificOutput.additionalContext).toContain(
      "카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요",
    );
    expect(questionPayload.hookSpecificOutput.additionalContext).not.toContain(
      "update-first routing guard",
    );
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

  test("approval ladder carries the pre-injection-invalid clause in all four execution skills", () => {
    // 실측 A/B: 이 조항 없이는 선주입 승인 문구가 3/3 실행으로 이어졌어요 (R6·AP-12).
    for (const skill of ["deploy", "bootstrap", "import", "scaffold"] as const) {
      const source = readFileSync(join(outDir, "skills", skill, "SKILL.md"), "utf8");
      expect(source, skill).toContain("미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요");
      expect(source, skill).not.toContain("네이티브 선택 UI 가 있으면 그걸로 묻고");
    }
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
    expect(autoUpdatePrompt).not.toContain("-codex-codex");
    const restartPrompt = readFileSync(
      join(outDir, "hooks", "plugin-restart-confirm-prompt.md"),
      "utf8",
    );
    expect(restartPrompt).toContain(".plugin-update-restart-codex");
    expect(restartPrompt).not.toContain("-codex-codex");
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

  test("codex skill descriptions stay within the 1,024-char catalog budget (KTD7)", () => {
    for (const skill of SKILLS) {
      const source = readFileSync(join(outDir, "skills", skill, "SKILL.md"), "utf8");
      const frontmatter = source.slice(0, source.indexOf("\n---\n", 4));
      const description = frontmatter.match(/^description: ([\s\S]*?)(?=\n[A-Za-z-]+:|$)/m);
      expect(description, skill).toBeTruthy();
      expect([...description![1].trim()].length, `${skill} description over 1,024 chars`).toBeLessThanOrEqual(1024);
    }
  });

  test("hook additionalContext payloads stay within the exposure budget", () => {
    // 노출 다이어트 회귀 방지 — 훅이 emit 하는 컨텍스트가 무한정 자라지 않게 상한을 둬요.
    const hooksDir = join(outDir, "hooks");
    let checked = 0;
    for (const script of readdirSync(hooksDir).filter((file) => file.endsWith(".sh"))) {
      const source = readFileSync(join(hooksDir, script), "utf8");
      for (const match of source.matchAll(/"additionalContext":\s*"((?:[^"\\]|\\.)*)"/g)) {
        checked += 1;
        expect(match[1].length, `${script} context over budget`).toBeLessThanOrEqual(4096);
      }
    }
    expect(checked).toBeGreaterThan(0);
  });

  // U3 (R2·KTD2): codex 는 SKILL.md 를 **파일 선두 기준** 8,000B 에서 절단해요
  // (rust-v0.148.0 ext/skills/src/render.rs — frontmatter 포함 파일 전체가 대상).
  // 승인 게이트가 절단선 밖으로 나가면 모델이 게이트를 아예 못 봐요. recovery-line
  // 존재 assert 로는 이걸 못 잡아서, 게이트 문자열의 byte offset 을 직접 계약으로 고정해요.
  const CODEX_SKILL_PROMPT_LIMIT = 8_000;
  const GATE_STRINGS: Record<string, readonly string[]> = {
    bootstrap: [
      "어떤 템플릿으로 시작할까요?",
      "앱 이름을 무엇으로 할까요?",
      "지금 만들고 배포까지 진행할까요?",
      "--execute",
      "카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요",
    ],
    deploy: ["axhub로 지금 배포를 진행할까요?", "명시 텍스트 승인 1회", "카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요"],
    import: ["미리보기대로 진행할까요?", "명시 텍스트 승인 1회", "카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요"],
    scaffold: ["명시 텍스트 승인 1회", "미리 넣어 둔 문구", "카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요"],
    update: ["update apply --execute"],
  };

  test("approval gates sit inside the codex 8,000B truncation window (R2)", () => {
    const failures: string[] = [];
    for (const [skill, gates] of Object.entries(GATE_STRINGS)) {
      const buf = readFileSync(join(outDir, "skills", skill, "SKILL.md"));
      for (const gate of gates) {
        const offset = buf.indexOf(Buffer.from(gate, "utf8"));
        if (offset < 0) {
          failures.push(`${skill}: gate string absent — "${gate}"`);
        } else if (offset >= CODEX_SKILL_PROMPT_LIMIT) {
          failures.push(`${skill}: "${gate}" @${offset}B (limit ${CODEX_SKILL_PROMPT_LIMIT})`);
        }
      }
    }
    expect(failures).toEqual([]);
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

describe("codex marketplace and version wiring (U7)", () => {
  const CODEX_BUNDLE_DIR = join(REPO_ROOT, "plugins", "axhub-codex");

  test("committed codex bundle matches a freshly generated bundle", () => {
    const generated = walk(outDir).map((file) => relative(outDir, file)).sort();
    const committed = walk(CODEX_BUNDLE_DIR).map((file) => relative(CODEX_BUNDLE_DIR, file)).sort();
    expect(committed).toEqual(generated);
    for (const file of generated) {
      const expected = readFileSync(join(outDir, file), "utf8");
      const actual = readFileSync(join(CODEX_BUNDLE_DIR, file), "utf8");
      expect(actual, `codex bundle drift in ${file}`).toBe(expected);
    }
  });

  test(".agents manifest is git-tracked with version-less entries matching bundle names (KTD3)", () => {
    const manifestPath = join(REPO_ROOT, ".agents", "plugins", "marketplace.json");
    const tracked = Bun.spawnSync({
      cmd: ["git", "ls-files", "--error-unmatch", ".agents/plugins/marketplace.json"],
      cwd: REPO_ROOT,
      stdout: "pipe",
      stderr: "pipe",
    });
    expect(tracked.exitCode, "gitignore 캐스케이드가 .agents manifest 를 제외하면 안 돼요").toBe(0);

    const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
      plugins: Array<{ name: string; source: string; version?: string }>;
    };
    // Codex 카탈로그엔 codex 번들 하나만 노출해요 — Claude Code 는 `.agents` 를 읽지
    // 않아 무영향(U1-(g) 실측)이고, 타 host 가 `.agents` 를 채택하면 그때 재병기해요.
    expect(manifest.plugins.map((entry) => entry.name)).toEqual(["axhub-codex"]);
    for (const entry of manifest.plugins) {
      // KTD3: 버전은 플러그인 매니페스트가 소유 — .agents 엔트리는 version 생략.
      expect("version" in entry, `${entry.name} entry must omit version`).toBe(false);
      // U1-(l): 엔트리 name ≠ plugin.json name 은 설치 거부 hard error.
      const pluginJson = JSON.parse(
        readFileSync(join(REPO_ROOT, entry.source, ".claude-plugin", "plugin.json"), "utf8"),
      ) as { name: string };
      expect(pluginJson.name).toBe(entry.name);
    }
  });

  test("versionrc bumps all three codex manifests to the package version", () => {
    const packageVersion = (
      JSON.parse(readFileSync(join(REPO_ROOT, "package.json"), "utf8")) as { version: string }
    ).version;
    const versionrc = JSON.parse(readFileSync(join(REPO_ROOT, ".versionrc.json"), "utf8")) as {
      bumpFiles: Array<{ filename: string; type?: string; updater?: string }>;
    };
    const codexBumpFiles = versionrc.bumpFiles.filter((entry) =>
      entry.filename.startsWith("plugins/axhub-codex/"),
    );
    expect(codexBumpFiles.map((entry) => entry.filename).sort()).toEqual([
      "plugins/axhub-codex/.claude-plugin/marketplace.json",
      "plugins/axhub-codex/.claude-plugin/plugin.json",
      "plugins/axhub-codex/.codex-plugin/plugin.json",
    ]);
    for (const entry of codexBumpFiles) {
      const raw = readFileSync(join(REPO_ROOT, entry.filename), "utf8");
      if (entry.updater) {
        const updater = require(join(REPO_ROOT, entry.updater)) as {
          readVersion: (contents: string) => string;
        };
        expect(updater.readVersion(raw)).toBe(packageVersion);
      } else {
        expect((JSON.parse(raw) as { version: string }).version).toBe(packageVersion);
      }
    }
  });
});
