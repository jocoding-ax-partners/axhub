import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const read = (path: string) => readFileSync(join(root, path), "utf8");

describe("apps delete explicit-confirmation UX contract", () => {
  test("apps create slug path keeps preview confirmation before mutation", () => {
    const skill = read("skills/apps/SKILL.md");
    const registry = JSON.parse(read("tests/fixtures/ask-defaults/registry.json"));

    expect(skill).toContain("앱을 만들까요?");
    expect(skill).toContain("AskUserQuestion preview plus explicit confirmation");
    expect(skill).toContain('axhub apps create --name "$NAME" --slug "$SLUG" --json');
    expect(skill).toContain("one mutation command per Bash tool call");
    expect(registry.apps["앱을 만들까요?"]?.safe_default).toBe("abort");
  });

  test("apps skill binds one command target across preview and delete command", () => {
    const skill = read("skills/apps/SKILL.md");

    expect(skill).toContain("COMMAND_TARGET");
    expect(skill).toContain("read-only data");
    expect(skill).toContain("use only `COMMAND_TARGET`");
    expect(skill).toContain('axhub apps delete "$COMMAND_TARGET" --execute --json');
    expect(skill).not.toContain('axhub apps delete "$APP" --dry-run --json');
    expect(skill).toContain("Keep `COMMAND_TARGET` identical");
  });

  test("delete confirmation has a non-interactive abort default", () => {
    const registry = JSON.parse(read("tests/fixtures/ask-defaults/registry.json"));
    expect(registry.apps["앱을 삭제할까요?"]?.safe_default).toBe("abort");
    expect(registry.apps["앱을 삭제할까요?"]?.rationale).toContain("삭제");
  });

  test("apps command metadata is mutation-aware, not read-only only", () => {
    const commandDoc = read("commands/apps.md");
    const descriptionLine = commandDoc.split("\n").find((line) => line.startsWith("description:"));

    expect(descriptionLine).toContain("관리");
    expect(descriptionLine).toContain("승인");
    expect(descriptionLine).not.toContain("읽기 전용");
  });

  test("delete corpus and baselines no longer say app deletion is unsupported", () => {
    const files = [
      "PLAN.md",
      "tests/corpus.jsonl",
      "tests/corpus.20.jsonl",
      "tests/corpus.100.jsonl",
      "tests/baseline-results.claude-native.20.json",
      "tests/baseline-results.claude-native.100.json",
      "tests/baseline-results.docs-only.20.json",
      "tests/baseline-results.docs-only.100.json",
    ];

    for (const file of files) {
      const contents = read(file);
      expect(contents).not.toContain("delete 미지원");
      expect(contents).not.toContain("unsupported in v0.1.0");
    }
  });
});
