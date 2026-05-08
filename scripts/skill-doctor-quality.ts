// Phase 8 sub-task 8.2 — skill description quality lint helpers.
// Phase 9 sub-task 9.1.1 — frontmatter examples field validation.
//
// Extracted from skill-doctor.ts so unit tests can import without triggering
// the doctor's top-level filesystem scan + process.exit.

export const MIN_TRIGGER_COUNT = 5;
export const MIN_PER_LANG = 2;
export const MIN_EXAMPLES = 5;
export const MAX_EXAMPLES = 10;

export interface QualityIssue {
  slug: string;
  kind:
    | "min_trigger"
    | "ko_balance"
    | "en_balance"
    | "examples_missing"
    | "examples_min"
    | "examples_max"
    | "examples_lang_ko"
    | "examples_lang_en"
    | "intent_lang";
  detail: string;
}

export interface SkillExample {
  utterance: string;
  intent: string;
}

export const isKorean = (s: string): boolean => /[가-힯]/.test(s);

export function computeQualityIssues(slug: string, phrases: string[]): QualityIssue[] {
  const issues: QualityIssue[] = [];
  const koCount = phrases.filter(isKorean).length;
  const enCount = phrases.length - koCount;
  if (phrases.length < MIN_TRIGGER_COUNT) {
    issues.push({ slug, kind: "min_trigger", detail: `${phrases.length} < ${MIN_TRIGGER_COUNT}` });
  }
  if (koCount < MIN_PER_LANG) {
    issues.push({ slug, kind: "ko_balance", detail: `ko=${koCount} < ${MIN_PER_LANG}` });
  }
  if (enCount < MIN_PER_LANG) {
    issues.push({ slug, kind: "en_balance", detail: `en=${enCount} < ${MIN_PER_LANG}` });
  }
  return issues;
}

/**
 * Parse the `examples:` block from SKILL.md frontmatter (no js-yaml dep).
 *
 * Expected format:
 *   examples:
 *     - utterance: "..."
 *       intent: "..."
 *     - utterance: "..."
 *       intent: "..."
 *
 * Returns [] if the field is missing or malformed.
 */
export function parseExamples(frontmatter: string): SkillExample[] {
  const examples: SkillExample[] = [];
  const lines = frontmatter.split("\n");
  let inExamples = false;
  let current: { utterance?: string; intent?: string } = {};
  const flush = () => {
    if (current.utterance !== undefined && current.intent !== undefined) {
      examples.push({ utterance: current.utterance, intent: current.intent });
    }
    current = {};
  };
  for (const line of lines) {
    if (/^examples:\s*$/.test(line)) {
      inExamples = true;
      continue;
    }
    if (!inExamples) continue;
    if (/^\S/.test(line)) {
      flush();
      inExamples = false;
      break;
    }
    const dashUtter = line.match(/^\s+-\s+utterance:\s*['"](.+?)['"]\s*$/);
    if (dashUtter && dashUtter[1] !== undefined) {
      flush();
      current.utterance = dashUtter[1];
      continue;
    }
    const intentMatch = line.match(/^\s+intent:\s*['"](.+?)['"]\s*$/);
    if (intentMatch && intentMatch[1] !== undefined) {
      current.intent = intentMatch[1];
    }
  }
  flush();
  return examples;
}

export function computeExamplesIssues(slug: string, frontmatter: string): QualityIssue[] {
  const issues: QualityIssue[] = [];
  const examples = parseExamples(frontmatter);
  if (!/^examples:\s*$/m.test(frontmatter)) {
    issues.push({ slug, kind: "examples_missing", detail: "frontmatter has no examples: field" });
    return issues;
  }
  if (examples.length < MIN_EXAMPLES) {
    issues.push({ slug, kind: "examples_min", detail: `${examples.length} < ${MIN_EXAMPLES}` });
  }
  if (examples.length > MAX_EXAMPLES) {
    issues.push({ slug, kind: "examples_max", detail: `${examples.length} > ${MAX_EXAMPLES}` });
  }
  const koUtter = examples.filter((e) => isKorean(e.utterance)).length;
  const enUtter = examples.length - koUtter;
  if (koUtter < MIN_PER_LANG) {
    issues.push({ slug, kind: "examples_lang_ko", detail: `ko=${koUtter} < ${MIN_PER_LANG}` });
  }
  if (enUtter < MIN_PER_LANG) {
    issues.push({ slug, kind: "examples_lang_en", detail: `en=${enUtter} < ${MIN_PER_LANG}` });
  }
  for (const ex of examples) {
    if (isKorean(ex.intent)) {
      issues.push({
        slug,
        kind: "intent_lang",
        detail: `intent must be English verb phrase (got: "${ex.intent.slice(0, 40)}")`,
      });
      break;
    }
  }
  return issues;
}
