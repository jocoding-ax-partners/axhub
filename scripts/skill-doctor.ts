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
 *   3. !command preflight injection — required when frontmatter `needs-preflight: true`
 *
 * Output (default mode):
 *   skills/deploy/SKILL.md:
 *     ✓ D1 sentinel
 *     ✓ TodoWrite (multi-step: true)
 *     ✓ !command preflight (needs-preflight: true)
 *   skills/foo/SKILL.md:
 *     ❌ D1 sentinel missing
 *     ❌ TodoWrite missing (frontmatter says multi-step: true)
 *     ✓ !command preflight (needs-preflight: false → exempt)
 *
 *   2 SKILLs scanned, 1 OK, 1 with 2 missing pattern(s).
 *
 * --strict mode: same checks, machine-parseable output, exit 1 on any miss.
 */
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS_DIR = join(REPO_ROOT, "skills");
const SENTINEL = "Non-interactive AskUserQuestion guard (D1)";

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
      {
        name: "!command preflight",
        required: needsPreflight,
        present: content.includes("axhub-helpers preflight --json"),
        reason: needsPreflight ? "frontmatter needs-preflight: true" : "frontmatter needs-preflight: false → exempt",
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

if (!STRICT) {
  const pluralS = checks.length === 1 ? "" : "s";
  const pluralM = totalMissing === 1 ? "" : "s";
  process.stdout.write(
    `${checks.length} SKILL${pluralS} scanned, ${okSkills} OK, ${totalMissing} missing pattern${pluralM}.\n`
  );
}

process.exit(totalMissing > 0 && STRICT ? 1 : 0);
