import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const skillPath = join(process.cwd(), "skills", "deploy", "SKILL.md");
const registryPath = join(
  process.cwd(),
  "tests",
  "fixtures",
  "ask-defaults",
  "registry.json",
);

function deploySkill(): string {
  return readFileSync(skillPath, "utf8");
}

function deployRegistry(): any {
  return JSON.parse(readFileSync(registryPath, "utf8")).deploy;
}

describe("deploy SKILL Phase 3.5 wire-up invariants", () => {
  test("default path uses deploy-prep and legacy resolve/preflight are guarded", () => {
    const body = deploySkill();
    expect(body).toContain('"$HELPER" deploy-prep --intent deploy');
    expect(body).toContain('if [[ "${AXHUB_DEPLOY_PREP:-1}" == "0" ]]; then');
    expect(body).toContain("[deploy:Step 2 preflight legacy] entered");

    const deployPrepIndex = body.indexOf('"$HELPER" deploy-prep --intent deploy');
    const legacyResolveIndex = body.indexOf("[deploy:Step 1 resolve refresh]");
    const legacyPreflightIndex = body.indexOf("[deploy:Step 2 preflight legacy]");
    expect(deployPrepIndex).toBeGreaterThan(-1);
    expect(legacyResolveIndex).toBeGreaterThan(deployPrepIndex);
    expect(legacyPreflightIndex).toBeGreaterThan(legacyResolveIndex);
  });

  test("cli_too_new bridge reads config before prompting and persists only dismiss path", () => {
    const body = deploySkill();
    const bridgeIndex = body.indexOf("cli_too_new dismiss bridge");
    const configGetIndex = body.indexOf("config get ignore_too_new_until --json");
    const askIndex = body.indexOf('"question": "axhub CLI 가 더 최신 버전인데 계속할까요?"');
    const caseIndex = body.indexOf('case "${CLI_TOO_NEW_ANSWER:-continue}" in');
    const dismissCaseIndex = body.indexOf("dismiss)", caseIndex);
    const configSetIndex = body.indexOf("config set ignore_too_new_until");
    const explainCaseIndex = body.indexOf("explain)", caseIndex);
    const continueCaseIndex = body.indexOf("continue|*)", caseIndex);
    const continueIndex = body.indexOf('"value": "continue"', askIndex);
    const dismissIndex = body.indexOf('"value": "dismiss"', askIndex);

    expect(bridgeIndex).toBeGreaterThan(-1);
    expect(configGetIndex).toBeGreaterThan(bridgeIndex);
    expect(caseIndex).toBeGreaterThan(configGetIndex);
    expect(dismissCaseIndex).toBeGreaterThan(caseIndex);
    expect(configSetIndex).toBeGreaterThan(dismissCaseIndex);
    expect(explainCaseIndex).toBeGreaterThan(configSetIndex);
    expect(continueCaseIndex).toBeGreaterThan(explainCaseIndex);
    expect(askIndex).toBeGreaterThan(configGetIndex);
    expect(dismissIndex).toBeGreaterThan(continueIndex);
    expect(body).toContain("Non-interactive");
    expect(body).toContain("preferences 는 바꾸지 않아요");
  });

  test("token freshness gate runs after preview and before consent-mint", () => {
    const body = deploySkill();
    const previewIndex = body.indexOf("3. **Render preview card via AskUserQuestion**");
    const gateIndex = body.indexOf("3.5. **Token freshness gate");
    // sh/ps1-absorption Phase 4 (T8): SKILL now calls `axhub-helpers token-gate`
    // directly. Legacy `bash hooks/token-freshness-gate.sh` invocation removed;
    // the shim file still exists for backward compat but the SKILL no longer
    // references it.
    const helperIndex = body.indexOf('"$HELPER" token-gate', gateIndex);
    const consentIndex = body.indexOf("axhub-helpers consent-mint");

    expect(previewIndex).toBeGreaterThan(-1);
    expect(gateIndex).toBeGreaterThan(previewIndex);
    expect(helperIndex).toBeGreaterThan(gateIndex);
    expect(consentIndex).toBeGreaterThan(helperIndex);
  });

  test("cli_too_new registry default continues without mutating preferences", () => {
    const registry = deployRegistry();
    const entry = registry["axhub CLI 가 더 최신 버전인데 계속할까요?"];
    expect(entry.safe_default).toBe("계속해요");
    expect(entry.allowed_safe_defaults).toContain("이 버전부터는 묻지 마요");
    expect(entry.rationale).toContain("without mutating user preferences");
  });
});
