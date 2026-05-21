#!/usr/bin/env bun
/**
 * SKILL `!command` preflight injection codegen — single source for the Node runner line
 * applied to every needs-preflight SKILL + 1 template (lite variant 15 곳 + deploy variant 1 곳).
 *
 * In iteration 4 of `.omc/plans/preflight-permission-ux-fix.md`, the lite variant and
 * deploy variant share the same `!`...`` body — the only deploy-specific concern is the
 * PowerShell `$env:PATH` setup prose block at deploy:85-95 (preserved as separate prose,
 * NOT codegen-managed).
 *
 * The injected Node runner captures inner stderr via `stdio:['inherit','inherit','pipe']`,
 * matches Claude Code's permission denial via a strict-anchor regex, emits a Korean
 * `systemMessage` JSON on match (exit 0), and passes through unrecognized stderr to the
 * parent process otherwise — preserving ADR-0010 §42 "raw stderr 가 chat 으로 흘러요"
 * graceful degradation.
 *
 * See: docs/adr/0011-skill-preflight-permission-fallback.md
 * Plan: .omc/plans/preflight-permission-ux-fix.md (iter4 §4 Step 1)
 *
 * Idempotent: re-running with no drift is a no-op.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

const SYSTEM_MESSAGE =
  "[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)";

/**
 * Builds the single-line Node runner used as the `!command` injection body for
 * **lite-variant** SKILLs (14 SKILL + 1 template).
 *
 * The shell sees `node -e "<script>"` with double-quoted JS so that
 * `${CLAUDE_PLUGIN_ROOT}` expands at the shell layer (same mechanism as the
 * pre-iteration-4 raw `!`${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json``
 * pattern). Inside the JS string, single quotes wrap literals — no shell escapes,
 * no backticks, no `$`-prefixed JS templates that could collide with shell expansion.
 *
 * Backward-compat alias `getPreflightInjectionLine()` is preserved for the
 * skill-doctor + skill-new + test importers that pre-date the deploy variant split.
 */
export function getLiteInjectionLine(): string {
  const script = [
    "const cp=require('child_process');",
    "const env={...process.env};",
    "const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';",
    "const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});",
    "const stderrText=String(result.stderr??'');",
    "const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;",
    // PR #99 security M2: redact common secret token patterns from stderr passthrough.
    // Prevents accidental leak when helper emits RUST_LOG=debug, dependency panic, or
    // transport debug output containing API keys / OAuth tokens to the Claude Code chat surface.
    "const redactRe=/(sk-[A-Za-z0-9_-]{20,}|github_pat_[A-Za-z0-9_]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\\\s+[A-Za-z0-9._~+\\\\/-]+=*)/g;",
    `if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\\"${SYSTEM_MESSAGE}\\"}));process.exit(0)}`,
    "else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}",
    "process.exit(typeof result.status==='number'?result.status:0)",
  ].join("");
  return "!`node -e \"" + script + "\"`";
}

/** Backward-compat alias. Prefer `getLiteInjectionLine()` in new code. */
export const getPreflightInjectionLine = getLiteInjectionLine;

/**
 * Deploy-variant Node runner — preserves the Phase 17 US-1706 cross-platform
 * root-resolution logic (Windows `.exe`, `path.delimiter` PATH splitting,
 * `CLAUDE_SKILL_DIR` fallback, `bin/` cwd fallback) while adding the iteration-4
 * stderr-pipe capture + strict-anchor denialRegex fallback + unrecognized stderr
 * passthrough. Used only by `skills/deploy/SKILL.md` — the only SKILL that runs
 * without a preceding shell `if` / PowerShell `if` setup prose block that
 * sets `CLAUDE_PLUGIN_ROOT` in the user shell.
 */
export function getDeployInjectionLine(): string {
  const script = [
    "const fs=require('fs'),path=require('path'),cp=require('child_process'),isWin=process.platform==='win32';",
    "let root=process.env.CLAUDE_PLUGIN_ROOT||'';",
    "const env=Object.assign({},process.env);",
    "let pathKey='PATH';",
    "for(const key of Object.keys(env)){if(key.toLowerCase()==='path'){pathKey=key;break;}}",
    "if(root.length===0&&process.env.CLAUDE_SKILL_DIR){const candidate=path.resolve(process.env.CLAUDE_SKILL_DIR,'..','..');if(fs.existsSync(candidate))root=candidate;}",
    "if(root.length===0){const helperName=isWin?'axhub-helpers.exe':'axhub-helpers';for(const dir of (env[pathKey]||'').split(path.delimiter)){const helperPath=path.join(dir,helperName);if(fs.existsSync(helperPath)){root=path.resolve(dir,'..');break;}}}",
    "if(root.length===0&&fs.existsSync(path.resolve('bin',isWin?'axhub-helpers.exe':'axhub-helpers')))root=process.cwd();",
    "if(root.length>0){env.CLAUDE_PLUGIN_ROOT=root;env[pathKey]=path.join(root,'bin')+path.delimiter+(env[pathKey]||'');}",
    "const helper=root.length>0?path.join(root,'bin',isWin?'axhub-helpers.exe':'axhub-helpers'):(isWin?'axhub-helpers.exe':'axhub-helpers');",
    "const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});",
    "const stderrText=String(result.stderr??'');",
    "const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;",
    // PR #99 security M2: same redaction as lite variant — secret token leak prevention.
    "const redactRe=/(sk-[A-Za-z0-9_-]{20,}|github_pat_[A-Za-z0-9_]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\\\s+[A-Za-z0-9._~+\\\\/-]+=*)/g;",
    `if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\\"${SYSTEM_MESSAGE}\\"}));process.exit(0)}`,
    "else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}",
    "process.exit(typeof result.status==='number'?result.status:0)",
  ].join("");
  return "!`node -e \"" + script + "\"`";
}

/** Returns the codegen line for a target's variant. */
export function getInjectionLineForVariant(variant: "lite" | "deploy"): string {
  return variant === "deploy" ? getDeployInjectionLine() : getLiteInjectionLine();
}

export interface PreflightTarget {
  /** Path relative to repo root. */
  file: string;
  /** Variant taxonomy from plan §4 Step 1 ASCII box. */
  variant: "lite" | "deploy";
}

/**
 * 15 SKILL + 1 template (16 targets total).
 *
 * `deploy` keeps the lite body too — its uniqueness is the PowerShell `$env:PATH`
 * setup prose block at deploy:85-95 which sits ABOVE the `!command` line and stays
 * outside codegen scope.
 */
export const TARGETS: PreflightTarget[] = [
  { file: "skills/axhub-debug/SKILL.md", variant: "lite" },
  { file: "skills/axhub-diagnose/SKILL.md", variant: "lite" },
  { file: "skills/axhub-plan/SKILL.md", variant: "lite" },
  { file: "skills/axhub-review/SKILL.md", variant: "lite" },
  { file: "skills/axhub-ship/SKILL.md", variant: "lite" },
  { file: "skills/axhub-tdd/SKILL.md", variant: "lite" },
  { file: "skills/apps/SKILL.md", variant: "lite" },
  { file: "skills/env/SKILL.md", variant: "lite" },
  { file: "skills/github/SKILL.md", variant: "lite" },
  { file: "skills/recover/SKILL.md", variant: "lite" },
  { file: "skills/routing-stats/SKILL.md", variant: "lite" },
  { file: "skills/trace/SKILL.md", variant: "lite" },
  { file: "skills/verify/SKILL.md", variant: "lite" },
  { file: "skills/deploy/SKILL.md", variant: "deploy" },
  { file: "skills/_template/SKILL.md.tmpl", variant: "lite" },
];

/**
 * Matches the legacy raw shell substitution form:
 *   `!\`${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json\``
 */
const OLD_RAW_RE = /^!`\$\{CLAUDE_PLUGIN_ROOT\}\/bin\/axhub-helpers preflight --json`$/m;

/**
 * Matches any single-line `!`node -e "...axhub-helpers...preflight..."`` Node runner.
 * Covers the deploy:101 pre-iteration-4 cross-shell runner (which contained extensive
 * platform-detection logic) and any iteration-4 lite Node runner produced by this codegen.
 *
 * Pattern keeps it line-anchored to avoid eating multiple `!command` blocks if a SKILL
 * ever stacks them.
 */
const OLD_NODE_RUNNER_RE = /^!`node -e ".*axhub-helpers.*preflight.*"`$/m;

export interface ApplyResult {
  file: string;
  variant: "lite" | "deploy";
  changed: boolean;
  before: string | null;
}

export function applyToFile(target: PreflightTarget): ApplyResult {
  const fullPath = join(REPO_ROOT, target.file);
  const content = readFileSync(fullPath, "utf8");
  const newLine = getInjectionLineForVariant(target.variant);

  const rawMatch = content.match(OLD_RAW_RE);
  const nodeMatch = content.match(OLD_NODE_RUNNER_RE);

  // PR #99 review correctness M2: refuse partial-migration state where both raw shell
  // substitution AND Node runner pattern exist in the same file. Either an
  // in-progress migration was interrupted or a second `!command` block was added.
  // Silent first-match replace would corrupt the byte-identical lock invariant.
  if (rawMatch && nodeMatch) {
    throw new Error(
      `${target.file}: both raw and Node-runner injection patterns matched — partial migration / drift state. Manually resolve and re-run codegen.`
    );
  }

  // PR #99 review correctness M1: also refuse multi-match within a single pattern.
  // Greedy `.*` line-anchored regex matches one block per file by design; >1 means
  // the SKILL stacks multiple `!command` injections (e.g., for two preflight subcommands)
  // and codegen single-source semantics cannot pick which one to replace.
  const activeRe = rawMatch ? OLD_RAW_RE : nodeMatch ? OLD_NODE_RUNNER_RE : null;
  if (activeRe) {
    const allMatches = [...content.matchAll(new RegExp(activeRe.source, "gm"))];
    if (allMatches.length > 1) {
      throw new Error(
        `${target.file}: ${allMatches.length} preflight \`!command\` blocks found — codegen single-source cannot disambiguate; refactor SKILL to single block.`
      );
    }
  }

  const oldLine = rawMatch?.[0] ?? nodeMatch?.[0] ?? null;
  if (oldLine === null) {
    throw new Error(
      `${target.file}: no preflight \`!command\` injection found — expected raw \`!\`\${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json\`\` or a Node runner equivalent`
    );
  }
  if (oldLine === newLine) {
    return { file: target.file, variant: target.variant, changed: false, before: oldLine };
  }
  const re = rawMatch ? OLD_RAW_RE : OLD_NODE_RUNNER_RE;
  const updated = content.replace(re, newLine);
  writeFileSync(fullPath, updated);
  return { file: target.file, variant: target.variant, changed: true, before: oldLine };
}

export function applyAll(): ApplyResult[] {
  return TARGETS.map(applyToFile);
}

if (import.meta.main) {
  const results = applyAll();
  const changed = results.filter((r) => r.changed);
  if (changed.length === 0) {
    process.stdout.write(
      `codegen-preflight-injection: all ${results.length} targets in sync (no change)\n`
    );
  } else {
    process.stdout.write(
      `codegen-preflight-injection: ${changed.length}/${results.length} targets updated\n`
    );
    for (const r of changed) {
      process.stdout.write(`  · ${r.file} (${r.variant})\n`);
    }
  }
}
