import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const ONBOARDING_SKILL = join(REPO_ROOT, "skills/onboarding/SKILL.md");
const REGISTRY = join(REPO_ROOT, "tests/fixtures/ask-defaults/registry.json");
const ALLOWLIST = join(REPO_ROOT, "scripts/skill-doctor-allowlist.json");

const onboarding = () => readFileSync(ONBOARDING_SKILL, "utf8");
const registry = () => JSON.parse(readFileSync(REGISTRY, "utf8")) as Record<string, Record<string, unknown>>;
const allowlist = () =>
  JSON.parse(readFileSync(ALLOWLIST, "utf8")) as {
    allows_dependency_execution: Array<{ skill: string; rationale: string }>;
  };

const expectInOrder = (content: string, markers: string[]) => {
  let cursor = -1;
  for (const marker of markers) {
    const next = content.indexOf(marker, cursor + 1);
    expect(next, `missing or out-of-order marker: ${marker}`).toBeGreaterThan(cursor);
    cursor = next;
  }
};

type Transition = {
  gap: string;
  owner: string;
  next: string;
};

type GapTableRow = {
  gap: string;
  detect: string;
  owner: string;
  done: string;
};

const parseTransitions = (content: string): Transition[] => {
  const matches = [
    ...content.matchAll(/^\s*[├└]─\s+([a-z_]+)\s+→\s+(.+?)(?:\s+→\s+(DETECT_ALL|VIBE_READY_CARD))?\s*$/gm),
  ];
  return matches.map((match) => ({
    gap: match[1],
    owner: match[2].trim(),
    next: match[3] ?? "VIBE_READY_CARD",
  }));
};

const parseGapTable = (content: string): GapTableRow[] =>
  content
    .split("\n")
    .filter((line) => line.startsWith("   | `"))
    .map((line) => line.trim().split("|").map((cell) => cell.trim()))
    .filter((cells) => cells.length >= 6)
    .map((cells) => ({
      gap: cells[1].replaceAll("`", ""),
      detect: cells[2],
      owner: cells[3],
      done: cells[4],
    }));

const transitionForFirstGap = (transitions: Transition[], activeGaps: readonly string[]) => {
  const active = new Set(activeGaps);
  return transitions.find((transition) => active.has(transition.gap)) ?? transitions.find((transition) => transition.gap === "no_gap");
};

describe("onboarding skill evolution — VIBE_READY contract", () => {
  test("declares onboarding as the single user-facing onboarding entrypoint", () => {
    const content = onboarding();

    expect(content).toContain("온보딩 단일 진입점");
    expect(content).toContain("사용자는 sibling skill 이름이나 slash command 를 몰라도");
    expect(content).toContain("detect-first → 첫 gap 처리 → 재감지");
    expect(content).toContain("VIBE_READY");
    expect(content).toContain("READY_WITH_USER_ACTION");
    expect(content).toContain("SAFE_STOP_NONINTERACTIVE");
    expect(content).toContain("BLOCKED_UNSUPPORTED");
  });

  test("documents the full gap state machine and read-only detection surface", () => {
    const content = onboarding();

    for (const marker of [
      "Gap State Machine",
      "DETECT_ALL(read-only)",
      "cli_missing",
      "cli_old",
      "auth_missing",
      "git_missing",
      "node_missing",
      "node_mismatch",
      "github_app_missing",
      "existing_repo_gap",
      "no_manifest_empty",
      "deps_missing",
      "deploy_unverified",
      "doctor_gap",
      "온보딩 계속",
    ]) {
      expect(content).toContain(marker);
    }

    // Detection runs through the cross-platform `onboarding-detect` helper, not
    // inline bash/PowerShell. The SKILL routes on the helper's JSON fields; the
    // detection LOGIC (node version, github predicate, deploy, dir-ignore) is
    // unit-tested in crates/axhub-helpers/src/onboarding_detect.rs.
    expect(content).toContain("onboarding-detect");
    expect(content).toContain("first_gap");
    expect(content).toContain("cli_present");
    expect(content).toContain("cli_too_old");
    expect(content).toContain("cli_on_path");
    expect(content).toContain("on_disk_not_on_path");
    expect(content).toContain("has_update");
    expect(content).toContain("auth_ok");
    expect(content).toContain("git_present");
    expect(content).toContain("node_present");
    expect(content).toContain("node_required");
    expect(content).toContain("node_mismatch");
    expect(content).toContain("manifest_present");
    expect(content).toContain("deps_missing");
    expect(content).toContain("dir_empty");
    expect(content).toContain("github");
    expect(content).toContain("install_url");
    expect(content).toContain("multiple_installed");
    expect(content).toContain("github_app_missing");
    expect(content).toContain("deploy_verified");
    expect(content).toContain("deploy_unverified");
  });

  test("delegates empty-dir + environment detection to the cross-platform helper", () => {
    const content = onboarding();

    // The dual bash/PowerShell DETECT_ALL — and its Claude Desktop ignore-list —
    // moved into `axhub-helpers onboarding-detect`; the empty-dir logic
    // (DIR_IGNORE) is unit-tested in onboarding_detect.rs
    // (`dir_is_empty_at_ignores_scaffolding`). The SKILL now reads `dir_empty`.
    expect(content).toContain("onboarding-detect");
    expect(content).toContain("dir_empty");
    // The leak-prone inline detection scripts must not return to the SKILL body
    // (the original bug: a condensed copy was narrated into user-facing chat).
    expect(content).not.toContain('echo "dir_non_empty"');
    expect(content).not.toContain("Get-ChildItem -Force");
    expect(content).not.toContain("node -e");
  });

  test("locks first-gap state-machine order instead of only vocabulary", () => {
    const content = onboarding();

    expectInOrder(content, [
      "START",
      "DETECT_ALL(read-only)",
      "cli_missing",
      "cli_path_missing",
      "cli_old",
      "auth_missing",
      "git_missing",
      "node_missing",
      "node_mismatch",
      "github_app_missing",
      "existing_repo_gap",
      "no_manifest_empty",
      "deps_missing",
      "deploy_unverified",
      "doctor_gap",
      "no_gap",
      "VIBE_READY_CARD",
    ]);

    expect(content).toContain("첫 gap 하나만 처리하고 재감지");
    expect(content.match(/→ DETECT_ALL/g)?.length ?? 0).toBeGreaterThanOrEqual(10);
  });

  test("simulates the documented scenario matrix from first gap to owner and ready grade", () => {
    const content = onboarding();
    const transitions = parseTransitions(content);
    const rows = parseGapTable(content);

    expect(transitions).toHaveLength(14);
    expect(rows).toHaveLength(13);

    const scenarios = [
      { id: "S1", gaps: ["cli_missing"], handler: "install-cli", owner: "install-cli", ready: "READY_WITH_USER_ACTION", detect: "cli_present=false" },
      { id: "S2", gaps: ["cli_path_missing"], handler: "repair", owner: "repair", ready: "READY_WITH_USER_ACTION", detect: "on_disk_not_on_path" },
      { id: "S3", gaps: ["cli_old"], handler: "update", owner: "update", ready: "VIBE_READY", detect: "cli_too_old=true" },
      { id: "S4", gaps: ["auth_missing"], handler: "auth", owner: "auth", ready: "READY_WITH_USER_ACTION", detect: "auth_ok=false" },
      { id: "S5", gaps: ["git_missing"], handler: "install_git", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "git_present=false" },
      { id: "S6", gaps: ["node_missing"], handler: "install_node", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "node_present=false" },
      { id: "S7", gaps: ["node_mismatch"], handler: "fix_node", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "node_required" },
      { id: "S8", gaps: ["github_app_missing"], handler: "install_url", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "install_url" },
      { id: "S9", gaps: ["no_manifest_empty"], handler: "init", owner: "init", ready: "VIBE_READY", detect: "dir_empty" },
      { id: "S10", gaps: ["existing_repo_gap"], handler: "github", owner: "github", ready: "READY_WITH_USER_ACTION", detect: "git_repo=true" },
      { id: "S11", gaps: ["deps_missing"], handler: "install_deps", owner: "onboarding", ready: "VIBE_READY", detect: "node_modules" },
      { id: "S12", gaps: ["deps_missing"], handler: "install_deps", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "--ignore-scripts" },
      { id: "S13", gaps: ["cli_old"], handler: "update", owner: "update", ready: "SAFE_STOP_NONINTERACTIVE", detect: "CLAUDE_NON_INTERACTIVE" },
      { id: "S14", gaps: ["doctor_gap"], handler: "doctor", owner: "doctor", ready: "READY_WITH_USER_ACTION", detect: "PATH reload" },
      { id: "S15", gaps: ["doctor_gap"], handler: "doctor", owner: "doctor", ready: "READY_WITH_USER_ACTION", detect: "doctor 핵심 체크 fail" },
    ] as const;

    for (const scenario of scenarios) {
      const first = transitionForFirstGap(transitions, scenario.gaps);
      expect(first, `${scenario.id} missing first transition`).toBeDefined();
      expect(first!.owner, `${scenario.id} wrong handler`).toContain(scenario.handler);
      expect(first!.next, `${scenario.id} must re-detect after first gap`).toBe("DETECT_ALL");
      expect(content, `${scenario.id} ready grade missing`).toContain(scenario.ready);

      const row = rows.find((candidate) => candidate.gap === scenario.gaps[0]);
      expect(row, `${scenario.id} missing gap table row`).toBeDefined();
      expect(row!.owner, `${scenario.id} wrong table owner`).toContain(scenario.owner);
      expect(`${row!.detect} ${row!.done} ${content}`, `${scenario.id} missing detection evidence`).toContain(scenario.detect);
    }

    const clean = transitionForFirstGap(transitions, []);
    expect(clean).toEqual({ gap: "no_gap", owner: "VIBE_READY_CARD", next: "VIBE_READY_CARD" });
    expect(content).toContain("승인했어");
    expect(content).toContain("온보딩 계속");
    expect(content).toContain("다시 온보딩해줘");
  });

  // The node-version and GitHub-App predicate logic moved from inline SKILL
  // scripts into `axhub-helpers onboarding-detect`. Its behavior (auth_error vs
  // empty vs installed/mixed, node major/range matching) is unit-tested in
  // crates/axhub-helpers/src/onboarding_detect.rs — not re-executed here.
  test("invokes the onboarding-detect helper instead of inline predicates", () => {
    const content = onboarding();

    expect(content).toContain('"$HELPER" onboarding-detect --json');
    // auth-error is now a distinct github state (no longer collapsed to unknown).
    expect(content).toContain("auth_error");
    // No inline node / github predicate scripts remain in the SKILL body.
    expect(content).not.toContain("node -e");
    expect(content).not.toContain("GITHUB_APP_STATE");
  });

  test("locks non-interactive mutation denylist including git and node system changes", () => {
    const content = onboarding();

    expect(content).toContain("install/update/auth/init/deploy/deps mutation");
    expect(content).toContain("git/node system install");
    expect(content).toContain("version switch");
    expect(content).toContain("SAFE_STOP_NONINTERACTIVE");
  });

  test("locks GitHub install_url-only frontloading and forbids duplicate deploy after init", () => {
    const content = onboarding();

    expect(content).toContain("install_url");
    expect(content).toContain("계정레벨 GitHub App 설치");
    expect(content).toContain("OAuth device-flow 인가는 connect 단계");
    expect(content).toContain("init 경로는 saga 배포 URL surface");
    expect(content).toContain("재배포 X");
    expect(content).toContain("status/watch");
  });

  test("always surfaces github install_url + connect guidance, even when installed", () => {
    const content = onboarding();

    // User contract: show the GitHub App install_url unconditionally — even for
    // already-installed users — and actively tell them to connect.
    expect(content).toContain("github.install_url");
    expect(content).toContain("설치 여부와 무관하게");
    expect(content).toContain("무조건");
    expect(content).toContain("연결");
    // An auth-error github state routes to re-login instead of swallowing the
    // install_url surface as the old "unknown" did.
    expect(content).toContain("auth_error");
    expect(content).toContain("다시 로그인해줘");
  });

  test("allows dependency execution only with lockfile, explicit confirmation, D1, and ignore-scripts", () => {
    const content = onboarding();
    const fm = content.split("\n---\n")[0];

    expect(fm).toContain("allows-dependency-execution: true");
    expect(content).toContain("lockfile 있을 때만");
    expect(content).toContain("명시 확인 필수");
    expect(content).toContain("deps_missing");
    expect(content).toContain("lockfile 없으면 package manager 선택을 묻지 말고 skip");
    expect(content).toContain("--ignore-scripts");
    expect(content).toContain("postinstall 자동 실행 금지");
    expect(content).toContain("SAFE_STOP_NONINTERACTIVE");

    const onboardingAllowlist = allowlist().allows_dependency_execution.find((entry) => entry.skill === "onboarding");
    expect(onboardingAllowlist).toBeDefined();
    expect(onboardingAllowlist!.rationale.length).toBeGreaterThanOrEqual(50);
    expect(onboardingAllowlist!.rationale).toContain("--ignore-scripts");
  });

  test("registers conservative safe defaults for every new onboarding confirmation", () => {
    const onboardingRegistry = registry()["onboarding"] as Record<string, { safe_default?: string; allowed_safe_defaults?: string[] }>;

    for (const [question, expectedDefault] of [
      ["axhub CLI 업데이트를 적용할까요?", "취소"],
      ["git 이 없어요. 지금 설치할까요?", "나중에"],
      ["node 권장 버전으로 맞출까요?", "나중에"],
      ["GitHub App 을 먼저 설치할까요?", "나중에"],
      ["의존성을 설치할까요?", "나중에"],
      ["기존 repo 를 axhub 앱에 연결할까요?", "아니요"],
      ["첫 앱 만들래요?", "아니요"],
    ] as const) {
      const entry = onboardingRegistry[question];
      expect(entry, `missing registry entry for ${question}`).toBeDefined();
      expect(entry.safe_default).toBe(expectedDefault);
      expect(entry.allowed_safe_defaults).toContain(expectedDefault);
    }
  });
});
