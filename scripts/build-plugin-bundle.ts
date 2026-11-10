import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";

const REPO_ROOT = resolve(join(import.meta.dir, ".."));
const DEFAULT_OUT_DIR = join(REPO_ROOT, "dist", "axhub-plugin");
const DEFAULT_CODEX_OUT_DIR = join(REPO_ROOT, "dist", "axhub-plugin-codex");
const CODEX_OVERRIDES_DIR = join(REPO_ROOT, "codex-overrides");
const ROOT_FILES = ["README.md", "LICENSE", "POLICY.md"] as const;
const ROOT_DIRS = [".claude-plugin", "skills", "hooks"] as const;
const DENY_NAMES = new Set([
  ".DS_Store",
  ".axhub",
  ".axhub-state",
  ".autoplan",
  ".bun",
  ".cache",
  ".cargo",
  ".claude",
  ".codegraph",
  ".git",
  ".github",
  ".gitnexus",
  ".gjc",
  ".graphify_python",
  ".obsidian",
  ".omc",
  ".omx",
  ".ouroboros",
  ".plan",
  ".qa-live",
  ".serena",
  ".specify",
  ".understand-anything",
  "CHANGELOG.md",
  "CLAUDE.md",
  "AGENTS.md",
  "node_modules",
  "graphify-out",
  "tests",
  "scripts",
  "dist",
  "test-results.json",
]);

type Host = "claude" | "codex";

interface Options {
  outDir: string;
  json: boolean;
  host: Host;
}

interface BundleStats {
  outDir: string;
  files: number;
  bytes: number;
  host: Host;
}

// ── codex 파생 상수 (KTD1·KTD5·KTD7·KTD11, U5) ──────────────────────────────
// 치환은 longest-first 가 계약이에요 — 병기 구문 전체를 먼저 소비해 이중 적용을
// 막아요 (tests/codex-bundle.test.ts 가 정렬을 assert 해요).
export const CODEX_SUBSTITUTIONS: ReadonlyArray<readonly [from: string, to: string]> = [
  // AP-26: auto-update worker 의 플러그인 갱신 블록 — Claude 는 scope 를 읽어
  // `claude plugin update`, codex 는 git marketplace `upgrade` 뒤 로컬 marketplace
  // fallback 인 재 `plugin add` 예요. 블록 통째 치환이라 아래 짧은 명령 치환이
  // 이 블록을 반쯤 바꾸는 사고가 없어요 (FORBIDDEN 이 재유입을 막아요).
  [
    "  if ! command -v claude >/dev/null 2>&1; then\n    PLUGIN_RESULT=\"PLUGIN_SKIPPED reason=host_cli_missing latest=$PLAT\"\n  else\n    SCOPE=$(claude plugin list 2>/dev/null | awk '/axhub@axhub/{f=1} f&&/Scope:/{print $2; exit}')\n    [ -n \"$SCOPE\" ] || SCOPE=user\n    # exit 0 은 updated 뿐 아니라 up_to_date·skipped 도 돼요 — marketplace 새로고침이 실패하면\n    # 낡은 카탈로그로 \"already at the latest version\" 이 나와요. marker 는 updated 에만 써요.\n    MP=ok\n    claude plugin marketplace update axhub >/dev/null 2>&1 || MP=failed\n    UPD=$(claude plugin update axhub@axhub --scope \"$SCOPE\" --json 2>/dev/null)\n    URC=$?\n    case \"$URC:$UPD\" in\n      0:*'\"updateOutcome\":\"updated\"'*|0:*'\"updateOutcome\": \"updated\"'*)\n        printf '%s|%s' \"$PLAT\" \"$SCOPE\" > \"$RESTART_MARKER\" 2>/dev/null\n        PLUGIN_RESULT=\"PLUGIN_UPDATED plugin=$PV->$PLAT scope=$SCOPE\"\n        ;;\n      0:*)\n        PLUGIN_RESULT=\"PLUGIN_SKIPPED reason=not_updated plugin=$PV latest=$PLAT scope=$SCOPE marketplace=$MP\"\n        ;;\n      *)\n        PLUGIN_RESULT=\"PLUGIN_FAILED plugin=$PV latest=$PLAT scope=$SCOPE\"\n        ;;\n    esac\n  fi",
    "  if ! command -v codex >/dev/null 2>&1; then\n    PLUGIN_RESULT=\"PLUGIN_SKIPPED reason=host_cli_missing latest=$PLAT\"\n  else\n    if codex plugin marketplace upgrade axhub >/dev/null 2>&1 || codex plugin add axhub-codex@axhub >/dev/null 2>&1; then\n      printf '%s' \"$PLAT\" > \"$RESTART_MARKER\" 2>/dev/null\n      PLUGIN_RESULT=\"PLUGIN_UPDATED plugin=$PV->$PLAT\"\n    else\n      PLUGIN_RESULT=\"PLUGIN_FAILED plugin=$PV latest=$PLAT\"\n    fi\n  fi",
  ],
  [
    "```typescript\nTodoWrite({ todos: [\n  { content: \"테이블 생성\",   status: \"in_progress\", activeForm: \"테이블 만드는 중\" },\n  { content: \"환경변수 추가\", status: \"pending\",     activeForm: \"env 추가하는 중\" }\n]})\n```",
    "참고 항목: `테이블 생성`(진행 중) → `환경변수 추가`(대기). 인자 shape 은 host 가 노출한 도구 스키마를 그대로 따라요.",
  ],
  [
    "네이티브 선택 UI 가 있으면 그걸로 묻고, 없으면 같은 확인을 명시 텍스트 승인 1회로 받고, 둘 다 불가한 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.",
    "명시 텍스트 승인 1회만 받아요. 승인 채널 없는 headless 에서는 execute하지 않아요 — 승인을 조용히 건너뛰지 않아요.",
  ],
  [
    "`CI` 가 있음, `CLAUDE_NON_INTERACTIVE` 가 있음, `claude -p`·`codex exec` 같은 subprocess/headless 호출임",
    "`CI` 가 있음, `CODEX_NON_INTERACTIVE` 가 있음, `codex exec` 같은 subprocess/headless 호출임",
  ],
  [
    "Prefer separate short tool calls or an actual ScheduleWakeup.",
    "Prefer separate short tool calls.",
  ],
  ["`claude -p`·`codex exec`, CI, `$CLAUDE_NON_INTERACTIVE`, ", "`codex exec`, CI, "],
  ["(`claude -p` / CI / `$CLAUDE_NON_INTERACTIVE` / no TTY)", "(`codex exec` / CI / no TTY)"],
  ["`ScheduleWakeup`, 내부 task 이름, 브라우저/preview 도구 이름을", "내부 task 이름, 브라우저/preview 도구 이름을"],
  ["`claude -p`·CI·`$CLAUDE_NON_INTERACTIVE`·TTY 없음", "`codex exec`·CI·TTY 없음"],
  ["`실행됨 명령 N개`, `TaskOutput 사용함`, tool 카드,", "`실행됨 명령 N개`, tool 카드,"],
  ["[ -n \"${CLAUDE_NON_INTERACTIVE:-}\" ]", "[ -n \"${CODEX_NON_INTERACTIVE:-}\" ]"],
  ["claude plugin marketplace update", "codex plugin marketplace upgrade"],
  ["`Monitor 사용` 권한 카드가 뜨는 명령은 실패예요.", "백그라운드 감시 권한 카드가 뜨는 명령은 실패예요."],
  [
    "## First Visible Sentence",
    "**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.\n\n**장기 대기.** codex 는 긴 명령을 최대 30초에 yield 하고 백그라운드 터미널로 넘겨요 — yield 는 실패도 완료도 아니에요. `deploy verify --wait` 가 yield 되면 같은 명령을 다시 실행하지 말고 같은 터미널을 빈 입력으로 폴링해 완주를 기다려요. 성공 선언 규칙은 그대로예요.\n\n## 질문 계약 (요약)\n\ncodex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 이미 AXHub 에 있는 앱의 재배포라 승인 질문 없이 진행하고(AP-12), 배포 전 맞추기의 묻는 일이 있을 때만 물어요. 답은 명시 텍스트 1회로 받고, 미리 넣어 둔 문구·유사 표현·무응답은 답이 아니에요. 선택 카드에 빈 답변이 돌아오면 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요. headless 는 물을 일이 있으면 아무것도 바꾸지 않고 멈춰요.\n\n## First Visible Sentence",
  ],
  ["`claude -p`·`codex exec`", "`codex exec`"],
  ["Claude Desktop Code 모드", "Codex"],
  [".plugin-update-restart", ".plugin-update-restart-codex"],
  ["claude plugin update", "codex plugin marketplace upgrade"],
  [".plugin-update-check", ".plugin-update-check-codex"],
  ["claude plugin list", "codex plugin list"],
  ["command -v claude", "command -v codex"],
  ["AskUserQuestion", "명시 텍스트 승인"],
  ["claude mcp add", "codex mcp add"],
  ["Claude Desktop", "Codex"],
  ["Claude Code", "Codex"],
  ["HOST=claude", "HOST=codex"],
  ["claude -p", "codex exec"],
  ["TodoWrite", "update_plan"],
  ["Claude", "Codex"],
  // bootstrap(옛 scaffold 와 합친 판)·import(2026-11 개편판)·setup 이 같은 `## 순서` 앞에 받는 승인 게이트 요약이에요
  // — codex 8,000B 절단선 안에 들어가요.
  [
    "## 순서",
    "**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.\n\n## 승인 게이트 계약 (요약)\n\ncodex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 승인 방식은 명시 텍스트 승인 1회예요 — preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요. 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.\n\n## 순서",
  ],
  ["AUQ", "명시 텍스트 승인"],
];

const CODEX_TEXT_EXTENSIONS = new Set([".md", ".sh", ".json"]);
// codex-overrides 에서 번들 경로로 mirror-copy 하지 않는 예약 자산이에요.
const CODEX_OVERRIDE_RESERVED = new Set(["routing", "SOURCE_HASHES.json"]);
const CODEX_OVERRIDE_RESERVED_HOOK_DIRS = new Set(["context"]);
const CODEX_PLUGIN_NAME = "axhub-codex";
const CODEX_RECOVERY_LINE =
  "> 이 본문이 중간에 끊겨 보이면 설치 경로의 이 SKILL.md 원문을 열어 전체 절차를 확인한 뒤 진행해요.";

const parseArgs = (): Options => {
  let outDir: string | undefined;
  let json = false;
  let host: Host = "claude";
  const args = Bun.argv.slice(2);
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--json") {
      json = true;
    } else if (arg === "--out") {
      const value = args[i + 1];
      if (!value) throw new Error("--out requires a path");
      outDir = resolve(value);
      i += 1;
    } else if (arg === "--host") {
      const value = args[i + 1];
      if (value !== "claude" && value !== "codex") throw new Error("--host requires claude|codex");
      host = value;
      i += 1;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return { outDir: outDir ?? (host === "codex" ? DEFAULT_CODEX_OUT_DIR : DEFAULT_OUT_DIR), json, host };
};

const assertSafeOutDir = (outDir: string): void => {
  const normalized = resolve(outDir);
  if (normalized === REPO_ROOT || normalized === dirname(REPO_ROOT) || normalized === "/") {
    throw new Error(`refusing to clear unsafe output path: ${outDir}`);
  }
};

const isDenied = (path: string): boolean => {
  const parts = relative(REPO_ROOT, path).split("/");
  return parts.some((part) => DENY_NAMES.has(part));
};

const copyTree = (src: string, dest: string): void => {
  if (isDenied(src)) return;
  const stat = statSync(src);
  if (stat.isDirectory()) {
    mkdirSync(dest, { recursive: true });
    for (const entry of readdirSync(src)) {
      copyTree(join(src, entry), join(dest, entry));
    }
    return;
  }
  if (!stat.isFile()) return;
  mkdirSync(dirname(dest), { recursive: true });
  copyFileSync(src, dest);
};

const collectStats = (dir: string): { files: number; bytes: number } => {
  let files = 0;
  let bytes = 0;
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      const child = collectStats(path);
      files += child.files;
      bytes += child.bytes;
    } else if (stat.isFile()) {
      files += 1;
      bytes += stat.size;
    }
  }
  return { files, bytes };
};

const walkFiles = (dir: string): string[] => {
  const files: string[] = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) files.push(...walkFiles(path));
    else if (stat.isFile()) files.push(path);
  }
  return files;
};

// ── codex 파생 단계들 ────────────────────────────────────────────────────────

// codex-overrides 의 mirror 경로 파일을 번들 위에 스왑해요 (routing/·hooks/context/
// ·SOURCE_HASHES.json 은 transform 입력 자산이라 mirror 대상이 아니에요).
const applyCodexOverrides = (outDir: string): void => {
  if (!existsSync(CODEX_OVERRIDES_DIR)) return;
  for (const entry of readdirSync(CODEX_OVERRIDES_DIR)) {
    if (CODEX_OVERRIDE_RESERVED.has(entry)) continue;
    const src = join(CODEX_OVERRIDES_DIR, entry);
    if (entry === "hooks" && statSync(src).isDirectory()) {
      for (const hookEntry of readdirSync(src)) {
        if (CODEX_OVERRIDE_RESERVED_HOOK_DIRS.has(hookEntry)) continue;
        copyTree(join(src, hookEntry), join(outDir, "hooks", hookEntry));
      }
      continue;
    }
    copyTree(src, join(outDir, entry));
  }
};

const applyCodexSubstitutions = (outDir: string): void => {
  const claudeHostTrigger = "Claude에 설치해줘";
  const preservedTrigger = "__AXHUB_PRESERVED_CLAUDE_HOST_TRIGGER__";
  for (const file of walkFiles(outDir)) {
    if (!CODEX_TEXT_EXTENSIONS.has(extname(file))) continue;
    const original = readFileSync(file, "utf8");
    const isPluginsSkill = file.endsWith(join("skills", "plugins", "SKILL.md"));
    let next = isPluginsSkill
      ? original.replaceAll(claudeHostTrigger, preservedTrigger)
      : original;
    for (const [from, to] of CODEX_SUBSTITUTIONS) {
      next = next.replaceAll(from, to);
    }
    if (isPluginsSkill) next = next.replaceAll(preservedTrigger, claudeHostTrigger);
    if (next !== original) writeFileSync(file, next);
  }
};

interface CodexHookEntry {
  type: string;
  command: string;
  commandWindows?: string;
  [key: string]: unknown;
}

interface HooksJson {
  hooks: Record<string, Array<{ matcher?: string; hooks: CodexHookEntry[] }>>;
}

// Windows 의 codex 는 commandWindows 를 cmd 가 아니라 Windows PowerShell 5.1 로 바로 실행해요
// (2026-11-03 실측: 훅 프로세스 부모 powershell.exe). 5.1 은 `&&`·`||` 를 몰라 파서 오류로
// exit 1 이 나서 메시지마다 Hook failed 가 떴어요. 훅 스크립트는 bash 전용이라 PowerShell 에서
// 돌릴 방법이 없고 WSL bash 로도 못 돌려서, Windows 에선 cmd·PowerShell 둘 다에서 도는 `exit 0`
// 으로 조용히 건너뛰어요. 지금도 Windows 에선 훅이 한 번도 성공하지 못해 잃는 기능은 없어요.
const CODEX_COMMAND_WINDOWS = "exit 0";

const transformCodexHooks = (outDir: string): void => {
  const hooksDir = join(outDir, "hooks");

  // hooks.json 재작성 — shell 키 제거(U1-(c): 런타임 무시), commandWindows 추가.
  const hooksJsonPath = join(hooksDir, "hooks.json");
  const hooksJson = JSON.parse(readFileSync(hooksJsonPath, "utf8")) as HooksJson;
  for (const eventEntries of Object.values(hooksJson.hooks)) {
    for (const entry of eventEntries) {
      const nextHooks: CodexHookEntry[] = [];
      for (const hook of entry.hooks) {
        // timeout 은 Claude 판 전용이에요 — Claude 는 async 실행 중 timeout 을 무시하지만
        // codex 는 async 훅에도 적용(기본 600s)하므로 5초면 apply 가 잘려요 (AP-26).
        const { shell: _shell, timeout: _timeout, ...rest } = hook as CodexHookEntry & {
          shell?: string;
          timeout?: number;
        };
        nextHooks.push({ ...rest, commandWindows: CODEX_COMMAND_WINDOWS });
      }
      entry.hooks = nextHooks;
    }
  }
  writeFileSync(hooksJsonPath, `${JSON.stringify(hooksJson, null, 2)}\n`);
};

const transformCodexManifests = (outDir: string, version: string): void => {
  const pluginJsonPath = join(outDir, ".claude-plugin", "plugin.json");
  const plugin = JSON.parse(readFileSync(pluginJsonPath, "utf8")) as Record<string, unknown> & {
    keywords?: string[];
  };
  plugin.name = CODEX_PLUGIN_NAME;
  if (Array.isArray(plugin.keywords)) {
    plugin.keywords = plugin.keywords.map((keyword) =>
      keyword === "claude-code-plugin" ? "codex-plugin" : keyword,
    );
  }
  writeFileSync(pluginJsonPath, `${JSON.stringify(plugin, null, 2)}\n`);

  const marketplacePath = join(outDir, ".claude-plugin", "marketplace.json");
  if (existsSync(marketplacePath)) {
    const marketplace = JSON.parse(readFileSync(marketplacePath, "utf8")) as {
      plugins?: Array<{ name?: string; source?: string }>;
    };
    const bundledPlugin = marketplace.plugins?.[0];
    if (bundledPlugin) bundledPlugin.name = CODEX_PLUGIN_NAME;
    writeFileSync(marketplacePath, `${JSON.stringify(marketplace, null, 2)}\n`);
  }

  // KTD11: 미동봉 시 codex 가 자동 생성해 표시 메타 통제권을 잃어요. hooks 필드는
  // 넣지 않아요 (codex 스캐폴드 지침).
  const codexPluginDir = join(outDir, ".codex-plugin");
  mkdirSync(codexPluginDir, { recursive: true });
  const codexPlugin: Record<string, unknown> = { ...plugin, name: CODEX_PLUGIN_NAME, version };
  delete codexPlugin.hooks;
  writeFileSync(join(codexPluginDir, "plugin.json"), `${JSON.stringify(codexPlugin, null, 2)}\n`);
};

interface FrontmatterSplit {
  frontmatterLines: string[];
  body: string;
}

const splitFrontmatter = (source: string, file: string): FrontmatterSplit => {
  if (!source.startsWith("---\n")) throw new Error(`missing frontmatter in ${file}`);
  const end = source.indexOf("\n---\n", 4);
  if (end === -1) throw new Error(`unterminated frontmatter in ${file}`);
  return {
    frontmatterLines: source.slice(4, end).split("\n"),
    body: source.slice(end + "\n---\n".length),
  };
};

const isTopLevelKeyLine = (line: string): boolean => /^[A-Za-z_-]+:/.test(line);

// KTD7: codex 는 frontmatter examples 를 무시해요 — 재합성 description 은
// codex-overrides/routing/descriptions.json 이 소유하고, examples 블록은 카탈로그
// 예산을 위해 제거해요. KTD4 보조 수단인 절단 자기-복구 1줄도 여기서 prepend 해요.
const resynthesizeCodexDescriptions = (outDir: string): void => {
  const descriptionsPath = join(CODEX_OVERRIDES_DIR, "routing", "descriptions.json");
  const descriptions = existsSync(descriptionsPath)
    ? (JSON.parse(readFileSync(descriptionsPath, "utf8")) as Record<string, string>)
    : {};
  const skillsDir = join(outDir, "skills");
  if (!existsSync(skillsDir)) return;
  for (const skill of readdirSync(skillsDir)) {
    const skillPath = join(skillsDir, skill, "SKILL.md");
    if (!existsSync(skillPath)) continue;
    const source = readFileSync(skillPath, "utf8");
    const { frontmatterLines, body } = splitFrontmatter(source, skillPath);
    const nextLines: string[] = [];
    let index = 0;
    while (index < frontmatterLines.length) {
      const line = frontmatterLines[index];
      if (line.startsWith("description:")) {
        const override = descriptions[skill];
        const kept: string[] = [line];
        index += 1;
        while (index < frontmatterLines.length && !isTopLevelKeyLine(frontmatterLines[index])) {
          kept.push(frontmatterLines[index]);
          index += 1;
        }
        if (override) {
          nextLines.push(`description: ${JSON.stringify(override)}`);
        } else {
          nextLines.push(...kept);
        }
        continue;
      }
      if (line.startsWith("examples:")) {
        index += 1;
        while (index < frontmatterLines.length && !isTopLevelKeyLine(frontmatterLines[index])) {
          index += 1;
        }
        continue;
      }
      nextLines.push(line);
      index += 1;
    }
    const nextBody = body.startsWith(`${CODEX_RECOVERY_LINE}\n`)
      ? body
      : `${CODEX_RECOVERY_LINE}\n\n${body}`;
    writeFileSync(skillPath, `---\n${nextLines.join("\n")}\n---\n${nextBody}`);
  }
};

const buildBundle = ({ outDir, host }: Options): BundleStats => {
  assertSafeOutDir(outDir);
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });

  for (const file of ROOT_FILES) {
    copyTree(join(REPO_ROOT, file), join(outDir, file));
  }

  for (const dir of ROOT_DIRS) {
    copyTree(join(REPO_ROOT, dir), join(outDir, dir));
  }

  if (host === "codex") {
    applyCodexSubstitutions(outDir);
    applyCodexOverrides(outDir);
    transformCodexHooks(outDir);
    const version = (
      JSON.parse(readFileSync(join(REPO_ROOT, ".claude-plugin", "plugin.json"), "utf8")) as {
        version: string;
      }
    ).version;
    transformCodexManifests(outDir, version);
    resynthesizeCodexDescriptions(outDir);
  }

  const pluginJson = join(outDir, ".claude-plugin", "plugin.json");
  if (!existsSync(pluginJson)) {
    throw new Error(`bundle is missing ${relative(outDir, pluginJson)}`);
  }

  const marketplaceJson = join(outDir, ".claude-plugin", "marketplace.json");
  if (existsSync(marketplaceJson)) {
    const marketplace = JSON.parse(readFileSync(marketplaceJson, "utf8")) as {
      plugins?: Array<{ source?: string }>;
    };
    const bundledPlugin = marketplace.plugins?.[0];
    if (!bundledPlugin) {
      throw new Error(`bundle marketplace is missing plugins[0]: ${relative(outDir, marketplaceJson)}`);
    }
    bundledPlugin.source = ".";
    writeFileSync(marketplaceJson, `${JSON.stringify(marketplace, null, 2)}\n`);
  }

  const stats = collectStats(outDir);
  return { outDir, host, ...stats };
};

const main = (): void => {
  const options = parseArgs();
  const stats = buildBundle(options);
  if (options.json) {
    console.log(JSON.stringify(stats));
  } else {
    console.log(
      `Built ${stats.host === "codex" ? "axhub-codex" : "axhub"} plugin bundle at ${stats.outDir} (${stats.files} files, ${stats.bytes} bytes)`,
    );
  }
};

if (import.meta.main) {
  main();
}
