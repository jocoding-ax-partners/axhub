import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const POLICY_PATH = join(REPO_ROOT, "docs", "policy", "agent-policy.md");

interface PolicyRule {
  id: string;
  files: string[];
  invariants: string[];
}

const parseRules = (markdown: string): PolicyRule[] => {
  const blocks = markdown.split(/^## (?=AP-)/m).slice(1);
  return blocks.map((block) => {
    const id = block.split(/\s/, 1)[0] ?? "";
    const fileLine = block.match(/^- 적용: (.+)$/m)?.[1] ?? "";
    const invariantLine = block.match(/^- invariant: (.+)$/m)?.[1] ?? "";
    const files = fileLine
      .split(",")
      .map((part) => part.trim().replaceAll("`", ""))
      .filter((part) => part.length > 0);
    const invariants = [...invariantLine.matchAll(/"([^"]+)"/g)].map((match) => match[1]!);
    return { id, files, invariants };
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
});
