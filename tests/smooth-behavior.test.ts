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
    const flowRows = [
      "| 첫 셋업 | `onboarding` |",
      "| 앱 생성 | `bootstrap` |",
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
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const deploy = readRepo("skills/deploy/SKILL.md");
    const clarity = readRepo("skills/clarity/SKILL.md");
    const diagnosis = readRepo("skills/diagnosis/SKILL.md");
    const importSkill = readRepo("skills/import/SKILL.md");
    const onboardingAuth = readRepo("skills/onboarding/references/install-channels-and-auth.md");
    const bootstrapAndLocal = readRepo("skills/bootstrap/references/bootstrap-and-local.md");
    const workflowDetails = readRepo("skills/deploy/references/workflow-details.md");

    expect(onboarding).toContain("axhub plugin-support onboarding-detect --json");
    expect(onboarding).toContain("cli_missing");
    expect(onboarding).toContain("cli_old");
    expect(onboarding).toContain("detect-first");
    expect(onboarding).toContain("`first_gap`, `gaps`, `cli_state`, `auth_error_code` 같은 detect 필드명이나 enum 값은 내부 라우팅용으로만");
    expect(onboarding).toContain("NEVER `first_gap`, `gaps`, `cli_state`, `auth_error_code` 같은 detect 필드명");
    // Regression: CLI installed on disk but not on PATH (new session, rc not re-sourced)
    // must route to PATH repair, not reinstall. See Step 2 on-disk elif + Step 4b loop-breaker.
    // -f (existence) not -x so Git Bash .exe (MSYS perm emulation) probes reliably on Windows.
    expect(onboarding).toContain('[ -f "$HOME/.axhub/bin/axhub" ]');
    expect(onboarding).toContain('[ -f "$HOME/.axhub/bin/axhub.exe" ]');
    expect(onboarding).toContain('"first_gap":"cli_path_missing"');
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
    expect(bootstrap).toContain("내부 라벨 노출 금지");
    expect(bootstrap).toContain("제품명·명령어·영어 단어에 `ing`/`ed` 를 붙인 제목");
    expect(bootstrap).toContain("label 과 target 모두 확인된 `https://...` 절대 URL");
    expect(bootstrap).toContain("`[$PUBLIC_URL]($PUBLIC_URL)` 형태");
    expect(bootstrap).toContain("NEVER GitHub device flow code 를 긴 watch tool 안에 숨긴 채");
    expect(bootstrap).toContain("승인 완료를 채팅으로 알려 달라고 쓰지 않고");
    expect(bootstrapAndLocal).toContain(".data.repo_full_name // .data.status.repo_full_name // empty");
    expect(bootstrapAndLocal).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap");
    expect(bootstrapAndLocal).toContain("auto_poll");
    expect(bootstrapAndLocal).toContain("Never leave the user staring at an empty GitHub code-entry screen with no code");
    expect(bootstrapAndLocal).toContain("따로 `승인했어`라고 말하지 않아도 돼요");
    expect(bootstrapAndLocal).toContain("do not end the response asking the user to report approval");
    expect(bootstrapAndLocal).not.toContain("브라우저에서 승인한 다음 \"승인했어\"");

    expect(deploy).toContain("axhub deploy verify <deployment-id>");
    expect(deploy).toContain("axhub deploy verify \"$DEPLOY_ID\"");
    expect(deploy).toContain("NEVER call `axhub deploy watch`");
    expect(deploy).toContain("bounded `axhub deploy verify \"$DEPLOY_ID\"` loop");
    expect(deploy).toContain("exit 6");
    expect(deploy).toContain("exit 7");
    expect(deploy).toContain("성공을 선언하지 않아요");
    expect(deploy).not.toContain("deploy-approved-run");
    expect(deploy).toContain(".data.id // .data.deployment_id // .id // .deployment_id // empty");
    expect(deploy).toContain("canonical workflow");
    expect(deploy).toContain("diagnosis");
    expect(deploy).toContain("Deploy failure → diagnosis handoff");
    expect(deploy).toContain("재배포나 롤백은 하지 않아요");
    expect(deploy).toContain("axhub deploy logs <deployment-id>");
    expect(deploy).toContain("axhub --json deploy diagnose <앱>");
    expect(deploy).not.toContain("deployment_diagnosis if callable");
    expect(deploy).toContain("User-visible Bash/tool call titles must be Korean noun phrases only");
    expect(deploy).toContain("`manifesting`, `manifested`, `gitted`, `pushed`, `Push`, `resumed`, `bootstraped`, `deploy-prep`, `in-flight`, `dry-run`, `token-gate`, `execute`, `production`, `terminal success`, `grep pipe`, `gitignore`, `gitignoring`, `gitting`, `checking`, `Build passed`, `Working tree clean`, or `Not ignored`");
    expect(deploy).toContain("Use `운영` for the user-facing environment");
    expect(deploy).toContain("Never write `User explicitly authorized`, `Proceeding`, `Push 성공`, or `Push failed`");
    expect(deploy).toContain("Display the environment as `운영`, not `prod`, `production`, or raw profile values");
    expect(deploy).toContain("AXHUB_GATE_POLL_ITERATIONS=0 axhub plugin-support token-gate");
    expect(deploy).toContain("never wraps token-gate in `grep`, `head`, or a multi-command pipe");
    expect(deploy).toContain("Present this step as `인증 상태 확인`, not as token-gate");
    expect(deploy).toContain("NEVER commit, push, or add `.omc/`, `.claude/`, `.codex/`, `.serena/`");
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
    expect(workflowDetails).toContain("COMMIT_SHA=$(git rev-parse \"${COMMIT_SHA:-HEAD}^{commit}\")");
    expect(workflowDetails).toContain("git push -u origin \"HEAD:$BRANCH\"");
    expect(workflowDetails).toContain("Judge push success by `PUSH_EXIT`, not by stderr text");
    expect(workflowDetails).toContain("git merge-base --is-ancestor \"$COMMIT_SHA\" \"origin/$BRANCH\"");
    expect(workflowDetails).toContain("Never run `axhub deploy create --execute` for a local-only commit");
    expect(workflowDetails).toContain("Do not call `axhub deploy watch`");
    expect(workflowDetails).toContain("Do not substitute `axhub deploy watch`");
    expect(workflowDetails).toContain("axhub deploy logs <deployment-id>");

    expect(importSkill).toContain("axhub --json plugin-support import --mode preview");
    expect(importSkill).toContain('axhub --json plugin-support import --mode preview --slug "$APP_SLUG" --tenant "$TENANT"');
    expect(importSkill).toContain("static lane 에서는 사용자가 명시적으로");
    expect(importSkill).toContain("axhub --json plugin-support import --mode execute --approved");
    expect(importSkill).toContain('axhub --json plugin-support import --mode execute --approved --slug "$APP_SLUG" --tenant "$TENANT"');
    expect(importSkill).toContain("foreground 로 실행하고 완료 출력을 받을 때까지");
    expect(importSkill).toContain("background output");
    expect(importSkill).toContain("existing_axhub_app_repair");
    expect(importSkill).toContain("CLI 가 현재 `gh` 로그인과 app slug 로 repo 를 정하고");
    expect(importSkill).toContain("capabilities.import.schemas");
    expect(importSkill).toContain("Static 성공은");
    expect(importSkill).toContain("정적 사이트 확인 증거가 부족해요");
    expect(importSkill).toContain("raw JSON body");
    expect(importSkill).toContain("low-level 명령을 조합해서 우회하지 않아요");
    expect(importSkill).toContain("axhub deploy --explain --json");
    expect(importSkill).toContain("사용자에게 보이는 Bash/tool call 제목은 한국어 명사구로만");
    expect(importSkill).toContain("`importing`, `imported`, `manifested`, `gitted`, `pushed`, `raw JSON`, `token-gate`, `manifest_create`, `verification_status`, `deployment`, `execute`, `git remote`, `curl`");
    expect(importSkill).toContain("`.omc/`, `.claude/`, `.codex/`, `.serena/` 같은 런타임 상태는 제외");
    expect(importSkill).toContain("label 과 target 모두 `https://...` 절대 URL");
    expect(importSkill).toContain("target 에 scheme 이 빠진 `[...](uqa.../)` 링크는 금지");
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
    expect(importSkill).toContain("`첫 배포 검증이 끝났어요. 운영 URL: [https://...](https://...)`");
    expect(importSkill).toContain("비공개 접근 제어 때문에 로그인 없는 요청으로는 앱 본문을 직접 확인하지 못했어요");
    expect(importSkill).toContain("비공개 앱에서 로그인 없는 HTTP 요청이 axhub 로그인 화면 HTML 을 200 으로 돌려주면");
    expect(importSkill).toContain("그건 앱의 `/healthz` 또는 루트 응답 검증이 아니에요");
    expect(importSkill).toContain("`/healthz HTTP 200 확인`이라고 쓰지 않아요");
    expect(importSkill).toContain("body 가 axhub 로그인 포털이면 실패한 본문 검증으로 취급");
    expect(importSkill).not.toContain("axhub manifest validate");
    expect(clarity).toContain("공개 표면만");
    expect(clarity).toContain("plugin-support");
    expect(clarity).toContain("탐색·실행 대상이 아니에요");
    expect(clarity).toContain("**CLI-only.**");
    expect(clarity).toContain("Bash/명령 도구로 실행하는 `axhub` CLI 만 사용");
    expect(clarity).toContain("`App list (axhub)`, `Tenant recent deployments (axhub)`, `App get (axhub)`, `Deployment status (axhub)`");
    expect(clarity).toContain("read-only 조회라도 MCP/App tool 로 빠지면");
    expect(clarity).toContain("**Desktop-visible command allowlist.**");
    expect(clarity).toContain("탐색(`--json-schema`), 사용법 확인(`--help`), 실행(`--json`) 모두 공통");
    expect(clarity).toContain("`bash -lc`, `sh -c`, `| head`, `| grep`, `| sed`, `| awk`, shell pipe");
    expect(clarity).toContain("`2>`, `&>`, `;`, `&&`, `||`, `echo`, `cat`, `wc`, `tee`, `xargs`, `jq`, `python`, `node`, `perl`, `mktemp`");
    expect(clarity).toContain("`--field-expr` 문자열 내부의 `|` 는 허용되지만 shell pipe 로 출력 후처리하면 실패");
    expect(clarity).toContain("출력이 크면 `head -c` 로 자르지 말고 더 좁은 `--field-expr` 경로");
    expect(clarity).toContain("Use the axhub clarity skill");
    expect(clarity).toContain("show current app status");
    expect(clarity).toContain("is production healthy?");
    expect(clarity).toContain("영어로 clarity skill 을 직접 지정한 요청도 반드시 이 스킬로 라우팅");
    expect(clarity).toContain("직전 답변을 재사용해서 끝내지 말고");
    expect(clarity).toContain("slash 명령이 실패한 직후라도 자연어 요청은 독립된 새 요청으로 취급");
    expect(clarity).toContain("상태 확인 범위에서 멈춰요");
    expect(clarity).toContain("최근 배포 이력 전체, 로그, 실패 커밋 분석까지 확장하지 않아요");
    expect(clarity).toContain("최근 배포 시도는 실패했지만 현재 운영은 정상이에요");
    expect(clarity).toContain("앱 상태 조회와 새 앱 생성이 한 요청에 섞이면 clarity 는 상태만 조회하고");
    expect(clarity).toContain("concept/name/slug/template 질문 없이 bootstrap 으로 넘겨요");
    expect(clarity).toContain("native Question/AskUserQuestion 을 절대 열지 말고");
    expect(clarity).toContain("question 제목 `예약 사이트 컨셉`");
    expect(clarity).toContain("질문 `새로 만들 예약 웹사이트, 어떤 컨셉으로 할까요?`");
    expect(clarity).toContain("추천 후보는 bootstrap 이 묻고 clarity 는 만들지 않아요");
    expect(clarity).toContain("단순 상태 확인은 대표 여정의 마지막 조회 단계라 **빠른 경로**");
    expect(clarity).toContain("계정 전체 상태 요청도 빠른 경로예요");
    expect(clarity).toContain("프로젝트 폴더를 스캔하지 말아요");
    expect(clarity).toContain("영어로 `app status` 처럼 특정 앱을 말하지 않고 앱 목록·상태를 묻는다면");
    expect(clarity).toContain("일반 clarity 탐색을 시작하지 않아요");
    expect(clarity).toContain("`axhub --json-schema`, `--help`, `keys[]`, `.commands.apps.workspace`, `.commands.apps.get`, `.commands.apps.status` 같은 schema/help 탐색을 모두 건너뛰어요");
    expect(clarity).toContain("optional `axhub update check --json` 도 건너뛰어요");
    expect(clarity).toContain("바로 `앱 상태 조회` 제목으로 `axhub apps list --page-size 5 --json`");
    expect(clarity).toContain("계정 전체 앱 상태 fast path 의 정상 tool call 은 최대 2개예요");
    expect(clarity).toContain("3개 이상 `명령 표면 확인` 카드가 보이면 실패예요");
    expect(clarity).toContain("`--all` 로 전체 50개 이상을 길게 뽑지 말고");
    expect(clarity).toContain("`--field-expr` 가 null/0 으로 오해될 수 있으니 이 fast path 에서는 쓰지 않아요");
    expect(clarity).toContain("디렉토리 구조 확인");
    expect(clarity).toContain("`ls`, `find`, `pwd` 류 명령을 쓰지 않아요");
    expect(clarity).toContain("계정 전체 상태 조회 중에는 `App list (axhub)` 또는 `Tenant recent deployments (axhub)`");
    expect(clarity).toContain("반드시 `axhub apps list --page-size 5 --json` CLI 로 확인해요");
    expect(clarity).toContain("단일 leaf CLI 호출");
    expect(clarity).toContain("`> /tmp/...`, `2>&1`, `;`, `&&`, `||`, `echo`, `wc`, `jq`, `cat`, `mktemp`, command substitution");
    expect(clarity).toContain("tool 출력은 assistant 내부에서 읽고 요약");
    expect(clarity).toContain("Read/파일 읽기 도구로 `*.txt`, `/tmp/*`, command output snapshot, 임시 결과 파일을 열지 않아요");
    expect(clarity).toContain("Claude Desktop 이 command output 을 파일로 접어 보여줘도 그 파일을 읽지 말고");
    expect(clarity).toContain("tool call 이 4개를 넘기면 멈추고");
    expect(clarity).toContain("전체 `--json-schema` 탐색으로 돌아가지 말고");
    expect(clarity).toContain("먼저 `앱 상태 조회` 제목으로 앱 상세 조회 help gate 를 통과");
    expect(clarity).toContain("운영 배포 확인이 추가로 필요할 때만 `운영 상태 확인` 제목");
    expect(clarity).toContain("사용자에게 보이는 Bash/tool call 제목은 한국어 명사구만");
    expect(clarity).toContain("`명령 표면 확인`, `명령 사용법 확인`, `앱 상태 조회`, `운영 상태 확인`, `결과 정리`");
    expect(clarity).toContain("description/title/summary 필드는 반드시 위 고정 문구 중 하나로 직접 채워요");
    expect(clarity).toContain("도구가 자동으로 제목을 만들도록 비워두면 `axhub: App get 사용 중` 같은 이름이 보이므로 금지");
    expect(clarity).toContain("앱 상세를 조회할 때 tool 제목은 정확히 `앱 상태 조회`");
    expect(clarity).toContain("운영 배포 상태를 조회할 때는 정확히 `운영 상태 확인`");
    expect(clarity).toContain("CLI 표면이나 help 를 볼 때는 각각 `명령 표면 확인` / `명령 사용법 확인`");
    expect(clarity).toContain("`axhubing`, `axhubed`, `productioning`, `productioned`, `checking`, `executing`, `Usage 확인`, `app get`, `deploy status`, `deploy list`");
    expect(clarity).toContain("`axhub: App get 사용 중`, `productioning 배포 상태`, `productioned 배포 상태`, `Usage 확인 끝`");
    expect(clarity).toContain("tool 제목에는 제품명 `axhub` 자체를 넣지 않아요");
    expect(clarity).toContain("중간 문구에도 `Usage`, `app get`, `deploy status`, `apps get` 같은 명령·영어 단어를 쓰지 말고");
    expect(clarity).toContain("사용법 확인 끝. 앱 상태와 운영 상태만 확인할게요.");
    expect(clarity).toContain("`Ap 상태`, `App 상태`, `앱 status`, `status 조회`");
    expect(clarity).toContain("`앱 상태 조회할게요.` 또는 `앱 상태를 확인할게요.`");
    expect(clarity).toContain("raw 필드명·불리언·상태 enum");
    expect(clarity).toContain("`status: deployed`, `operating_status`, `last_deployment_status`, `production_deployment_id`, `resource: XS`, `succeeded`, `failed`, `commit_not_found`, `resolve`, `healthy: true`");
    expect(clarity).toContain("`operating_status` 값이 `dev` 라고 해서 \"운영 배포 승격 전\" 또는 \"production 이 아니다\" 라고 해석하지 않아요");
    expect(clarity).toContain("현재 운영 서비스는 정상이에요");
    expect(clarity).toContain("deployment id, commit SHA, deployment 목록을 보여주지 않아요");
    expect(clarity).toContain("`커밋 못 찾음`, \"설정 문제\", `remote push`, `commit`, 브랜치·SHA 같은 실패 분석은 diagnosis 의 책임");
    expect(clarity).toContain("이모지나 raw 화살표 목록을 쓰지 말고");
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
    expect(clarity).toContain("Claude Desktop 에서는 캐시 파일·stamp 파일·shell wrapper 없이 `버전 확인` 제목으로 단일 명령 `axhub update check --json`");
    const clarityCodeBlocksForShell = clarity.match(/```(?:bash|sh)?\n[\s\S]*?```/g) ?? [];
    expect(clarityCodeBlocksForShell.join("\n")).not.toContain("2>/dev/null");
    expect(clarityCodeBlocksForShell.join("\n")).not.toContain("head -c");
    expect(diagnosis).toContain("axhub deploy diagnose");
    expect(diagnosis).toContain("diagnose failed deployment <id> for app <slug>");
    expect(diagnosis).toContain("Diagnose failed deployment 96728617 for app my-app");
    expect(diagnosis).toContain("why did my deploy fail?");
    expect(diagnosis).toContain("영어 실패 배포 진단 요청도 반드시 이 스킬로 라우팅");
    expect(diagnosis).toContain("axhub deploy status <deployment-id>");
    expect(diagnosis).toContain("axhub deploy logs <deployment-id>");
    expect(diagnosis).toContain("CLI 전용");
    expect(diagnosis).toContain("MCP `deployment_diagnosis` 같은 deployment MCP 도구가 보여도 호출하지 않아요");
    expect(diagnosis).toContain("MCP `App list`, `App get`, `deployment_diagnosis` 같은 도구만으로 진단을 대신하지 않아요");
    expect(diagnosis).toContain("사용자에게 보이는 진행 문구와 Bash/tool call 제목은 한국어로만");
    expect(diagnosis).toContain("CLI 표면 확인");
    expect(diagnosis).toContain("실패 배포 상태 확인");
    expect(diagnosis).toContain("실패 배포 로그 확인");
    expect(diagnosis).toContain("실패 배포 상태·로그 확인");
    expect(diagnosis).toContain("현재 라이브 상태 확인");
    expect(diagnosis).toContain("Bash/명령 tool 을 호출할 때 description/title/summary 필드는 반드시 위 고정 문구 중 하나로 직접 채워요");
    expect(diagnosis).toContain("도구가 자동으로 제목을 만들도록 비워두지 말고");
    expect(diagnosis).toContain("`axhubing CLI 확인` 같은 자동 생성 제목이 보이면 같은 명령이라도 `CLI 표면 확인` 으로 제목을 고쳐서 호출해요");
    expect(diagnosis).toContain("`axhubing`, `axhubed`, `diagnosing`, `checking` 처럼");
    expect(diagnosis).toContain("`axhubed CLI 확인` 대신 항상 `CLI 표면 확인`");
    expect(diagnosis).toContain("도구 제목에는 제품명 `axhub` 자체도 넣지 않아요");
    expect(diagnosis).toContain('CLI_NAME="ax""hub"');
    expect(diagnosis).toContain('CLI_BIN="$(command -v "$CLI_NAME" || true)"');
    expect(diagnosis).toContain('"$CLI_BIN" deploy status --help');
    expect(diagnosis).toContain('"$CLI_BIN" deploy logs --help');
    expect(diagnosis).toContain('"$CLI_BIN" deploy diagnose --help');
    expect(diagnosis).toContain("command line 첫 단어의 bare `axhub` 를 쓰지 않아요");
    expect(diagnosis).toContain("변수명에도 `axhub` 를 넣지 않아요");
    expect(diagnosis).toContain('"$CLI_BIN" deploy status <deployment-id>');
    expect(diagnosis).toContain('"$CLI_BIN" --json deploy diagnose <앱>');
    expect(diagnosis).toContain("사용자에게 보이는 문장에서는 영어 진행 문장을 쓰지 않아요");
    expect(diagnosis).toContain("`Read-only` 도 쓰지 말고 `읽기 전용`");
    expect(diagnosis).toContain("명령 이름(`axhub deploy status`, `status/logs/diagnose`)은 필요할 때만 짧게 허용");
    expect(diagnosis).toContain("description/title/summary 도 정확히 `CLI 표면 확인`");
    expect(diagnosis).toContain("description/title/summary 도 같은 고정 문구로 설정해요");
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

    const update = readRepo("skills/update/SKILL.md");
    expect(update).toContain("description: '현재 버전을 확인할게요.");
    expect(update).toContain("**CRITICAL desktop first line.**");
    expect(update).toContain('First visible assistant text must start exactly with "현재 버전을 확인할게요."');
    expect(update).toContain("Start directly; do not explain why this path was chosen.");
    expect(update).toContain("사용자에게 보이는 첫 문장은 반드시 정확히 `현재 버전을 확인할게요.`");
    expect(update).toContain("그 앞에 어떤 설명도 붙이지 않아요");
    expect(update).toContain("assistant 본문에는 같은 말을 반복하지 않아요");
    expect(update).toContain("`axhubing`, `axhubed`, `updating` 처럼 제품명을 영어 동사처럼");
    expect(update).toContain("선택 이유를 설명하지 않아요");
    for (const leakedPhrase of ["전용 스킬", "스킬을 사용", "사용하겠습니다", "라우팅", "axhub:update 스킬", "/axhub:update", "update skill"]) {
      expect(update).not.toContain(leakedPhrase);
    }
    expect(update).toContain("라벨 안에 `axhub` 를 넣지 않아요");
    expect(update).toContain("**Desktop-visible command allowlist.**");
    expect(update).toContain("Bash/명령 도구로 사용자에게 보일 수 있는 command 는 아래 계열만 써요");
    expect(update).toContain("플러그인 캐시 파일을 읽지 않아요");
    expect(update).toContain("정확히 `claude plugin list` 만 실행한 출력");
    expect(update).toContain("`claude plugin list 2>&1`");
    expect(update).toContain("`claude plugin list 2>&1 | grep ...`");
    expect(update).toContain("pipe, redirect, text filter");
    expect(update).toContain("실행하려는 command 가 `claude plugin list 2>&1` 로 떠오르면 **반드시 `claude plugin list` 로 바꿔요.**");
    expect(update).toContain("플러그인 버전, 설치 scope, 다음 CLI 확인을 영어 내부 로그처럼 chat 에 쓰지 말고");
    expect(update).toContain("scope 원문, 영어 진행 로그는 사용자에게 말하지 않아요");
    expect(update).toContain("파일 읽기 도구를 쓰지 않아요");
    expect(update).toContain("command substitution, shell wrapper, file test, pipe, redirect, text filter, 파일 읽기 도구를 쓰지 않아요");
    expect(update).toContain("이 스킬은 **버전 확인/업데이트 결과까지만** 처리해요");
    expect(update).toContain("read 작업이어도 `clarity` 소관이에요");
    expect(update).toContain("| CLI 존재 확인 (`command -v axhub`) | `CLI 설치 확인` |");
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
    expect(update).not.toContain("Plugin version");
  });

  test("development skill follows the current SDK raw-db surface", () => {
    const development = readRepo("skills/development/SKILL.md");
    const connectorSafety = readRepo("skills/development/references/connector-safety.md");
    const writeGate = readRepo("skills/development/references/write-gate.md");

    expect(development).toContain("legacy `/data` 데이터플레인");
    expect(development).toContain("sdk.apps.rawDb.tables(appId)");
    expect(development).toContain("sdk.apps.rawDb.tableRows(appId, table");
    expect(development).toContain("제거된 SDK data-plane API");
    expect(development).toContain("사용자에게 보이는 설명·툴 제목·중간 메모는 한국어로만");
    expect(development).toContain("optional 파일·설정 확인은 없어도 정상인 경우 실패처럼 보이면 안 돼요");
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
    expect(bootstrap).toContain("0-install gate 는 맥락과 무관하게 그대로 실행해요");

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

  test("onboarding MCP restart resume hook is wired", () => {
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
    expect(entries).toHaveLength(2);

    const resume = entries[1];
    expect(resume.type).toBe("command");
    expect(resume.shell).toBe("bash");
    expect(resume.command).toContain("AXHUB_NO_ONBOARDING_RESUME");
    expect(resume.command).toContain(".onboarding-mcp-restart");
    expect(resume.command).toContain("-mmin -10080");
    expect(resume.command).toContain("claude mcp get axhub");
    expect(resume.command).toContain("Resume After Restart");
    // hook is read-only: never deletes the marker, never spawns the axhub binary
    expect(resume.command).not.toContain("rm -f");
    expect(resume.command).not.toContain("axhub plugin-support");
  });

  test("mcp-ready-card encodes restart handoff and resume contracts", () => {
    const card = readRepo("skills/onboarding/references/mcp-ready-card.md");

    // marker lifecycle: write on fresh add, delete on final card
    expect(card).toContain('date > "$HOME/.axhub/cache/.onboarding-mcp-restart"');
    expect(card).toContain('rm -f "$HOME/.axhub/cache/.onboarding-mcp-restart"');

    // restart handoff card replaces same-session /mcp guidance after a fresh add
    expect(card).toContain("## Restart Handoff Card");
    expect(card).toContain("도구 활성화에는 Claude Code 재시작이 필요해요. [READY_WITH_USER_ACTION]");
    expect(card).toContain("이 세션에서 `/mcp` OAuth 를 안내하지 않아요");

    // resume procedure owned by this reference, pointed at by the SessionStart hook
    expect(card).toContain("## Resume After Restart");
    expect(card).toContain("SAFE_STOP_NONINTERACTIVE");

    // the old impossible instruction must be gone
    expect(card).not.toContain("It may require a new session before tools appear");
  });

  test("onboarding SKILL encodes MCP restart handoff invariants", () => {
    const onboarding = readRepo("skills/onboarding/SKILL.md");

    expect(onboarding).toContain(".onboarding-mcp-restart");
    expect(onboarding).toContain("Restart Handoff Card");
    expect(onboarding).toContain("Resume After Restart");
    expect(onboarding).toContain(
      "NEVER `claude mcp add` 를 실행한 그 세션에서 `/mcp` OAuth 완료나 `mcp__axhub__*` 도구 활성화를 안내하지 말아요",
    );
    expect(onboarding).toContain("NEVER `VIBE_READY` 출력 후 marker");
  });
});
