import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// 대표 여정(첫 셋업 → gap 정리 → 마무리 → 재시작 resume) 계약이 SKILL 본문과
// reference 들 사이에서 어긋나지 않는지 구조적으로 검증해요. 문구 pin 이 아니라
// gap id 집합·섹션 순서·계약 토큰의 존재를 봐요 — 드리프트를 잡는 게 목적이에요.

const ROOT = join(import.meta.dir, "..");
const read = (path: string) => readFileSync(join(ROOT, path), "utf8");

const SKILL = read("skills/onboarding/SKILL.md");
const GAPS = read("skills/onboarding/references/gap-state-machine.md");
const CARD = read("skills/onboarding/references/mcp-ready-card.md");
const INSTALL = read("skills/onboarding/references/install-channels-and-auth.md");
const GITHUB = read("skills/onboarding/references/github-app.md");
const DEPS = read("skills/onboarding/references/dependency-install.md");

const section = (doc: string, heading: string): string => {
  const start = doc.indexOf(heading);
  expect(start, `missing heading: ${heading}`).toBeGreaterThan(-1);
  const rest = doc.slice(start + heading.length);
  const next = rest.search(/\n#{2,3} /);
  return next === -1 ? rest : rest.slice(0, next);
};

const routerGaps = new Set(
  [...section(SKILL, "### 3. first_gap router").matchAll(/^\| `([a-z_]+)` \|/gm)]
    .map((m) => m[1]!)
    .filter((id) => id !== "first_gap"),
);
const loopGaps = new Set(
  [...section(GAPS, "## Loop").matchAll(/^ {2}([a-z_]+) +->/gm)].map((m) => m[1]!),
);
const completionGaps = new Set(
  [...section(GAPS, "## Gap Completion Rules").matchAll(/^\| `([a-z_]+)` \|/gm)].map(
    (m) => m[1]!,
  ),
);

describe("onboarding representative journey", () => {
  test("gap id sets stay in parity across SKILL router, loop diagram, completion rules", () => {
    expect(routerGaps.size).toBeGreaterThanOrEqual(13);
    expect([...loopGaps].sort()).toEqual([...routerGaps].sort());
    // no_gap ends the loop at the Ready card and has no completion rule row.
    const loopMinusNoGap = [...loopGaps].filter((id) => id !== "no_gap").sort();
    expect([...completionGaps].sort()).toEqual(loopMinusNoGap);
  });

  test("CLI null first_gap is accepted as no_gap completion (schema parity with real CLI)", () => {
    // CLI 는 gap 이 없으면 first_gap 을 null 로 반환해요 — "no_gap" 문자열은 CLI 가 만들지 않아요.
    expect(SKILL).toContain("null/부재이고 `gaps` 가 비어 있으면 `no_gap` 과 동일한 완료");
    expect(GAPS).toContain("null/부재 + 빈 `gaps` 는 `no_gap` 과 같은 완료");
    const shim = read("tests/e2e/claude-cli/fixtures/bin/axhub");
    expect(shim).toContain('"first_gap":null');
    expect(shim).not.toContain('"first_gap":"no_gap"');
  });

  test("journey stages appear in order: guard -> detect -> router -> finish", () => {
    const stages = [
      "### 0. Non-interactive guard",
      "### 1. DETECT_ALL(read-only)",
      "### 3. first_gap router",
      "### 4. Telemetry opt-in, MCP and Ready card",
    ];
    const positions = stages.map((s) => SKILL.indexOf(s));
    for (const [i, pos] of positions.entries()) {
      expect(pos, `missing stage: ${stages[i]}`).toBeGreaterThan(-1);
      if (i > 0) expect(pos).toBeGreaterThan(positions[i - 1]!);
    }
  });

  test("session re-entry lane: bin-path fixture in SKILL, absolute-path lane in reference", () => {
    expect(SKILL).toContain('cat "$HOME/.axhub/bin-path"');
    expect(SKILL).toContain('"cli_resolved_path\\":\\"$AXHUB_BIN_LOC\\"');
    expect(INSTALL).toContain("cli_resolved_path");
    expect(INSTALL).toContain("current_session_stale");
  });

  test("finish stage keeps the one-restart contract chain", () => {
    expect(SKILL).toContain("재시작 최대 1회 · 카드 1장 · 질문은 옵트인 1개");
    expect(CARD).toContain("마지막 단계예요");
    // opt-in question shape: skip-first default + later-enable phrase
    expect(CARD).toContain('"이번엔 건너뛰기"(첫 번째 옵션)');
    expect(CARD).toContain("AI 활용 기록 켜줘");
    // resume without re-asking, and marker cleanup after the final card
    expect(CARD).toContain("다시 묻지 않아요");
    expect(CARD).toContain('rm -f "$HOME/.axhub/cache/.onboarding-mcp-restart"');
    for (const exit of [
      "VIBE_READY",
      "READY_WITH_USER_ACTION",
      "SAFE_STOP_NONINTERACTIVE",
      "BLOCKED_UNSUPPORTED",
    ]) {
      expect(CARD).toContain(exit);
    }
  });

  test("telemetry opt-in keeps the axrouter CLI contract in the card", () => {
    // status 파싱 계약 — CLI JSON 필드명이 바뀌면 여기서 잡아요
    expect(CARD).toContain("axhub axrouter status --json");
    expect(CARD).toContain("`data.workspaces[]`");
    expect(CARD).toContain("`available: true`");
    expect(CARD).toContain("`data.active_workspace`");
    expect(CARD).toContain("`data.local_monitoring`");
    // 켜기·동의·철회 lane — CLI-native consent 먼저, 실패 시 콘솔 fallback
    expect(CARD).toContain("axhub axrouter monitor --tenant <slug> --json");
    expect(CARD).toContain("consent_required");
    expect(CARD).toContain("axhub axrouter consent --agree --execute --tenant <slug> --json");
    expect(CARD).toContain("error.doc_url");
    expect(CARD).toContain("axhub axrouter revoke-consent --execute");
    // headless 는 묻지도 실행하지도 않아요 (AP-10)
    expect(CARD).toContain("headless 면 이 섹션을 통째로 건너뛰어요");
  });

  test("github-app reference stays aligned with device-flow and relogin contracts", () => {
    expect(GITHUB).toContain("github_relogin_required");
    expect(GITHUB).toContain("axhub github link");
    expect(GITHUB).toContain("설치 끝났어");
    // device-flow-only phrase must not gate the browser App install step
    expect(GITHUB).not.toContain("`승인했어`");
  });

  test("dependency reference does not invent detect fields", () => {
    expect(DEPS).not.toContain("recorded by detect");
    expect(DEPS).toContain("--ignore-scripts");
  });
});
