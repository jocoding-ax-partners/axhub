import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");
const readJson = <T>(path: string): T => JSON.parse(readRepo(path)) as T;

interface PackageLike {
  description: string;
}

interface MarketplaceLike {
  description: string;
  plugins: Array<{ description: string }>;
}

const runShim = (args: string[], env: Record<string, string> = {}) => {
  const caseDir = mkdtempSync(join(tmpdir(), "axhub-shim-"));
  return Bun.spawnSync({
    cmd: [join(REPO_ROOT, "tests/e2e/claude-cli/fixtures/bin/axhub"), ...args],
    env: {
      ...process.env,
      SHIM_CASE_DIR: caseDir,
      ...env,
    },
  });
};

describe("smooth behavior contracts", () => {
  test("public metadata advertises the seven official skills", () => {
    const packageJson = readJson<PackageLike>("package.json");
    const pluginJson = readJson<PackageLike>(".claude-plugin/plugin.json");
    const marketplace = readJson<MarketplaceLike>(".claude-plugin/marketplace.json");

    const descriptions = [packageJson.description, pluginJson.description, marketplace.description, marketplace.plugins[0]?.description ?? ""];
    for (const description of descriptions) {
      expect(description).toContain("ax-hub-cli");
      expect(description).not.toContain("onboarding/init/deploy/cli");
      expect(description).not.toContain("3개 스킬");
    }
    expect(descriptions.join("\n")).toContain("onboarding/init/deploy/development/diagnosis/clarity/update");
  });

  test("docs carry representative journey and exactly three Korean UX samples", () => {
    const readme = readRepo("README.md");
    const agents = readRepo("AGENTS.md");
    const claude = readRepo("CLAUDE.md");

    expect(readme).toContain("첫 셋업 → 앱 생성 → 배포 → 상태 확인");
    expect(agents).toContain("첫 셋업 → 앱 생성 → 배포 → 상태 확인");
    expect(claude).toContain("첫 셋업 → 앱 생성 → 배포 → 상태 확인");
    const flowRows = [
      "| 첫 셋업 | `onboarding` |",
      "| 앱 생성 | `init` |",
      "| 배포 | `deploy` |",
      "| 상태 확인 | `clarity` |",
    ];
    let previousIndex = -1;
    for (const row of flowRows) {
      const index = readme.indexOf(row);
      expect(index, `missing representative flow row: ${row}`).toBeGreaterThan(previousIndex);
      previousIndex = index;
    }


    const sampleLabels = readme.match(/Action-first success|Evidence-balanced failure|Debug-friendly repeated failure/g) ?? [];
    expect(sampleLabels).toHaveLength(3);
  });

  test("skills encode the required guard boundaries", () => {
    const onboarding = readRepo("skills/onboarding/SKILL.md");
    const init = readRepo("skills/init/SKILL.md");
    const deploy = readRepo("skills/deploy/SKILL.md");
    const clarity = readRepo("skills/clarity/SKILL.md");
    const diagnosis = readRepo("skills/diagnosis/SKILL.md");

    expect(onboarding).toContain("axhub plugin-support onboarding-detect --json");
    expect(onboarding).toContain("cli_missing");
    expect(onboarding).toContain("cli_old");
    expect(onboarding).toContain("detect-first");

    expect(init).toContain("axhub apps bootstrap");
    expect(init).toContain("대표 여정에서의 역할");
    expect(init).toContain("raw JSON/stderr");

    expect(deploy).toContain("axhub deploy verify <deployment-id>");
    expect(deploy).toContain("exit 6");
    expect(deploy).toContain("exit 7");
    expect(deploy).toContain("성공을 선언하지 않아요");
    expect(deploy).not.toContain("deploy-approved-run");
    expect(deploy).toContain("canonical workflow");
    expect(deploy).toContain("diagnosis");

    expect(clarity).toContain("공개 표면만");
    expect(clarity).toContain("plugin-support");
    expect(clarity).toContain("탐색·실행 대상이 아니에요");
    expect(clarity).toContain("axhub 에 그 기능은 없어요");
    expect(clarity).toContain("diagnosis");
    expect(clarity).toContain("배포 실패 원인 진단");
    expect(diagnosis).toContain("axhub deploy diagnose");
    expect(diagnosis).toContain("deployment_diagnosis");
    expect(diagnosis).toContain("정상이에요");
    expect(diagnosis).toContain("진단 대상이 아니에요");
    expect(diagnosis).toContain("해결 후보가 있어요");
    expect(diagnosis).toContain("대상을 못 찾았어요");
    expect(diagnosis).toContain("로그인/권한이 필요해요");
    expect(diagnosis).toContain("진단을 못 했어요");
    expect(diagnosis).toContain("재배포·롤백");
    expect(diagnosis).toContain("직접 실행하지 않아요");
    const clarityCodeBlocks = clarity.match(/```(?:bash|sh)?\n[\s\S]*?```/g) ?? [];
    expect(clarityCodeBlocks.join("\n")).not.toContain("axhub plugin-support");
  });

  test("fixture exposes onboarding detect-first contracts", () => {
    const missing = runShim(["plugin-support", "onboarding-detect", "--json"], { AXHUB_FIXTURE_ONBOARDING: "cli_missing" });
    expect(missing.exitCode).toBe(0);
    const missingJson = JSON.parse(missing.stdout.toString()) as { first_gap: string; cli_present: boolean };
    expect(missingJson.first_gap).toBe("cli_missing");
    expect(missingJson.cli_present).toBe(false);

    const old = runShim(["plugin-support", "onboarding-detect", "--json"], { AXHUB_FIXTURE_ONBOARDING: "cli_old" });
    expect(old.exitCode).toBe(0);
    const oldJson = JSON.parse(old.stdout.toString()) as { first_gap: string; cli_too_old: boolean; has_update: boolean };
    expect(oldJson.first_gap).toBe("cli_old");
    expect(oldJson.cli_too_old).toBe(true);
    expect(oldJson.has_update).toBe(true);
  });

  test("fixture exposes deploy verify failed and in-progress contracts", () => {
    const inProgress = runShim(["deploy", "verify", "dep-123"], { AXHUB_FIXTURE_VERIFY: "in_progress" });
    expect(inProgress.exitCode).toBe(6);
    expect(JSON.parse(inProgress.stdout.toString())).toMatchObject({ id: "dep-123", status: "running" });

    const failed = runShim(["deploy", "verify", "dep-123"], { AXHUB_FIXTURE_VERIFY: "failed" });
    expect(failed.exitCode).toBe(7);
    expect(JSON.parse(failed.stdout.toString())).toMatchObject({ id: "dep-123", status: "failed" });
  });

  test("session carry-over handoff contract is wired (Phase 1, instruction-first)", () => {
    const carryover = readRepo("skills/deploy/references/session-carryover.md");
    const init = readRepo("skills/init/SKILL.md");
    const deploy = readRepo("skills/deploy/SKILL.md");
    const clarity = readRepo("skills/clarity/SKILL.md");

    // Shared single-source contract carries all four elements (DRY).
    expect(carryover).toContain("감지 휴리스틱");
    expect(carryover).toContain("Confabulation 가드");
    expect(carryover).toContain("마찰 억제 범위");
    expect(carryover).toContain("D1 헤드리스 가드");
    // Confabulation default: no evidence -> stay silent, never invent.
    expect(carryover).toContain("조회한 적 없으면 carry-over 침묵");
    // Friction suppression must never bypass correctness gates.
    expect(carryover).toContain("accounts list");
    expect(carryover).toContain("owner-pick");
    expect(carryover).toContain("0-install gate");

    // init: evidence-gated carry-over + shared-contract include.
    expect(init).toContain("같은 대화 맥락 이어받기");
    expect(init).toContain("이미 본 것만");
    expect(init).toContain("../deploy/references/session-carryover.md");
    // E4: infer-tables-env also weighs actually-queried resources.
    expect(init).toContain("infer-tables-env 분석은 scaffold 코드뿐 아니라");
    // Confabulation negative guard (PR-gating proxy for the nightly behavioral case):
    // with no evidence, init must go cold and never invent a resource.
    expect(init).toContain("리소스를 지어내지 않아요");
    expect(init).toContain("carry-over 를 주장하지 않아요");
    // M2: gate relaxation suppresses re-narration only, never the gate.
    expect(init).toContain("install-link 를 보여줬으면 재안내는 생략");
    expect(init).toContain("0-install gate 는 맥락과 무관하게 그대로 실행해요");

    // deploy: carry-over applies only AFTER the route gate (no vercel hijack).
    expect(deploy).toContain("route gate 통과 후에만 적용해서 다른 타깃");
    expect(deploy).toContain("references/session-carryover.md");

    // clarity: pure-prose continuation, stays out of plugin-support.
    expect(clarity).toContain("## 다음 단계 이어주기");
    expect(clarity).toContain("이걸로 앱 만들어줘");
    const clarityCodeBlocks = clarity.match(/```(?:bash|sh)?\n[\s\S]*?```/g) ?? [];
    expect(clarityCodeBlocks.join("\n")).not.toContain("axhub plugin-support");
  });
});
