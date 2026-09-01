import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const SKILL_ROOTS = ["skills", "plugins/axhub/skills", "plugins/axhub-codex/skills"] as const;
const DIET = [
  "onboarding",
  "bootstrap",
  "scaffold",
  "plugins",
  "deploy",
  "up",
  "import",
  "development",
  "diagnosis",
  "clarity",
  "update",
] as const;

const readSkill = (root: (typeof SKILL_ROOTS)[number], skill: string): string =>
  readFileSync(join(ROOT, root, skill, "SKILL.md"), "utf8");

const sliceSection = (source: string, start: string, end: string): string => {
  const from = source.indexOf(start);
  const to = source.indexOf(end, from + start.length);
  expect(from, `missing section start: ${start}`).toBeGreaterThanOrEqual(0);
  expect(to, `missing section end: ${end}`).toBeGreaterThan(from);
  return source.slice(from, to);
};

describe("per-app git backend bootstrap contracts", () => {
  test("fresh bootstrap lets the user choose a backend and passes it explicitly", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const backendGate = sliceSection(bootstrap, "### 6. Git Backend Gate", "### 7. Availability Check");
      const dryRun = sliceSection(bootstrap, "### 8. Dry-Run Preview", "### 9. Execute Bootstrap Saga");
      const execute = sliceSection(bootstrap, "### 9. Execute Bootstrap Saga", "### 9.1 Desktop Error Recovery");
      const resume = readFileSync(join(ROOT, root, "bootstrap", "references", "resume-and-tenant.md"), "utf8");

      expect(backendGate, root).toContain("tenant backend는 추천값");
      expect(backendGate, root).toContain("이 앱의 코드 저장 위치를 선택해 주세요.");
      expect(backendGate, root).toContain(
        root === "plugins/axhub-codex/skills"
          ? "native Question/명시 텍스트 승인 card"
          : "native Question/AskUserQuestion card",
      );
      expect(backendGate, root).toContain("`GitHub`와 `Axhub self-hosted`");
      expect(backendGate, root).toContain("사용자가 발화에서 backend를 명시했다면 그 값을 써요");
      expect(dryRun, root).toContain("--git-backend selfhosted");
      expect(dryRun, root).toContain("--git-backend github");
      expect(execute, root).toContain("--git-backend selfhosted");
      expect(execute, root).toContain("--git-backend github");
      expect(resume, root).toContain("persisted `git_backend.backend`");
      expect(resume, root).toContain("코드 저장 위치를 다시 선택하지 않아요");
    }
  });
});

describe("spec 236 self-hosted git skill contracts", () => {
  test("T144 positive: bootstrap resolves selfhosted before every repository-provider prompt", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const backendGate = sliceSection(bootstrap, "### 6. Git Backend Gate", "### 7. Availability Check");

      expect(backendGate, root).toContain("axhub apps get <app> --json");
      expect(backendGate, root).toContain("axhub apps git-backend --tenant <tenant> --json");
      expect(backendGate, root).toContain("`git_backend.backend=selfhosted`");
      expect(backendGate, root).toContain("tenant 응답 source는 `tenant|platform_default`");
      expect(backendGate, root).toContain("`references/templates-and-github.md` 전체를 읽지 않아요");
      expect(backendGate, root).toContain("계정 인증·저장소 App 설치 질문을 0회로 유지해요");
    }
  });

  test("bootstrap carries the selected workspace through setup, clone, and repository-local follow-up", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const clone = sliceSection(bootstrap, "### 10. Clone And Manifest", "### 11. Result");
      const reference = readFileSync(join(ROOT, root, "bootstrap", "references", "bootstrap-and-local.md"), "utf8");

      for (const source of [clone, reference]) {
        expect(source, root).toContain("axhub --tenant <tenant> git setup --json");
        expect(source, root).toContain("axhub --tenant <tenant> repo clone <app-slug> --json");
        expect(source, root).toContain("`data.destination`");
        expect(source, root).toContain("repository-local 후속 tool call의 `cwd`");
      }
    }
  });

  test("new selfhosted surfaces are capability-gated and old CLIs route to update", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const deploy = readSkill(root, "deploy");
      const onboarding = readSkill(root, "onboarding");

      expect(bootstrap, root).toContain("capabilities.self_hosted_git.apps_git_backend");
      expect(bootstrap, root).toContain("capabilities.self_hosted_git.app_git_backend");
      expect(deploy, root).toContain("capabilities.self_hosted_git.app_git_backend");
      expect(deploy, root).toContain("capabilities.self_hosted_git.git_setup");
      expect(deploy, root).toContain("capabilities.self_hosted_git.repo_clone");
      expect(onboarding, root).toContain("capabilities.self_hosted_git.apps_git_backend");
      expect(onboarding, root).toContain("capabilities.self_hosted_git.app_git_backend");
      expect(onboarding, root).toContain("capabilities.self_hosted_git.git_setup");
      for (const skill of [bootstrap, deploy, onboarding]) {
        expect(skill, root).toContain("axhub CLI를 최신 버전으로 업데이트해 주세요.");
        expect(skill, root).toContain("malformed/false");
      }
    }
  });

  test("T144 negative: GitHub bootstrap keeps the existing account, preview, and execute path", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      expect(bootstrap, root).toContain("axhub github accounts list --json");
      expect(bootstrap, root).toContain("--github-owner realitsyourman");
      expect(bootstrap, root).toContain("GitHub App 설치를 끝냈을까요?");
    }
  });

  test("T144 resume: backend check precedes every emitted status or resume command", () => {
    for (const root of SKILL_ROOTS) {
      const resume = readFileSync(join(ROOT, root, "bootstrap", "references", "resume-and-tenant.md"), "utf8");
      const gate = resume.indexOf("Before showing the resume question or running either");
      expect(gate, root).toBeGreaterThanOrEqual(0);
      expect(resume, root).toContain("`watch_status`이거나 state에 `bootstrap_id`가 있으면");
      expect(resume, root).toContain("`resume_last`이고 `bootstrap_id`가 없으면");
      expect(resume, root).toContain("axhub apps git-backend --tenant <tenant> --json");
      expect(resume.indexOf("- `watch_status`: run `args.status_command`"), root).toBeGreaterThan(gate);
      expect(resume.indexOf("- `resume_last`: use `args.resume_command`"), root).toBeGreaterThan(gate);
      expect(resume, root).toContain("If `git_backend.backend=selfhosted`, discard a stale provider-auth `resume_last` route");
      expect(resume, root).toContain("may be entered only after the Resume Route check confirmed");
    }
  });

  test("T145 positive: deploy clones an un-cloned selfhosted app, pushes, and waits for its webhook deployment", () => {
    for (const root of SKILL_ROOTS) {
      const deploy = readSkill(root, "deploy");
      const selfhosted = sliceSection(deploy, "### Self-hosted repository lane", "### GitHub and upload lanes");

      expect(selfhosted, root).toContain("axhub apps get <app> --json");
      expect(selfhosted, root).toContain("axhub repo clone <app> --json");
      expect(selfhosted, root).toContain("`data.destination`");
      expect(selfhosted, root).toContain("tool `cwd`");
      expect(selfhosted, root).toContain("same returned `data.destination` cwd");
      expect(selfhosted, root).toContain("git push -u origin \"HEAD:$BRANCH\"");
      expect(selfhosted, root).toContain("webhook 자동 배포");
      expect(selfhosted, root).not.toContain("axhub up");
    }
  });

  test("T145 negative: GitHub deploy remains intact and repository-less deploy delegates to up", () => {
    for (const root of SKILL_ROOTS) {
      const deploy = readSkill(root, "deploy");
      const github = sliceSection(deploy, "### GitHub and upload lanes", "### Verify loop");
      expect(github, root).toContain("axhub deploy create");
      expect(github, root).toContain("이 lane 의 절차는 `up` 스킬이 소유해요");
      expect(github, root).toContain("skills/up/SKILL.md");
      expect(github, root).toContain("github_connected");
    }
  });
  test("T146 positive: onboarding suppresses provider auth for tenant-default selfhosted apps", () => {
    for (const root of SKILL_ROOTS) {
      const onboarding = readSkill(root, "onboarding");
      const backendGate = sliceSection(onboarding, "### 2. Git backend gate", "### 3. Repository provider surface");

      const cliReadiness = backendGate.indexOf("`cli_missing`·`cli_path_missing`·`cli_old`·`auth_missing`");
      expect(cliReadiness, root).toBeGreaterThanOrEqual(0);
      expect(cliReadiness, root).toBeLessThan(backendGate.indexOf("axhub plugin-support preflight --json"));
      expect(backendGate, root).toContain("CLI·auth gap을 닫기 전에는 preflight와 backend read를 절대 실행하지 않아요");
      expect(backendGate, root).toContain("axhub apps get <app> --json");
      expect(backendGate, root).toContain("axhub apps git-backend --tenant <tenant> --json");
      expect(backendGate, root).toContain("`git_backend.backend=selfhosted`");
      expect(backendGate, root).toContain("tenant source는 `tenant|platform_default`");
      expect(backendGate, root).toContain("`github_link_missing`·`github_app_missing`을 처리하지 않아요");
      expect(backendGate, root).toContain("계정 로그인·App 설치 대사를 0회로 유지해요");
    }
  });

  test("T146 negative: GitHub onboarding keeps account linking and App installation", () => {
    for (const root of SKILL_ROOTS) {
      const onboarding = readSkill(root, "onboarding");
      expect(onboarding, root).toContain("axhub github accounts list --json");
      expect(onboarding, root).toContain("github_link_missing");
      expect(onboarding, root).toContain("github_app_missing");
    }
  });

  test("selfhosted visible copy contains no provider-login vocabulary", () => {
    const forbidden = /GitHub|device|install|설치/i;
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const deploy = readSkill(root, "deploy");
      const readyCard = readFileSync(join(ROOT, root, "onboarding", "references", "ready-card.md"), "utf8");
      const visibleCopy = [
        bootstrap.match(/selfhosted 사용자-facing 대사는 `([^`]+)`/)?.[1],
        deploy.match(/After push exit 0, say only `([^`]+)`/)?.[1],
        readyCard.match(/For selfhosted, render only `([^`]+)`/)?.[1],
      ];
      expect(visibleCopy.every(Boolean), root).toBe(true);
      expect(visibleCopy.join("\n"), root).not.toMatch(forbidden);
    }
  });

  test("policy pins CLI-only backend selection and the 11-skill diet", () => {
    const agentPolicy = readFileSync(join(ROOT, "docs", "policy", "agent-policy.md"), "utf8");
    const devPolicy = readFileSync(join(ROOT, "docs", "policy", "dev-policy.md"), "utf8");
    expect(agentPolicy).toContain("## AP-23 app git backend 선판정");
    expect(agentPolicy).toContain("top-level `git_backend.backend`·`git_backend.source`");
    expect(agentPolicy).toContain("Gitea API·C1 HTTP·remote URL");
    expect(devPolicy).toContain("공개 skill 은 11개");
    expect(devPolicy).toContain("SKILL.md 11개 합산 246,000B");
  });

  test("source and both generated surfaces keep the 11-skill diet", () => {
    const expected = [...DIET].sort();
    for (const root of SKILL_ROOTS) {
      const actual = readdirSync(join(ROOT, root), { withFileTypes: true })
        .filter((entry) => entry.isDirectory())
        .map((entry) => entry.name)
        .sort();
      expect(actual, root).toEqual(expected);
    }
  });
});
