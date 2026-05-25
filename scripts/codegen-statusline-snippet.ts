#!/usr/bin/env bun
/**
 * statusline wiring snippet codegen — drift lock.
 *
 * `bin/statusline.sh:13-18` 의 Unix wiring 코멘트 JSON 블록과
 * `skills/enable-statusline/SKILL.md` 의 두 snippet 블록(_UNIX + _WINDOWS)이
 * byte-identical 하도록 강제해요. `scripts/preflight-block.ts` 의 canonical-block
 * SSOT 선례를 그대로 따라요.
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

// New markers — _UNIX / _WINDOWS suffix
const UNIX_BEGIN = "<!-- BEGIN STATUSLINE_SNIPPET_UNIX";
const UNIX_END = "<!-- END STATUSLINE_SNIPPET_UNIX -->";
const WIN_BEGIN = "<!-- BEGIN STATUSLINE_SNIPPET_WINDOWS";
const WIN_END = "<!-- END STATUSLINE_SNIPPET_WINDOWS -->";

// Legacy markers (no suffix) — detected for backward-compat migration
const LEGACY_BEGIN = "<!-- BEGIN STATUSLINE_SNIPPET";
const LEGACY_END = "<!-- END STATUSLINE_SNIPPET -->";

/**
 * Returns the canonical Unix wiring snippet (bin/statusline.sh).
 * Byte-identical to the comment block in bin/statusline.sh:13-18.
 */
export function getStatuslineSnippetUnix(): string {
  return [
    "{",
    '  "statusLine": {',
    '    "type": "command",',
    '    "command": "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh"',
    "  }",
    "}",
  ].join("\n");
}

/** Backward-compat alias — callers using getStatuslineSnippet() keep working. */
export const getStatuslineSnippet = getStatuslineSnippetUnix;

/**
 * Returns the canonical Windows wiring snippet (bin/statusline.ps1).
 * Uses explicit `powershell.exe -NoProfile -ExecutionPolicy Bypass -File` form
 * to survive stock Win10/11 ExecutionPolicy=Restricted (Option C, ADR v0.5.12).
 */
export function getStatuslineSnippetWindows(): string {
  return [
    "{",
    '  "statusLine": {',
    '    "type": "command",',
    '    "command": "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \\"${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1\\""',
    "  }",
    "}",
  ].join("\n");
}

function readSkillContent(): string {
  return readFileSync(SKILL_PATH, "utf8");
}

/**
 * Extracts the JSON snippet between markers for the given variant.
 *
 * unix  — tries _UNIX markers first, falls back to legacy (no suffix).
 * windows — tries _WINDOWS markers only.
 *
 * Returns null if markers absent.
 */
function extractSnippet(content: string, variant: "unix" | "windows"): string | null {
  let beginMarker: string;
  let endMarker: string;

  if (variant === "unix") {
    if (content.includes(UNIX_END)) {
      beginMarker = UNIX_BEGIN;
      endMarker = UNIX_END;
    } else if (content.includes(LEGACY_END)) {
      // Legacy form — treat as UNIX
      beginMarker = LEGACY_BEGIN;
      endMarker = LEGACY_END;
    } else {
      return null;
    }
  } else {
    if (!content.includes(WIN_END)) return null;
    beginMarker = WIN_BEGIN;
    endMarker = WIN_END;
  }

  const beginIdx = content.indexOf(beginMarker);
  const endIdx = content.indexOf(endMarker);
  if (beginIdx === -1 || endIdx === -1) return null;

  const between = content.slice(beginIdx, endIdx);
  const fenceMatch = between.match(/```json\n([\s\S]*?)\n```/);
  return fenceMatch ? fenceMatch[1] : null;
}

/**
 * Replaces the content between markers with the canonical snippet.
 *
 * unix  — migrates legacy markers to _UNIX if still in old form.
 * windows — requires _WINDOWS markers to already exist.
 */
function replaceSnippet(content: string, variant: "unix" | "windows", snippet: string): string {
  let beginMarker: string;
  let endMarker: string;
  let newBeginLine: string;
  let newEndMarker: string;

  if (variant === "unix") {
    newBeginLine = `${UNIX_BEGIN} (codegen-managed by scripts/codegen-statusline-snippet.ts) -->`;
    newEndMarker = UNIX_END;
    if (content.includes(UNIX_END)) {
      beginMarker = UNIX_BEGIN;
      endMarker = UNIX_END;
    } else if (content.includes(LEGACY_END)) {
      // Migrate legacy → _UNIX
      beginMarker = LEGACY_BEGIN;
      endMarker = LEGACY_END;
    } else {
      process.stderr.write(
        "STATUSLINE_SNIPPET_UNIX (또는 legacy) markers 없어요 — SKILL.md 에 markers 가 포함되어 있는지 확인해줘요.\n"
      );
      process.exit(1);
    }
  } else {
    if (!content.includes(WIN_END)) {
      process.stderr.write(
        "STATUSLINE_SNIPPET_WINDOWS markers 없어요 — SKILL.md 에 Windows snippet 블록 markers 추가 후 다시 실행해줘요.\n"
      );
      process.exit(1);
    }
    beginMarker = WIN_BEGIN;
    endMarker = WIN_END;
    newBeginLine = `${WIN_BEGIN} (codegen-managed by scripts/codegen-statusline-snippet.ts) -->`;
    newEndMarker = WIN_END;
  }

  const beginIdx = content.indexOf(beginMarker);
  const endIdx = content.indexOf(endMarker);
  if (beginIdx === -1 || endIdx === -1) {
    process.stderr.write(
      `markers 못 찾았어요 (${variant}). SKILL.md 상태를 확인해줘요.\n`
    );
    process.exit(1);
  }

  const beforeBegin = content.slice(0, beginIdx);
  const afterEnd = content.slice(endIdx + endMarker.length);
  const newBlock = `${newBeginLine}\n\`\`\`json\n${snippet}\n\`\`\`\n${newEndMarker}`;
  return beforeBegin + newBlock + afterEnd;
}

if (import.meta.main) {
  const arg = process.argv[2];

  if (arg === "--check") {
    const content = readSkillContent();

    // Check UNIX block
    const extractedUnix = extractSnippet(content, "unix");
    if (extractedUnix === null) {
      process.stderr.write(
        "STATUSLINE_SNIPPET_UNIX markers 없어요 — SKILL.md 에 Unix snippet 블록 markers 가 있는지 확인해줘요.\n"
      );
      process.exit(1);
    }
    const canonicalUnix = getStatuslineSnippetUnix();
    if (extractedUnix !== canonicalUnix) {
      process.stderr.write("❌ Unix statusline snippet drift 감지됐어요.\n");
      process.stderr.write(`  SKILL.md 의 Unix snippet:\n${extractedUnix}\n`);
      process.stderr.write(`  canonical Unix snippet:\n${canonicalUnix}\n`);
      process.stderr.write("`bun run scripts/codegen-statusline-snippet.ts --write` 실행해줘요.\n");
      process.exit(1);
    }

    // Check WINDOWS block
    const extractedWin = extractSnippet(content, "windows");
    if (extractedWin === null) {
      process.stderr.write(
        "STATUSLINE_SNIPPET_WINDOWS markers 없어요 — Task #5 (SKILL.md Windows snippet 추가) 완료 후 통과해요.\n"
      );
      process.exit(1);
    }
    const canonicalWin = getStatuslineSnippetWindows();
    if (extractedWin !== canonicalWin) {
      process.stderr.write("❌ Windows statusline snippet drift 감지됐어요.\n");
      process.stderr.write(`  SKILL.md 의 Windows snippet:\n${extractedWin}\n`);
      process.stderr.write(`  canonical Windows snippet:\n${canonicalWin}\n`);
      process.stderr.write("`bun run scripts/codegen-statusline-snippet.ts --write` 실행해줘요.\n");
      process.exit(1);
    }

    process.stdout.write("codegen-statusline-snippet: Unix + Windows snippets in sync (no drift)\n");
    process.exit(0);

  } else if (arg === "--write") {
    let content = readSkillContent();

    // Replace UNIX block (migrates legacy → _UNIX if needed)
    content = replaceSnippet(content, "unix", getStatuslineSnippetUnix());

    // Replace WINDOWS block (requires _WINDOWS markers to exist)
    content = replaceSnippet(content, "windows", getStatuslineSnippetWindows());

    writeFileSync(SKILL_PATH, content);
    process.stdout.write("codegen-statusline-snippet: SKILL.md Unix + Windows snippets 업데이트 완료\n");
    process.exit(0);

  } else {
    process.stdout.write(
      [
        "codegen-statusline-snippet — statusline wiring snippet drift lock (Unix + Windows)",
        "",
        "사용법:",
        "  bun scripts/codegen-statusline-snippet.ts --check   drift 여부 확인 (exit 0 = OK)",
        "  bun scripts/codegen-statusline-snippet.ts --write   SKILL.md Unix + Windows snippets 최신화",
        "",
        "canonical Unix snippet:",
        getStatuslineSnippetUnix(),
        "",
        "canonical Windows snippet:",
        getStatuslineSnippetWindows(),
        "",
      ].join("\n")
    );
    process.exit(0);
  }
}
