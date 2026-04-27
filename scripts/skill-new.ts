#!/usr/bin/env bun
/**
 * Phase 18 R2/US-1803 — `bun run skill:new <slug> [flags]` scaffold.
 *
 * Generates skills/<slug>/SKILL.md from skills/_template/SKILL.md.tmpl with
 * Phase 17/18 patterns pre-populated. Author fills in TODO placeholders + runs
 * `bun run skill:doctor` to verify.
 *
 * Defaults (R2 mutate-aware):
 *   --multi-step    (true)   skills with workflow ≥4 numbered steps
 *   --needs-preflight (true)  skills that mutate state OR need live context
 *
 * Flags:
 *   --no-multi-step       declare frontmatter `multi-step: false`
 *   --no-preflight        declare frontmatter `needs-preflight: false` + omit !command line
 *   --action <verb>       verb for "To <verb>:" (default: "do something")
 *   --title <text>        H1 title (default: SLUG capitalized)
 *
 * Side effects:
 *   1. mkdir skills/<slug>
 *   2. write skills/<slug>/SKILL.md from template substitution
 *   3. append empty stub to tests/fixtures/ask-defaults/registry.json:
 *        "<slug>": { "_note": "TODO: add per-question safe_default + rationale" }
 *
 * Validation: rejects literal `TODO`/`{{` left in description after substitution.
 * Run `bun run skill:doctor` to see what patterns the new SKILL is missing.
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const TEMPLATE = join(REPO_ROOT, "skills/_template/SKILL.md.tmpl");
const REGISTRY = join(REPO_ROOT, "tests/fixtures/ask-defaults/registry.json");

const argv = process.argv.slice(2);
const slug = argv[0];

if (!slug || slug.startsWith("-") || !/^[a-z][a-z0-9-]*$/.test(slug)) {
  process.stderr.write(`usage: bun run skill:new <slug> [--no-multi-step] [--no-preflight] [--action <verb>] [--title <text>]\n`);
  process.stderr.write(`  slug must be lowercase alphanumeric + hyphens (e.g. "my-skill")\n`);
  process.exit(1);
}

const flag = (name: string): boolean => argv.includes(name);
const flagValue = (name: string): string | undefined => {
  const idx = argv.indexOf(name);
  return idx >= 0 && idx + 1 < argv.length ? argv[idx + 1] : undefined;
};

const multiStep = !flag("--no-multi-step");
const needsPreflight = !flag("--no-preflight");
const action = flagValue("--action") ?? "do something";
const title = flagValue("--title") ?? slug.replace(/-/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());

const targetDir = join(REPO_ROOT, "skills", slug);
const targetFile = join(targetDir, "SKILL.md");

if (existsSync(targetFile)) {
  process.stderr.write(`error: skills/${slug}/SKILL.md already exists\n`);
  process.exit(1);
}
if (!existsSync(TEMPLATE)) {
  process.stderr.write(`error: template missing at ${TEMPLATE}\n`);
  process.exit(1);
}

mkdirSync(targetDir, { recursive: true });

let content = readFileSync(TEMPLATE, "utf8");
content = content.replace(/\{\{SLUG\}\}/g, slug);
content = content.replace(/\{\{TITLE\}\}/g, title);
content = content.replace(/\{\{ACTION\}\}/g, action);
content = content.replace(/\{\{MULTI_STEP\}\}/g, multiStep ? "true" : "false");
content = content.replace(/\{\{NEEDS_PREFLIGHT\}\}/g, needsPreflight ? "true" : "false");

const todoWriteBlock = multiStep
  ? `0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   \`\`\`typescript
   TodoWrite({ todos: [
     { content: "TODO 단계 1",      status: "in_progress", activeForm: "TODO 진행 중" },
     { content: "TODO 단계 2",      status: "pending",     activeForm: "TODO 진행 중" },
     { content: "TODO 단계 3",      status: "pending",     activeForm: "TODO 진행 중" },
     { content: "결과 안내",        status: "pending",     activeForm: "마무리하는 중" }
   ]})
   \`\`\``
  : "";

const d1GuardBlock = `**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. \`if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]\` 인 subprocess (\`claude -p\`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 \`tests/fixtures/ask-defaults/registry.json\` 참조 — TODO 질문 별 safe_default.`;

content = content.replace(/\{\{TODOWRITE_BLOCK\}\}/g, todoWriteBlock);
content = content.replace(/\{\{D1_GUARD_BLOCK\}\}/g, d1GuardBlock);

if (!needsPreflight) {
  content = content.replace(/!`\$\{CLAUDE_PLUGIN_ROOT\}\/bin\/axhub-helpers preflight --json`/, "");
  content = content.replace(/이 줄은 Claude Code SKILL preprocessing.*?주입돼요\.\n\n/, "");
}

writeFileSync(targetFile, content);

const registry: Record<string, unknown> = JSON.parse(readFileSync(REGISTRY, "utf8"));
if (!registry[slug]) {
  registry[slug] = {
    _note: "TODO: add per-question safe_default + rationale entries as AskUserQuestion call sites are added to skills/" + slug + "/SKILL.md",
  };
  writeFileSync(REGISTRY, JSON.stringify(registry, null, 2) + "\n");
}

process.stdout.write(`✓ Created skills/${slug}/SKILL.md\n`);
process.stdout.write(`  multi-step: ${multiStep}, needs-preflight: ${needsPreflight}\n`);
process.stdout.write(`✓ Appended registry stub to tests/fixtures/ask-defaults/registry.json\n`);
process.stdout.write(`\nNext steps:\n`);
process.stdout.write(`  1. Edit skills/${slug}/SKILL.md — replace TODO placeholders\n`);
process.stdout.write(`  2. Run: bun run skill:doctor (see what patterns are missing)\n`);
process.stdout.write(`  3. Run: bun run lint:tone --strict + bun test (verify)\n`);
