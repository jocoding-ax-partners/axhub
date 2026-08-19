import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
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
