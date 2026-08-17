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
  test("public metadata advertises the eight official skills", () => {
    const packageJson = readJson<PackageLike>("package.json");
    const pluginJson = readJson<PackageLike>(".claude-plugin/plugin.json");
    const marketplace = readJson<MarketplaceLike>(".claude-plugin/marketplace.json");

    const descriptions = [packageJson.description, pluginJson.description, marketplace.description, marketplace.plugins[0]?.description ?? ""];
    for (const description of descriptions) {
      expect(description).toContain("ax-hub-cli");
      expect(description).not.toContain("onboarding/init/deploy/cli");
      expect(description).not.toContain("3개 스킬");
      // description 은 Discover/Installed UI 에 그대로 노출되는 사용자-facing
      // 필드예요 — 라우팅 계약 본문은 스킬 frontmatter·훅이 소유하고, 여기엔
      // 재수록하지 않아요 (간결 유지).
      expect(description.length).toBeLessThanOrEqual(320);
      expect(description).not.toContain("should route to");
      expect(description).not.toContain("Routing priority is strict");
      expect(description).not.toContain("Import priority is strict");
      expect(description).not.toContain("현재 버전을 확인할게요");
    }
    expect(descriptions.join("\n")).toContain("onboarding/bootstrap/deploy/import/development/diagnosis/clarity/update");
  });

  test("docs carry representative journey and exactly three Korean UX samples", () => {
    const readme = readRepo("README.md");
    const agents = readRepo("AGENTS.md");
    const claude = readRepo("CLAUDE.md");

    expect(readme).toContain("첫 셋업 → 앱 생성 → 배포 → 상태 확인");
    expect(agents).toContain("첫 셋업 → 앱 생성 → 배포 → 상태 확인");
    expect(claude).toContain("첫 셋업 → 앱 생성 → 배포 → 상태 확인");
    expect(readme).toContain("Claude Desktop 에 axhub App/MCP 도구가 같이 보여도 플러그인 스킬 흐름은 CLI-only");
    expect(readme).toContain("버전·최신 확인이 들어간 요청은 언제나 `update` 가 먼저 끝나요");
    expect(readme).toContain("존재하지 않는 `axhub app list` 단수 명령을 추측하지 않고 `axhub apps --help`");
    expect(readme).toContain("정확히 `axhub apps list --json` 읽기 전용 명령으로 이어가요");
    expect(readme).toContain("`axhub apps get <app> --json`, `axhub deploy list --app <app> --json`");
    expect(readme).toContain("존재하지 않는 `axhub deployment list`");
    expect(readme).toContain("`| head`, `2>/dev/null`, `grep` 같은 shell 후처리를 붙이지 않아요");
    expect(readme).not.toContain("내 앱들이 지금 어떤 상태인지 모르겠어");
    expect(readme).toContain("`Tenant recent deployments`, `Deployment list`, `App list`, `App get` 같은 App/MCP 도구 권한 팝업으로 빠지지 않고 CLI 계약을 따라요");
    const flowRows = [
      "| 첫 셋업 | `onboarding` |",
      "| 앱 생성 | `bootstrap` |",
      "| 배포 | `deploy` |",
      "| 상태 확인 | `deploy` |",
    ];
    let previousIndex = -1;
    for (const row of flowRows) {
      const index = readme.indexOf(row);
      expect(index, `missing representative flow row: ${row}`).toBeGreaterThan(previousIndex);
      previousIndex = index;
    }


    const sampleLabels = readme.match(/Action-first success|Evidence-balanced progress|Debug-friendly repeated failure/g) ?? [];
    expect(sampleLabels).toHaveLength(3);
    expect(readme).toContain("같은 deployment id 로 계속 확인할게요");
    expect(readme).not.toContain("'배포 상태 확인해줘'라고 말하면 이어서 볼게요");
  });

  test("skills encode the required guard boundaries", () => {
    const onboarding = readRepo("skills/onboarding/SKILL.md");
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const deploy = readRepo("skills/deploy/SKILL.md") + readRepo("skills/deploy/references/user-facing-language.md");
    const clarity = readRepo("skills/clarity/SKILL.md") + readRepo("skills/clarity/references/execution-guardrails.md");
    const clarityFrontmatter = clarity.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
    const diagnosis = readRepo("skills/diagnosis/SKILL.md") + readRepo("skills/diagnosis/references/output-contract.md");
    const importSkill =
      readRepo("skills/import/SKILL.md") +
      readRepo("skills/import/references/visibility-rules.md") +
      readRepo("skills/import/references/manifest-authoring.md");
    const onboardingAuth = readRepo("skills/onboarding/references/install-channels-and-auth.md");
    const bootstrapAndLocal = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const workflowDetails = readRepo("skills/deploy/references/workflow-details.md");
    const deployErrors = readRepo("skills/deploy/references/error-empathy-catalog.md");

    expect(onboarding).toContain("axhub plugin-support onboarding-detect --json");
    expect(onboarding).toContain("cli_missing");
    expect(onboarding).toContain("cli_old");
    expect(onboarding).toContain("detect-first");
    expect(onboarding).toContain("`first_gap`, `gaps`, `cli_state`, `auth_error_code` 같은 detect 필드명이나 enum 값은 내부 라우팅용으로만");
    expect(onboarding).toContain("NEVER `first_gap`, `gaps`, `cli_state`, `auth_error_code` 같은 detect 필드명");
    // Regression: CLI installed on disk but not on PATH (new session, rc not re-sourced)
    // must route to PATH repair, not reinstall. Each -f check stays a standalone
    // Desktop-safe command and remains portable to Git Bash .exe handling.
    expect(onboarding).toContain('test -f "$HOME/.axhub/bin/axhub"');
    expect(onboarding).toContain('test -f "$HOME/.axhub/bin/axhub.exe"');
    expect(onboarding).toContain("내부적으로 `cli_path_missing`");
    expect(onboarding).toContain("무한 루프 방지");
    expect(onboardingAuth).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub auth login --json");
    expect(onboardingAuth).not.toContain("axhub auth login --no-browser --json");
    expect(onboardingAuth).not.toContain("after `승인했어`");
    expect(onboardingAuth).not.toContain("승인했어`, re-detect");

    expect(bootstrap).toContain("axhub apps bootstrap");
    expect(bootstrap).toContain("## Fast Start");
    expect(bootstrap).toContain("CLI 확인과 템플릿 질문까지 바로 진행");
    expect(bootstrap).toContain("raw JSON/stderr");
    expect(bootstrap).toContain("비어 있지 않은 기존 로컬 앱");
    expect(bootstrap).toContain("`import` 스킬로 넘겨요");
    expect(bootstrap).toContain("idempotency key 는 OS별 UUID 생성 명령으로 만들지 말고");
    expect(bootstrap).toContain("`axhub plugin-support init-resume put` 에 생성을 맡겨요");
    expect(bootstrapAndLocal).toContain("APP_SLUG=\"$APP_SLUG\" perl -0pi");
    expect(bootstrap).toContain("url_checked=false");
    expect(bootstrap).toContain("axhub apps check-availability --tenant <tenant> --slug <app-slug> --subdomain <app-slug> --json");
    expect(bootstrap).toContain("Tool 제목은 `앱 주소 확인`");
    expect(bootstrap).toContain("dry-run preview 전 pre-preview guard");
    expect(bootstrap).toContain("slug 또는 subdomain 중 하나라도 unavailable 이면 bootstrap dry-run/execute 금지");
    expect(bootstrap).toContain("내부 라벨 노출 금지");
    expect(bootstrap).toContain("제품명·명령어·영어 단어에 `ing`/`ed` 를 붙인 제목");
    expect(bootstrap).toContain("사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL");
    expect(bootstrap).toContain("Markdown URL 링크 문법은 전부 금지");
    expect(bootstrap).toContain("[https://x](https://x)`, `[열기](https://x)`, `<https://x>`");
    expect(bootstrap).toContain("다른 플러그인/워크플로 상태를 정리하지 않아요");
    expect(bootstrap).toContain("외부 자동화·취소·state 정리 도구를 부르지 않고");
    expect(bootstrap).toContain("chat/tool/progress 에 다른 플러그인 이름, 자동화 정리 문구");
    expect(bootstrap).not.toContain("autopilot");
    expect(bootstrap).not.toContain("oh-my-claudecode");
    expect(bootstrap).toContain('axhub publish --app "$APP_SLUG" --visibility public --execute --json');
    expect(bootstrap).toContain("publish dry-run 을 먼저 호출하지 않고");
    expect(bootstrap).toContain("`Dry-run 기본값` 같은 내부 CLI dry-run semantics");
    expect(bootstrap).toContain("NEVER GitHub device flow code 를 긴 watch tool 안에 숨긴 채");
    expect(bootstrap).toContain("승인 완료를 채팅으로 알려 달라고 쓰지 않아요");
    expect(bootstrapAndLocal).toContain(".data.repo_full_name // .data.status.repo_full_name // empty");
    expect(bootstrapAndLocal).toContain("axhub --no-input apps bootstrap");
    expect(bootstrapAndLocal).toContain("auto_poll");
    expect(bootstrapAndLocal).toContain("Never leave the user staring at an empty GitHub code-entry screen with no code");
    expect(bootstrapAndLocal).toContain("따로 `승인했어`라고 말하지 않아도 돼요");
    expect(bootstrapAndLocal).toContain("do not end the response asking the user to report approval");
    expect(bootstrapAndLocal).not.toContain("브라우저에서 승인한 다음 \"승인했어\"");

    expect(deploy).toContain("axhub deploy verify <deployment-id> --app <app>");
    expect(deploy).toContain("axhub deploy verify \"$DEPLOY_ID\" --app \"$APP_ID\"");
    expect(deploy).toContain("NEVER call `axhub deploy watch`");
    expect(deploy).toContain("bounded `axhub deploy verify \"$DEPLOY_ID\" --app \"$APP_ID\"` loop");
    // AP-16 대기 수단 우선: verify 연타 폴링 대신 CLI 내부 대기 단일 호출.
    expect(deploy).toContain("--wait --wait-interval 20s --wait-timeout 10m --json");
    expect(deploy).toContain("권한 카드 한 번으로 끝나는");
    expect(deploy).toContain("연타 폴링은 UX 실패예요");
    expect(deploy).toContain("capabilities.import.verify_wait");
    expect(deploy).toContain("exit 6");
    expect(deploy).toContain("exit 7");
    expect(deploy).toContain("계속 확인할게요");
    expect(deploy).toContain("Do not add those paths to `.gitignore`");
    expect(deploy).toContain("If a CLI check still blocks solely because of these runtime paths");
    expect(deploy).toContain("do not collapse polling into one long `while`/`for`/`until` shell loop");
    expect(deploy).toContain("Do not end the response by asking the user to say `배포 상태 확인해줘`");
    expect(deploy).toContain("ScheduleWakeup");
    expect(deploy).not.toContain("suggest `배포 상태 확인해줘`");
    expect(deploy).not.toContain("'배포 상태 확인해줘'라고 말하면 이어서 볼게요");
    expect(deploy).toContain("성공을 선언하지 않아요");
    expect(deploy).not.toContain("deploy-approved-run");
    expect(deploy).toContain(".data.id // .data.deployment_id // .id // .deployment_id // empty");
    expect(deploy).toContain("canonical workflow");
    expect(deploy).toContain("diagnosis");
    expect(deploy).toContain("Deploy failure → diagnosis handoff");
    expect(deploy).toContain("재배포나 롤백은 하지 않아요");
    expect(deploy).toContain("axhub deploy logs --app");
    expect(deploy).toContain("axhub --json deploy diagnose <앱>");
    expect(deploy).not.toContain("deployment_diagnosis if callable");
    expect(deploy).toContain("User-visible Bash/tool call titles must be Korean noun phrases only");
    expect(deploy).toContain("`manifesting`, `manifested`, `gitted`, `pushed`, `Push`, `resumed`, `bootstraped`, `deploy-prep`, `in-flight`, `dry-run`, `token-gate`, `execute`, `production`, `terminal success`, `grep pipe`, `gitignore`, `gitignoring`, `gitting`, `checking`, `Build passed`, `Working tree clean`, or `Not ignored`");
    expect(deploy).toContain("Use `운영` for the user-facing environment");
    expect(deploy).toContain("Never write `User explicitly authorized`, `Proceeding`, `Push 성공`, or `Push failed`");
    expect(deploy).toContain("Display the environment as `운영`, not `prod`, `production`, or raw profile values");
    expect(deploy).toContain("사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL");
    expect(deploy).toContain("Markdown URL 링크 문법은 전부 금지");
    expect(deploy).toContain("AXHUB_GATE_POLL_ITERATIONS=0 axhub plugin-support token-gate");
    expect(deploy).toContain("never wraps token-gate in `grep`, `head`, or a multi-command pipe");
    expect(deploy).toContain("Present this step as `인증 상태 확인`, not as token-gate");
    expect(deploy).toContain("NEVER commit, push, or add `.omc/`, `.claude/`, `.codex/`, `.serena/`, `.omx/`, `.omo/`");
    expect(deploy).toContain("Never deploy a local-only commit SHA");
    expect(deploy).toContain("git push -u origin \"HEAD:$BRANCH\"");
    expect(deploy).toContain("Judge push success by exit code");
    expect(deploy).toContain("Use a short deploy utterance like `현재 앱 배포해`");
    expect(deploy).toContain("NEVER call `axhub deploy create --execute` for a commit that is only local");
    expect(workflowDetails).toContain("AXHUB_GATE_POLL_ITERATIONS=0 axhub plugin-support token-gate");
    expect(workflowDetails).toContain("Do not pipe token-gate through `grep`, `head`, or a combined dry-run pipeline");
    expect(workflowDetails).toContain("The visible environment label is `운영`");
    expect(workflowDetails).toContain("Do not show `deploy-prep`, `in-flight`, `dry-run`, `token-gate`, `execute`, `production`, `terminal success`, `gitignore`, `gitting`, `checking`, `Build passed`, `Working tree clean`, `Not ignored`, `User explicitly authorized`, `Proceeding`, or `Push 성공`");
    expect(workflowDetails).toContain("':(exclude).serena'");
    expect(workflowDetails).toContain("':(exclude).omx'");
    expect(workflowDetails).toContain("':(exclude).omo'");
    expect(workflowDetails).toContain("If deploy-prep or another target check still reports only those runtime paths");
    expect(workflowDetails).toContain("Do not combine polling into one long `while`/`for` shell loop");
    expect(workflowDetails).toContain("do not collapse polling into one long `while`/`for`/`until` shell loop");
    expect(workflowDetails).toContain("A Claude Desktop permission request for a long polling shell block is a failed watch UX");
    expect(workflowDetails).toContain("COMMIT_SHA=$(git rev-parse \"${COMMIT_SHA:-HEAD}^{commit}\")");
    expect(workflowDetails).toContain("git push -u origin \"HEAD:$BRANCH\"");
    expect(workflowDetails).toContain("Judge push success by `PUSH_EXIT`, not by stderr text");
    expect(workflowDetails).toContain("git merge-base --is-ancestor \"$COMMIT_SHA\" \"origin/$BRANCH\"");
    expect(workflowDetails).toContain("Never run `axhub deploy create --execute` for a local-only commit");
    expect(workflowDetails).toContain("Do not call `axhub deploy watch`");
    expect(workflowDetails).toContain("Do not substitute `axhub deploy watch`");
    expect(workflowDetails).toContain("Do not end by asking the user to say `배포 상태 확인해줘`");
    expect(workflowDetails).toContain("continue the bounded verify loop automatically");
    expect(workflowDetails).not.toContain("invite \"배포 상태 확인해줘\"");
    expect(workflowDetails).toContain("axhub deploy logs --app");
    expect(deployErrors).toContain("제가 같은 배포를 계속 확인할게요");
    expect(deployErrors).toContain("진행 중인 그 배포를 제가 계속 확인할게요");
    expect(deployErrors).not.toContain("말씀하시면 끝날 때까지");

    expect(importSkill).toContain("axhub --json plugin-support import --mode preview");
    expect(importSkill).toContain('axhub --json plugin-support import --mode preview --slug "$APP_SLUG" --tenant "$TENANT"');
    expect(importSkill).toContain("static lane 에서는 사용자가 명시적으로");
    expect(importSkill).toContain("axhub --json plugin-support import --mode execute --approved");
    expect(importSkill).toContain('axhub --json plugin-support import --mode execute --approved --slug "$APP_SLUG" --tenant "$TENANT"');
    expect(importSkill).toContain("권한 카드 한 번으로 끝나는");
    expect(importSkill).toContain("--wait --wait-interval 20s --wait-timeout 10m --json");
    expect(importSkill).toContain("GitHub owner 만 명시되고 repo 이름이 따로 없으면 `$REPO_NAME` 은 반드시 `$APP_SLUG` 와 정확히 같게 둬요");
    expect(importSkill).toContain("같은 verify 를 반복 호출하거나");
    expect(importSkill).toContain("existing_axhub_app_repair");
    expect(importSkill).toContain("CLI 가 현재 `gh` 로그인과 app slug 로 repo 를 정하고");
    expect(importSkill).toContain("capabilities.import.schemas");
    expect(importSkill).toContain("Static 성공은");
    expect(importSkill).toContain("고친 뒤에만 다시 시도해요");
    expect(importSkill).toContain("raw JSON body");
    expect(importSkill).toContain("low-level 명령을 조합해서 우회하지 않아요");
    expect(importSkill).toContain("axhub deploy --explain --json");
    expect(importSkill).toContain("사용자에게 보이는 Bash/tool call 제목은 한국어 명사구로만");
    expect(importSkill).toContain("`importing`, `imported`, `manifested`, `gitted`, `pushed`, `raw JSON`, `token-gate`, `manifest_create`, `verification_status`, `deployment`, `execute`, `git remote`, `curl`");
    expect(importSkill).toContain("`.omc/`, `.claude/`, `.codex/`, `.serena/` 같은 런타임 상태는 제외");
    expect(importSkill).toContain("사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL");
    expect(importSkill).toContain("Markdown URL 링크 문법은 전부 금지");
    expect(importSkill).toContain("[https://...](https://...)`, `[열기](https://...)`, `<https://...>`");
    expect(importSkill).toContain("첫 visible chat sentence 는 반드시 정확히 `기존 앱을 axhub에 가져올 준비를 확인할게요.` 로 시작");
    expect(importSkill).toContain("그 앞에는 공백·설명·스킬 선택 이유를 포함해 어떤 문장도 쓰지 않아요");
    expect(importSkill).toContain("스킬 선택 이유, route label, slash command label 을 절대 설명하지 않아요");
    expect(importSkill).toContain("`/axhub:import`, `axhub:import`, `import 스킬`, `스킬을 사용할게요`, `스킬 호출`");
    expect(importSkill).toContain("`앱 slug 미확정` 대신 `앱 이름이 아직 정해지지 않아 package.json 이름으로 확인할게요`");
    expect(importSkill).toContain("`manifest_create 있으니` 대신 `앱 설정 파일이 필요해서 프로젝트 파일 근거로 작성할게요`");
    expect(importSkill).toContain("`git remote 아직 없음` 대신 `원격 저장소가 아직 없어 새 저장소 생성 경로로 진행해요`");
    expect(importSkill).toContain("`execute 호출한다` 대신 `가져오기를 실행할게요`");
    expect(importSkill).toContain("`Envelope`, `preview`, `import 지원`, `deployment verification`, `success`, `raw endpoint`, `raw 엔드포인트`, `public`, `HTML, 200`");
    expect(importSkill).toContain("이미 발견된 버그 예시");
    expect(importSkill).toContain("같은 대화 안에서 재시도하거나 이어서 import 할 때도 이전 표현을 재사용하지 말고");
    expect(importSkill).toContain("`Port 8080, /healthz 확인. preview 진행.`, `git remote 없음`, `execute 실행한다`, `deployment verification: success`, `HTML, 200`");
    expect(importSkill).toContain("`import 지원 확인됐다` 대신 `가져오기 기능을 사용할 수 있어요`");
    expect(importSkill).toContain("`preview 진행` 대신 `미리보기를 확인할게요`");
    expect(importSkill).toContain("`Envelope 정상` 대신 `응답 형식 확인이 끝났어요`");
    expect(importSkill).toContain("`deployment verification: success` 대신 `첫 배포 검증 성공`");
    expect(importSkill).toContain("`raw endpoint`/`raw 엔드포인트` 대신 `원문 응답`");
    expect(importSkill).toContain("`public으로` 대신 `공개 접근으로`");
    expect(importSkill).toContain("최종 성공 요약은 아래 형태를 벗어나지 않아요");
    expect(importSkill).toContain("`첫 배포 검증이 끝났어요. 운영 URL: https://...`");
    expect(importSkill).toContain("비공개 접근 제어 때문에 로그인 없는 요청으로는 앱 본문을 직접 확인하지 못했어요");
    expect(importSkill).toContain("비공개 앱에서 로그인 없는 HTTP 요청이 axhub 로그인 화면 HTML 을 200 으로 돌려주면");
    expect(importSkill).toContain("그건 앱의 `/healthz` 또는 루트 응답 검증이 아니에요");
    expect(importSkill).toContain("`/healthz HTTP 200 확인`이라고 쓰지 않아요");
    expect(importSkill).toContain("body 가 axhub 로그인 포털이면 실패한 본문 검증으로 취급");
    expect(importSkill).not.toContain("axhub manifest validate");
    expect(clarity).toContain("공개 표면만");
    expect(clarity).toContain("plugin-support");
    expect(clarity).toContain("탐색·실행 대상이 아니에요");
    expect(clarity).toContain("로그, 환경변수, 롤백, 테이블/컬럼/데이터, connector grant, GitHub 재연결");
    expect(clarity).toContain("**CLI-only.**");
    expect(clarity).toContain("Bash/명령 도구로 실행하는 `axhub` CLI 만 사용");
    expect(clarity).toContain("read-only 조회라도 MCP/App tool 로 빠지면");
    expect(clarity).toContain("**Desktop-visible command allowlist.**");
    expect(clarity).toContain("탐색(`--json-schema`), 사용법 확인(`--help`), 실행(`--json`) 모두 공통");
    expect(clarity).toContain("`bash -lc`, `sh -c`, `| head`, `| grep`, `| sed`, `| awk`, shell pipe");
    expect(clarity).toContain("`2>`, `&>`, `;`, `&&`, `||`, `echo`, `cat`, `wc`, `tee`, `xargs`, `jq`, `python`, `node`, `perl`, `mktemp`");
    expect(clarity).toContain("`--field-expr` 문자열 내부의 `|` 는 허용되지만 shell pipe 로 출력 후처리하면 실패");
    expect(clarity).toContain("출력이 크면 `head -c` 로 자르지 말고 더 좁은 `--field-expr` 경로");
    expect(clarity).toContain("Use the axhub clarity skill");
    expect(clarity).toContain("Show logs for <app>");
    expect(clarity).toContain("set an environment variable");
    expect(clarity).toContain("reconnect my GitHub account with axhub");
    expect(clarity).toContain("GitHub device code");
    expect(clarity).toContain("사용자가 아래처럼 영어로 직접 clarity 를 지목해도 이 스킬을 실행");
    expect(clarity).toContain("직전 답변을 재사용해서 끝내지 말고");
    expect(clarityFrontmatter).not.toContain("disable-model-invocation");
    expect(clarity).toContain("frontmatter description 라우팅으로 자연어에서도 이 스킬이 직접 받아요");
    expect(clarity).toContain("slash 명령이 실패한 직후라도 자연어 요청은 독립된 새 요청으로 취급");
    expect(clarity).toContain("`스킬 가이드가 반환됐네요`");
    expect(clarity).toContain("메타 설명을 사용자에게 말하지 않아요");
    expect(clarity).toContain("첫 visible 문장은 사용자가 요청한 일을 바로 하는 말");
    expect(clarity).toContain("새 앱 생성을 clarity 가 직접 질문");
    expect(clarity).toContain("concept/name/slug/template 추천 질문을 만들지 않아요");
    expect(clarity).toContain("question 제목 `예약 사이트 컨셉`");
    expect(clarity).toContain("질문 `새로 만들 예약 웹사이트, 어떤 컨셉으로 할까요?`");
    expect(clarity).toContain("추천 후보는 bootstrap 이 묻고 clarity 는 만들지 않아요");
    expect(clarity).not.toContain("operational-lookups.md");
    expect(clarity).not.toContain("show a read-only axhub overview");
    expect(clarity).not.toContain("is production healthy?");
    expect(clarity).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --tenant <tenant>");
    expect(clarity).toContain("shell loop, background watcher, persistent monitor 를 쓰지 않아요");
    expect(clarity).toContain("`Monitor 사용` 권한 카드가 뜨는 명령은 실패");
    expect(clarity).toContain("device flow fast path 에서는 다른 사전 점검을 건너뛰어요");
    expect(clarity).toContain("device flow 를 시작하는 Bash/tool call 제목과 description 은 모두 정확히 `계정 인증 시작`");
    expect(clarity).toContain("사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL");
    expect(clarity).toContain("Markdown URL 링크 문법은 전부 금지");
    expect(clarity).toContain("device-flow URL 은 Claude Desktop 이 자동 링크로 바꾸지 못하도록 URL 부분만 inline code span 으로 써요");
    expect(clarity).toContain("[https://github.com/login/device](https://github.com/login/device)`, `[https://github.com/login/device](github.com/login/device)`, `[GitHub 열기](https://github.com/login/device)`, `<https://github.com/login/device>`, bare `https://github.com/login/device` 같은 링크/자동링크 형태는 모두 금지");
    expect(clarity).toContain("bare `https://github.com/login/device` 같은 링크/자동링크 형태는 모두 금지");
    expect(clarity).toContain("인증 URL: \\`https://github.com/login/device\\`");
    expect(clarity).toContain("입력 코드: <USER_CODE>");
    expect(clarity).toContain("승인 확인이나 계정 목록 조회를 시작하기 전에 먼저 assistant 본문 문장으로 URL과 코드를 노출");
    expect(clarity).toContain("title 과 description 이 모두 정확히 `인증 확인` 인 단일 `axhub github accounts list --json` 조회");
    expect(clarity).toContain("승인 확인용 `while true ... accounts list ... sleep ...` 루프나 persistent monitor 는 쓰지 않아요");
    expect(clarity).toContain("axhub 에 그 기능은 없어요");
    expect(clarity).toContain("schema/help/실행 명령에 `2>/dev/null | head -c 2000`, `| grep`, `| jq`, `bash -lc` 같은 shell 후처리 붙이기");
    expect(clarity).toContain("출력 축소는 더 좁은 `--field-expr` 로만");
    expect(clarity).toContain("diagnosis");
    expect(clarity).toContain("배포 실패 원인 진단");
    expect(clarity).toContain("본인 범위");
    expect(clarity).toContain("axhub connectors mine");
    expect(clarity).toContain("tenant-admin 전체 카탈로그");
    expect(clarity).toContain('`connectors list` / `--enabled-only` tenant-admin 전체 목록을 "내가 조회 가능한 커넥터" 로 표현');
    expect(clarity).toContain("Claude Desktop 에 노출된 `axhub` App/MCP 도구 호출");
    expect(clarity).toContain("clarity 는 항상 CLI help gate 뒤 `axhub` 명령으로 실행해요");
    expect(clarity).toContain("읽기 전용 leaf CLI 를 `> /tmp/...`, `2>&1`, `;`, `&&`, `||`, `echo`, `wc`, `jq`, `cat`, `mktemp`, command substitution 같은 shell wrapper 로 감싸기");
    expect(clarity).toContain("단일 `axhub ... --json` 호출을 실행하고 tool 결과를 assistant 내부에서 해석");
    expect(clarity).toContain("`읽는 중 <랜덤>.txt`, `Read /tmp/...`, `파일 읽기` 같은 임시 출력 파일 재읽기");
    expect(clarity).toContain("파일 읽기 팝업/단계가 보이면 실패");
    const clarityCodeBlocksForShell = clarity.match(/```(?:bash|sh)?\n[\s\S]*?```/g) ?? [];
    expect(clarityCodeBlocksForShell.join("\n")).not.toContain("2>/dev/null");
    expect(clarityCodeBlocksForShell.join("\n")).not.toContain("head -c");
    expect(diagnosis).toContain("axhub deploy diagnose");
    expect(diagnosis).toContain("diagnose failed deployment <id> for app <slug>");
    expect(diagnosis).toContain("Diagnose failed deployment 96728617 for app my-app");
    expect(diagnosis).toContain("why did my deploy fail?");
    expect(diagnosis).toContain("방금 배포된 앱 혹시 실패 원인 같은 거 있으면 진단해줘");
    expect(diagnosis).toContain("재배포는 하지 말고 원인만 봐줘");
    expect(diagnosis).toContain("`혹시 실패 원인 같은 거 있으면`, `원인만 봐줘`, `재배포는 하지 말고` 같은 조건부·소극적 표현도 실패 원인 진단 의도");
    expect(diagnosis).toContain("실패 배포 id 없이 \"혹시 실패 원인 있으면\"처럼 물었고 현재 라이브가 정상이면 `정상이에요` 로 끝내고");
    expect(diagnosis).toContain("영어 실패 배포 진단 요청도 반드시 이 스킬로 라우팅");
    expect(diagnosis).toContain("axhub deploy status <deployment-id>");
    expect(diagnosis).toContain("axhub deploy logs --app");
    expect(diagnosis).toContain("CLI 전용");
    expect(diagnosis).toContain("MCP `deployment_diagnosis` 같은 deployment MCP 도구가 보여도 호출하지 않아요");
    expect(diagnosis).toContain("MCP `App list`, `App get`, `deployment_diagnosis` 같은 도구만으로 진단을 대신하지 않아요");
    expect(diagnosis).toContain("사용자에게 보이는 진행 문구와 Bash/tool call 제목은 한국어로만");
    expect(diagnosis).toContain("CLI 표면 확인");
    expect(diagnosis).toContain("실패 배포 상태 확인");
    expect(diagnosis).toContain("실패 배포 로그 확인");
    expect(diagnosis).toContain("각 CLI 확인은 별도 tool 로 실행해요");
    expect(diagnosis).not.toContain("실패 배포 상태·로그 확인");
    expect(diagnosis).toContain("현재 라이브 상태 확인");
    expect(diagnosis).toContain("Bash/명령 tool 을 호출할 때 description/title/summary 필드는 반드시 위 고정 문구 중 하나로 직접 채워요");
    expect(diagnosis).toContain("도구가 자동으로 제목을 만들도록 비워두지 말고");
    expect(diagnosis).toContain("`axhubing CLI 확인` 같은 자동 생성 제목이 보이면 같은 명령이라도 `CLI 표면 확인` 으로 제목을 고쳐서 호출해요");
    expect(diagnosis).toContain("`axhubing`, `axhubed`, `diagnosing`, `checking` 처럼");
    expect(diagnosis).toContain("`axhubed CLI 확인` 대신 `현재 라이브 상태 확인`");
    expect(diagnosis).toContain("도구 제목에는 제품명 `axhub` 자체도 넣지 않아요");
    expect(diagnosis).not.toContain('CLI_NAME="ax""hub"');
    expect(diagnosis).not.toContain('CLI_BIN="$(command -v "$CLI_NAME" || true)"');
    expect(diagnosis).not.toContain("deploy status --help");
    expect(diagnosis).not.toContain("deploy logs --help");
    expect(diagnosis).not.toContain("deploy diagnose --help");
    expect(diagnosis).toContain("한 개의 bare 명령");
    expect(diagnosis).toContain("axhub deploy status <deployment-id> --app <앱> --json");
    expect(diagnosis).toContain("axhub --json deploy diagnose <앱>");
    expect(diagnosis).toContain("사용자에게 보이는 문장에서는 영어 진행 문장을 쓰지 않아요");
    expect(diagnosis).toContain("`Read-only` 도 쓰지 말고 `읽기 전용`");
    expect(diagnosis).toContain("명령 이름(`axhub deploy status`, `status/logs/diagnose`)은 필요할 때만 짧게 허용");
    expect(diagnosis).toContain("description/title/summary 도 정확히 `현재 라이브 상태 확인`");
    expect(diagnosis).toContain("중간 요약과 최종 메시지에서 raw category/stage/code 이름을 그대로 쓰지 않아요");
    expect(diagnosis).toContain("`configuration`, `auth`, `build`, `infrastructure`, `timeout`, `resolve`, `backend_unimplemented`, `commit_not_found`");
    expect(diagnosis).toContain('"배포할 버전 찾기"');
    expect(diagnosis).toContain("`실패 배포(96728617)` 처럼 일부만 보여주는 것도 금지");
    expect(diagnosis).toContain("항상 \"방금 실패한 배포\" 또는 \"이 실패한 배포\" 라고 말해요");
    expect(diagnosis).toContain("`healthy: true`, `healthy=false`, `applicable=false`, `services[]`, `reason.category`");
    expect(diagnosis).toContain("raw 필드명·불리언");
    expect(diagnosis).toContain("정상이에요");
    expect(diagnosis).toContain("진단 대상이 아니에요");
    expect(diagnosis).toContain("해결 후보가 있어요");
    expect(diagnosis).toContain("대상을 못 찾았어요");
    expect(diagnosis).toContain("로그인/권한이 필요해요");
    expect(diagnosis).toContain("진단을 못 했어요");
    expect(diagnosis).toContain("재배포·롤백");
    expect(diagnosis).toContain("직접 실행하지 않아요");
    expect(diagnosis).not.toContain("MCP `deployment_diagnosis` 를 1순위");
    expect(diagnosis).not.toContain("CLI-only diagnosis");
    expect(diagnosis).not.toContain("Check CLI surface first");
    expect(diagnosis).not.toContain("Checking axhub CLI availability");
    const clarityCodeBlocks = clarity.match(/```(?:bash|sh)?\n[\s\S]*?```/g) ?? [];
    expect(clarityCodeBlocks.join("\n")).not.toContain("axhub plugin-support");

    const update =
      readRepo("skills/update/SKILL.md") +
      readRepo("skills/update/references/plugin-update.md") +
      readRepo("skills/update/references/post-update-continuation.md");
    const updateBody = update.replace(/^---\n[\s\S]*?\n---\n?/, "");
    expect(update).toContain("description: 'axhub 최신 확인, 버전 확인, 업데이트 전용 skill");
    expect(update).toContain('일반 UX 역할 문구나 "알아서 진행"만으로는 update 가 아니며');
    expect(update).not.toContain("명령어는 잘 몰라");
    expect(update).toContain("일반 Code-mode script");
    expect(update).toContain("app status, app creation, deployment 는 update 뒤에 이어서 처리해요");
    expect(update).toContain("**CRITICAL desktop first line.**");
    expect(update).toContain("사용자에게 보이는 첫 문장은 반드시 정확히 `현재 버전을 확인할게요.`");
    expect(update).toContain("선택 이유를 설명하지 않아요");
    expect(update).toContain("선택한 스킬 이름이나 선택 이유도 말하지 않아요");
    expect(update).toContain("사용자에게 보이는 첫 문장은 반드시 정확히 `현재 버전을 확인할게요.`");
    expect(update).toContain("그 앞에 어떤 설명도 붙이지 않아요");
    expect(update).toContain("선택한 스킬 이름이나 선택 이유도 말하지 않아요");
    expect(update).toContain("assistant 본문에는 같은 말을 반복하지 않아요");
    expect(update).toContain("`axhubing`, `axhubed`, `updating` 처럼 제품명을 영어 동사처럼");
    expect(update).toContain("선택 이유를 설명하지 않아요");
    for (const leakedPhrase of ["전용 스킬", "스킬을 사용", "사용하겠습니다", "라우팅", "axhub:update 스킬", "/axhub:update", "update skill"]) {
      expect(updateBody).not.toContain(leakedPhrase);
    }
    expect(update).toContain("라벨 안에 `axhub` 를 넣지 않아요");
    expect(update).toContain("**Desktop-visible command allowlist.**");
    expect(update).toContain("Bash/명령 도구로 사용자에게 보일 수 있는 command 는 아래 계열만 써요");
    expect(update).toContain("플러그인 캐시 파일을 읽지 않아요");
    expect(update).toContain("정확히 `claude plugin list` 만 실행한 출력");
    expect(update).toContain("이 권한 카드의 Desktop-visible command 는 글자 하나도 더하지 말고 정확히 `claude plugin list` 예요");
    expect(update).toContain("`claude plugin list 2>&1`");
    expect(update).toContain("`claude plugin list 2>&1 | grep ...`");
    expect(update).toContain("pipe, redirect, text filter");
    expect(update).toContain("실행하려는 command 가 `claude plugin list 2>&1` 또는 `command -v claude && claude plugin list` 로 떠오르면 **반드시 `claude plugin list` 로 바꿔요.**");
    expect(update).toContain("플러그인 버전, 설치 scope, 다음 CLI 확인을 영어 내부 로그처럼 chat 에 쓰지 말고");
    expect(update).toContain("scope 원문, 영어 진행 로그는 사용자에게 말하지 않아요");
    expect(update).toContain("버전과 설치 위치를 같은 문장에 섞지 않아요");
    expect(update).toContain("설치 위치값은 업데이트 명령의 `--scope` 인자에만 쓰고 chat 에는 쓰지 않아요");
    expect(update).toContain("플러그인 확인 직후에는 `현재 플러그인 버전을 확인했어요.` 만 보여줘요");
    expect(update).toContain("파일 읽기 도구를 쓰지 않아요");
    expect(update).toContain("command substitution, shell wrapper, file test, pipe, redirect, text filter, 파일 읽기 도구를 쓰지 않아요");
    expect(update).toContain("이 스킬은 **버전 확인/업데이트 결과를 먼저** 처리해요");
    expect(update).toContain("read 작업이어도 CLI 계약을 우선해요");
    expect(update).toContain("원문이 영어로 `then`, `and then`, `after that`, `help me understand` 를 써도");
    expect(update).toContain("업데이트 뒤 남은 요청을 버리지 않아요");
    expect(update).toContain("사용자의 추가 프롬프트를 기다리지 말고 다음 적절한 axhub 흐름을 시작해요");
    expect(update).toContain("`앱 상태 조회`, `배포 상태 조회`, `최근 배포 조회`, `GitHub 연결 상태 확인` 같은 tool 제목이 떠올랐다면");
    expect(update).toContain("업데이트 확인은 끝났어요. 이어서 GitHub 계정 연결을 확인할게요.");
    expect(update).toContain("GitHub device-flow fast path");
    expect(update).toContain("`axhub git_connection_status`");
    expect(update).toContain("`AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link`");
    expect(update).toContain("`axhub github accounts list --json`");
    expect(update).toContain("업데이트 확인은 끝났어요. 이어서 요청하신 작업을 계속할게요.");
    expect(update).toContain("Task/Subagent/Agent/백그라운드 작업으로 우회하지 않아요");
    expect(update).toContain("NEVER Task/Subagent/Agent/백그라운드 작업으로 mixed request 의 남은 앱 상태 확인을 우회하지 말아요");
    expect(update).toContain("update 결과 뒤 같은 assistant 흐름에서 직접 이어가요");
    expect(update).toContain("사용자가 `앱 상태 확인해줘`, `배포해줘`, `새 앱 만들어줘` 같은 말을 다시 하지 않아도 돼요");
    expect(update).toContain("바로 다음 axhub 흐름으로 이어가요");
    expect(update).not.toContain("다음 동작은 `clarity`");
    expect(update).not.toContain("그 다음 작업은 `clarity`");
    expect(update).toContain("| CLI 존재 확인 (`command -v axhub`) | `CLI 설치 확인` |");
    expect(update).toContain("| 플러그인 설치 위치 확인 (정확히 `claude plugin list`) | `플러그인 설치 위치 확인` |");
    expect(update).toContain("| 플러그인 마켓플레이스 새로고침 | `플러그인 카탈로그 새로고침` |");
    expect(update).toContain("| 버전 확인 (`axhub update check ...`) | `버전 확인` |");
    expect(update).toContain("수동 확인 기록은 Claude Desktop 경로에서 갱신하지 않아요");
    expect(update).toContain("별도 `mkdir`/touch/marker command 를 실행하지 말아요");
    expect(update).toContain("별도 로컬 기록 작업명은 chat 에 쓰지 않아요");
    expect(update).not.toContain("plugin-update-check");
    expect(update).not.toContain('mkdir -p "$HOME/.axhub/cache"');
    expect(update).not.toContain("캐시만 갱신할게요");
    for (const commandLeak of ["rtk read", "grep version", "test -f", "cat ", "sed -n"]) {
      expect(update).not.toContain(commandLeak);
    }
    expect(update).toContain("설치 경로, Scope, manifest 경로, raw 목록,");
    expect(update).toContain("| 플러그인 업데이트 적용 | `플러그인 업데이트 받기` |");
    expect(update).toContain("`현재 플러그인 버전을 확인했어요.`, `CLI는 이미 최신이에요. 플러그인 새 버전을 받을게요.`, `플러그인 설치 위치를 확인했어요.`, `플러그인 새 버전을 받았어요.`");
    expect(update).toContain("영어 라벨, 내부 필드명, 설치 위치 원문, raw 상태값, 반말형 짧은 메모가 섞인 문장");
    expect(update).toContain("플러그인: vX -> vY 받음 (재시작 필요)");
    expect(update).toContain("정확히 `받았어요. Claude Code 를 재시작하면 새 버전이 적용돼요.` 라고 말해요");
    expect(update).toContain("알 수 없는 로마자 단어를 만들지 않아요");
    expect(update).toContain("NEVER 플러그인 업데이트 성공 뒤 `받았어요. Claude Code 를 재시작하면 새 버전이 적용돼요.` 가 아닌 재시작 안내 문장을 만들지 말아요");
    expect(update).toContain("확인·비교 결과를 설명하는 영어 디버그 문장이나 raw 확인 줄은 쓰지 않아요");
    expect(update).not.toContain("Plugin version");
    for (const leakedUpdatePhrase of ["PLUGIN_UPDATED_VERSION", "matching plugin.latest", "Confirmed:", "Confirmed", "plugin.latest"]) {
      expect(update).not.toContain(leakedUpdatePhrase);
    }
  });

  test("development skill follows the current SDK raw-db surface", () => {
    const development = readRepo("skills/development/SKILL.md") + readRepo("skills/development/references/sdk-db-surface.md");
    const connectorSafety = readRepo("skills/development/references/connector-safety.md");
    const writeGate = readRepo("skills/development/references/write-gate.md");

    expect(development).toContain("legacy `/data` 데이터플레인");
    expect(development).toContain("sdk.apps.rawDb.tables(appId)");
    expect(development).toContain("sdk.apps.rawDb.tableRows(appId, table");
    expect(development).toContain("제거된 SDK data-plane API");
    expect(development).toContain("기능 코드 추가·수정");
    expect(development).toContain("todo priority/filter/search, tabs, forms, CRUD, UI/page/API route 개선");
    expect(development).toContain("기능이 connector/table/DB 데이터를 쓰면");
    expect(development).toContain("순수 UI 정리, todo priority/filter/search, tabs, forms, API route 리팩터");
    expect(development).toContain("사용자에게 보이는 설명·툴 제목·중간 메모는 한국어로만");
    expect(development).toContain("optional 파일·설정 확인은 없어도 정상인 경우 실패처럼 보이면 안 돼요");
    expect(development).toContain("Desktop preview/issue check guard");
    expect(development).toContain("Claude Desktop Code 모드");
    expect(development).toContain("preview 확인은 `lint`/`build` 통과 뒤 한 번만");
    expect(development).toContain("Next `1 Issue` 배지·overlay 가 보이면 최대 1회만");
    expect(development).toContain("preview/issue 확인에 90초 이상 쓰지 않아요");
    expect(development).toContain("`ScheduleWakeup`, 내부 task 이름, 브라우저/preview 도구 이름을 chat 에 쓰지 않아요");
    expect(development).toContain("장시간 overlay 탐색으로 사용자를 기다리게 하지 않아요");
    expect(connectorSafety).toContain("legacy data-plane DSL 은 제거");
    expect(writeGate).toContain("legacy data-plane write DSL 은 새로 만들지 않아요");

    const retiredExamples = [
      "sdk_search (MANDATORY",
      "MCP 가 authority",
      'import { AxHubClient, defineSchema, where }',
      'sdk.tenant("test").app("uqa152-node-fix").data.table',
      "data.table(Products)",
      "`where(...).isNotNull()`",
      "`data.table(\"<name>\", schema)`",
      "Now the file's read",
    ];
    for (const retiredExample of retiredExamples) {
      expect(development).not.toContain(retiredExample);
    }
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
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const deploy = readRepo("skills/deploy/SKILL.md") + readRepo("skills/deploy/references/user-facing-language.md");
    const clarity = readRepo("skills/clarity/SKILL.md") + readRepo("skills/clarity/references/execution-guardrails.md");

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

    // bootstrap: evidence-gated carry-over + shared-contract include.
    expect(bootstrap).toContain("같은 대화 맥락 이어받기");
    expect(bootstrap).toContain("이미 본 것만");
    expect(bootstrap).toContain("## Reference Loading Policy");
    expect(bootstrap).not.toContain("../deploy/references/session-carryover.md");
    expect(bootstrap).not.toContain("references/bootstrap-and-local.md");
    // E4: infer-tables-env also weighs actually-queried resources.
    expect(bootstrap).toContain("infer-tables-env 분석은 scaffold 코드뿐 아니라");
    // Confabulation negative guard (PR-gating proxy for the nightly behavioral case):
    // with no evidence, bootstrap must go cold and never invent a resource.
    expect(bootstrap).toContain("리소스를 지어내지 않아요");
    expect(bootstrap).toContain("carry-over 를 주장하지 않아요");
    // M2: gate relaxation suppresses re-narration only, never the gate.
    expect(bootstrap).toContain("install-link 를 보여줬으면 재안내는 생략");
    expect(bootstrap).toContain("0-install gate 는 항상 실행해요");

    const bootstrapAndLocal = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    expect(bootstrapAndLocal).toContain(".data.repo_full_name // .data.status.repo_full_name // empty");

    // deploy: carry-over applies only AFTER the route gate (no vercel hijack).
    expect(deploy).toContain("route gate 통과 후에만 적용해서 다른 타깃");
    expect(deploy).toContain("references/session-carryover.md");

    // clarity: pure-prose continuation, stays out of plugin-support.
    expect(clarity).toContain("## 다음 단계 이어주기");
    expect(clarity).toContain("이걸로 대시보드 만들어줘");
    const clarityCodeBlocks = clarity.match(/```(?:bash|sh)?\n[\s\S]*?```/g) ?? [];
    expect(clarityCodeBlocks.join("\n")).not.toContain("axhub plugin-support");
  });

  test("SessionStart hooks are wired", () => {
    interface HookEntry {
      type: string;
      shell?: string;
      command: string;
    }
    interface HooksFile {
      hooks: { SessionStart: Array<{ hooks: HookEntry[] }> };
    }
    const hooksFile = readJson<HooksFile>("hooks/hooks.json");
    const entries = hooksFile.hooks.SessionStart.flatMap((group) => group.hooks);
    expect(entries).toHaveLength(5);

    // every SessionStart hook injects context invisibly: suppressed JSON only,
    // never plain echo stdout and never a user-facing systemMessage banner
    for (const entry of entries) {
      expect(entry.command).toContain("suppressOutput");
      expect(entry.command).toContain("additionalContext");
      expect(entry.command).not.toContain("systemMessage");
      expect(entry.command).not.toContain('echo "');
      // Windows: backslash plugin-root paths must be slash-normalized before
      // JSON interpolation, or C:\... produces invalid JSON escapes
      if (entry.command.includes("CLAUDE_PLUGIN_ROOT")) {
        expect(entry.command).toContain("${CLAUDE_PLUGIN_ROOT//");
      }
    }

    // 4번째 entry: 플러그인 업데이트 재시작 확인 (restart-confirm)
    const restartConfirm = entries[3];
    expect(restartConfirm.type).toBe("command");
    expect(restartConfirm.shell).toBe("bash");
    expect(restartConfirm.command).toContain("AXHUB_NO_AUTO_UPDATE");
    expect(restartConfirm.command).toContain("no-auto-update");
    expect(restartConfirm.command).toContain(".plugin-update-restart");
    expect(restartConfirm.command).toContain("-mmin -10080");
    expect(restartConfirm.command).toContain("plugin-restart-confirm-prompt.md");
    // dev 가드는 이 entry 에 적용하지 않아요 — marker 는 머신 전역이라
    // 어느 세션의 확인도 유효해요 (eng review #1)
    expect(restartConfirm.command).not.toContain("../../.git");
    // hook 은 읽기 전용: marker 삭제는 confirm prompt 몫, 바이너리 무접촉
    expect(restartConfirm.command).not.toContain("rm -f");
    expect(restartConfirm.command).not.toContain("command -v axhub");
  });

  test("AP-19 feedback auto-report contract hook is wired", () => {
    interface HookEntry {
      type: string;
      shell?: string;
      command: string;
    }
    interface HooksFile {
      hooks: { SessionStart: Array<{ hooks: HookEntry[] }> };
    }
    const hooksFile = readJson<HooksFile>("hooks/hooks.json");
    const entries = hooksFile.hooks.SessionStart.flatMap((group) => group.hooks);

    const feedback = entries[4];
    expect(feedback.type).toBe("command");
    expect(feedback.shell).toBe("bash");
    // kill switch: env + marker counterpart (repo-wide 훅 계약)
    expect(feedback.command).toContain("AXHUB_NO_FEEDBACK_REPORT");
    expect(feedback.command).toContain("no-feedback-report");
    // CLI 존재 가드는 AP-17 의 3-경로만 봐요 — 네트워크·바이너리 실행 없음
    expect(feedback.command).toContain("command -v axhub");
    expect(feedback.command).toContain(".axhub/bin-path");
    expect(feedback.command).toContain(".axhub/bin/axhub");
    // AP-19 invariants (agent-policy parity)
    expect(feedback.command).toContain("axhub feedback -m");
    expect(feedback.command).toContain("예상된 거절은 리포트하지 않아요");
    // hook 은 계약 emit 만 해요: marker 삭제·업데이트·리포트 실행 없음
    expect(feedback.command).not.toContain("rm -f");
    expect(feedback.command).not.toContain("axhub update");
  });

  test("update freshness prompt router hook is wired", () => {
    interface HookEntry {
      type: string;
      shell?: string;
      command: string;
    }
    interface HooksFile {
      hooks: {
        SessionStart: Array<{ hooks: HookEntry[] }>;
        UserPromptSubmit: Array<{ hooks: HookEntry[] }>;
      };
    }
    const hooksFile = readJson<HooksFile>("hooks/hooks.json");
    const entries = hooksFile.hooks.UserPromptSubmit.flatMap((group) => group.hooks);
    expect(entries).toHaveLength(1);

    // update 라우터는 hooks/update-router.sh 로 추출됐어요 — hooks.json 은
    // 형제 라우터와 같은 bare 위임만 갖고, kill switch(env·marker)와 계약
    // 본문은 스크립트가 소유해요.
    const router = entries[0];
    expect(router.type).toBe("command");
    expect(router.shell).toBe("bash");
    expect(router.command).toBe('bash "${CLAUDE_PLUGIN_ROOT}/hooks/update-router.sh"');

    const routerScript = readRepo("hooks/update-router.sh");
    // prompt-field 매칭 계약: "prompt": 키 이후 구간만 보고, 키 부재 시 fail-closed
    expect(routerScript).toContain('prompt_part=${input#*\\"prompt\\":}');
    expect(routerScript).toContain("fail-closed");
    expect(routerScript).toContain("*axhub*|*Axhub*|*AxHub*|*AXHUB*");
    expect(routerScript).toContain("AXHUB_NO_UPDATE_ROUTER");
    expect(routerScript).toContain("no-update-router");
    expect(routerScript).toContain("hookSpecificOutput");
    expect(routerScript).toContain("hookEventName");
    expect(routerScript).toContain("additionalContext");
    expect(routerScript).toContain("suppressOutput");
    expect(routerScript).not.toContain("systemMessage");
    expect(routerScript).toContain("axhub freshness/update");
    expect(routerScript).toContain("Finding tools");
    expect(routerScript).toContain("invoke the axhub update skill");
    expect(routerScript).toContain("현재 버전을 확인할게요");
    expect(routerScript).toContain("App/MCP tool");
    expect(routerScript).toContain("app list/status tool");
    expect(routerScript).toContain("Finish update first");
    expect(routerScript).toContain("GitHub reconnect-device-code");
    expect(routerScript).toContain("axhub clarity device-flow contract inline");
    expect(routerScript).toContain("do not invoke /axhub:clarity");
    expect(routerScript).toContain("failing skill badge");
    expect(routerScript).toContain("axhub git_connection_status");
    expect(routerScript).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link");
    expect(routerScript).toContain("axhub github accounts list --json");
    expect(routerScript).toContain("계정 인증 시작");
    expect(routerScript).toContain("인증 확인");
    expect(routerScript).toContain("The assistant body must print exactly two normal chat lines with the URL in inline code");
    expect(routerScript).toContain("never print the URL as a bare auto-link or Markdown link such as [https://github.com/login/device](github.com/login/device)");
    expect(routerScript).toContain("Do not finish after showing the code");
    expect(routerScript).toContain("continue to 인증 확인 in the same assistant turn");
    expect(routerScript).toContain("배포 상태 확인해줘");
    expect(routerScript).toContain("승인했어");
    expect(routerScript).toContain("CLI-only axhub apps get <app> --json and axhub deploy list --app <app> --json");
    expect(routerScript).toContain("Deployment list (axhub)");
    expect(routerScript).toContain("axhub deployment list");
    expect(routerScript).not.toContain("python3");
    expect(routerScript).not.toContain("node ");
    expect(routerScript).not.toContain("axhub update check");
    expect(routerScript).not.toContain("claude plugin update");

    // diet 계약: update 외 UserPromptSubmit 라우터는 wiring 도 스크립트도 없어요
    for (const removed of ["clarity-router.sh", "import-router.sh", "status-resume-router.sh"]) {
      expect(entries.some((entry) => entry.command.includes(removed))).toBe(false);
    }

    const sessionEntries = hooksFile.hooks.SessionStart.flatMap((group) => group.hooks);
    const sessionRouter = sessionEntries[2];
    expect(sessionRouter.type).toBe("command");
    expect(sessionRouter.shell).toBe("bash");
    expect(sessionRouter.command).toContain("AXHUB_NO_UPDATE_ROUTER");
    expect(sessionRouter.command).toContain("update-first routing guard is active for Code mode");
    expect(sessionRouter.command).toContain("Finding tools");
    expect(sessionRouter.command).toContain("현재 버전을 확인할게요");
    expect(sessionRouter.command).toContain("App/MCP tools");
    expect(sessionRouter.command).toContain("generic Code-mode handling");
    expect(sessionRouter.command).toContain("CLI-only axhub apps get <app> --json and axhub deploy list --app <app> --json");
    expect(sessionRouter.command).toContain("Deployment list (axhub)");
    expect(sessionRouter.command).toContain("axhub deployment list");
    expect(sessionRouter.command).not.toContain("axhub update check");
    expect(sessionRouter.command).not.toContain("claude plugin update");
  });

  test("AP-13 Windows execution contract hook is wired", () => {
    interface HookEntry {
      type: string;
      shell?: string;
      command: string;
    }
    interface HooksFile {
      hooks: { SessionStart: Array<{ hooks: HookEntry[] }> };
    }
    const hooksFile = readJson<HooksFile>("hooks/hooks.json");
    const entries = hooksFile.hooks.SessionStart.flatMap((group) => group.hooks);

    const windows = entries[1];
    expect(windows.type).toBe("command");
    expect(windows.shell).toBe("bash");
    // Windows-only guard + killswitch
    expect(windows.command).toContain("AXHUB_NO_WINDOWS_CONTRACT");
    expect(windows.command).toContain("Windows_NT");
    // emits the Git Bash execution contract (invariant with agent-policy AP-13)
    expect(windows.command).toContain("Git Bash 전용");
    expect(windows.command).toContain("PowerShell");
    // read-only / non-blocking: only echoes, never deletes markers
    expect(windows.command).not.toContain("rm -f");
  });

  test("onboarding and ready-card stay MCP-free", () => {
    const onboarding = readRepo("skills/onboarding/SKILL.md");
    const card = readRepo("skills/onboarding/references/ready-card.md");

    // 온보딩은 MCP 등록·OAuth 를 안내하지 않아요 — CLI 준비까지만 담당해요.
    expect(onboarding).toContain(
      "NEVER MCP 서버 등록(`claude mcp add`)이나 MCP OAuth 인증을 안내·실행하지 말아요",
    );
    expect(onboarding).toContain("references/ready-card.md");
    expect(onboarding).not.toContain(".onboarding-mcp-restart");
    expect(onboarding).not.toContain("Restart Handoff Card");

    // ready card 는 옵트인 → 최종 카드만 담당해요. MCP 절차·marker 는 없어요.
    expect(card).toContain("AI 활용 기록 옵트인");
    expect(card).toContain("VIBE_READY");
    expect(card).not.toContain("claude mcp");
    expect(card).not.toContain(".onboarding-mcp-restart");
    expect(card).toContain("사용자에게는 enum 이름이나 대괄호 표식을 출력하지 말고");
  });

  test("import frontmatter starts directly and avoids preamble leaks", () => {
    const importSkill =
      readRepo("skills/import/SKILL.md") +
      readRepo("skills/import/references/visibility-rules.md") +
      readRepo("skills/import/references/manifest-authoring.md");
    const importFrontmatter = importSkill.match(/^---\n([\s\S]*?)\n---/)?.[1];
    expect(importFrontmatter, "import SKILL frontmatter not found").not.toBeNull();
    const importDescription = importFrontmatter?.match(/^description:\s*'([^']*)'/m)?.[1];
    expect(importDescription, "import description not found in frontmatter").toBeDefined();
    expect(importDescription?.split(".")[0]).toBe("기존 앱을 axhub에 가져올 준비를 확인할게요");
    expect(importDescription).toContain("스킬 실행 전 사용자 문장 0개");
    expect(importDescription).toContain("do not explain why this path was chosen");
    expect(importDescription).toContain("route/skill label");
    expect(importDescription).not.toContain("스킬을 사용");
    expect(importDescription).not.toContain("사용하겠습니다");
    expect(importDescription).not.toContain("axhub:import 스킬");
  });
});
