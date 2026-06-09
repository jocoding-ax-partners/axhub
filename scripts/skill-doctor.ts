#!/usr/bin/env bun
/**
 * Phase 18 R5/US-1806 — skill:doctor.
 *
 * Iterates skills/*\/SKILL.md, prints a colored human-readable diagnostic of
 * Phase 17/18 patterns each SKILL declares vs implements. Authors run this
 * for friendly feedback; CI runs `--strict` for exit code (used by meta-test
 * tests/ux-skill-template-completeness.test.ts).
 *
 * Patterns checked (all per-SKILL frontmatter declaration → body presence):
 *   1. D1 sentinel — required only when body references AskUserQuestion
 *   2. TodoWrite Step 0 — required when frontmatter `multi-step: true`
 *   3. in-body preflight — needs-preflight:true requires in-body `axhub-helpers preflight --json`
 *      and NO load-time `!command` injection (ADR-0013, supersedes ADR-0011)
 *   4. dep-execution — requires frontmatter `allows-dependency-execution: true|false`
 *   5. model routing — Phase 25 PR 25.5a. Optional `model:` frontmatter; when declared
 *      it must be one of haiku|sonnet|opus. Missing field is allowed (no-op for the
 *      19 existing SKILLs; bulk migration happens in 25.5b/25.5c).
 *
 * Output (default mode):
 *   skills/deploy/SKILL.md:
 *     ✓ D1 sentinel
 *     ✓ TodoWrite (multi-step: true)
 *     ✓ in-body preflight (needs-preflight: true)
 *   skills/foo/SKILL.md:
 *     ❌ D1 sentinel missing
 *     ❌ TodoWrite missing (frontmatter says multi-step: true)
 *     ✓ in-body preflight (needs-preflight: false → no injection)
 *
 *   2 SKILLs scanned, 1 OK, 1 with 2 missing pattern(s).
 *
 * --strict mode: same checks, machine-parseable output, exit 1 on any miss.
 */
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { readSkillDescription } from "./codegen-skill-keywords-from-rust";
import { computeExamplesIssues, computeQualityIssues, type QualityIssue } from "./skill-doctor-quality";
import { getStatuslineSnippet } from "./codegen-statusline-snippet";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS_DIR = join(REPO_ROOT, "skills");
const SENTINEL = "Non-interactive AskUserQuestion guard (D1)";
const COLLISION_ALLOWLIST_PATH = join(REPO_ROOT, "scripts/skill-doctor-collision-allowlist.json");

interface CollisionAllowlistEntry {
  phrase: string;
  skills: string[];
  reason: string;
}
interface CollisionAllowlist {
  allowed_collisions: CollisionAllowlistEntry[];
}

function loadCollisionAllowlist(): CollisionAllowlist {
  if (!existsSync(COLLISION_ALLOWLIST_PATH)) {
    return { allowed_collisions: [] };
  }
  return JSON.parse(readFileSync(COLLISION_ALLOWLIST_PATH, "utf8")) as CollisionAllowlist;
}

function isAllowed(allowlist: CollisionAllowlist, phrase: string, skills: string[]): boolean {
  const sortedSkills = [...skills].sort();
  for (const entry of allowlist.allowed_collisions) {
    if (entry.phrase !== phrase) continue;
    const allowedSkills = [...entry.skills].sort();
    if (
      allowedSkills.length === sortedSkills.length &&
      allowedSkills.every((s, i) => s === sortedSkills[i])
    ) {
      return true;
    }
  }
  return false;
}

const DEP_EXEC_PATTERNS = {
  P1_DIRECT: /!\s*"?(?:npm|pnpm|yarn|bun)"?\s+(?:i|install|add)\b/,
  P2_SHELL_WRAP: /!\s*(?:sh|bash|eval)\s+(?:-c\s+)?"[^"]*(?:npm|pnpm|yarn|bun)\s+(?:i|install|add)/,
  P3_CHAIN: /!\s*[^;&|]*(?:&&|;|\|)\s*(?:npm|pnpm|yarn|bun)\s+(?:i|install|add)\b/,
  P4_INDIRECT: /!\s*(?:[A-Z_]+=(?:npm|pnpm|yarn|bun)\s*;?\s*\$\{?[A-Z_]+\}?\s+(?:i|install|add)\b|npx\s+--package\s+(?:npm|pnpm|yarn|bun)\b)/,
} as const;

interface AllowlistEntry {
  skill: string;
  rationale: string;
}
interface Allowlist {
  allows_dependency_execution: AllowlistEntry[];
}

const loadDependencyAllowlist = (): Allowlist => {
  try {
    const raw = readFileSync(join(REPO_ROOT, "scripts", "skill-doctor-allowlist.json"), "utf8");
    return JSON.parse(raw) as Allowlist;
  } catch {
    return { allows_dependency_execution: [] };
  }
};

const DEP_EXEC_ALLOWLIST = loadDependencyAllowlist();

// A4 — tenant-picker gated check manifest.
// Required only for skills in scripts/tenant-target-skills.json.
// Non-listed ~45 skills are exempt → day-1 redball guard.
const TENANT_TARGET_PATH = join(REPO_ROOT, "scripts/tenant-target-skills.json");

interface TenantTargetManifest {
  tenant_picker_targets: string[];
}

const loadTenantTargets = (): Set<string> => {
  try {
    const raw = readFileSync(TENANT_TARGET_PATH, "utf8");
    const manifest = JSON.parse(raw) as TenantTargetManifest;
    return new Set(manifest.tenant_picker_targets ?? []);
  } catch {
    return new Set();
  }
};

const TENANT_TARGETS = loadTenantTargets();

const VALID_MODELS = ["haiku", "sonnet", "opus"] as const;

// FU-3 — extracted to scripts/skill-doctor-step-numbering.ts so tests can import
// the helper without triggering this script's top-level immediate execution.
import { findTopLevelStepCollisions } from "./skill-doctor-step-numbering";

const STRICT = process.argv.includes("--strict");
const NO_COLOR = !process.stdout.isTTY || process.env["NO_COLOR"] || STRICT;

const c = {
  green: (s: string) => (NO_COLOR ? s : `\x1b[32m${s}\x1b[0m`),
  red: (s: string) => (NO_COLOR ? s : `\x1b[31m${s}\x1b[0m`),
  gray: (s: string) => (NO_COLOR ? s : `\x1b[90m${s}\x1b[0m`),
  bold: (s: string) => (NO_COLOR ? s : `\x1b[1m${s}\x1b[0m`),
};

const ok = c.green("✓");
const miss = c.red("❌");

interface SkillCheck {
  slug: string;
  path: string;
  patterns: { name: string; required: boolean; present: boolean; reason: string }[];
}

const inspectSkill = (slug: string): SkillCheck => {
  const path = join(SKILLS_DIR, slug, "SKILL.md");
  const content = readFileSync(path, "utf8");
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  const fm = fmMatch?.[1] ?? "";
  const multiStep = /^multi-step:\s*true\s*$/m.test(fm);
  const needsPreflight = /^needs-preflight:\s*true\s*$/m.test(fm);
  const referencesAUQ = content.includes("AskUserQuestion");

  // dep-execution rule
  const depExecFieldMatch = fm.match(/^allows-dependency-execution:\s*(true|false)\s*$/m);
  const depExecDeclared = depExecFieldMatch !== null;
  const depExecAllowed = depExecFieldMatch?.[1] === "true";

  const allowlistEntry = DEP_EXEC_ALLOWLIST.allows_dependency_execution.find((e) => e.skill === slug);

  let depExecReason: string;
  let depExecRequired: boolean;
  let depExecPresent: boolean;

  if (!depExecDeclared) {
    depExecRequired = true;
    depExecPresent = false;
    depExecReason = `frontmatter 'allows-dependency-execution: <true|false>' 필수`;
  } else if (!depExecAllowed) {
    // false — body must NOT contain any dep-exec pattern
    const bodyLines = content.split("\n");
    const matched = bodyLines.some((line) =>
      Object.values(DEP_EXEC_PATTERNS).some((re) => re.test(line))
    );
    depExecRequired = matched;
    depExecPresent = !matched;
    depExecReason = matched
      ? "allows-dependency-execution: false 이지만 body 에 dep-exec 패턴 발견"
      : "allows-dependency-execution: false → exempt";
  } else {
    // true — must be in allowlist + axhub-helpers helper pattern must NOT appear
    const inAllowlist = allowlistEntry !== undefined;
    const rationale = allowlistEntry?.rationale ?? "";
    const rationaleTooShort = rationale.length < 50;
    const hasHelperAbuse = /axhub-helpers\s+(?:install-deps|verify-install|run-install)/.test(content);

    if (!inAllowlist) {
      depExecRequired = true;
      depExecPresent = false;
      depExecReason = `allows-dependency-execution: true 이지만 allowlist 에 없는 SKILL`;
    } else if (rationaleTooShort) {
      depExecRequired = true;
      depExecPresent = false;
      depExecReason = `allowlist rationale 가 50자 미만 (현재 ${rationale.length}자)`;
    } else if (hasHelperAbuse) {
      depExecRequired = true;
      depExecPresent = false;
      depExecReason = "axhub-helpers install-deps/verify-install/run-install 금지 패턴 발견";
    } else {
      depExecRequired = false;
      depExecPresent = true;
      depExecReason = "allows-dependency-execution: true + allowlist 검증 통과";
    }
  }

  // Phase 25 PR 25.5a — model routing field. Optional today; required to be a
  // valid value only when declared. Existing 19 SKILLs lack the field and must
  // continue to pass (no-op effective). Bulk migration arrives in 25.5b/25.5c.
  const modelFieldMatch = fm.match(/^model:\s*([a-z]+)\s*$/m);
  const modelDeclared = modelFieldMatch !== null;
  const modelValue = modelFieldMatch?.[1] ?? null;
  const modelValid = modelValue !== null && (VALID_MODELS as readonly string[]).includes(modelValue);
  let modelRequired: boolean;
  let modelPresent: boolean;
  let modelReason: string;
  if (!modelDeclared) {
    modelRequired = false;
    modelPresent = true;
    modelReason = "frontmatter 'model' 미선언 → exempt (Phase 25 PR 25.5b/25.5c 까지 일괄 미적용 OK)";
  } else if (modelValid) {
    modelRequired = true;
    modelPresent = true;
    modelReason = `frontmatter model: ${modelValue}`;
  } else {
    modelRequired = true;
    modelPresent = false;
    modelReason = `frontmatter model: ${modelValue ?? "(unparsed)"} — must be one of ${VALID_MODELS.join("|")}`;
  }

  return {
    slug,
    path: `skills/${slug}/SKILL.md`,
    patterns: [
      {
        name: "D1 sentinel",
        required: referencesAUQ,
        present: content.includes(SENTINEL),
        reason: referencesAUQ ? "body references AskUserQuestion" : "body has no AskUserQuestion → exempt",
      },
      {
        name: "TodoWrite Step 0",
        required: multiStep,
        present: content.includes("TodoWrite({ todos: ["),
        reason: multiStep ? "frontmatter multi-step: true" : "frontmatter multi-step: false → exempt",
      },
      ((): { name: string; required: boolean; present: boolean; reason: string } => {
        // ADR-0013 (supersedes ADR-0011): needs-preflight:true means the SKILL runs
        // `axhub-helpers preflight --json` as an in-body bash step, NOT a load-time
        // `!command` injection. The injection hard-failed on first run (raw English
        // "requires approval") because Claude Code permission-gates the outer `node -e`
        // wrapper itself — and the inner denialRegex fallback could never catch its own
        // denial (dead path). Invariant: NO skill may carry the dead `!`node -e
        // ...preflight...`` injection; a needs-preflight:true skill MUST invoke preflight
        // in its body so the call goes through the standard interactive Bash permission flow.
        const hasInjection = /^!`node -e "[^\n]*axhub-helpers[^\n]*preflight[^\n]*"`$/m.test(content);
        // Require the canonical block's assignment signature, NOT a bare `preflight --json`
        // mention. A loose `/preflight\s+--json/` would false-pass any skill that references
        // preflight in a later/legacy step (e.g. deploy's command-reference at deploy:388)
        // even if its upfront canonical block were deleted. The `PREFLIGHT_JSON=$("$HELPER" ...`
        // form is unique to the canonical block and also enforces the helper-pick fallback.
        const invokesPreflight = /PREFLIGHT_JSON=\$\("\$HELPER" preflight --json/.test(content);
        const present = needsPreflight ? !hasInjection && invokesPreflight : !hasInjection;
        return {
          name: "in-body preflight",
          required: true,
          present,
          reason: needsPreflight
            ? "needs-preflight: true — body must invoke `axhub-helpers preflight --json` and carry NO `!command` injection"
            : "needs-preflight: false → only checks NO `!command` injection remains",
        };
      })(),
      {
        name: "dep-execution",
        required: depExecRequired,
        present: depExecPresent,
        reason: depExecReason,
      },
      {
        name: "model routing",
        required: modelRequired,
        present: modelPresent,
        reason: modelReason,
      },
      ((): { name: string; required: boolean; present: boolean; reason: string } => {
        const collisions = findTopLevelStepCollisions(content);
        if (collisions.length === 0) {
          return {
            name: "step numbering",
            required: true,
            present: true,
            reason: "no top-level step number collisions in ## Workflow",
          };
        }
        return {
          name: "step numbering",
          required: true,
          present: false,
          reason: `duplicate top-level step numbers in ## Workflow: ${collisions.join(", ")} (FU-3 — F′ regression guard; sub-steps like 3.5 and ### subsection numbering are exempt)`,
        };
      })(),
      // A4 — tenant-picker L1 gated check (required only for manifest-listed skills)
      {
        name: "tenant-picker L1",
        required: TENANT_TARGETS.has(slug),
        present: /axhub-tenant-picker:L1/.test(content) && /\.axhub\/state\/tenant\.json/.test(content),
        reason: TENANT_TARGETS.has(slug)
          ? "tenant-target-skills.json 등재 skill — L1 sentinel(axhub-tenant-picker:L1) + .axhub/state/tenant.json marker 필수"
          : "tenant-target-skills.json 미등재 → exempt",
      },
      // A4 — tenant-picker L2 gated check (required only for manifest-listed skills)
      {
        name: "tenant-picker L2",
        required: TENANT_TARGETS.has(slug),
        present: /axhub-tenant-picker:L2/.test(content) && /\.axhub\/state\/tenant\.json/.test(content),
        reason: TENANT_TARGETS.has(slug)
          ? "tenant-target-skills.json 등재 skill — L2 sentinel(axhub-tenant-picker:L2) + .axhub/state/tenant.json write-back marker 필수"
          : "tenant-target-skills.json 미등재 → exempt",
      },
    ],
  };
};

const skillSlugs = readdirSync(SKILLS_DIR).filter((d) => {
  try {
    readFileSync(join(SKILLS_DIR, d, "SKILL.md"), "utf8");
    return true;
  } catch {
    return false;
  }
}).sort();

const checks = skillSlugs.map(inspectSkill);

let totalMissing = 0;
let okSkills = 0;

for (const check of checks) {
  const missing = check.patterns.filter((p) => p.required && !p.present);
  totalMissing += missing.length;
  if (missing.length === 0) okSkills++;

  if (STRICT) {
    for (const p of missing) {
      process.stdout.write(`${check.path}: missing ${p.name}\n`);
    }
  } else {
    process.stdout.write(`${c.bold(check.path)}:\n`);
    for (const p of check.patterns) {
      if (!p.required) {
        process.stdout.write(`  ${c.gray("○")} ${c.gray(p.name + " — " + p.reason)}\n`);
      } else if (p.present) {
        process.stdout.write(`  ${ok} ${p.name} ${c.gray("(" + p.reason + ")")}\n`);
      } else {
        process.stdout.write(`  ${miss} ${c.red(p.name + " missing")} ${c.gray("(" + p.reason + ")")}\n`);
      }
    }
    process.stdout.write("\n");
  }
}

// Phase 1 sub-task 1.3: collision lint — two skills sharing a description trigger phrase.
const allowlist = loadCollisionAllowlist();
const phraseToSkills = new Map<string, Set<string>>();
for (const slug of skillSlugs) {
  const path = join(SKILLS_DIR, slug, "SKILL.md");
  const region = readSkillDescription(path);
  if (!region) continue;
  for (const phrase of region.existingPhrases) {
    let set = phraseToSkills.get(phrase);
    if (!set) {
      set = new Set();
      phraseToSkills.set(phrase, set);
    }
    set.add(slug);
  }
}

let collisionCount = 0;
const collisions: Array<{ phrase: string; skills: string[]; allowed: boolean; reason?: string }> = [];
for (const [phrase, slugs] of phraseToSkills) {
  if (slugs.size < 2) continue;
  const skills = [...slugs].sort();
  const allowed = isAllowed(allowlist, phrase, skills);
  const reason = allowed
    ? allowlist.allowed_collisions.find(
        (e) =>
          e.phrase === phrase &&
          [...e.skills].sort().every((s, i) => s === skills[i]),
      )?.reason
    : undefined;
  collisions.push({ phrase, skills, allowed, reason });
  if (!allowed) collisionCount += 1;
}

if (STRICT) {
  for (const col of collisions) {
    if (!col.allowed) {
      process.stdout.write(
        `collision: phrase "${col.phrase}" appears in skills [${col.skills.join(", ")}] — not in allowlist\n`,
      );
    }
  }
} else {
  if (collisions.length > 0) {
    process.stdout.write(`${c.bold("Description trigger phrase collisions:")}\n`);
    for (const col of collisions) {
      const icon = col.allowed ? c.green("●") : c.red("✗");
      const tail = col.allowed
        ? c.gray(`(allowlisted: ${col.reason ?? ""})`)
        : c.red("(NOT allowlisted)");
      process.stdout.write(
        `  ${icon} "${col.phrase}" → [${col.skills.join(", ")}] ${tail}\n`,
      );
    }
    process.stdout.write("\n");
  }
}

// Phase 8 sub-task 8.2: description quality lint — minimum trigger count + lang balance.
//   - 각 SKILL 의 trigger phrase ≥ 5
//   - 각 SKILL 의 ko phrase ≥ 2 + en phrase ≥ 2
// 통과 못 하면 STRICT 시 exit 1.

const qualityIssues: QualityIssue[] = [];
for (const slug of skillSlugs) {
  const path = join(SKILLS_DIR, slug, "SKILL.md");
  const region = readSkillDescription(path);
  if (!region) continue;
  qualityIssues.push(...computeQualityIssues(slug, region.existingPhrases));
  // Phase 9 — frontmatter examples field validation.
  const content = readFileSync(path, "utf8");
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  const fm = fmMatch?.[1] ?? "";
  qualityIssues.push(...computeExamplesIssues(slug, fm));
}

if (STRICT) {
  for (const issue of qualityIssues) {
    process.stdout.write(`quality: ${issue.slug} ${issue.kind} (${issue.detail})\n`);
  }
} else if (qualityIssues.length > 0) {
  process.stdout.write(`${c.bold("Description quality gate:")}\n`);
  for (const issue of qualityIssues) {
    process.stdout.write(`  ${miss} ${issue.slug}: ${issue.kind} ${c.gray("(" + issue.detail + ")")}\n`);
  }
  process.stdout.write("\n");
}

if (!STRICT) {
  const pluralS = checks.length === 1 ? "" : "s";
  const pluralM = totalMissing === 1 ? "" : "s";
  const pluralC = collisionCount === 1 ? "" : "s";
  const pluralQ = qualityIssues.length === 1 ? "" : "s";
  process.stdout.write(
    `${checks.length} SKILL${pluralS} scanned, ${okSkills} OK, ${totalMissing} missing pattern${pluralM}, ${collisionCount} unallowed collision${pluralC}, ${qualityIssues.length} quality issue${pluralQ}.\n`,
  );
}

// Phase 26 — statusline snippet codegen drift check (--strict only).
// Skips silently when skills/enable-statusline/SKILL.md does not yet exist
// (bootstrapping order: SKILL may not be authored before this check runs).
let statuslineSnippetDrift = false;
{
  const enableStatuslinePath = join(SKILLS_DIR, "enable-statusline", "SKILL.md");
  if (existsSync(enableStatuslinePath)) {
    const content = readFileSync(enableStatuslinePath, "utf8");
    const BEGIN_MARKER = "<!-- BEGIN STATUSLINE_SNIPPET";
    const END_MARKER = "<!-- END STATUSLINE_SNIPPET -->";
    const beginIdx = content.indexOf(BEGIN_MARKER);
    const endIdx = content.indexOf(END_MARKER);
    if (beginIdx !== -1 && endIdx !== -1) {
      const between = content.slice(beginIdx, endIdx);
      const fenceMatch = between.match(/```json\n([\s\S]*?)\n```/);
      const extracted = fenceMatch?.[1] ?? null;
      const canonical = getStatuslineSnippet();
      if (extracted !== canonical) {
        statuslineSnippetDrift = true;
        if (STRICT) {
          process.stdout.write(
            "statusline-snippet: drift detected — run `bun run scripts/codegen-statusline-snippet.ts --write`\n"
          );
        } else {
          process.stdout.write(
            `  ${miss} ${c.red("statusline snippet drift")} ${c.gray("(bun run scripts/codegen-statusline-snippet.ts --write 실행해주세요)")}\n`
          );
        }
      } else {
        if (!STRICT) {
          process.stdout.write(
            `  ${ok} statusline snippet ${c.gray("(codegen-statusline-snippet.ts 와 byte-identical)")}\n`
          );
        }
      }
    }
    // If markers absent: skip silently (SKILL body not yet fully authored)
  }
  // If SKILL does not exist: skip silently
}

process.exit(
  (totalMissing > 0 || collisionCount > 0 || qualityIssues.length > 0 || statuslineSnippetDrift) && STRICT ? 1 : 0,
);
