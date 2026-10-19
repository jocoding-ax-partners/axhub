import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { DEFAULT_MAX_TOTAL_BYTES } from "../scripts/check-plugin-context-budget.ts";

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
      expect(backendGate, root).toContain("발화나 같은 대화에 backend가 있으면 쓰고");
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
      expect(selfhosted, root).toContain('git -c credential.interactive=false push -u origin "HEAD:$BRANCH"');
      expect(selfhosted, root).toContain("webhook 자동 배포");
      expect(selfhosted, root).not.toContain("axhub up");
    }
  });

  test("T145 credential: deploy repairs the git helper before every push and never lets git prompt", () => {
    for (const root of SKILL_ROOTS) {
      const deploy = readSkill(root, "deploy");
      const selfhosted = sliceSection(deploy, "### Self-hosted repository lane", "### GitHub and upload lanes");
      const setup = selfhosted.indexOf("axhub --tenant <tenant> git setup --json");
      const push = selfhosted.indexOf("push -u origin");
      expect(setup, root).toBeGreaterThanOrEqual(0);
      expect(setup, root).toBeLessThan(push);
      expect(selfhosted, root).not.toContain("If the current folder is not yet the resolved app's clone, prepare");
      expect(selfhosted, root).toContain('git -c credential.interactive=false push -u origin "HEAD:$BRANCH"');
      expect(selfhosted, root).not.toMatch(/GIT_TERMINAL_PROMPT=|GCM_INTERACTIVE=/);
      expect(selfhosted, root).toContain("axhub --tenant <tenant> git setup --rotate");
      expect(selfhosted, root).toContain("폴더가 이미 clone이어도 push 전마다");
      expect(selfhosted, root).not.toContain("clone이 아닐 때만");
      expect(selfhosted, root).toContain("told us to quit");
      expect(selfhosted, root).toContain("unable to get password from user");
      expect(selfhosted, root).toContain("`capabilities.self_hosted_git.git_setup_rotate`가 `true`여야 하고");
      const globalGuard = deploy.slice(0, deploy.indexOf("### Self-hosted repository lane"));
      expect(globalGuard, root).not.toContain("git_setup_rotate");
      const details = readFileSync(join(ROOT, root, "deploy", "references", "workflow-details.md"), "utf8");
      expect(details, root).toContain('git -c credential.interactive=false push -u origin "HEAD:$BRANCH"');
      expect(details, root).not.toMatch(/GIT_TERMINAL_PROMPT=|GCM_INTERACTIVE=/);
      expect(details, root).toContain('if [ "$GIT_BACKEND" = "selfhosted" ]; then');
      expect(details, root).toContain("Before the push, run `axhub --tenant <tenant> git setup --json`");
      expect(details, root).not.toContain("older CLIs rotate");
    }
  });

  // #455: clone 실패가 타입 있는 exit 9 로 오고, 같은 저장소면 성공(already_cloned)이에요. 네 곳 모두 hint 대로 다뤄요.
  test("clone results follow the hint: credential_missing, destination_exists, already_cloned (#455)", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const clone = sliceSection(bootstrap, "### 10. Clone And Manifest", "### 11. Result");
      const reference = readFileSync(join(ROOT, root, "bootstrap", "references", "bootstrap-and-local.md"), "utf8");
      const deploy = readSkill(root, "deploy");
      const selfhosted = sliceSection(deploy, "### Self-hosted repository lane", "### GitHub and upload lanes");
      const details = readFileSync(join(ROOT, root, "deploy", "references", "workflow-details.md"), "utf8");
      const branch = sliceSection(details, "## Self-hosted push branch", "## Git readiness");
      for (const source of [clone, reference, selfhosted, branch]) {
        expect(source, root).toContain("repository.credential_missing");
        expect(source, root).toContain("repository.destination_exists");
        expect(source, root).toContain("already_cloned: true");
        // setup 뒤에도 credential_missing 이면 예상 밖 실패예요. exit 9 는 AP-19 의 '예상된 거절'이라
        // 'report' 만으로는 리포트가 안 일어나니 axhub feedback 을 명시해요.
        expect(source, root).toContain("axhub feedback");
        // hint 의 `next:` 명령은 clone 이 고정한 --profile·--endpoint(쓴 경우 --config·--no-keychain)를 담아요.
        // 스킬이 `axhub --tenant <tenant> git setup` 을 직접 조립하면 helper 가 읽지 않는 곳에 자격 증명을
        // 저장해 credential_missing 이 되풀이될 수 있어요. CLI 계약대로 `next:` 명령을 적힌 그대로 실행해요.
        const credential = source.slice(
          source.indexOf("repository.credential_missing"),
          source.indexOf("repository.destination_exists"),
        );
        expect(credential, root).toContain("`next:`");
        expect(credential, root).toContain("as printed");
        expect(credential, root).not.toContain("git setup");
      }
      // `next:` 가 없는 hint(자격 증명 저장소를 못 읽는 경우 등)는 setup 으로 풀리지 않아요.
      expect(branch, root).toContain("without `next:`");
      expect(reference, root).toContain("without `next:`");
    }
  });

  // #832: 주소 없는 앱에 push 하면 webhook 이 조용히 건너뛰어 배포가 시작되지 않아요.
  test("an app without an address sets it before the self-hosted push (#832)", () => {
    for (const root of SKILL_ROOTS) {
      const deploy = readSkill(root, "deploy");
      const selfhosted = sliceSection(deploy, "### Self-hosted repository lane", "### GitHub and upload lanes");
      // deploy check 항목 code 로 막아요 — `deploy.subdomain_not_configured` 는 error.subcode 예요.
      const gate = selfhosted.indexOf("`subdomain_not_configured`");
      expect(gate, root).toBeGreaterThan(-1);
      expect(gate, root).toBeLessThan(selfhosted.indexOf("push -u origin"));
      const recovery = sliceSection(deploy, "## Recovery Summary", "## NEVER");
      expect(recovery, root).toContain("`subdomain_not_configured`");
      expect(recovery, root).toContain("axhub apps update <app> --subdomain <address> --json");
      const details = readFileSync(join(ROOT, root, "deploy", "references", "workflow-details.md"), "utf8");
      expect(details, root).toContain("deploy.subdomain_not_configured");
      expect(details, root).toContain("Headless never sets an address");
    }
  });

  // #832: lane 보다 먼저 도는 `## 배포 전 맞추기` 의 ④ push 도 주소가 없으면 하지 않아요. 주소는 AP-29 의
  // 맞출 일이 아니라 Recovery 질문이에요. 주소 없이 이미 원격에 올라간 커밋은 다시 push 하지 않아요 —
  // webhook 은 그 push 를 건너뛰었지만, backend 의 code_repo_state_reconcile 이 매분 기본 브랜치 tip 에
  // 배포가 없으면 놓친 webhook 을 다시 재생해서 주소를 정하면 1분 안에 배포가 생겨요. 그 배포를 deploy list 로
  // 찾아 verify 해요. self-hosted 앱은 저장소 기본 브랜치만 배포해요 — git 연결 브랜치가 기본 브랜치로 고정이라
  // 다른 브랜치 push 는 주소와 무관하게 webhook 이 건너뛰어요(branch split 은 플랫폼 앱 전용). 그래서 '다음 push 가
  // 배포' 안내는 어느 경우에도 틀려요. `git ls-remote` 가 실패하면 기본 브랜치가 아니라고 보지 않고 reconcile
  // 조회로 가요(최악이 pending 이에요).
  test("the common pre-deploy push waits for the address, and a commit already on the remote is found via reconcile and verified (#832)", () => {
    for (const root of SKILL_ROOTS) {
      const deploy = readSkill(root, "deploy");
      const prep = sliceSection(deploy, "## 배포 전 맞추기 (AP-29)", "## Headless Contract");
      const fixes = sliceSection(prep, "5. **맞추기", "6. **배포**");
      const gate = fixes.indexOf("`subdomain_not_configured` 항목이 fail 이면 push 하지 않고 Recovery Summary 대로 주소부터 정해요");
      expect(gate, root).toBeGreaterThan(fixes.indexOf("④"));
      expect(gate, root).toBeLessThan(fixes.indexOf("⑤"));
      const recovery = sliceSection(deploy, "## Recovery Summary", "## NEVER");
      expect(recovery, root).toContain("never re-push a commit already on the remote");
      expect(recovery, root).toContain("reconcile");
      expect(recovery, root).toContain("`deploy list`");
      expect(recovery, root).not.toContain("next push deploys");
      const details = readFileSync(join(ROOT, root, "deploy", "references", "workflow-details.md"), "utf8");
      const routing = sliceSection(details, "## Error routing", "## Secondary commands");
      const subdomain = sliceSection(routing, "`subdomain_not_configured`", "GitHub connection required");
      expect(subdomain, root).not.toContain("never fires the webhook again");
      expect(subdomain, root).toContain('git merge-base --is-ancestor "$PUSHED_SHA" "origin/$BRANCH"');
      expect(subdomain, root).toContain("reconcile");
      expect(subdomain, root).toContain("git ls-remote --symref origin HEAD");
      expect(subdomain, root).toContain("axhub deploy list --app <app-id> --all --json");
      // reconcile 은 PUSHED_SHA 가 아니라 원격 tip 을 배포해요. push 전 baseline 으로 빼면 먼저 생긴 행을 놓쳐요.
      expect(subdomain, root).toContain('git rev-parse "origin/$BRANCH"');
      expect(subdomain, root).toContain("no pre-push baseline exclusion");
      expect(subdomain, root).toContain("axhub deploy verify <deployment-id> --app <app-id> --json");
      expect(subdomain, root).toContain("Budget expiry is pending");
      const otherBranch = subdomain.indexOf("not the default branch");
      expect(otherBranch, root).toBeGreaterThan(-1);
      expect(subdomain.indexOf("deploys only its repository default branch"), root).toBeGreaterThan(otherBranch);
      expect(subdomain, root).toContain("regardless of the address");
      expect(subdomain, root).not.toContain("next pushed commit deploys");
      expect(subdomain, root).not.toContain("branch split");
      const lsRemoteFailed = subdomain.indexOf("If `git ls-remote` fails");
      expect(lsRemoteFailed, root).toBeGreaterThan(-1);
      expect(subdomain.slice(lsRemoteFailed), root).toContain("`deploy list` lookup");
    }
    const policy = readFileSync(join(ROOT, "docs", "policy", "agent-policy.md"), "utf8");
    const ap29 = sliceSection(policy, "## AP-29", "## AP-30");
    expect(ap29).toContain("`subdomain_not_configured` 항목");
  });

  // #832: deploy check JSON 의 항목은 `data.checks[].code` 가 `subdomain_not_configured` 예요.
  // `deploy.subdomain_not_configured` 는 맨 앞 실패의 `error.subcode` 로만 나와요. 항목 이름으로 쓰면
  // checks[].code 에서 찾는 에이전트가 일치 항목을 못 찾아 통과로 볼 수 있어요. up 스킬도 항목 code 로 불러요.
  test("deploy check names the item by its code and keeps deploy.subdomain_not_configured for error.subcode (#832)", () => {
    const policy = readFileSync(join(ROOT, "docs", "policy", "agent-policy.md"), "utf8");
    const sources: Array<[string, string]> = [["agent-policy AP-29", sliceSection(policy, "## AP-29", "## AP-30")]];
    for (const root of SKILL_ROOTS) {
      sources.push([`${root}/deploy/SKILL.md`, readSkill(root, "deploy")]);
      sources.push([
        `${root}/deploy/references/workflow-details.md`,
        readFileSync(join(ROOT, root, "deploy", "references", "workflow-details.md"), "utf8"),
      ]);
    }
    let subcodeMentions = 0;
    for (const [name, text] of sources) {
      for (const match of text.matchAll(/deploy\.subdomain_not_configured/g)) {
        const at = match.index ?? 0;
        expect(text.slice(Math.max(0, at - 24), at), `${name}: "${text.slice(Math.max(0, at - 60), at + 40)}"`).toContain("subcode");
        subcodeMentions += 1;
      }
    }
    // AP-29 와 세 사본의 workflow-details 는 subcode 를 함께 적어 출력과 대조할 수 있게 해요.
    expect(subcodeMentions).toBeGreaterThanOrEqual(1 + SKILL_ROOTS.length);
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
  test("T146 positive: fresh onboarding asks once and selected selfhosted suppresses provider auth", () => {
    for (const root of SKILL_ROOTS) {
      const onboarding = readSkill(root, "onboarding");
      const backendGate = sliceSection(onboarding, "### 2. Git backend gate", "### 4. first_gap router");

      const cliReadiness = backendGate.indexOf("`cli_missing`·`cli_path_missing`·`cli_old`·`auth_missing`");
      expect(cliReadiness, root).toBeGreaterThanOrEqual(0);
      expect(cliReadiness, root).toBeLessThan(backendGate.indexOf("axhub plugin-support preflight --json"));
      expect(backendGate, root).toContain("CLI·auth gap을 닫기 전에는 preflight와 backend read를 절대 실행하지 않아요");
      expect(backendGate, root).toContain("axhub apps get <app> --json");
      expect(backendGate, root).toContain("axhub apps git-backend --tenant <tenant> --json");
      expect(backendGate, root).toContain("fresh tenant 응답은 추천값");
      expect(backendGate, root).toContain("첫 앱의 코드 저장 위치를 선택해 주세요.");
      expect(backendGate, root).toContain(
        root === "plugins/axhub-codex/skills"
          ? "native Question/명시 텍스트 승인 card"
          : "native Question/AskUserQuestion card",
      );
      expect(backendGate, root).toContain("`GitHub`와 `Axhub self-hosted`");
      expect(backendGate, root).toContain("SELECTED_GIT_BACKEND");
      expect(backendGate, root).toContain("`git_backend.backend=selfhosted`");
      expect(backendGate, root).toContain("tenant source는 `tenant|platform_default`");
      expect(backendGate, root).toContain("`github_link_missing`·`github_app_missing`을 처리하지 않아요");
      expect(backendGate, root).toContain("계정 로그인·App 설치 대사를 0회로 유지해요");
      expect(backendGate, root).toContain("axhub --tenant <tenant> git setup --json");
    }
  });

  test("T146 negative: selected GitHub onboarding keeps account linking and App installation", () => {
    for (const root of SKILL_ROOTS) {
      const onboarding = readSkill(root, "onboarding");
      expect(onboarding, root).toContain("SELECTED_GIT_BACKEND");
      expect(onboarding, root).toContain("axhub github accounts list --json");
      expect(onboarding, root).toContain("github_link_missing");
      expect(onboarding, root).toContain("github_app_missing");
    }
  });

  test("related routes do not confuse selfhosted Git with repositoryless upload or GitHub scaffold", () => {
    for (const root of SKILL_ROOTS) {
      const up = readSkill(root, "up");
      const scaffold = readSkill(root, "scaffold");
      expect(up, root).toContain("`GitHub 없이`만으로 repositoryless upload를 확정하지 않아요");
      expect(up, root).toContain("`selfhosted`면 `deploy`의 self-hosted repository lane");
      expect(scaffold, root).toContain("Axhub self-hosted");
      expect(scaffold, root).toContain("두 요구는 동시에 충족할 수 없어요");
      expect(scaffold, root).toContain("코드 저장 위치를 먼저 선택");
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
    // DP-5 의 합산 예산은 게이트 상수와 같은 값이어야 해요 — 상수만 올리고 정책을 두면 기준 문서가 어긋나요.
    expect(devPolicy).toContain(`SKILL.md 11개 합산 ${DEFAULT_MAX_TOTAL_BYTES.toLocaleString("en-US")}B`);
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

describe("spec 236 2차: scaffold and import follow the code storage location", () => {
  test("scaffold decides the storage location before any GitHub command", () => {
    for (const root of SKILL_ROOTS) {
      const scaffold = readSkill(root, "scaffold");
      const gate = sliceSection(scaffold, "### 1.5 코드 저장 위치", "### 2. GitHub 계정 연동 확인");

      expect(scaffold.indexOf("### 1.5 코드 저장 위치"), root).toBeLessThan(
        scaffold.indexOf("axhub github accounts list --json"),
      );
      expect(scaffold, root).toContain("capabilities.self_hosted_git.apps_git_backend");
      expect(gate, root).toContain("axhub apps git-backend --tenant <tenant> --json");
      expect(gate, root).toContain("GitHub 로 추측하지 않고 멈춰요");
      expect(gate, root).toContain("`코드 저장 위치를 먼저 선택해 주세요.`");
      expect(gate, root).toContain("`내 계정`·`org`·`회사 저장소` 는 provider 명시가 아니에요");
      expect(gate, root).toContain("provider 선택이 명시되지 않은 headless");
      expect(gate, root).toContain("`SELECTED_GIT_BACKEND=selfhosted`");
      expect(gate, root).toContain("`--git-backend selfhosted`");
      expect(gate, root).toContain("GitHub 계정·device flow·App 설치 대사는 0회예요");
      expect(gate, root).not.toContain("axhub github");
      expect(scaffold, root).toContain("NEVER 1.5단계 판정 전에 `axhub github ...` 를 실행하지 않아요");
    }
  });

  test("scaffold hands a GitHub-backed app to import even in a self-hosted tenant", () => {
    for (const root of SKILL_ROOTS) {
      const scaffold = readSkill(root, "scaffold");
      const handoff = sliceSection(scaffold, "### 8. 여기서 한 번 멈춰요", "## NEVER");
      expect(handoff, root).toContain("`--git-backend github`");
    }
  });

  test("bootstrap does not bounce a self-hosted handoff back to scaffold", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      expect(bootstrap, root).toContain("사용자 GitHub 계정/조직 저장소 요청만 scaffold로 양보해요");
      expect(bootstrap, root).not.toContain("사용자 계정/조직 소유 저장소 요청은 scaffold로 양보해요");
    }
  });

  test("import carries the self-hosted lane contract and never guesses GitHub", () => {
    for (const root of SKILL_ROOTS) {
      const importSkill = readSkill(root, "import");
      expect(importSkill, root).toContain("`selfhosted_repo_push`");
      expect(importSkill, root).toContain("`--git-backend github|selfhosted`");
      expect(importSkill, root).toContain("`--git-backend`, `--env-from-dotenv`, `--env-only`, `--json` 뿐이에요");
      expect(importSkill, root).toContain("`capabilities.import.selfhosted !== true`");
      expect(importSkill, root).toContain("GitHub 로 추측하지 않고 멈춰요");
      expect(importSkill, root).toContain("device flow 대사를 0회로 유지");
      expect(importSkill, root).toContain("`코드 저장 위치:` 줄을 그대로 보여줘요");
    }
  });
});
