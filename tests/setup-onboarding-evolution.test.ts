import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
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

const extractPosixNodePredicate = (content: string) => {
  const match = content.match(/node -e '([^'\n]+)' "\$NODE_ACTIVE" "\$NODE_REQUIRED"/);
  expect(match, "missing POSIX node_mismatch predicate script").toBeTruthy();
  return match![1];
};

const extractPosixGithubPredicate = (content: string) => {
  const match = content.match(/GITHUB_APP_STATE="\$\(printf '%s' "\$GITHUB_ACCOUNTS_JSON" \| node -e '([^']+)'/);
  expect(match, "missing POSIX GitHub App predicate script").toBeTruthy();
  return match![1];
};

const runNodeScript = (script: string, args: string[], stdin = "") => {
  const result = spawnSync("node", ["-e", script, ...args], { input: stdin, encoding: "utf8" });
  expect(result.error, result.stderr).toBeUndefined();
  return result;
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

    expect(content).toContain("axhub --version");
    expect(content).toContain('MIN_AXHUB_CLI_VERSION="0.17.3"');
    expect(content).toContain("axhub update check --json");
    expect(content).toContain("cli_too_old");
    expect(content).toContain("has_update");
    expect(content).toContain("git --version");
    expect(content).toContain("node --version");
    expect(content).toContain("node_modules");
    expect(content).toContain("dir_empty");
    expect(content).toContain("dir_non_empty");
    expect(content).toContain("NODE_ACTIVE");
    expect(content).toContain("NODE_REQUIRED");
    expect(content).toContain("axhub github accounts list --json");
    expect(content).toContain("github_app_missing");
    expect(content).toContain("installed=true");
    expect(content).toContain("installation_id");
    expect(content).toContain(".axhub/bootstrap.state.json");
    expect(content).toContain("last_deploy_id");
    expect(content).toContain("deploy_unverified");
    expect(content).toContain("succeeded/live/running/deployed");
  });

  test("locks first-gap state-machine order instead of only vocabulary", () => {
    const content = onboarding();

    expectInOrder(content, [
      "START",
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

    expect(transitions).toHaveLength(13);
    expect(rows).toHaveLength(12);

    const scenarios = [
      { id: "S1", gaps: ["cli_missing"], handler: "install-cli", owner: "install-cli", ready: "READY_WITH_USER_ACTION", detect: "axhub --version" },
      { id: "S2", gaps: ["cli_old"], handler: "update", owner: "update", ready: "VIBE_READY", detect: "cli_too_old=true" },
      { id: "S3", gaps: ["auth_missing"], handler: "auth", owner: "auth", ready: "READY_WITH_USER_ACTION", detect: "auth_ok=false" },
      { id: "S4", gaps: ["git_missing"], handler: "install_git", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "git --version" },
      { id: "S5", gaps: ["node_missing"], handler: "install_node", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "node --version" },
      { id: "S6", gaps: ["node_mismatch"], handler: "fix_node", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "NODE_REQUIRED" },
      { id: "S7", gaps: ["github_app_missing"], handler: "install_url", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "install_url" },
      { id: "S8", gaps: ["no_manifest_empty"], handler: "init", owner: "init", ready: "VIBE_READY", detect: "manifest 없음 + 빈 dir" },
      { id: "S9", gaps: ["existing_repo_gap"], handler: "github", owner: "github", ready: "READY_WITH_USER_ACTION", detect: ".git" },
      { id: "S10", gaps: ["deps_missing"], handler: "install_deps", owner: "onboarding", ready: "VIBE_READY", detect: "node_modules" },
      { id: "S11", gaps: ["deps_missing"], handler: "install_deps", owner: "onboarding", ready: "READY_WITH_USER_ACTION", detect: "--ignore-scripts" },
      { id: "S12", gaps: ["cli_old"], handler: "update", owner: "update", ready: "SAFE_STOP_NONINTERACTIVE", detect: "CLAUDE_NON_INTERACTIVE" },
      { id: "S13", gaps: ["doctor_gap"], handler: "doctor", owner: "doctor", ready: "READY_WITH_USER_ACTION", detect: "PATH reload" },
      { id: "S14", gaps: ["doctor_gap"], handler: "doctor", owner: "doctor", ready: "READY_WITH_USER_ACTION", detect: "doctor 핵심 체크 fail" },
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

  test("executes node version predicate fixtures to prevent VIBE_READY false green", () => {
    const script = extractPosixNodePredicate(onboarding());

    expect(runNodeScript(script, ["v20.11.1", "20.11.1"]).status).toBe(0);
    expect(runNodeScript(script, ["v20.0.0", "20.11.1"]).status).toBe(1);
    expect(runNodeScript(script, ["v22.0.0", ">=20 <23"]).status).toBe(0);
    expect(runNodeScript(script, ["v25.0.0", ">=20 <23"]).status).toBe(1);
    expect(runNodeScript(script, ["v20.11.1", "unsupported-range"]).status).toBe(1);
  });

  test("executes GitHub App predicate fixtures without conflating unknown and missing", () => {
    const script = extractPosixGithubPredicate(onboarding());

    expect(runNodeScript(script, [], "").stdout.trim()).toBe("unknown");
    expect(runNodeScript(script, [], "not-json").stdout.trim()).toBe("unknown");
    expect(runNodeScript(script, [], JSON.stringify({ accounts: [] })).stdout.trim()).toBe("unknown");
    expect(runNodeScript(script, [], JSON.stringify({ install_url: "https://github.com/apps/axhub/installations/new", accounts: [] })).stdout.trim()).toBe(
      "missing:https://github.com/apps/axhub/installations/new",
    );
    expect(runNodeScript(script, [], JSON.stringify({ accounts: [{ installed: true }] })).stdout.trim()).toBe("installed");
    expect(runNodeScript(script, [], JSON.stringify({ installations: [{ installation_id: 123 }] })).stdout.trim()).toBe("installed");
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

  test("allows dependency execution only with lockfile, consent, D1, and ignore-scripts", () => {
    const content = onboarding();
    const fm = content.split("\n---\n")[0];

    expect(fm).toContain("allows-dependency-execution: true");
    expect(content).toContain("lockfile 있을 때만");
    expect(content).toContain("consent 필수");
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

  test("registers conservative safe defaults for every new onboarding consent", () => {
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
