import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const POLICY_PATH = join(REPO_ROOT, "docs", "policy", "agent-policy.md");

interface PolicyRule {
  id: string;
  files: string[];
  invariants: string[];
  codexFiles: string[];
  codexInvariants: string[];
}

const parseFileLine = (line: string): string[] =>
  line
    .split(",")
    .map((part) => part.trim().replaceAll("`", ""))
    .filter((part) => part.length > 0);

const parseInvariantLine = (line: string): string[] =>
  [...line.matchAll(/"([^"]+)"/g)].map((match) => match[1]!);

// KTD10: host-scoped invariant 문법 — `- 적용(codex):` / `- invariant(codex):` 는
// codex 파생 파일에만 검사해요. 공용 문법(- 적용: / - invariant:)과 독립이라
// host 별 명령 문자열이 서로를 깨뜨리지 않아요.
const parseRules = (markdown: string): PolicyRule[] => {
  const blocks = markdown.split(/^## (?=AP-)/m).slice(1);
  return blocks.map((block) => {
    const id = block.split(/\s/, 1)[0] ?? "";
    const files = parseFileLine(block.match(/^- 적용: (.+)$/m)?.[1] ?? "");
    const invariants = parseInvariantLine(block.match(/^- invariant: (.+)$/m)?.[1] ?? "");
    const codexFiles = parseFileLine(block.match(/^- 적용\(codex\): (.+)$/m)?.[1] ?? "");
    const codexInvariants = parseInvariantLine(block.match(/^- invariant\(codex\): (.+)$/m)?.[1] ?? "");
    return { id, files, invariants, codexFiles, codexInvariants };
  });
};

// KD4 — dev-policy 의 skill 개수 표기 드리프트를 기계로 잠가요. policy-parity 가
// 원래 읽는 것은 agent-policy.md 뿐이라, DP-1·DP-3·DP-5 의 개수는 지금까지
// 어떤 게이트도 보지 않았고 실제로 9·8·9 로 어긋나 있었어요.
const DEV_POLICY_PATH = join(REPO_ROOT, "docs", "policy", "dev-policy.md");
const SKILLS_DIR = join(REPO_ROOT, "skills");

const skillCount = (): number =>
  readdirSync(SKILLS_DIR, { withFileTypes: true }).filter((entry) => entry.isDirectory()).length;

// "11개 SKILL.md" / "11개 skill" 그리고 "skill 은 11개" / "SKILL.md 11개" 양쪽 어순.
const COUNT_PATTERNS = [/(\d+)\s*개\s*(?:SKILL\.md|skill)/g, /(?:SKILL\.md|skill)\s*(?:은\s*)?(\d+)\s*개/g];

describe("dev policy skill counts (KD4)", () => {
  test("dev-policy 의 모든 skill 개수 표기가 실제 skill 수와 같아요", () => {
    expect(existsSync(DEV_POLICY_PATH), `missing ${DEV_POLICY_PATH}`).toBe(true);
    const markdown = readFileSync(DEV_POLICY_PATH, "utf8");
    const expected = skillCount();
    const found: number[] = [];
    for (const pattern of COUNT_PATTERNS) {
      for (const match of markdown.matchAll(pattern)) found.push(Number(match[1]));
    }
    expect(found.length, "dev-policy 에 skill 개수 표기가 하나도 없어요").toBeGreaterThan(0);
    for (const value of found) {
      expect(value, `dev-policy 개수 표기 ${value} != 실제 skill 수 ${expected}`).toBe(expected);
    }
  });
});

describe("agent policy parity", () => {
  test("policy document exists", () => {
    expect(existsSync(POLICY_PATH), `missing ${POLICY_PATH}`).toBe(true);
  });

  test("every rule has files and invariants, and every invariant exists in every target file", () => {
    const rules = parseRules(readFileSync(POLICY_PATH, "utf8"));
    expect(rules.length).toBeGreaterThanOrEqual(9);

    for (const rule of rules) {
      expect(rule.files.length, `${rule.id}: 적용 대상 없음`).toBeGreaterThan(0);
      expect(rule.invariants.length, `${rule.id}: invariant 없음`).toBeGreaterThan(0);

      for (const file of rule.files) {
        const target = join(REPO_ROOT, file);
        expect(existsSync(target), `${rule.id}: 적용 파일 없음 — ${file}`).toBe(true);
        const content = readFileSync(target, "utf8");
        for (const invariant of rule.invariants) {
          expect(
            content.includes(invariant),
            `${rule.id}: "${invariant}" 가 ${file} 에 없음 — 정책과 SKILL.md 가 어긋남`,
          ).toBe(true);
        }
      }
    }
  });

  test("codex-scoped invariants exist in every codex target file (KTD10)", () => {
    const rules = parseRules(readFileSync(POLICY_PATH, "utf8"));
    for (const rule of rules) {
      // 짝 계약: 한쪽만 있는 host-scoped 선언은 무의미해서 문서 오류로 봐요.
      expect(
        rule.codexInvariants.length > 0 === rule.codexFiles.length > 0,
        `${rule.id}: 적용(codex) 와 invariant(codex) 는 함께 선언해야 해요`,
      ).toBe(true);

      for (const file of rule.codexFiles) {
        const target = join(REPO_ROOT, file);
        expect(existsSync(target), `${rule.id}: 적용(codex) 파일 없음 — ${file}`).toBe(true);
        const content = readFileSync(target, "utf8");
        for (const invariant of rule.codexInvariants) {
          expect(
            content.includes(invariant),
            `${rule.id}: "${invariant}" 가 ${file} 에 없음 — 정책과 codex 파생본이 어긋남`,
          ).toBe(true);
        }
      }
    }
  });
});
