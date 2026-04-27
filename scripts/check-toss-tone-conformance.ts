#!/usr/bin/env bun
/**
 * Toss UX Writing tone conformance lint.
 *
 * Scans Phase 13 file scope (runtime + commands + install/hook) for
 * forbidden Korean tokens that violate Toss UX Writing rules
 * (https://developers-apps-in-toss.toss.im/design/ux-writing.html).
 *
 * Modes:
 *   default          warn-only — print violations, exit 0
 *   --strict         exit 1 on any violation (PR2 commit gate)
 *   --baseline N     warn-only with baseline N tolerated, exit 1 if exceeded
 *   --json           emit structured violations for CI consumption
 *
 * File scope (Phase 13 v2):
 *   - src/axhub-helpers/catalog.ts (Tier A — exit-code Korean templates)
 *   - src/axhub-helpers/keychain.ts (Tier A — 4-part errors)
 *   - src/axhub-helpers/keychain-windows.ts (Tier A — 5 Windows errors)
 *   - src/axhub-helpers/list-deployments.ts (Tier A — auth/parse errors)
 *   - src/axhub-helpers/index.ts (Tier A — USAGE + cmdSessionStart strings)
 *   - commands/*.md (Tier C — 9 slash commands)
 *   - bin/install.sh + bin/install.ps1 (Tier D — installer Korean)
 *   - hooks/session-start.sh + hooks/session-start.ps1 (Tier D — hook Korean)
 *
 * Phase 14 (docs + SKILL workflow) and Phase 15 (SKILL descriptions) are
 * EXCLUDED until those phases ship — running lint on them now would create
 * false positives.
 *
 * Forbidden tokens map to Toss rule IDs (per plan v2 §1):
 *   T-01: 합니다 / 입니다 (해요체 의무 — verbatim Toss rule)
 *   T-06: 시겠어요 (과도한 경어 제거 — verbatim)
 *   T-06: 시나요 (3 exceptions — checked separately, warn-only)
 *   T-06: 드립니다 (과도한 경어 — extension)
 *   T-09 (axhub): 당신 (호칭 회피 — axhub-specific extension, NOT Toss-mandated)
 *   axhub deprecation: 아이고 (Phase 13 US-1302 deprecates this emotion prefix)
 */

import { readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { glob } from "node:fs/promises";

const REPO_ROOT = join(import.meta.dir, "..");

interface ForbiddenToken {
  pattern: RegExp;
  rule: string;
  reason: string;
  severity: "error" | "warn";
}

const FORBIDDEN: ForbiddenToken[] = [
  { pattern: /합니다/g, rule: "T-01", reason: "해요체 의무 (Toss verbatim) — use 해요/예요/이에요", severity: "error" },
  { pattern: /입니다(?!\s*\?)/g, rule: "T-01", reason: "해요체 의무 (Toss verbatim) — use 이에요/예요", severity: "error" },
  { pattern: /시겠어요/g, rule: "T-06", reason: "과도한 경어 (Toss verbatim) — use ~할래요/~할까요", severity: "error" },
  { pattern: /드립니다/g, rule: "T-06", reason: "과도한 경어 — use ~줄게요/~할게요", severity: "error" },
  { pattern: /당신(?![에을])/g, rule: "T-09 (axhub)", reason: "직접 호칭 회피 (axhub extension) — drop or use 사용자/회원", severity: "error" },
  { pattern: /아이고/g, rule: "axhub-deprecation", reason: "Phase 13 deprecates 아이고 emotion prefix — use 잠깐만요/이상해요", severity: "error" },
  // T-06 시나요? has 3 exceptions (사용자 맥락 활용 / 상황 추정 / 선의) — warn-only
  // because text scan cannot distinguish exception cases reliably.
  { pattern: /시나요/g, rule: "T-06 (3 exceptions)", reason: "과도한 경어 (Toss has 3 exception cases — manual review)", severity: "warn" },
];

const PHASE_13_FILES = async (): Promise<string[]> => {
  const explicit = [
    "src/axhub-helpers/catalog.ts",
    "src/axhub-helpers/keychain.ts",
    "src/axhub-helpers/keychain-windows.ts",
    "src/axhub-helpers/list-deployments.ts",
    "src/axhub-helpers/index.ts",
    "bin/install.sh",
    "bin/install.ps1",
    "hooks/session-start.sh",
    "hooks/session-start.ps1",
  ];
  const commandFiles: string[] = [];
  for await (const f of glob("commands/*.md", { cwd: REPO_ROOT })) {
    commandFiles.push(f);
  }
  const all = [...explicit, ...commandFiles].map((f) => join(REPO_ROOT, f));
  return all.filter((f) => existsSync(f));
};

interface Violation {
  file: string;
  line: number;
  col: number;
  match: string;
  rule: string;
  reason: string;
  severity: "error" | "warn";
  context: string;
}

const scan = async (): Promise<Violation[]> => {
  const files = await PHASE_13_FILES();
  const out: Violation[] = [];
  for (const file of files) {
    const content = readFileSync(file, "utf8");
    const lines = content.split("\n");
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (!line) continue;
      for (const token of FORBIDDEN) {
        token.pattern.lastIndex = 0;
        let m: RegExpExecArray | null;
        while ((m = token.pattern.exec(line)) !== null) {
          out.push({
            file: file.replace(REPO_ROOT + "/", ""),
            line: i + 1,
            col: m.index + 1,
            match: m[0],
            rule: token.rule,
            reason: token.reason,
            severity: token.severity,
            context: line.trim().slice(0, 120),
          });
          if (m.index === token.pattern.lastIndex) token.pattern.lastIndex++;
        }
      }
    }
  }
  return out;
};

const main = async (): Promise<number> => {
  const args = process.argv.slice(2);
  const strict = args.includes("--strict");
  const wantJson = args.includes("--json");
  const baselineFlag = args.find((a) => a.startsWith("--baseline"));
  const baseline = baselineFlag ? parseInt(baselineFlag.split("=")[1] ?? args[args.indexOf(baselineFlag) + 1] ?? "0", 10) : null;

  const violations = await scan();
  const errors = violations.filter((v) => v.severity === "error");
  const warns = violations.filter((v) => v.severity === "warn");

  if (wantJson) {
    process.stdout.write(JSON.stringify({ errors, warns, total: violations.length }, null, 2) + "\n");
  } else {
    for (const v of violations) {
      const tag = v.severity === "error" ? "ERROR" : "WARN ";
      process.stdout.write(`${tag} ${v.file}:${v.line}:${v.col}  [${v.rule}]  ${v.match}  — ${v.reason}\n      ${v.context}\n`);
    }
    process.stdout.write(`\n${errors.length} error(s), ${warns.length} warning(s) across ${(await PHASE_13_FILES()).length} file(s)\n`);
  }

  if (strict && errors.length > 0) return 1;
  if (baseline !== null && errors.length > baseline) {
    process.stderr.write(`Baseline ${baseline} exceeded — ${errors.length} errors found.\n`);
    return 1;
  }
  return 0;
};

if (import.meta.main) {
  main().then((code) => process.exit(code));
}

export { scan, FORBIDDEN, PHASE_13_FILES };
export type { Violation };
