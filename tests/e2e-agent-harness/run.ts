/**
 * run.ts — e2e-agent-harness 실행 오케스트레이터
 *
 * 사용법:
 *   bun tests/e2e-agent-harness/run.ts --condition packs-only
 *   bun tests/e2e-agent-harness/run.ts --condition packs-mcp --mcp-config /path/to/mcp.json
 *   bun tests/e2e-agent-harness/run.ts --condition packs-only --lang node
 *   bun tests/e2e-agent-harness/run.ts --condition packs-only --smoke
 *   bun tests/e2e-agent-harness/run.ts --condition packs-only --dry-run
 *   bun tests/e2e-agent-harness/run.ts --grade-only --condition packs-only
 *   bun tests/e2e-agent-harness/run.ts --compare-ab   (양쪽 output 존재 시)
 *
 * 비용 가드:
 *   --smoke      : node/packs-only 1회만 (비용 최소화)
 *   --dry-run    : claude 호출 없이 실행 계획만 출력
 *   --grade-only : 기존 output/ 정적 채점만 (과금 0)
 *
 * retry: 서브프로세스 실패(exit≠0) 시 1회 자동 재시도. retry_log 에 기록.
 */

import { spawnSync, type SpawnSyncReturns } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { grade, buildReport, compareAB, computeGate, type GradeResult } from "./grade.ts";

const HARNESS_DIR = dirname(new URL(import.meta.url).pathname);
const SDK_PACKS_DIR = "/Users/wongil/Desktop/work/jocoding/sdk/dist/sdk-knowledge";
const OUTPUT_DIR = join(HARNESS_DIR, "output");

const ALL_LANGS = ["node", "python", "go", "java", "kotlin", "ruby"] as const;
type Lang = (typeof ALL_LANGS)[number];

// ── 인자 파싱 ──────────────────────────────────────────────────────────────────
interface RunArgs {
  condition: "packs-only" | "packs-mcp";
  langs: Lang[];
  smoke: boolean;
  dryRun: boolean;
  gradeOnly: boolean;
  compareAb: boolean;
  mcpConfig: string | undefined;
  maxTurns: number;
}

function parseArgs(): RunArgs {
  const argv = process.argv.slice(2);
  const get = (flag: string) => {
    const i = argv.indexOf(flag);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const has = (flag: string) => argv.includes(flag);

  if (has("--help")) { printHelp(); process.exit(0); }

  const conditionRaw = get("--condition");
  if (!conditionRaw && !has("--grade-only") && !has("--compare-ab")) {
    console.error("오류: --condition packs-only|packs-mcp 가 필요해요");
    process.exit(1);
  }

  const condition = (conditionRaw ?? "packs-only") as RunArgs["condition"];
  if (condition !== "packs-only" && condition !== "packs-mcp") {
    console.error("오류: --condition 은 packs-only 또는 packs-mcp 여야 해요");
    process.exit(1);
  }

  const langArg = get("--lang");
  const langs: Lang[] = langArg
    ? [langArg as Lang]
    : (ALL_LANGS as unknown as Lang[]);

  const maxTurnsRaw = get("--max-turns");
  const maxTurns = maxTurnsRaw ? parseInt(maxTurnsRaw, 10) : 40;

  const mcpConfig = get("--mcp-config");

  // packs-mcp 는 --mcp-config 필수 (dry-run 제외)
  if (condition === "packs-mcp" && !mcpConfig && !has("--dry-run") && !has("--grade-only")) {
    console.error("오류: --condition packs-mcp 에는 --mcp-config <path> 가 필요해요");
    process.exit(1);
  }

  return {
    condition,
    langs,
    smoke: has("--smoke"),
    dryRun: has("--dry-run"),
    gradeOnly: has("--grade-only"),
    compareAb: has("--compare-ab"),
    mcpConfig,
    maxTurns,
  };
}

function printHelp() {
  console.log(`
e2e-agent-harness — AxHub SDK 함정 하니스

사용법:
  bun run.ts --condition <cond> [옵션]

옵션:
  --condition packs-only|packs-mcp   실행 조건 (필수)
  --mcp-config <path>                MCP 설정 JSON 파일 (packs-mcp 필수)
  --lang <lang>                      단일 언어만 실행
  --max-turns <n>                    claude 최대 턴 수 (기본: 40)
  --smoke                            node/packs-only 1회만 (비용 최소화)
  --dry-run                          실행 계획만 출력 (claude 호출 없음)
  --grade-only                       기존 output/ 채점만 (과금 0)
  --compare-ab                       packs-only vs packs-mcp A/B 비교 + ab-report.json

함정 매트릭스:
  node    : or( combinator      — 비푸시 OR 결합자
  python  : after= cursor       — LegacyCursorError
  go      : axhub.Or(           — 비푸시 OR 결합자
  java    : raw HTTP /data/     — SDK 우회 직접 호출
  kotlin  : filterless list     — non-owner 테이블 무필터 list/count
  ruby    : or_(                — 비푸시 OR 결합자

기계 판정 기준 (Plan §D.4):
  함정 수 ≥ 6 AND 각 언어 ≥1
  A/B: packs+MCP 통과 수 ≥ packs-only 통과 수 (동률 허용)
`);
}

// ── claude 바이너리 탐색 ────────────────────────────────────────────────────────
function findClaudeBin(): string {
  const candidates = [
    `${process.env.HOME}/.local/bin/claude`,
    `${process.env.HOME}/.claude/local/claude`,
    "/usr/local/bin/claude",
    "/opt/homebrew/bin/claude",
  ];
  for (const c of candidates) {
    if (existsSync(c)) return c;
  }
  const r = spawnSync("which", ["claude"], { encoding: "utf8" });
  if (r.status === 0 && r.stdout.trim()) {
    const p = r.stdout.trim();
    if (!p.includes("superset")) return p;
  }
  throw new Error("claude 바이너리를 찾을 수 없어요. ~/.local/bin/claude 를 확인해요.");
}

// ── MCP 헬스체크 ─────────────────────────────────────────────────────────────
/**
 * MCP 서버 URL 결정: AXHUB_E2E_MCP_URL env 가 mcp-config 보다 우선해요.
 */
function resolveMcpServerUrl(mcpConfigPath: string): { url: string; source: string } {
  const envUrl = process.env.AXHUB_E2E_MCP_URL;
  if (envUrl) return { url: envUrl, source: "env AXHUB_E2E_MCP_URL" };
  const cfg = JSON.parse(readFileSync(mcpConfigPath, "utf8"));
  const url = cfg?.mcpServers?.["axhub-mcp"]?.url;
  if (typeof url !== "string" || url === "") {
    throw new Error(`mcp-config 에 mcpServers["axhub-mcp"].url 이 없어요: ${mcpConfigPath}`);
  }
  return { url, source: mcpConfigPath };
}

/**
 * packs-mcp 실행 전 서버 도달성 확인. 도달 불가면 hard-fail 해요.
 * (서버가 죽어 있으면 B 조건이 사실상 A 조건으로 측정돼 A/B 비교가 무효화돼요.)
 * HTTP 응답이 오기만 하면 도달로 간주해요 (상태코드 무관 — 목적은 reachability).
 */
async function checkMcpReachable(url: string): Promise<void> {
  try {
    await fetch(url, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        accept: "application/json, text/event-stream",
      },
      body: JSON.stringify({ jsonrpc: "2.0", id: 0, method: "ping" }),
      signal: AbortSignal.timeout(5_000),
    });
  } catch (e) {
    throw new Error(
      `MCP 서버 도달 불가: ${url}\n  원인: ${String(e)}\n  서버를 먼저 띄우거나 AXHUB_E2E_MCP_URL 로 URL/포트를 override 해요.`
    );
  }
}

/**
 * AXHUB_E2E_MCP_URL override 시 URL 을 치환한 effective mcp-config 를 생성해요.
 */
function materializeEffectiveMcpConfig(mcpConfigPath: string, url: string): string {
  const cfg = JSON.parse(readFileSync(mcpConfigPath, "utf8"));
  const next = {
    ...cfg,
    mcpServers: {
      ...cfg.mcpServers,
      "axhub-mcp": { ...cfg.mcpServers?.["axhub-mcp"], url },
    },
  };
  mkdirSync(OUTPUT_DIR, { recursive: true });
  const effPath = join(OUTPUT_DIR, "mcp-config.effective.json");
  writeFileSync(effPath, JSON.stringify(next, null, 2));
  return effPath;
}

// ── TASK.md frontmatter 제거 ──────────────────────────────────────────────────
/**
 * TASK.md 의 YAML frontmatter (trap_id/trap_kind 등 채점 메타데이터) 를 제거해요.
 * 함정 정답이 프롬프트로 누출되면 "에이전트가 스스로 함정을 피하는가" 측정이 무효화돼요.
 */
export function stripTaskFrontmatter(md: string): string {
  return md.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n/, "");
}

// ── 시스템 프롬프트 구성 ──────────────────────────────────────────────────────
function buildSystemPrompt(lang: Lang, condition: RunArgs["condition"]): string {
  const packPath = join(SDK_PACKS_DIR, `${lang}.md`);
  if (!existsSync(packPath)) throw new Error(`SDK pack 없음: ${packPath}`);
  const pack = readFileSync(packPath, "utf8");

  const base = `You are an expert ${lang} developer using the AxHub SDK.
You have access to the following AxHub SDK knowledge pack for ${lang}.
Answer coding questions accurately using ONLY the patterns documented in this pack.

${pack}`;

  if (condition === "packs-mcp") {
    return `${base}

Additionally, you have access to the \`sdk_search\` MCP tool to query the AxHub SDK knowledge index.
Call it to verify patterns before generating code when uncertain.`;
  }
  return base;
}

// ── retry 기록 타입 ───────────────────────────────────────────────────────────
interface RetryEntry {
  lang: Lang;
  condition: string;
  attempt: number;
  exit_status: number;
  reason: string;
}

// ── claude -p 실행 (retry 포함) ────────────────────────────────────────────────
interface ClaudeResult {
  lang: Lang;
  condition: string;
  stdout: string;
  stderr: string;
  status: number;
  duration_ms: number;
  attempts: number;
}

function runClaudeOnce(
  lang: Lang,
  condition: RunArgs["condition"],
  claudeBin: string,
  systemPrompt: string,
  taskText: string,
  mcpConfig: string | undefined,
  maxTurns: number,
  outDir: string
): SpawnSyncReturns<string> {
  // A/B 조건 대칭화: 양 조건 동일 베이스 툴 + --strict-mcp-config (전역 MCP 차단).
  // 차이는 mcp-config + sdk_search 허용 여부뿐이어야 측정이 유효해요.
  const BASE_TOOLS = "Bash,Read,Write,Edit,Glob,Grep";
  const cliArgs = [
    "-p",
    "--system-prompt", systemPrompt,
    "--output-format", "text",
    "--max-turns", String(maxTurns),
    "--strict-mcp-config",
  ];

  if (mcpConfig && condition === "packs-mcp") {
    cliArgs.push("--mcp-config", mcpConfig);
    // MCP tool 명시 허용: 서버명 axhub-mcp → mcp__axhub-mcp__sdk_search
    cliArgs.push("--allowedTools", `${BASE_TOOLS},mcp__axhub-mcp__sdk_search`);
  } else {
    cliArgs.push("--allowedTools", BASE_TOOLS);
  }

  return spawnSync(claudeBin, cliArgs, {
    input: taskText,
    encoding: "utf8",
    timeout: 180_000,  // 3분 (40-turn 여유)
    env: {
      ...process.env,
      AXHUB_E2E_HARNESS: "1",  // 라이브 자격/배포/테이블 생성 금지 신호
    },
  });
}

function runClaude(
  lang: Lang,
  condition: RunArgs["condition"],
  claudeBin: string,
  args: RunArgs,
  retryLog: RetryEntry[]
): ClaudeResult {
  const taskPath = join(HARNESS_DIR, "tasks", lang, "TASK.md");
  // frontmatter (trap_id 등 채점 메타) 는 프롬프트 파이프 전에 strip — 정답 누출 방지
  const taskText = stripTaskFrontmatter(readFileSync(taskPath, "utf8"));
  const systemPrompt = buildSystemPrompt(lang, condition);

  const outDir = join(OUTPUT_DIR, condition, lang);
  mkdirSync(outDir, { recursive: true });

  writeFileSync(join(outDir, "system.txt"), systemPrompt);

  if (args.dryRun) {
    const mcpNote = args.mcpConfig ? ` --mcp-config ${args.mcpConfig}` : "";
    console.log(`[dry-run] ${claudeBin} -p --system-prompt <system> --max-turns ${args.maxTurns}${mcpNote}`);
    console.log(`[dry-run] task: ${taskPath}`);
    return { lang, condition, stdout: "", stderr: "", status: 0, duration_ms: 0, attempts: 0 };
  }

  const t0 = Date.now();
  let result = runClaudeOnce(lang, condition, claudeBin, systemPrompt, taskText, args.mcpConfig, args.maxTurns, outDir);
  let attempts = 1;

  // 1회 재시도 (실패 시)
  if (result.status !== 0) {
    retryLog.push({
      lang,
      condition,
      attempt: 1,
      exit_status: result.status ?? -1,
      reason: (result.stderr ?? "").slice(0, 120),
    });
    process.stdout.write(" [재시도]");
    result = runClaudeOnce(lang, condition, claudeBin, systemPrompt, taskText, args.mcpConfig, args.maxTurns, outDir);
    attempts = 2;
  }

  const duration_ms = Date.now() - t0;
  writeFileSync(join(outDir, "response.txt"), result.stdout ?? "");

  return {
    lang,
    condition,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
    status: result.status ?? -1,
    duration_ms,
    attempts,
  };
}

// ── 리포트 출력 ───────────────────────────────────────────────────────────────
function printReport(report: ReturnType<typeof buildReport>) {
  console.log(`
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 리포트  condition=${report.condition}
 총 ${report.total}  통과 ${report.passed}  실패 ${report.failed}  미결 ${report.uncertain}
 기준 충족: ${report.meets_criteria ? "✓" : "✗"}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  for (const r of report.results) {
    const icon = r.verdict === "PASS" ? "✓" : r.verdict === "FAIL" ? "✗" : "?";
    console.log(`  ${icon} ${r.lang.padEnd(8)} ${r.verdict.padEnd(9)} ${r.evidence.slice(0, 70)}`);
  }
}

// ── 단일 조건 실행 ────────────────────────────────────────────────────────────
function runCondition(
  args: RunArgs,
  claudeBin: string,
  retryLog: RetryEntry[]
): GradeResult[] {
  const targetLangs = args.smoke ? (["node"] as Lang[]) : args.langs;
  const condition = args.smoke ? "packs-only" : args.condition;

  console.log(`\n▶ condition=${condition}  langs=${targetLangs.join(",")}  max-turns=${args.maxTurns}`);
  if (args.dryRun) console.log("  (dry-run)\n");

  const gradeResults: GradeResult[] = [];

  for (const lang of targetLangs) {
    process.stdout.write(`  ${lang.padEnd(8)} 실행 중...`);

    const r = runClaude(lang, condition, claudeBin, args, retryLog);

    if (args.dryRun) {
      gradeResults.push({ lang, trap_id: "dry-run", verdict: "UNCERTAIN", bad_hit: [], good_hit: [], evidence: "dry-run" });
      console.log(" [dry-run]");
      continue;
    }

    if (r.status !== 0) {
      console.log(` ✗ exit=${r.status} (시도${r.attempts}회)  ${r.stderr.slice(0, 60)}`);
      gradeResults.push({ lang, trap_id: "error", verdict: "UNCERTAIN", bad_hit: [], good_hit: [], evidence: `claude exit=${r.status} (시도${r.attempts}회)` });
      continue;
    }

    const gr = grade(lang, r.stdout);
    gradeResults.push(gr);

    const icon = gr.verdict === "PASS" ? "✓" : gr.verdict === "FAIL" ? "✗" : "?";
    const retryNote = r.attempts > 1 ? ` (재시도${r.attempts}회)` : "";
    console.log(` ${icon} ${gr.verdict}  (${r.duration_ms}ms${retryNote})  ${gr.evidence.slice(0, 55)}`);
  }

  return gradeResults;
}

// ── A/B 비교 + ab-report.json ─────────────────────────────────────────────────
function runCompareAb(retryLog?: RetryEntry[]) {
  const readResults = (cond: string): GradeResult[] => {
    const results: GradeResult[] = [];
    for (const lang of ALL_LANGS) {
      const p = join(OUTPUT_DIR, cond, lang, "response.txt");
      if (!existsSync(p)) {
        results.push({ lang, trap_id: "missing", verdict: "UNCERTAIN", bad_hit: [], good_hit: [], evidence: "출력 없음" });
        continue;
      }
      results.push(grade(lang, readFileSync(p, "utf8")));
    }
    return results;
  };

  const poResults = readResults("packs-only");
  const pmResults = readResults("packs-mcp");

  const po = buildReport("packs-only", poResults);
  const pm = buildReport("packs-mcp", pmResults);

  printReport(po);
  printReport(pm);

  const cmp = compareAB(po, pm);
  // 게이트 산술: A/B 비교 + 기준충족 + UNCERTAIN 0건 (실행 실패가 게이트를 통과 못 하게)
  const gate = computeGate(po, pm);
  const gate_pass = gate.pass;

  console.log(`\nA/B 판정: ${cmp.ok ? "✓" : "✗"}  ${cmp.reason}`);
  console.log(`게이트:   ${gate_pass ? "PASS ✓" : "FAIL ✗"}  (${gate.reason})`);

  // 언어×조건 표
  console.log("\n언어×조건 표:");
  console.log("  lang     packs-only  packs-mcp");
  for (const lang of ALL_LANGS) {
    const a = po.results.find((r) => r.lang === lang)?.verdict ?? "?";
    const b = pm.results.find((r) => r.lang === lang)?.verdict ?? "?";
    const aIcon = a === "PASS" ? "✓" : a === "FAIL" ? "✗" : "?";
    const bIcon = b === "PASS" ? "✓" : b === "FAIL" ? "✗" : "?";
    console.log(`  ${lang.padEnd(8)} ${aIcon} ${a.padEnd(10)}  ${bIcon} ${b}`);
  }

  const abReport = {
    generated_at: new Date().toISOString(),
    a: po,
    b: pm,
    comparison: cmp,
    gate_pass,
    retry_log: retryLog ?? [],
  };

  const reportPath = join(OUTPUT_DIR, "ab-report.json");
  writeFileSync(reportPath, JSON.stringify(abReport, null, 2));
  console.log(`\nab-report.json: ${reportPath}`);

  return gate_pass;
}

// ── 메인 ─────────────────────────────────────────────────────────────────────
async function main() {
  const args = parseArgs();
  const retryLog: RetryEntry[] = [];

  if (args.compareAb) {
    const ok = runCompareAb(retryLog);
    process.exit(ok ? 0 : 1);
    return;
  }

  if (args.gradeOnly) {
    const results: GradeResult[] = [];
    for (const lang of args.langs) {
      const p = join(OUTPUT_DIR, args.condition, lang, "response.txt");
      if (!existsSync(p)) {
        results.push({ lang, trap_id: "missing", verdict: "UNCERTAIN", bad_hit: [], good_hit: [], evidence: `파일 없음: ${p}` });
        continue;
      }
      results.push(grade(lang, readFileSync(p, "utf8")));
    }
    const report = buildReport(args.condition, results);
    printReport(report);
    const rp = join(OUTPUT_DIR, args.condition, "report.json");
    writeFileSync(rp, JSON.stringify(report, null, 2));
    process.exit(report.meets_criteria ? 0 : 1);
    return;
  }

  let claudeBin = "";
  if (!args.dryRun) {
    try {
      claudeBin = findClaudeBin();
      console.log(`claude: ${claudeBin}`);
    } catch (e) {
      console.error(String(e));
      process.exit(1);
    }
  }

  // packs-mcp 라이브 실행 전 MCP 헬스체크 (도달 불가 → hard-fail)
  let effectiveArgs = args;
  if (args.condition === "packs-mcp" && !args.smoke && !args.dryRun && args.mcpConfig) {
    try {
      const { url, source } = resolveMcpServerUrl(args.mcpConfig);
      await checkMcpReachable(url);
      console.log(`MCP 헬스체크 ✓ ${url} (${source})`);
      if (process.env.AXHUB_E2E_MCP_URL) {
        effectiveArgs = { ...args, mcpConfig: materializeEffectiveMcpConfig(args.mcpConfig, url) };
      }
    } catch (e) {
      console.error(String(e instanceof Error ? e.message : e));
      process.exit(1);
    }
  }

  const gradeResults = runCondition(effectiveArgs, claudeBin, retryLog);

  if (!args.dryRun) {
    const condition = args.smoke ? "packs-only" : args.condition;
    const report = buildReport(condition, gradeResults);
    printReport(report);

    if (retryLog.length > 0) {
      console.log(`\n재시도 기록 (${retryLog.length}건):`);
      for (const r of retryLog) {
        console.log(`  ${r.lang}/${r.condition} 시도${r.attempt} exit=${r.exit_status} ${r.reason}`);
      }
    }

    const rp = join(OUTPUT_DIR, condition, "report.json");
    mkdirSync(join(OUTPUT_DIR, condition), { recursive: true });
    writeFileSync(rp, JSON.stringify({ ...report, retry_log: retryLog }, null, 2));
    console.log(`\n리포트: ${rp}`);

    process.exit(report.meets_criteria ? 0 : 1);
  }
}

// 테스트에서 export 헬퍼 (stripTaskFrontmatter 등) import 시 실행 방지
if (import.meta.main) {
  main().catch((e) => { console.error(e); process.exit(1); });
}
