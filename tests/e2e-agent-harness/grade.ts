/**
 * grade.ts — e2e-agent-harness 함정 채점기 (정적 스캔 전용)
 *
 * 에이전트 산출 코드를 정적으로 스캔해서 함정 패턴 존재 여부를 판정해요.
 * 실제 SDK / API 호출 없음. 과금 0.
 *
 * 사용:
 *   bun tests/e2e-agent-harness/grade.ts <lang> <output_text_file>
 *   bun tests/e2e-agent-harness/grade.ts --all <output_dir>
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";

export type TrapKind =
  | "or_combinator"
  | "after_cursor"
  | "raw_http_fetch"
  | "filterless_list"
  | "wrong_env_var";

export interface TrapRule {
  lang: string;
  trap_id: string;
  trap_kind: TrapKind;
  /** 이 패턴이 있으면 함정에 빠진 것 (FAIL) */
  bad_patterns: RegExp[];
  /** 이 패턴 중 하나라도 있으면 함정을 피한 것 (PASS 신호) */
  good_patterns: RegExp[];
  /** bad 가 없고 good 도 없을 때의 기본 판정 */
  ambiguous_default: "PASS" | "FAIL";
}

export interface GradeResult {
  lang: string;
  trap_id: string;
  verdict: "PASS" | "FAIL" | "UNCERTAIN";
  bad_hit: string[];
  good_hit: string[];
  evidence: string;
}

// ── 함정 규칙 정의 ───────────────────────────────────────────────────────────
// 규칙 근거: sdk/dist/sdk-knowledge/<lang>.md §6 Live data contract
//   - or/not 필터: 푸시 불가 → ValidationError
//   - after/before 커서: LegacyCursorError
//   - raw HTTP /data/ 직타: packs 가 "do not hand-roll requests" 명시
//   - AXHUB_TENANT (no suffix): 잘못된 env var 이름
export const TRAP_RULES: TrapRule[] = [
  {
    lang: "node",
    trap_id: "or_combinator",
    trap_kind: "or_combinator",
    bad_patterns: [
      /\bor\s*\(/,       // or( — 비푸시 OR 결합자
    ],
    good_patterns: [
      /\.in\s*\(\s*\[/,  // .in([...]) — 올바른 대안
    ],
    ambiguous_default: "FAIL",
  },
  {
    lang: "python",
    trap_id: "after_cursor",
    trap_kind: "after_cursor",
    bad_patterns: [
      /\bafter\s*=/,    // after= 키워드 인자 → LegacyCursorError
      /\bbefore\s*=/,   // before= 키워드 인자 → LegacyCursorError
      /"after"\s*:/,    // "after": dict 키
      /"before"\s*:/,   // "before": dict 키
    ],
    good_patterns: [
      /\bpage\s*=/,     // page= 오프셋 페이지네이션
      /\bcursor\s*=/,   // cursor= 숫자 커서
      /LegacyCursorError/, // 에러 클래스 언급 (에이전트가 경고)
    ],
    ambiguous_default: "FAIL",
  },
  {
    lang: "go",
    trap_id: "or_combinator",
    trap_kind: "or_combinator",
    bad_patterns: [
      /axhub\.Or\s*\(/,  // axhub.Or( — 비푸시 OR
      /\bOr\s*\(\s*axhub/, // Or(axhub. 패턴 변형
    ],
    good_patterns: [
      /\.In\s*\(/,       // .In(...) — 올바른 대안
    ],
    ambiguous_default: "FAIL",
  },
  {
    lang: "java",
    trap_id: "raw_http_fetch",
    trap_kind: "raw_http_fetch",
    // java 함정: raw HTTP 로 /data/ 직접 호출 요청 → 올바른 에이전트는 SDK 사용 권고
    bad_patterns: [
      // raw HTTP 클라이언트 + /data/ URL 조합
      /HttpClient[\s\S]{0,300}\/data\//,
      /HttpURLConnection[\s\S]{0,300}\/data\//,
      /OkHttpClient[\s\S]{0,300}\/data\//,
      /RestTemplate[\s\S]{0,300}\/data\//,
      /WebClient[\s\S]{0,300}\/data\//,
      // 역순 검사
      /\/data\/[\s\S]{0,300}HttpClient/,
      /\/data\/[\s\S]{0,300}HttpURLConnection/,
    ],
    good_patterns: [
      /orders\.list\s*\(/,       // SDK 메서드 사용
      /data\.table\s*\(/,        // SDK data.table
      /data\.discover\s*\(/,     // SDK discover
      /do not hand.roll/i,       // 에이전트가 raw HTTP 금지 경고
      /use the SDK/i,
      /DataTableClient/,         // SDK 타입 명시
    ],
    ambiguous_default: "FAIL",
  },
  {
    lang: "kotlin",
    trap_id: "filterless_list",
    trap_kind: "filterless_list" as TrapKind,
    // non-owner-scoped 테이블에 무필터 list/count → ValidationError(where_required)
    // bad: ListOptions.create() 에 .where() 체이닝 없음, 또는 빈 인자 list()
    bad_patterns: [
      /\.list\s*\(\s*ListOptions\.create\s*\(\s*\)\s*\)/, // .list(ListOptions.create())
      /reports\.list\s*\(\s*\)/,                          // reports.list() 인자 없음
      /\.count\s*\(\s*\)/,                                // .count() 인자 없음
    ],
    good_patterns: [
      /\.where\s*\(/,  // .where( 필터 포함
    ],
    // bad/good 둘 다 없으면 where 없는 체인으로 간주 → FAIL
    ambiguous_default: "FAIL",
  },
  {
    lang: "ruby",
    trap_id: "or_combinator",
    trap_kind: "or_combinator",
    bad_patterns: [
      /\bor_\s*\(/,    // or_( — Ruby DSL 비푸시 OR
      /\.or\s*\(/,     // .or( 패턴
    ],
    good_patterns: [
      /\.in_\s*\(\s*(\[|%w\[)/,  // .in_([...]) 또는 .in_(%w[...]) — 올바른 대안
    ],
    ambiguous_default: "FAIL",
  },
];

// ── 코드 블록 추출 ────────────────────────────────────────────────────────────
/**
 * 마크다운 코드 블록(``` ... ```) 내부 텍스트만 추출해요.
 * 코드 블록이 없으면 원문 그대로 반환해요 (하위 호환 — 합성 smoke 케이스 지원).
 */
export function extractCodeBlocks(text: string): string {
  const blocks: string[] = [];
  const re = /```[^\n]*\n([\s\S]*?)```/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    blocks.push(m[1]);
  }
  return blocks.length > 0 ? blocks.join("\n") : text;
}

/**
 * 코드에서 주석 줄을 제거해요 (# // -- * 로 시작하는 줄).
 * 주석 안에 bad pattern 을 설명 목적으로 인용한 경우 false-negative 방지.
 */
export function stripCommentLines(code: string): string {
  return code
    .split("\n")
    .filter((line) => !/^\s*(#|\/\/|--|\/\*)/.test(line))
    .join("\n");
}

// ── 채점 로직 ─────────────────────────────────────────────────────────────────
export function grade(lang: string, outputText: string): GradeResult {
  const rule = TRAP_RULES.find((r) => r.lang === lang);
  if (!rule) {
    return {
      lang,
      trap_id: "unknown",
      verdict: "UNCERTAIN",
      bad_hit: [],
      good_hit: [],
      evidence: `규칙 없음: ${lang}`,
    };
  }

  // 빈 출력 = 채점 불가 → UNCERTAIN (FAIL 로 둔갑시키지 않고 게이트에서 차단)
  if (outputText.trim() === "") {
    return {
      lang,
      trap_id: rule.trap_id,
      verdict: "UNCERTAIN",
      bad_hit: [],
      good_hit: [],
      evidence: "빈 출력 — 채점 불가",
    };
  }

  // 코드 블록 내부만 스캔, 주석 줄 제거 — 설명·주의문·인라인 주석 false-negative 방지
  const scanText = stripCommentLines(extractCodeBlocks(outputText));

  const bad_hit: string[] = [];
  for (const pat of rule.bad_patterns) {
    const m = scanText.match(pat);
    if (m) bad_hit.push(m[0].slice(0, 60).replace(/\n/g, "↵"));
  }

  const good_hit: string[] = [];
  for (const pat of rule.good_patterns) {
    const m = scanText.match(pat);
    if (m) good_hit.push(m[0].slice(0, 60).replace(/\n/g, "↵"));
  }

  let verdict: "PASS" | "FAIL" | "UNCERTAIN";
  let evidence: string;

  if (bad_hit.length > 0) {
    // bad 패턴 발견 → 함정에 빠짐
    verdict = "FAIL";
    evidence = `bad_pattern 매칭: ${bad_hit.join(", ")}`;
  } else if (good_hit.length > 0) {
    // bad 없고 good 있음 → 함정 회피
    verdict = "PASS";
    evidence = `good_pattern 매칭: ${good_hit.join(", ")}`;
  } else {
    // 둘 다 없음 → ambiguous_default 적용
    verdict = rule.ambiguous_default === "PASS" ? "PASS" : "FAIL";
    evidence = `패턴 미매칭 → ambiguous_default=${rule.ambiguous_default}`;
  }

  return {
    lang,
    trap_id: rule.trap_id,
    verdict,
    bad_hit,
    good_hit,
    evidence,
  };
}

// ── 요약 리포트 ───────────────────────────────────────────────────────────────
export interface HarnessReport {
  condition: string;
  total: number;
  passed: number;
  failed: number;
  uncertain: number;
  meets_criteria: boolean;
  /** 기계 판정 기준: 함정 수 ≥ SDK 언어 수(현 6) AND 각 언어 ≥1 */
  lang_coverage: Record<string, boolean>;
  results: GradeResult[];
}

export function buildReport(
  condition: string,
  results: GradeResult[]
): HarnessReport {
  const total = results.length;
  const passed = results.filter((r) => r.verdict === "PASS").length;
  const failed = results.filter((r) => r.verdict === "FAIL").length;
  const uncertain = results.filter((r) => r.verdict === "UNCERTAIN").length;

  // 기계 판정 기준 (Plan §D.4 Architect F3 산술 정정):
  //   함정 수 ≥ SDK 언어 수(현 6) AND 각 언어 ≥1
  // UNCERTAIN (실행 실패/출력 없음) 은 "측정된 함정" 이 아니므로 집계에서 제외해요.
  // 제외하지 않으면 전부 실행 실패해도 meets_criteria 가 true 가 되는 구멍이 생겨요.
  const SDK_LANG_COUNT = 6;
  const graded = results.filter((r) => r.verdict !== "UNCERTAIN");
  const lang_coverage: Record<string, boolean> = {};
  for (const r of graded) {
    lang_coverage[r.lang] = r.verdict === "PASS";
  }

  const trap_count_ok = graded.length >= SDK_LANG_COUNT;
  const lang_coverage_ok = Object.keys(lang_coverage).length === SDK_LANG_COUNT;
  const meets_criteria = trap_count_ok && lang_coverage_ok;

  return {
    condition,
    total,
    passed,
    failed,
    uncertain,
    meets_criteria,
    lang_coverage,
    results,
  };
}

// ── A/B 비교 ─────────────────────────────────────────────────────────────────
/**
 * Plan §D.4: packs+MCP 통과 수 ≥ packs-only 통과 수 (동률 허용)
 */
export function compareAB(
  packsOnly: HarnessReport,
  packsMcp: HarnessReport
): { ok: boolean; reason: string } {
  if (packsMcp.passed >= packsOnly.passed) {
    return {
      ok: true,
      reason: `packs+MCP(${packsMcp.passed}) ≥ packs-only(${packsOnly.passed}) ✓`,
    };
  }
  return {
    ok: false,
    reason: `packs+MCP(${packsMcp.passed}) < packs-only(${packsOnly.passed}) ✗ — 회귀`,
  };
}

/**
 * 최종 게이트: A/B 비교 통과 AND 기준 충족 AND 양 조건 UNCERTAIN 0건.
 * UNCERTAIN 이 하나라도 있으면 측정 자체가 불완전하므로 게이트 FAIL 이에요.
 */
export function computeGate(
  packsOnly: HarnessReport,
  packsMcp: HarnessReport
): { pass: boolean; reason: string } {
  const cmp = compareAB(packsOnly, packsMcp);
  const uncertainOk = packsOnly.uncertain === 0 && packsMcp.uncertain === 0;
  const pass = cmp.ok && packsOnly.meets_criteria && packsMcp.meets_criteria && uncertainOk;

  const parts = [
    `A/B ${cmp.ok ? "✓" : "✗"}`,
    `기준충족 A=${packsOnly.meets_criteria ? "✓" : "✗"} B=${packsMcp.meets_criteria ? "✓" : "✗"}`,
    `UNCERTAIN A=${packsOnly.uncertain} B=${packsMcp.uncertain}${uncertainOk ? " ✓" : " ✗"}`,
  ];
  return { pass, reason: parts.join("  ") };
}

// ── CLI 진입점 ────────────────────────────────────────────────────────────────
if (import.meta.main) {
  const args = process.argv.slice(2);

  if (args[0] === "--help" || args.length === 0) {
    console.log(`사용법:
  bun grade.ts <lang> <output_file>       # 단일 채점
  bun grade.ts --smoke                    # smoke fixture 채점 (고정 출력)
  bun grade.ts --all <output_dir>         # 디렉토리 일괄 채점
    `);
    process.exit(0);
  }

  if (args[0] === "--smoke") {
    // smoke: 사전 정의된 합성 출력으로 채점기 검증 (과금 0)
    runSmokeGrade();
    process.exit(0);
  }

  if (args[0] === "--all" && args[1]) {
    const outputDir = args[1];
    gradeDirectory(outputDir);
    process.exit(0);
  }

  const [lang, filePath] = args;
  if (!lang || !filePath) {
    console.error("오류: lang 과 output_file 이 필요해요");
    process.exit(1);
  }

  const text = readFileSync(filePath, "utf8");
  const result = grade(lang, text);
  console.log(JSON.stringify(result, null, 2));
  process.exit(result.verdict === "FAIL" ? 1 : 0);
}

// ── smoke: 합성 출력으로 채점기 자가검증 ─────────────────────────────────────
function runSmokeGrade() {
  console.log("▶ grade.ts smoke — 합성 출력 채점기 자가검증\n");

  const cases: Array<{ lang: string; text: string; expect: "PASS" | "FAIL" }> =
    [
      // node: or( 함정에 빠진 케이스
      {
        lang: "node",
        text: `const result = await orders.list({ where: or(where('status').eq('paid'), where('status').eq('pending')) });`,
        expect: "FAIL",
      },
      // node: .in([ 올바른 케이스
      {
        lang: "node",
        text: `const result = await orders.list({ where: where('status').in(['paid', 'pending']) });`,
        expect: "PASS",
      },
      // python: after= 함정
      {
        lang: "python",
        text: `page = events.list(after=prev_cursor, page_size=50)`,
        expect: "FAIL",
      },
      // python: page= 올바른 케이스
      {
        lang: "python",
        text: `page = events.list(where=where('id').gte(0), page=2, page_size=50)`,
        expect: "PASS",
      },
      // go: axhub.Or( 함정
      {
        lang: "go",
        text: `page, err := tasks.List(ctx, &axhub.ListOptions{ Where: axhub.Or(axhub.Where("priority").Eq("high"), axhub.Where("priority").Eq("urgent")) })`,
        expect: "FAIL",
      },
      // go: .In( 올바른 케이스
      {
        lang: "go",
        text: `w := axhub.Where("priority").In("high", "urgent"); page, err := tasks.List(ctx, &axhub.ListOptions{Where: &w})`,
        expect: "PASS",
      },
      // java: raw HTTP 함정
      {
        lang: "java",
        text: `HttpClient client = HttpClient.newHttpClient(); HttpRequest req = HttpRequest.newBuilder().uri(URI.create("https://api.axhub.ai/data/my-tenant/my-app/orders")).header("X-Api-Key", token).build();`,
        expect: "FAIL",
      },
      // java: SDK 사용 올바른 케이스
      {
        lang: "java",
        text: `DataTableClient orders = data.table(Schema.defineSchema("orders", ...)); PaginatedList page = orders.list(ListOptions.create().where(Ops.where("status").eq("paid")));`,
        expect: "PASS",
      },
      // kotlin: 무필터 list — 빈 ListOptions (non-owner 테이블 함정)
      {
        lang: "kotlin",
        text: `val results = reports.list(ListOptions.create())`,
        expect: "FAIL",
      },
      // kotlin: .where( 필터 포함 — 올바른 케이스
      {
        lang: "kotlin",
        text: `val results = reports.list(ListOptions.create().where(Ops.where("created_at").gte("1970-01-01T00:00:00Z")).pageSize(50))`,
        expect: "PASS",
      },
      // ruby: or_( 함정
      {
        lang: "ruby",
        text: `result = products.list(where: or_(where('category').eq('electronics'), where('category').eq('appliances')))`,
        expect: "FAIL",
      },
      // ruby: .in_( 올바른 케이스
      {
        lang: "ruby",
        text: `result = products.list(where: where('category').in_(['electronics', 'appliances']))`,
        expect: "PASS",
      },
      // ── false-negative 회귀 케이스 (설명문 bad 언급 + 코드 블록 정상) ──────
      // node: 설명에 or( 언급, 코드 블록은 .in([ 정상 → PASS
      {
        lang: "node",
        text: "SDK rejects `or()` combinator — not pushable.\n```ts\nconst r = await orders.list({ where: where('status').in(['paid','pending']) });\n```",
        expect: "PASS",
      },
      // python: 설명에 after= 언급, 코드 블록은 page= 정상 → PASS
      {
        lang: "python",
        text: "Do not use after=/before= (LegacyCursorError).\n```python\npage2 = logs.list(where=where('id').gte(0), page=2, page_size=50)\n```",
        expect: "PASS",
      },
      // go: 설명에 axhub.Or( 언급, 코드 블록은 .In( 정상 → PASS
      {
        lang: "go",
        text: "axhub.Or(...) causes ValidationError.\n```go\nw := axhub.Where(\"priority\").In(\"high\", \"urgent\")\npage, err := tasks.List(ctx, &axhub.ListOptions{Where: &w})\n```",
        expect: "PASS",
      },
      // ruby: %w[ 구문 .in_(%w[...]) → PASS
      {
        lang: "ruby",
        text: "result = products.list(where: where('category').in_(%w[electronics appliances]))",
        expect: "PASS",
      },
      // python: 코드 블록 내 주석에 after= 언급, 실제 코드는 page= → PASS
      {
        lang: "python",
        text: "```python\n# after=prev_cursor → LegacyCursorError (사용 금지)\npage2 = logs.list(where=where('id').gte(0), page=2, page_size=50)\n```",
        expect: "PASS",
      },
    ];

  let all_ok = true;
  for (const c of cases) {
    const result = grade(c.lang, c.text);
    const ok = result.verdict === c.expect;
    const icon = ok ? "✓" : "✗";
    if (!ok) all_ok = false;
    console.log(
      `${icon} [${c.lang}/${c.expect}] verdict=${result.verdict}  ${result.evidence}`
    );
  }

  console.log(`\nsmoke 결과: ${all_ok ? "모두 통과 ✓" : "실패 있음 ✗"}`);
  if (!all_ok) process.exit(1);
}

// ── 디렉토리 일괄 채점 ────────────────────────────────────────────────────────
function gradeDirectory(outputDir: string) {
  const LANGS = ["node", "python", "go", "java", "kotlin", "ruby"] as const;
  const results: GradeResult[] = [];

  for (const lang of LANGS) {
    const filePath = join(outputDir, lang, "response.txt");
    try {
      const text = readFileSync(filePath, "utf8");
      results.push(grade(lang, text));
    } catch {
      results.push({
        lang,
        trap_id: "missing",
        verdict: "UNCERTAIN",
        bad_hit: [],
        good_hit: [],
        evidence: `출력 파일 없음: ${filePath}`,
      });
    }
  }

  const condition = outputDir.includes("packs-mcp") ? "packs-mcp" : "packs-only";
  const report = buildReport(condition, results);

  console.log(JSON.stringify(report, null, 2));
  console.log(
    `\n기준 충족: ${report.meets_criteria ? "✓" : "✗"}  (통과 ${report.passed}/${report.total})`
  );
}
