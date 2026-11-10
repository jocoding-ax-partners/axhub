import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { DEFAULT_MAX_TOTAL_BYTES } from "../scripts/check-plugin-context-budget.ts";

const ROOT = join(import.meta.dir, "..");
const SKILL_ROOTS = ["skills", "plugins/axhub/skills", "plugins/axhub-codex/skills"] as const;
const DIET = [
  "setup",
  "bootstrap",
  "plugins",
  "deploy",
  "up",
  "import",
  "development",
  "diagnosis",
  "clarity",
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

// bootstrap(옛 scaffold 통합판)은 서버 판정 하나로 Axhub 저장소·내 GitHub·회사 GitHub 조직을 모두 맡아요.
// 판정이 없는 구 서버용 `--git-backend` 분기·init-resume 이어하기는 없앴어요.
describe("spec 236 self-hosted git skill contracts", () => {
  test("T144 positive: bootstrap reads the company decision before any GitHub command", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const decision = bootstrap.indexOf("axhub apps git-backend --tenant <회사> --json");
      expect(decision, root).toBeGreaterThanOrEqual(0);
      expect(decision, root).toBeLessThan(bootstrap.indexOf("axhub github link --json"));
      expect(decision, root).toBeLessThan(bootstrap.indexOf("axhub github accounts list --json"));
      const githubOnly = sliceSection(bootstrap, "### 4. GitHub 일 때만", "### 5. 이름");
      expect(githubOnly, root).toContain("Axhub 저장소면 건너뛰어요. GitHub 연결·앱 설치 이야기는 꺼내지 않아요.");
    }
  });

  test("bootstrap carries the selected workspace through setup, clone, and follow-up", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const clone = sliceSection(bootstrap, "### 8. 코드 받기", "### 9. 배포 확인");
      expect(clone, root).toContain("axhub --tenant <회사> git setup --json");
      expect(clone, root).toContain("axhub --tenant <회사> repo clone <slug> --json");
      expect(clone, root).toContain("받은 폴더를 이후 작업 폴더로 써요");
    }
  });

  test("new storage decision is capability-gated and old CLIs are told to update", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const deploy = readSkill(root, "deploy");

      expect(bootstrap, root).toContain("`capabilities.self_hosted_git.git_policy_decision` 이 true 가 아니면 CLI 를 업데이트해 달라고 하고 멈춰요");
      expect(deploy, root).toContain("capabilities.self_hosted_git.app_git_backend");
      expect(deploy, root).toContain("capabilities.self_hosted_git.git_setup");
      expect(deploy, root).toContain("capabilities.self_hosted_git.repo_clone");
      expect(deploy, root).toContain("axhub CLI를 최신 버전으로 업데이트해 주세요.");
      expect(deploy, root).toContain("malformed/false");
    }
  });

  test("T144 negative: GitHub bootstrap keeps the account, owner, and app-install path", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      expect(bootstrap, root).toContain("axhub github accounts list --json");
      expect(bootstrap, root).toContain("`install_url`");
      expect(bootstrap, root).toContain("`--github-owner <계정>`");
      expect(bootstrap, root).toContain("\"어느 GitHub 계정에 둘까요?\"");
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
      const deploy = readSkill(root, "deploy");
      const selfhosted = sliceSection(deploy, "### Self-hosted repository lane", "### GitHub and upload lanes");
      const details = readFileSync(join(ROOT, root, "deploy", "references", "workflow-details.md"), "utf8");
      const branch = sliceSection(details, "## Self-hosted push branch", "## Git readiness");
      for (const source of [selfhosted, branch]) {
        expect(source, root).toContain("repository.credential_missing");
        expect(source, root).toContain("repository.destination_exists");
        expect(source, root).toContain("already_cloned: true");
        // setup 뒤에도 credential_missing 이면 예상 밖 실패예요. 'report' 만으로는 리포트가 안 일어나니
        // axhub feedback 을 명시해요.
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
      expect(fixes, root).toMatch(/subdomain_not_configured(?::fail`|` 항목이 fail).*push (?:대신|하지 않고).*Recovery Summary.*주소부터/);
      // The address guard must qualify the push step, before the next check.
      const pushStep = sliceSection(fixes, "④", "⑤");
      expect(pushStep, root).toContain("subdomain_not_configured");
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


  test("related routes do not confuse selfhosted Git with repositoryless upload", () => {
    for (const root of SKILL_ROOTS) {
      const up = readSkill(root, "up");
      expect(up, root).toContain("`GitHub 없이`만으로 repositoryless upload를 확정하지 않아요");
      expect(up, root).toContain("`selfhosted`면 `deploy`의 self-hosted repository lane");
    }
  });

  test("selfhosted visible copy contains no provider-login vocabulary", () => {
    const forbidden = /GitHub|device|install|설치/i;
    for (const root of SKILL_ROOTS) {
      // bootstrap 은 selfhosted 전용 대사 없이 GitHub 단계를 통째로 건너뛰어요(위 T144 positive).
      const deploy = readSkill(root, "deploy");
      const visibleCopy = [
        deploy.match(/After push exit 0, say only `([^`]+)`/)?.[1],
      ];
      expect(visibleCopy.every(Boolean), root).toBe(true);
      expect(visibleCopy.join("\n"), root).not.toMatch(forbidden);
    }
  });

  test("policy pins CLI-only backend selection and the 9-skill diet", () => {
    const agentPolicy = readFileSync(join(ROOT, "docs", "policy", "agent-policy.md"), "utf8");
    const devPolicy = readFileSync(join(ROOT, "docs", "policy", "dev-policy.md"), "utf8");
    expect(agentPolicy).toContain("## AP-23 app git backend 선판정");
    expect(agentPolicy).toContain("top-level `git_backend.backend`·`git_backend.source`");
    expect(agentPolicy).toContain("Gitea API·C1 HTTP·remote URL");
    expect(devPolicy).toContain("공개 skill 은 9개");
    // DP-5 의 합산 예산은 게이트 상수와 같은 값이어야 해요 — 상수만 올리고 정책을 두면 기준 문서가 어긋나요.
    expect(devPolicy).toContain(`SKILL.md 9개 합산 ${DEFAULT_MAX_TOTAL_BYTES.toLocaleString("en-US")}B`);
  });

  test("source and both generated surfaces keep the 9-skill diet", () => {
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

describe("spec 236 2차: import follows the code storage location", () => {
  test("bootstrap owns GitHub repositories itself and never yields to scaffold", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      expect(bootstrap, root).not.toContain("scaffold");
      expect(bootstrap, root).toContain("회사 정책에 맞춰 코드를 Axhub 저장소나 GitHub(내 계정·회사 조직)에 둬요");
    }
  });

  // 2026-11 import 개편: 저장 위치는 bootstrap 3·4단계와 같은 정책으로 정하고 고른 key 를 CLI 에 넘겨요.
  // 옛 `--git-backend` 분기·`capabilities.import.selfhosted` 가드·`코드 저장 위치:` 줄은 의도적으로 뺐어요.
  test("import follows bootstrap's storage policy and hands the pick to the CLI", () => {
    for (const root of SKILL_ROOTS) {
      const importSkill = readSkill(root, "import");
      expect(importSkill, root).toContain("`capabilities.self_hosted_git.git_policy_decision`");
      expect(importSkill, root).toContain("앱 서비스 권한·저장 위치·GitHub 연결은 bootstrap 스킬 3·4단계와 똑같이 해요");
      expect(importSkill, root).toContain("--git-choice <key>");
      expect(importSkill, root).toContain("`--github-owner <계정>`");
      expect(importSkill, root).not.toContain("--git-backend");
      expect(importSkill, root).toContain("Axhub 저장소로 올리는 실행은 홈 폴더에 git 자격증명을 써야 해서 처음부터 권한 상승으로 실행해요");
    }
  });
});

// self-hosted 작업공간의 static 앱은 업로드 release 가 아니라 앱 저장소 push → webhook deployment 예요.
// 저장소에는 seed 커밋만 있어 업로드로 배포하던 로컬 소스를 clone 으로 버리면 seed 를 다시 배포해요.
describe("self-hosted static deploys by git push", () => {
  test("deploy routes self-hosted static to the push lane and only GitHub static to the upload lane", () => {
    for (const root of SKILL_ROOTS) {
      const deploy = readSkill(root, "deploy");
      const staticBranch = sliceSection(deploy, "### Static branch", "### Deployment-record branch");
      const selfhosted = sliceSection(deploy, "### Self-hosted repository lane", "### GitHub and upload lanes");
      const never = deploy.slice(deploy.indexOf("## NEVER"));

      expect(staticBranch, root).toContain("`selfhosted` apps (static too) use the self-hosted lane below");
      expect(staticBranch, root).toContain("Only GitHub-backed `deploy_method=static` (`legacy_github` too) enters the static upload lane");
      expect(selfhosted, root).toContain("`git_backend.backend=selfhosted`, static included");
      expect(selfhosted, root).toContain("`apps static deploy`");
      expect(selfhosted, root).toContain("static이면 `capabilities.import.selfhosted_static`도요");
      expect(selfhosted, root).toContain("(upload 대체 금지)");
      expect(never, root).toContain("Self-hosted static is deployment-record based");
      expect(never, root).toContain("NEVER send non-static or `selfhosted` apps to the static upload lane");
    }
  });

  test("an unattached local source folder goes to import instead of repo clone", () => {
    for (const root of SKILL_ROOTS) {
      const deploy = readSkill(root, "deploy");
      const selfhosted = sliceSection(deploy, "### Self-hosted repository lane", "### GitHub and upload lanes");
      const probe = selfhosted.indexOf("axhub repo clone <app> . --json");
      expect(probe, root).toBeGreaterThan(-1);
      const local = selfhosted.slice(probe);
      expect(local, root).toContain("`already_cloned: true` → it is the clone; stay");
      expect(local, root).toContain("`repository.destination_exists` → not attached");
      expect(local, root).toContain("never clone elsewhere");
      expect(local, root).toContain("Hand off to `import` with `--app <app>` (static: `--deploy-method static`)");
      expect(selfhosted, root).toContain("Only an empty or unrelated folder clones");

      const details = readFileSync(join(ROOT, root, "deploy", "references", "workflow-details.md"), "utf8");
      const branch = sliceSection(details, "## Self-hosted push branch", "## Git readiness");
      expect(branch, root).toContain("axhub repo clone <app> . --json");
      expect(branch, root).toContain("Do not clone into another folder");
      expect(branch, root).toContain("Hand off to `import` for this existing app with `--app <app>`");
      expect(branch, root).not.toMatch(/git remote (add|set-url)/);
    }
  });

  test("self-hosted static is gated and aligns axhub.yaml before the push", () => {
    for (const root of SKILL_ROOTS) {
      const deploy = readSkill(root, "deploy");
      const fixes = sliceSection(deploy, "5. **맞추기", "6. **배포**");
      expect(fixes, root).toMatch(/self-hosted static\s*은 `axhub.yaml`\s*의 `build.deploy_method: static`·`build.static_output_dir`\s*도/);
      expect(fixes.indexOf("build.static_output_dir"), root).toBeLessThan(fixes.indexOf("④"));

      const details = readFileSync(join(ROOT, root, "deploy", "references", "workflow-details.md"), "utf8");
      const branch = sliceSection(details, "## Self-hosted push branch", "## Git readiness");
      expect(branch, root).toContain("`capabilities.import.selfhosted_static === true`");
      expect(branch, root).toContain("never fall back to the upload lane");
      expect(branch, root).toContain("`.` when `index.html` sits at the project root");
      expect(branch, root).toContain("build output folders need not be committed");
      const staticLane = sliceSection(details, "## App backend and static lane", "## Self-hosted push branch");
      expect(staticLane, root).toContain("Only `deploy_method=static` with `git_backend.backend=github`");
    }
  });

});

// spec 706: 서버가 회사 모드로 정한 저장 위치(`git_backend.decision`)를 플러그인은 따르기만 해요.
// 판정이 없는 구 서버 규칙(추천값·`remembered`·GitHub/Axhub self-hosted 질문)은 그 뒤에 남아요.
describe("spec 706: skills follow the server code storage decision", () => {
  test("bootstrap follows use/ask/link_github and passes the pick with --git-choice", () => {
    for (const root of SKILL_ROOTS) {
      const bootstrap = readSkill(root, "bootstrap");
      const gate = sliceSection(bootstrap, "### 3. 회사 정책", "### 4. GitHub 일 때만");
      expect(bootstrap, root).toContain("capabilities.self_hosted_git.git_policy_decision");
      expect(gate, root).toContain("`git_backend.decision`");
      expect(gate, root).toContain("`use`: 묻지 않고 그 선택지를 써요.");
      expect(gate, root).toContain("\"코드를 어디에 저장할까요?\"");
      expect(gate, root).toContain("`options` 순서 그대로");
      expect(gate, root).toContain("`link_github`: 아래 GitHub 계정 연결부터 하고 다시 조회해요.");
      expect(gate, root).toContain("없으면 이 회사는 그 위치를 쓸 수 없다고 알리고 멈춰요");
      expect(gate, root).toContain("`app_platform.enabled` 가 false 면");
      expect(bootstrap, root).toContain("axhub apps bootstrap --git-choice <key>");
      expect(bootstrap, root).not.toContain("--git-backend");
    }
  });

  test("no skill tells a selfhosted-only or unlinked person that a GitHub account is required", () => {
    for (const root of SKILL_ROOTS) {
      for (const name of ["bootstrap", "import"]) {
        expect(readSkill(root, name), `${root}/${name}`).not.toContain("GitHub 계정이 필요해요");
      }
    }
  });
});
