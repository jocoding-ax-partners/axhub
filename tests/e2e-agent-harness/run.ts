/**
 * run.ts — e2e-agent-harness 실행 오케스트레이터
 *
 * 사용법:
 *   bun tests/e2e-agent-harness/run.ts --condition packs-only
 *   bun tests/e2e-agent-harness/run.ts --condition packs-mcp
 *   bun tests/e2e-agent-harness/run.ts --condition packs-only --lang node
 *   bun tests/e2e-agent-harness/run.ts --condition packs-only --smoke
 *   bun tests/e2e-agent-harness/run.ts --condition packs-only --dry-run
 *   bun tests/e2e-agent-harness/run.ts --grade-only --condition packs-only
 *
 * 비용 가드:
 *   --smoke   : node/packs-only 1회만 실행 (과금 최소화)
 *   --dry-run : claude 호출 없이 실행 계획만 출력
 *   --grade-only : 기존 output/ 디렉토리 정적 채점만 수행 (과금 0)
 */

import {
  spawnSync,
  type SpawnSyncReturns,
} from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { grade, buildReport, compareAB, type GradeResult } from "./grade.ts";

const HARNESS_DIR = dirname(new URL(import.meta.url).pathname);
const REPO_ROOT = join(HARNESS_DIR, "..", "..");
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
}

function parseArgs(): RunArgs {
  const argv = process.argv.slice(2);
  const get = (flag: string) => {
    const i = argv.indexOf(flag);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const has = (flag: string) => argv.includes(flag);

  const conditionRaw = get("--condition");
  if (
    !conditionRaw &&
    !has("--grade-only") &&
    !has("--compare-ab") &&
    !has("--help")
  ) {
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

  if (has("--help")) {
    printHelp();
    process.exit(0);
  }

  return {
    condition,
    langs,
    smoke: has("--smoke"),
    dryRun: has("--dry-run"),
    gradeOnly: has("--grade-only"),
    compareAb: has("--compare-ab"),
  };
}

function printHelp() {
  console.log(`
e2e-agent-harness — AxHub SDK 함정 하니스

사용법:
  bun run.ts --condition <cond> [옵션]

옵션:
  --condition packs-only|packs-mcp   실행 조건 (필수)
  --lang <lang>                      단일 언어만 실행 (node|python|go|java|kotlin|ruby)
  --smoke                            node/packs-only 1회만 (비용 최소화)
  --dry-run                          실행 계획만 출력 (claude 호출 없음)
  --grade-only                       기존 output/ 채점만 (과금 0)
  --compare-ab                       packs-only vs packs-mcp A/B 비교 리포트

함정 매트릭스:
  node    : or( combinator — 비푸시 OR 결합자
  python  : after= cursor  — LegacyCursorError
  go      : axhub.Or(      — 비푸시 OR 결합자
  java    : raw HTTP /data/ — SDK 우회 직접 호출
  kotlin  : AXHUB_TENANT   — 잘못된 env var 이름
  ruby    : or_(            — 비푸시 OR 결합자

기계 판정 기준 (Plan §D.4 Architect F3):
  함정 수 ≥ 6 AND 각 언어 ≥1 충족 여부
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
  // fallback: PATH
  const r = spawnSync("which", ["claude"], { encoding: "utf8" });
  if (r.status === 0 && r.stdout.trim()) {
    const path = r.stdout.trim();
    // superset wrapper 제외 (기존 e2e 패턴)
    if (!path.includes("superset")) return path;
  }
  throw new Error("claude 바이너리를 찾을 수 없어요. ~/.local/bin/claude 를 확인해요.");
}

// ── 시스템 프롬프트 구성 ──────────────────────────────────────────────────────
function buildSystemPrompt(lang: Lang, condition: RunArgs["condition"]): string {
  const packPath = join(SDK_PACKS_DIR, `${lang}.md`);
  if (!existsSync(packPath)) {
    throw new Error(`SDK pack 없음: ${packPath}`);
  }
  const pack = readFileSync(packPath, "utf8");

  const base = `You are an expert ${lang} developer using the AxHub SDK.
You have access to the following AxHub SDK knowledge pack for ${lang}.
Answer coding questions accurately using ONLY the patterns documented in this pack.

${pack}`;

  if (condition === "packs-mcp") {
    return `${base}

Additionally, you have access to the \`sdk_search\` MCP tool to query the AxHub SDK index.
Use it to verify patterns before generating code.`;
  }
  return base;
}

// ── claude -p 실행 ────────────────────────────────────────────────────────────
interface ClaudeResult {
  lang: Lang;
  condition: string;
  stdout: string;
  stderr: string;
  status: number;
  duration_ms: number;
}

function runClaude(
  lang: Lang,
  condition: RunArgs["condition"],
  claudeBin: string,
  dryRun: boolean
): ClaudeResult {
  const taskPath = join(HARNESS_DIR, "tasks", lang, "TASK.md");
  const taskText = readFileSync(taskPath, "utf8");
  const systemPrompt = buildSystemPrompt(lang, condition);

  const outDir = join(OUTPUT_DIR, condition, lang);
  mkdirSync(outDir, { recursive: true });

  const systemFile = join(outDir, "system.txt");
  writeFileSync(systemFile, systemPrompt);

  if (dryRun) {
    console.log(`[dry-run] would run: ${claudeBin} -p --system-prompt <${systemFile}>`);
    console.log(`[dry-run] task: ${taskPath}`);
    return {
      lang,
      condition,
      stdout: "",
      stderr: "",
      status: 0,
      duration_ms: 0,
    };
  }

  const t0 = Date.now();
  // claude -p 에 task 내용을 stdin 으로 전달
  // --max-turns 1: 단일 응답
  // --output-format text: 텍스트 출력
  const result: SpawnSyncReturns<string> = spawnSync(
    claudeBin,
    [
      "-p",
      "--system-prompt",
      systemPrompt,
      "--output-format",
      "text",
      "--max-turns",
      "1",
    ],
    {
      input: taskText,
      encoding: "utf8",
      timeout: 60_000,
      env: {
        ...process.env,
        // 격리: 배포/자격/라이브 호출 금지 환경
        AXHUB_E2E_HARNESS: "1",
      },
    }
  );
  const duration_ms = Date.now() - t0;

  const responseFile = join(outDir, "response.txt");
  writeFileSync(responseFile, result.stdout ?? "");

  return {
    lang,
    condition,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
    status: result.status ?? -1,
    duration_ms,
  };
}

// ── 메인 실행 ─────────────────────────────────────────────────────────────────
async function main() {
  const args = parseArgs();

  if (args.compareAb) {
    runCompareAb();
    return;
  }

  if (args.gradeOnly) {
    const conditionDir = join(OUTPUT_DIR, args.condition);
    const results: GradeResult[] = [];
    for (const lang of args.langs) {
      const filePath = join(conditionDir, lang, "response.txt");
      if (!existsSync(filePath)) {
        results.push({
          lang,
          trap_id: "missing",
          verdict: "UNCERTAIN",
          bad_hit: [],
          good_hit: [],
          evidence: `출력 파일 없음: ${filePath}`,
        });
        continue;
      }
      const text = readFileSync(filePath, "utf8");
      results.push(grade(lang, text));
    }
    const report = buildReport(args.condition, results);
    printReport(report);
    process.exit(report.meets_criteria ? 0 : 1);
    return;
  }

  // smoke: node/packs-only 1회만
  const targetLangs: Lang[] = args.smoke ? ["node"] : args.langs;
  const condition = args.smoke ? "packs-only" : args.condition;

  console.log(`\n▶ e2e-agent-harness  condition=${condition}  langs=${targetLangs.join(",")}`);
  if (args.dryRun) console.log("  (dry-run 모드 — claude 호출 없음)\n");

  let claudeBin = "";
  if (!args.dryRun) {
    try {
      claudeBin = findClaudeBin();
      console.log(`  claude 바이너리: ${claudeBin}\n`);
    } catch (e) {
      console.error(String(e));
      process.exit(1);
    }
  }

  const gradeResults: GradeResult[] = [];

  for (const lang of targetLangs) {
    process.stdout.write(`  ${lang.padEnd(8)} 실행 중...`);
    const r = runClaude(lang, condition, claudeBin, args.dryRun);

    if (args.dryRun) {
      gradeResults.push({
        lang,
        trap_id: "dry-run",
        verdict: "UNCERTAIN",
        bad_hit: [],
        good_hit: [],
        evidence: "dry-run",
      });
      console.log(" [dry-run]");
      continue;
    }

    if (r.status !== 0) {
      console.log(` ✗ exit=${r.status}  stderr=${r.stderr.slice(0, 80)}`);
      gradeResults.push({
        lang,
        trap_id: "error",
        verdict: "UNCERTAIN",
        bad_hit: [],
        good_hit: [],
        evidence: `claude exit=${r.status}`,
      });
      continue;
    }

    const gr = grade(lang, r.stdout);
    gradeResults.push(gr);

    const icon = gr.verdict === "PASS" ? "✓" : gr.verdict === "FAIL" ? "✗" : "?";
    console.log(
      ` ${icon} ${gr.verdict}  (${r.duration_ms}ms)  ${gr.evidence.slice(0, 60)}`
    );
  }

  if (!args.dryRun) {
    const report = buildReport(condition, gradeResults);
    printReport(report);

    const outFile = join(OUTPUT_DIR, condition, "report.json");
    writeFileSync(outFile, JSON.stringify(report, null, 2));
    console.log(`\n리포트: ${outFile}`);

    process.exit(report.meets_criteria ? 0 : 1);
  }
}

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

function runCompareAb() {
  const packsOnlyDir = join(OUTPUT_DIR, "packs-only");
  const packsMcpDir = join(OUTPUT_DIR, "packs-mcp");

  const readReport = (dir: string, cond: string) => {
    const results: GradeResult[] = [];
    for (const lang of ALL_LANGS) {
      const filePath = join(dir, lang, "response.txt");
      if (!existsSync(filePath)) {
        results.push({
          lang,
          trap_id: "missing",
          verdict: "UNCERTAIN",
          bad_hit: [],
          good_hit: [],
          evidence: "출력 없음",
        });
        continue;
      }
      results.push(grade(lang, readFileSync(filePath, "utf8")));
    }
    return buildReport(cond, results);
  };

  const po = readReport(packsOnlyDir, "packs-only");
  const pm = readReport(packsMcpDir, "packs-mcp");

  printReport(po);
  printReport(pm);

  const cmp = compareAB(po, pm);
  console.log(`\nA/B 판정: ${cmp.ok ? "✓" : "✗"}  ${cmp.reason}`);
  process.exit(cmp.ok ? 0 : 1);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
