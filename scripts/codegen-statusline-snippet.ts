#!/usr/bin/env bun
/**
 * statusline wiring snippet codegen — drift lock.
 *
 * `bin/statusline.sh:13-18` 의 wiring 코멘트 JSON 블록과
 * `skills/enable-statusline/SKILL.md` body 안의 snippet 이 byte-identical 하도록
 * 강제해요. `scripts/codegen-preflight-injection.ts:43-60` SSOT 선례를 그대로 따라요.
 *
 * Idempotent: 이미 최신 상태면 파일을 변경하지 않아요.
 *
 * 사용법:
 *   bun scripts/codegen-statusline-snippet.ts --check   # drift 여부 확인 (exit 0 = OK, exit 1 = drift)
 *   bun scripts/codegen-statusline-snippet.ts --write   # SKILL.md 를 최신 snippet 으로 덮어써요
 *   bun scripts/codegen-statusline-snippet.ts           # 도움말 출력
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILL_PATH = join(REPO_ROOT, "skills/enable-statusline/SKILL.md");
const BEGIN_MARKER = "<!-- BEGIN STATUSLINE_SNIPPET";
const END_MARKER = "<!-- END STATUSLINE_SNIPPET -->";

/**
 * Returns the canonical 6-line JSON wiring snippet.
 * Byte-identical to the comment block in bin/statusline.sh:13-18.
 */
export function getStatuslineSnippet(): string {
  return [
    "{",
    '  "statusLine": {',
    '    "type": "command",',
    '    "command": "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh"',
    "  }",
    "}",
  ].join("\n");
}

function readSkillContent(): string {
  return readFileSync(SKILL_PATH, "utf8");
}

/**
 * Locates the BEGIN/END STATUSLINE_SNIPPET markers and extracts the JSON
 * inside the ```json ... ``` fence between them.
 * Returns null if markers are absent.
 */
function extractSnippetFromSkill(content: string): string | null {
  const beginIdx = content.indexOf(BEGIN_MARKER);
  const endIdx = content.indexOf(END_MARKER);
  if (beginIdx === -1 || endIdx === -1) return null;

  const between = content.slice(beginIdx, endIdx);
  const fenceMatch = between.match(/```json\n([\s\S]*?)\n```/);
  if (!fenceMatch) return null;
  return fenceMatch[1];
}

/**
 * Replaces the content between BEGIN/END markers with the canonical snippet.
 */
function replaceSnippetInSkill(content: string): string {
  const beginIdx = content.indexOf(BEGIN_MARKER);
  const endIdx = content.indexOf(END_MARKER);
  if (beginIdx === -1 || endIdx === -1) {
    process.stderr.write(
      "BEGIN/END STATUSLINE_SNIPPET markers 없어요 — SKILL.md scaffold 가 markers 포함하는지 확인해줘요.\n"
    );
    process.exit(1);
  }

  // Find end of BEGIN marker line
  const beginLineEnd = content.indexOf("\n", beginIdx) + 1;
  const newBlock =
    "```json\n" + getStatuslineSnippet() + "\n```\n" + END_MARKER;
  return content.slice(0, beginLineEnd) + newBlock + content.slice(endIdx + END_MARKER.length);
}

if (import.meta.main) {
  const arg = process.argv[2];

  if (arg === "--check") {
    const content = readSkillContent();
    const extracted = extractSnippetFromSkill(content);
    if (extracted === null) {
      process.stderr.write(
        "BEGIN/END STATUSLINE_SNIPPET markers 없어요 — worker-1 의 SKILL.md 작업이 아직 완료 안 됐어요. --check 는 markers 추가 후 통과할 거예요.\n"
      );
      process.exit(1);
    }
    const canonical = getStatuslineSnippet();
    if (extracted === canonical) {
      process.stdout.write("codegen-statusline-snippet: snippet in sync (no drift)\n");
      process.exit(0);
    } else {
      process.stderr.write("❌ statusline snippet drift 감지됐어요.\n");
      process.stderr.write(`  SKILL.md 에 있는 snippet:\n${extracted}\n`);
      process.stderr.write(`  canonical snippet:\n${canonical}\n`);
      process.stderr.write("`bun run scripts/codegen-statusline-snippet.ts --write` 실행해주세요.\n");
      process.exit(1);
    }
  } else if (arg === "--write") {
    const content = readSkillContent();
    const updated = replaceSnippetInSkill(content);
    writeFileSync(SKILL_PATH, updated);
    process.stdout.write("codegen-statusline-snippet: SKILL.md snippet 업데이트 완료\n");
    process.exit(0);
  } else {
    process.stdout.write(
      [
        "codegen-statusline-snippet — statusline wiring snippet drift lock",
        "",
        "사용법:",
        "  bun scripts/codegen-statusline-snippet.ts --check   drift 여부 확인 (exit 0 = OK)",
        "  bun scripts/codegen-statusline-snippet.ts --write   SKILL.md snippet 최신화",
        "",
        "canonical snippet:",
        getStatuslineSnippet(),
        "",
      ].join("\n")
    );
    process.exit(0);
  }
}
