import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

type Fixture = { id: string; state: Record<string, unknown>; user_utterance: string; expected_skill_invoke: string | null };

export function parseFixtures(path: string): Fixture[] {
  const raw = readFileSync(path, "utf8");
  const blocks = raw.split(/\n\s*- id: /).slice(1);
  return blocks.map((block) => {
    const id = block.split("\n", 1)[0].trim();
    const stateRaw = block.match(/state:\s*(\{.*\})/)?.[1] ?? "{}";
    const utt = block.match(/user_utterance:\s*"([^"]*)"/)?.[1] ?? "";
    const expectedRaw = block.match(/expected_skill_invoke:\s*(.*)/)?.[1]?.trim() ?? "null";
    return { id, state: JSON.parse(stateRaw), user_utterance: utt, expected_skill_invoke: expectedRaw === "null" ? null : expectedRaw.replace(/^"|"$/g, "") };
  });
}

export function classify(f: Fixture): string | null {
  const s = f.state;
  const u = f.user_utterance;
  if (s.env === "AXHUB_DISABLE_TRIGGERS" || s.corrupt) return null;
  if (/리뷰|코드 봐|PR 검토|변경 봐|품질/.test(u)) return "axhub-review";
  if (/디버그|왜 안|에러|테스트 실패|스택/.test(u) || s.last_test_failure_at) return "axhub-debug";
  if (/배포 준비|PR 만들어|릴리즈|push|출시/.test(u)) return "axhub-ship";
  if (/TDD|테스트 먼저|RED GREEN|실패 테스트|테스트 작성/.test(u)) return "axhub-tdd";
  if (/플랜|계획|아키텍처|구조|모듈/.test(u)) return "axhub-plan";
  if (Number(s.files_changed_since_review ?? 0) > 50 || s.major_arch_change) return "axhub-plan";
  if (Number(s.lines_since_review_user ?? 0) >= 50 || Number(s.files_changed_since_review ?? 0) > 5) return "axhub-review";
  if (s.new_source_file && Number(s.source_files_count ?? 0) > 0 && Number(s.test_files_count ?? 0) / Number(s.source_files_count) < 0.5) return "axhub-tdd";
  return null;
}

export function runEval(path: string, outName: string, gates: { overall: number; perSkill?: number; falsePositive?: number }) {
  const fixtures = parseFixtures(path);
  const rows = fixtures.map((f) => ({ id: f.id, expected: f.expected_skill_invoke, actual: classify(f) }));
  const passed = rows.filter((r) => r.expected === r.actual).length;
  const obedience_rate = passed / rows.length;
  const skillRates: Record<string, number> = {};
  for (const skill of ["axhub-review", "axhub-debug", "axhub-ship", "axhub-tdd", "axhub-plan"]) {
    const subset = rows.filter((r) => r.expected === skill);
    skillRates[skill] = subset.length ? subset.filter((r) => r.actual === skill).length / subset.length : 1;
  }
  const negatives = rows.filter((r) => r.expected === null);
  const false_positive_rate = negatives.length ? negatives.filter((r) => r.actual !== null).length / negatives.length : 0;
  const report = { fixtures: fixtures.length, passed, obedience_rate, per_skill: skillRates, false_positive_rate, rows };
  const home = process.env.HOME ?? ".";
  const dir = join(home, ".axhub", "analytics");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, outName), JSON.stringify(report, null, 2) + "\n");
  process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  if (obedience_rate < gates.overall) process.exit(1);
  if (gates.perSkill !== undefined && Object.values(skillRates).some((v) => v < gates.perSkill!)) process.exit(1);
  if (gates.falsePositive !== undefined && false_positive_rate >= gates.falsePositive) process.exit(1);
}
