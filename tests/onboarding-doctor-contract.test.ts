import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const PROFILE_FIXTURE = join(import.meta.dir, "fixtures", "onboarding-doctor-profile.json");
const LEGACY_SHIM = join(REPO_ROOT, "tests", "e2e", "claude-cli", "fixtures", "bin", "axhub");

const TOP_LEVEL_FIELDS = ["cards", "code_agent", "completed", "first_gap", "no_gap", "outcome", "profile", "schema_version", "total"];
const CODE_AGENT_FIELDS = ["kind", "label", "plugin_installed", "version"];
const CARD_FIELDS = ["action_id", "id", "label", "safety", "state"];
const CARD_IDS = ["cli", "tools", "agent", "plugin", "auth", "github_link", "github_app"] as const;
const CARD_STATES: Record<string, true> = { completed: true, active: true, pending: true, blocked: true, unknown: true };
const ACTION_IDS: Record<string, true> = {
  install_git: true,
  install_node: true,
  agent_install: true,
  auth_login: true,
  github_link: true,
  github_app_install: true,
};
const SAFETY_LEVELS: Record<string, true> = { none: true, auto_safe: true, user_present: true };
const OUTCOMES: Record<string, true> = { ok: true, broken: true, unknown: true, prerequisite_blocked: true };
const CODE_AGENT_KINDS: Record<string, true> = { claude: true, codex: true };
const FORBIDDEN_KEYS: Record<string, true> = { command: true, argv: true, shell: true, path: true };
const LEGACY_GAPS = ["existing_repo_gap", "no_manifest_existing", "no_manifest_empty", "deps_missing", "deploy_unverified"];

type JsonObject = Record<string, unknown>;
type ValidationResult = { ok: true } | { ok: false; reason: string };

const hasExactFields = (value: JsonObject, fields: string[]): boolean =>
  JSON.stringify(Object.keys(value).sort()) === JSON.stringify([...fields].sort());
const readProfileFixture = (): unknown => JSON.parse(readFileSync(PROFILE_FIXTURE, "utf8"));

const findForbiddenKey = (value: unknown): string | undefined => {
  if (Array.isArray(value)) {
    for (const item of value) {
      const found = findForbiddenKey(item);
      if (found) return found;
    }
    return undefined;
  }
  if (typeof value !== "object" || value === null) return undefined;
  for (const [key, nested] of Object.entries(value)) {
    if (FORBIDDEN_KEYS[key] === true) return key;
    const found = findForbiddenKey(nested);
    if (found) return found;
  }
  return undefined;
};

const validateDoctorProfile = (value: unknown): ValidationResult => {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return { ok: false, reason: "not object" };
  const profile = value as JsonObject;
  if (!hasExactFields(profile, TOP_LEVEL_FIELDS)) return { ok: false, reason: "bad top-level fields" };
  if (profile.schema_version !== "onboarding-doctor/v1") return { ok: false, reason: "unknown schema" };
  if (profile.profile !== "axhub") return { ok: false, reason: "unknown profile" };
  if (typeof profile.outcome !== "string" || OUTCOMES[profile.outcome] !== true) return { ok: false, reason: "unknown outcome" };
  if (typeof profile.no_gap !== "boolean") return { ok: false, reason: "bad no_gap" };
  if (profile.first_gap !== null && (typeof profile.first_gap !== "string" || !CARD_IDS.includes(profile.first_gap as (typeof CARD_IDS)[number]))) {
    return { ok: false, reason: "bad first_gap" };
  }
  if (typeof profile.completed !== "number" || !Number.isInteger(profile.completed) || profile.completed < 0) {
    return { ok: false, reason: "bad completed count" };
  }
  if (profile.total !== CARD_IDS.length) return { ok: false, reason: "bad total count" };
  if (typeof profile.code_agent !== "object" || profile.code_agent === null || Array.isArray(profile.code_agent)) {
    return { ok: false, reason: "bad code_agent fields" };
  }
  const codeAgent = profile.code_agent as JsonObject;
  if (!hasExactFields(codeAgent, CODE_AGENT_FIELDS)) return { ok: false, reason: "bad code_agent fields" };
  const agentKind = codeAgent.kind;
  if (agentKind !== null && (typeof agentKind !== "string" || CODE_AGENT_KINDS[agentKind] !== true)) {
    return { ok: false, reason: "unknown code agent" };
  }
  if (codeAgent.label !== null && typeof codeAgent.label !== "string") return { ok: false, reason: "bad agent label" };
  if (codeAgent.version !== null && typeof codeAgent.version !== "string") return { ok: false, reason: "bad agent version" };
  if (typeof codeAgent.plugin_installed !== "boolean") return { ok: false, reason: "bad plugin flag" };
  if (!Array.isArray(profile.cards) || profile.cards.length !== CARD_IDS.length) return { ok: false, reason: "bad cards" };

  for (let index = 0; index < CARD_IDS.length; index += 1) {
    const candidate = profile.cards[index];
    if (typeof candidate !== "object" || candidate === null || Array.isArray(candidate)) {
      return { ok: false, reason: "bad card fields" };
    }
    const card = candidate as JsonObject;
    if (!hasExactFields(card, CARD_FIELDS)) return { ok: false, reason: "bad card fields" };
    if (card.id !== CARD_IDS[index]) return { ok: false, reason: "bad card ids" };
    if (typeof card.label !== "string" || card.label.trim().length === 0) return { ok: false, reason: "bad card label" };
    if (typeof card.state !== "string" || CARD_STATES[card.state] !== true) return { ok: false, reason: "unknown card state" };
    if (card.action_id !== null && (typeof card.action_id !== "string" || ACTION_IDS[card.action_id] !== true)) {
      return { ok: false, reason: "unknown action_id" };
    }
    if (typeof card.safety !== "string" || SAFETY_LEVELS[card.safety] !== true) return { ok: false, reason: "unknown safety" };
    if (card.state === "active") {
      if (card.action_id === null || card.safety === "none") return { ok: false, reason: "inactive repair" };
    } else if (card.action_id !== null || card.safety !== "none") {
      return { ok: false, reason: "action outside active card" };
    }
  }
  const completedCards = profile.cards.filter(
    (candidate) => (candidate as JsonObject).state === "completed",
  ).length;
  if (profile.completed !== completedCards) return { ok: false, reason: "completed count mismatch" };

  if (profile.outcome === "prerequisite_blocked") {
    const agentCard = profile.cards[2] as JsonObject;
    if (agentCard.state !== "blocked" || agentCard.action_id !== null || agentCard.safety !== "none") {
      return { ok: false, reason: "bad prerequisite card" };
    }
  }
  const forbiddenKey = findForbiddenKey(profile);
  if (forbiddenKey) return { ok: false, reason: `forbidden key: ${forbiddenKey}` };
  const serialized = JSON.stringify(profile);
  if (LEGACY_GAPS.some((gap) => serialized.includes(gap))) return { ok: false, reason: "legacy gap leaked" };
  return { ok: true };
};

const actionableProfile = (cardId: (typeof CARD_IDS)[number], actionId: string, safety: "auto_safe" | "user_present"): unknown => {
  const profile = readProfileFixture() as JsonObject;
  profile.outcome = "broken";
  profile.first_gap = cardId;
  profile.code_agent = { kind: "claude", label: "Claude Code", version: "1.2.3", plugin_installed: true };
  profile.cards = (profile.cards as JsonObject[]).map((card) => ({ ...card, state: "completed", action_id: null, safety: "none" }));
  const card = (profile.cards as JsonObject[]).find((candidate) => candidate.id === cardId);
  if (card) Object.assign(card, { state: "active", action_id: actionId, safety });
  profile.completed = CARD_IDS.length - 1;
  return profile;
};

describe("onboarding doctor external JSON contract", () => {
  test("profile-only fixture pins the seven-card prerequisite contract", () => {
    const profile = readProfileFixture() as JsonObject;

    expect(validateDoctorProfile(profile)).toEqual({ ok: true });
    expect(Object.keys(profile).sort()).toEqual([...TOP_LEVEL_FIELDS].sort());
    expect(profile.code_agent).toEqual({ kind: null, label: null, version: null, plugin_installed: false });
    expect((profile.cards as JsonObject[]).map((card) => card.id)).toEqual(CARD_IDS);
    for (const card of profile.cards as JsonObject[]) expect(Object.keys(card).sort()).toEqual([...CARD_FIELDS].sort());
    expect((profile.cards as JsonObject[]).find((card) => card.id === "agent")).toEqual({
      id: "agent",
      state: "blocked",
      label: "AI 코딩 도구",
      action_id: null,
      safety: "none",
    });
    expect(profile).toMatchObject({
      schema_version: "onboarding-doctor/v1",
      profile: "axhub",
      outcome: "prerequisite_blocked",
      completed: 5,
      total: 7,
    });
    expect(findForbiddenKey(profile)).toBeUndefined();
    for (const gap of LEGACY_GAPS) expect(JSON.stringify(profile)).not.toContain(gap);
  });

  test("validator keeps action, state, safety, and outcome enums closed", () => {
    const validActions: Array<[(typeof CARD_IDS)[number], string, "auto_safe" | "user_present"]> = [
      ["tools", "install_git", "auto_safe"],
      ["tools", "install_node", "auto_safe"],
      ["plugin", "agent_install", "auto_safe"],
      ["auth", "auth_login", "user_present"],
      ["github_link", "github_link", "user_present"],
      ["github_app", "github_app_install", "user_present"],
    ];
    for (const [cardId, actionId, safety] of validActions) {
      expect(validateDoctorProfile(actionableProfile(cardId, actionId, safety))).toEqual({ ok: true });
    }

    const unknownAction = actionableProfile("auth", "run_shell", "user_present");
    expect(validateDoctorProfile(unknownAction)).toEqual({ ok: false, reason: "unknown action_id" });
    const unknownState = readProfileFixture() as JsonObject;
    (unknownState.cards as JsonObject[])[0].state = "skipped";
    expect(validateDoctorProfile(unknownState)).toEqual({ ok: false, reason: "unknown card state" });

    const unknownSafety = actionableProfile("auth", "auth_login", "user_present") as JsonObject;
    (unknownSafety.cards as JsonObject[])[4].safety = "silent";
    expect(validateDoctorProfile(unknownSafety)).toEqual({ ok: false, reason: "unknown safety" });

    const unknownOutcome = readProfileFixture() as JsonObject;
    unknownOutcome.outcome = "degraded";
    expect(validateDoctorProfile(unknownOutcome)).toEqual({ ok: false, reason: "unknown outcome" });

    const actionOutsideActiveCard = readProfileFixture() as JsonObject;
    Object.assign((actionOutsideActiveCard.cards as JsonObject[])[0], { action_id: "install_git", safety: "auto_safe" });
    expect(validateDoctorProfile(actionOutsideActiveCard)).toEqual({ ok: false, reason: "action outside active card" });

    const badCount = readProfileFixture() as JsonObject;
    badCount.completed = 4;
    expect(validateDoctorProfile(badCount)).toEqual({ ok: false, reason: "completed count mismatch" });

    for (const forbidden of Object.keys(FORBIDDEN_KEYS)) {
      const executableLeak = readProfileFixture() as JsonObject;
      (executableLeak.code_agent as JsonObject)[forbidden] = "forbidden";
      expect(validateDoctorProfile(executableLeak)).toEqual({ ok: false, reason: "bad code_agent fields" });
    }
  });

  test("legacy unprofiled fixture keeps onboarding-detect/v1 without profile cards", () => {
    const caseDir = mkdtempSync(join(tmpdir(), "axhub-onboarding-legacy-"));
    const result = Bun.spawnSync({
      cmd: [LEGACY_SHIM, "plugin-support", "onboarding-detect", "--json"],
      env: { ...process.env, SHIM_CASE_DIR: caseDir, AXHUB_FIXTURE_ONBOARDING: "cli_missing" },
      stdout: "pipe",
      stderr: "pipe",
    });
    expect(result.exitCode).toBe(0);
    const legacy = JSON.parse(result.stdout.toString()) as JsonObject;

    expect(legacy.schema_version).toBe("onboarding-detect/v1");
    expect("profile" in legacy).toBe(false);
    expect("cards" in legacy).toBe(false);
  });
});
