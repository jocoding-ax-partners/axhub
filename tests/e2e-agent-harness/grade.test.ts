/**
 * grade.test.ts — e2e-agent-harness 채점기/게이트/프롬프트 조립 회귀 테스트 (bun test 자동 수집)
 *
 * 과금 0 — 합성 fixture 와 정적 함수만 검증해요.
 * 케이스 본문은 fixtures.ts 단일 소스 (grade.ts --smoke CLI 와 공유).
 */

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import {
  grade,
  buildReport,
  compareAB,
  computeGate,
  extractCodeBlocks,
  stripCommentLines,
  hasCodeFence,
  type GradeResult,
} from "./grade.ts";
import { stripTaskFrontmatter } from "./run.ts";
import { SMOKE_CASES } from "./fixtures.ts";

const HARNESS_DIR = dirname(fileURLToPath(import.meta.url));
const ALL_LANGS = ["node", "python", "go", "java", "kotlin", "ruby"] as const;

describe("grade — smoke fixture 케이스", () => {
  for (const [i, c] of SMOKE_CASES.entries()) {
    const label = c.note ?? c.text.slice(0, 40).replace(/\n/g, "↵");
    test(`#${i + 1} [${c.lang}/${c.expect}] ${label}`, () => {
      const result = grade(c.lang, c.text);
      expect(result.verdict).toBe(c.expect);
    });
  }
});

describe("함정 정답 누출 차단 — TASK.md frontmatter strip", () => {
  for (const lang of ALL_LANGS) {
    test(`${lang}: 조립된 프롬프트에 trap_id/trap_kind 미포함`, () => {
      const raw = readFileSync(join(HARNESS_DIR, "tasks", lang, "TASK.md"), "utf8");
      // 원본에는 채점 메타가 존재해야 테스트가 유효 (전제 확인)
      expect(raw).toContain("trap_id:");
      const piped = stripTaskFrontmatter(raw);
      expect(piped).not.toContain("trap_id:");
      expect(piped).not.toContain("trap_kind:");
      expect(piped).not.toContain("packs_path:");
      // 본문 (# Task 헤더) 은 보존
      expect(piped).toContain("# Task:");
    });
  }

  test("frontmatter 없는 문서는 원문 유지", () => {
    const md = "# Task: plain\n\nbody";
    expect(stripTaskFrontmatter(md)).toBe(md);
  });
});

describe("게이트 산술 — UNCERTAIN 은 통과 못 함", () => {
  const emptyRun = (): GradeResult[] => ALL_LANGS.map((lang) => grade(lang, ""));

  test("빈 output 6건 → 전부 UNCERTAIN, meets_criteria=false", () => {
    const report = buildReport("packs-only", emptyRun());
    expect(report.uncertain).toBe(6);
    expect(report.meets_criteria).toBe(false);
  });

  test("빈 output → 게이트 FAIL", () => {
    const po = buildReport("packs-only", emptyRun());
    const pm = buildReport("packs-mcp", emptyRun());
    // compareAB 는 0≥0 으로 통과하지만 게이트는 uncertain/기준충족에서 막혀야 해요
    expect(compareAB(po, pm).ok).toBe(true);
    expect(computeGate(po, pm).pass).toBe(false);
  });

  test("UNCERTAIN 1건 섞이면 게이트 FAIL (나머지 전부 PASS 여도)", () => {
    const mixed: GradeResult[] = ALL_LANGS.map((lang, i) =>
      i === 0
        ? grade(lang, "")
        : { lang, trap_id: "x", verdict: "PASS", bad_hit: [], good_hit: ["g"], evidence: "t" }
    );
    const clean: GradeResult[] = ALL_LANGS.map((lang) => ({
      lang, trap_id: "x", verdict: "PASS", bad_hit: [], good_hit: ["g"], evidence: "t",
    }));
    const po = buildReport("packs-only", mixed);
    const pm = buildReport("packs-mcp", clean);
    expect(po.meets_criteria).toBe(false); // graded 5 < 6
    expect(computeGate(po, pm).pass).toBe(false);
  });

  test("정상 6/6 양 조건 → 게이트 PASS", () => {
    const clean: GradeResult[] = ALL_LANGS.map((lang) => ({
      lang, trap_id: "x", verdict: "PASS", bad_hit: [], good_hit: ["g"], evidence: "t",
    }));
    const po = buildReport("packs-only", clean);
    const pm = buildReport("packs-mcp", clean);
    expect(po.meets_criteria).toBe(true);
    expect(computeGate(po, pm).pass).toBe(true);
  });

  test("B 회귀 (pm.passed < po.passed) → 게이트 FAIL", () => {
    const mk = (verdicts: Array<"PASS" | "FAIL">): GradeResult[] =>
      ALL_LANGS.map((lang, i) => ({
        lang, trap_id: "x", verdict: verdicts[i] ?? "PASS", bad_hit: [], good_hit: [], evidence: "t",
      }));
    const po = buildReport("packs-only", mk(["PASS", "PASS", "PASS", "PASS", "PASS", "PASS"]));
    const pm = buildReport("packs-mcp", mk(["FAIL", "PASS", "PASS", "PASS", "PASS", "PASS"]));
    expect(computeGate(po, pm).pass).toBe(false);
  });
});

describe("extractCodeBlocks — 펜스 변형", () => {
  test("미종결 ``` 펜스는 EOF 까지 블록", () => {
    const out = extractCodeBlocks("prose or( here\n```ts\nconst x = 1;");
    expect(out).toContain("const x = 1;");
    expect(out).not.toContain("prose");
  });

  test("~~~ 펜스 인식", () => {
    const out = extractCodeBlocks("prose\n~~~py\nx = 1\n~~~\ntail");
    expect(out).toBe("x = 1");
  });

  test("펜스 없으면 원문 반환 (하위 호환)", () => {
    expect(extractCodeBlocks("plain text")).toBe("plain text");
  });

  test("hasCodeFence 판별", () => {
    expect(hasCodeFence("```ts\nx\n```")).toBe(true);
    expect(hasCodeFence("~~~\nx\n~~~")).toBe(true);
    expect(hasCodeFence("no fence")).toBe(false);
  });
});

describe("stripCommentLines — 주석 변형", () => {
  test("trailing // 주석 제거, :// 프로토콜 보존", () => {
    const out = stripCommentLines('fetch("https://api.example.com"); // or( 금지');
    expect(out).toContain("https://api.example.com");
    expect(out).not.toContain("or(");
  });

  test("trailing # 주석 제거, 루비 #{ 보간 보존", () => {
    const out = stripCommentLines('puts "id #{id}" # after= 금지');
    expect(out).toContain("#{id}");
    expect(out).not.toContain("after=");
  });

  test("여러 줄 블록 주석 상태 추적", () => {
    const out = stripCommentLines("a();\n/* bad or(\nstill bad after=\n*/ b();");
    expect(out).toContain("a();");
    expect(out).toContain("b();");
    expect(out).not.toContain("or(");
    expect(out).not.toContain("after=");
  });

  test("^\\s*\\* 블록 주석 연속행 제거", () => {
    const out = stripCommentLines("  * HttpClient /data/ forbidden\ncode();");
    expect(out).not.toContain("HttpClient");
    expect(out).toContain("code();");
  });
});
