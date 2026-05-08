// Phase 8 sub-task 8.2 — skill description quality lint helpers.
//
// Extracted from skill-doctor.ts so unit tests can import without triggering
// the doctor's top-level filesystem scan + process.exit.

export const MIN_TRIGGER_COUNT = 5;
export const MIN_PER_LANG = 2;

export interface QualityIssue {
  slug: string;
  kind: "min_trigger" | "ko_balance" | "en_balance";
  detail: string;
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
