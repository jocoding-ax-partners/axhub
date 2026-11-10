import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { HOST_EXPECTATIONS, SESSION_WRAPPER_SCRIPTS } from "./fixtures/host-expectations";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");
const readJson = <T>(path: string): T => JSON.parse(readRepo(path)) as T;
// host 결합 기대값은 Claude lane fixture 를 참조해요 — codex 열은 후속 유닛이
// tests/fixtures/host-expectations.ts 에 추가해요.
const CLAUDE = HOST_EXPECTATIONS.claude;

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
  test("public metadata advertises the nine official skills", () => {
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
    expect(descriptions.join("\n")).toContain("setup/bootstrap/plugins/deploy/up/import/development/diagnosis/clarity");
  });

  test("docs carry representative journey and exactly three Korean UX samples", () => {
    const readme = readRepo("README.md");
    const agents = readRepo("AGENTS.md");
    const claude = readRepo("CLAUDE.md");

    expect(readme).toContain("첫 셋업 → 앱 생성 → 배포 → 상태 확인");
    expect(agents).toContain("첫 셋업 → 앱 생성 → 배포 → 상태 확인");
    expect(claude).toContain("첫 셋업 → 앱 생성 → 배포 → 상태 확인");
    expect(readme).toContain(CLAUDE.readme.desktopCliOnly);
    expect(readme).toContain("버전·최신 확인은 `setup` 이 맡아요");
    expect(readme).not.toContain("내 앱들이 지금 어떤 상태인지 모르겠어");
    expect(readme).toContain("`Tenant recent deployments`, `Deployment list`, `App list`, `App get` 같은 App/MCP 도구 권한 팝업으로 빠지지 않고 CLI 계약을 따라요");
    const flowRows = [
      "| 첫 셋업 | `setup` |",
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
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");
    const deploy = readRepo("skills/deploy/SKILL.md") + readRepo("skills/deploy/references/user-facing-language.md");
    const clarity = readRepo("skills/clarity/SKILL.md") + readRepo("skills/clarity/references/execution-guardrails.md");
    const clarityFrontmatter = clarity.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
    const diagnosis = readRepo("skills/diagnosis/SKILL.md") + readRepo("skills/diagnosis/references/output-contract.md");
    const importSkill = readRepo("skills/import/SKILL.md");
    const up = readRepo("skills/up/SKILL.md");
    const workflowDetails = readRepo("skills/deploy/references/workflow-details.md");
    const deployErrors = readRepo("skills/deploy/references/error-empathy-catalog.md");

    // bootstrap(옛 scaffold 통합판): 기존 폴더는 import 로, 주소 확인 뒤 dry-run, 공개는 execute 신청, 내부 이름 비노출.
    expect(bootstrap).toContain("axhub apps bootstrap");
    expect(bootstrap).toContain("이미 코드가 있는 폴더는 import 로 가요");
    expect(bootstrap).toContain("axhub apps check-availability --tenant <회사> --slug <slug> --subdomain <slug> --json");
    expect(bootstrap.indexOf("apps check-availability")).toBeLessThan(bootstrap.indexOf("--dry-run --json"));
    expect(bootstrap).toContain("axhub publish --app <slug> --visibility public --execute --json");
    expect(bootstrap).not.toContain("--visibility public --dry-run");
    expect(bootstrap).toContain("사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요");
    expect(bootstrap).not.toContain(CLAUDE.forbidden.externalAutomationSkill);
    expect(bootstrap).not.toContain(CLAUDE.forbidden.externalAutomationPlugin);

    expect(deploy).toContain("axhub deploy verify <deployment-id> --app <app>");
    expect(deploy).toContain("axhub deploy verify \"$DEPLOY_ID\" --app \"$APP_ID\"");
    expect(deploy).toContain("NEVER call `axhub deploy watch`");
    expect(deploy).toContain("bounded `axhub deploy verify \"$DEPLOY_ID\" --app \"$APP_ID\"` loop");
    // AP-16 대기 수단 우선: verify 연타 폴링 대신 CLI 내부 대기 단일 호출.
    expect(deploy).toContain("--wait --wait-interval 20s --wait-timeout 9m --json");
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
    expect(deploy).toContain("The environment label comes from `STAGING_ENABLED`, never from `prod`, `production`, or raw profile values");
    expect(deploy).toContain("사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL");
    expect(deploy).toContain("Markdown URL 링크 문법은 전부 금지");
    expect(deploy).toContain("AXHUB_GATE_POLL_ITERATIONS=0 axhub plugin-support token-gate");
    expect(deploy).toContain("never wraps token-gate in `grep`, `head`, or a multi-command pipe");
    expect(deploy).toContain("Present this step as `인증 상태 확인`, not as token-gate");
    expect(deploy).toContain("NEVER commit, push, or add `.omc/`, `.claude/`, `.codex/`, `.serena/`, `.omx/`, `.omo/`");
    expect(deploy).toContain("Never deploy a local-only commit SHA");
    expect(deploy).toContain('git -c credential.interactive=false push -u origin "HEAD:$BRANCH"');
    expect(deploy).toContain("judge push by exit code");
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
    expect(workflowDetails).toContain(CLAUDE.deploySkill.desktopPermissionWatchFail);
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

    // import(2026-11 개편판): 미리보기 → 확인 한 번 → execute 한 번 → verify 대기. 옛 표시 규칙 목록
    // (도구 제목·URL 표기·첫 문장 고정·내부 용어 치환 예시·비공개 앱 본문 검증)은 의도적으로 뺐어요.
    expect(importSkill).toContain("axhub --json plugin-support import --mode preview");
    expect(importSkill).toContain("--mode execute --approved --commit-manifest --verify-wait none");
    expect(importSkill).toContain("axhub deploy --explain --json");
    expect(importSkill).toContain("--wait --wait-timeout 9m --json");
    expect(importSkill).toContain("실행은 한 번만 해요");
    expect(importSkill).toContain("사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요");
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
    expect(clarity).toContain(CLAUDE.claritySkill.desktopAutolinkGuard);
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
    expect(clarity).toContain(CLAUDE.claritySkill.desktopAppToolExposure);
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

    // AP-12 승인 fallback 사다리 — 새로 만드는 lane(plugins 게시)의 공통 문장 전문을 fixture 로 잠가요
    // (bootstrap·import 는 자기 문장 `지금 만들고 배포까지 진행할까요?`·`지금 이대로 고쳐서 axhub 에 올릴까요?` 를
    // 쓰고 bootstrap-ux-contract·import-skill-contract 테스트가 잠가요).
    expect(readRepo("skills/plugins/SKILL.md")).toContain(CLAUDE.approvalGate.approvalFallbackSentence);
    expect(importSkill).toContain("\"지금 이대로 고쳐서 axhub 에 올릴까요?\"(올리기 / 그만두기)");
    // 재배포 lane(deploy·up)은 승인 카드 대신 배포 전 맞추기를 해요 (AP-12·AP-29, spec 681).
    for (const redeploy of [deploy, up]) {
      expect(redeploy).not.toContain(CLAUDE.approvalGate.approvalFallbackSentence);
      expect(redeploy).toContain("승인 카드를 띄우지 않아요");
      expect(redeploy).toContain("axhub deploy check --app");
      expect(redeploy).toContain("capabilities.readiness.context_url");
      expect(redeploy).toContain("--from-dotenv .env --only <이름,…> --execute");
      expect(redeploy).toContain(".axhub/deploy-answers.json");
      expect(redeploy).toContain("이렇게 맞춰서 올렸어요");
      expect(redeploy).toMatch(/값을 대화로 받지 않|채팅(?:으로 값)? 수집 금지/);
    }
    // 웹훅 우회 방지: 설정값 등록·수정 커밋은 push 전에 끝나야 해요.
    expect(deploy).toMatch(/GitHub push보다 env·수정 커밋이 먼저/);
    expect(deploy).not.toContain("axhub로 지금 배포를 진행할까요?");
    // headless 정의는 3-lane 사다리 정합 재작성본으로 잠가요 — 정의 문장이
    // `claude -p`·`codex exec` 병기 토큰을 포함해요.
    expect(deploy).toContain(CLAUDE.approvalGate.headlessDefinitionDeploy);
    expect(importSkill).toContain("질문에 답할 수 없는 실행(headless)이면 `--headless` 미리보기와 요약까지만 하고");
  });

  // #405·#387: auth probe 가 답하지 않은 것(probe_timeout·probe_no_output·cli_spawn_failed)과
  // 앱 목록을 못 읽은 것(apps_list_parse_error)은 로그인 만료도, 첫 배포(import)도, not-found 도
  // 아니에요. 첫 배포 경계는 bootstrap_plan 하나예요.
  test("probe 실패·목록 실패를 로그인 필요·첫 배포로 오판하지 않아요 (#405·#387)", () => {
    const deploy = readRepo("skills/deploy/SKILL.md");
    const workflowDetails = readRepo("skills/deploy/references/workflow-details.md");
    const deployErrors = readRepo("skills/deploy/references/error-empathy-catalog.md");
    const development = readRepo("skills/development/SKILL.md");
    const PROBE_CODES = ["probe_timeout", "probe_no_output", "cli_spawn_failed"];

    // deploy 본문: preview 문구를 바로 따르지 않고 deploy-prep 판정으로 확정해요.
    expect(deploy).toContain("Only `bootstrap_plan`");
    expect(deploy).not.toContain("If `bootstrap_plan` is present or `app_id` is missing");
    expect(deploy).toContain("`처음 배포라 앱 등록 준비`·로그인 문구는 바로 따르지 않고");
    expect(deploy).not.toContain("`처음 배포라 앱 등록 준비` 면 첫 올리기라 `import` 몫이에요");
    for (const code of [...PROBE_CODES, "apps_list_parse_error", "app_ambiguous", "axhub auth status --json"]) {
      expect(deploy, code).toContain(code);
    }
    // 로그인 안내를 빼는 건 probe 3종뿐이에요 — 실제 로그아웃·만료(다른 auth 코드)는 auth recovery 예요.
    // auth status 는 미로그인·만료를 exit 4 + error.code auth 로 내요(token_expired 같은 코드는 없어요).
    expect(deploy).not.toContain("or a login prompt");
    expect(deploy).toContain("`axhub auth status --json` decides (exit 0 continue, exit 4 login)");
    expect(deploy).toContain("other `auth_` codes → auth recovery");
    // preflight 의 auth 판정(CLI guard)도 같은 오판 경로예요 (#405 형제 경로).
    const cliGuard = deploy.slice(deploy.indexOf("### CLI guard"), deploy.indexOf("### Routing and resolve"));
    expect(cliGuard).toContain("`auth_error_code` probe 실패는 로그인 안내 없이");

    // workflow-details: 첫 배포 경계와 재시도·판정 순서를 상세히 들고 있어요.
    expect(workflowDetails).not.toContain("If `bootstrap_plan` is non-null or `app_id` cannot be resolved");
    for (const phrase of [
      ...PROBE_CODES,
      "apps_list_parse_error",
      "axhub auth status --json",
      "axhub 서버 응답이 늦어요. 잠시 뒤 다시 시도해 주세요.",
      "앱 목록을 확인하지 못했어요. 잠시 뒤 다시 시도해 주세요.",
      "`--refresh-in-flight` exit 65",
      "exit 4 (`error.code` `auth`",
      "is a real logout or expiry",
    ]) {
      expect(workflowDetails, phrase).toContain(phrase);
    }
    expect(workflowDetails).not.toContain("and not a login prompt");
    expect(workflowDetails).not.toContain("`token_expired`/`not_logged_in`");

    // 오류 카탈로그: classify-exit 가 이 두 경우에 내는 키(4:auth.probe_failed·5:apps.list_unavailable)와
    // 같은 헤딩을 두고, 로그인 만료(exit 4·65)·not-found(exit 5·67) 항목은 그 키 항목으로 보내요.
    const catalogSection = (heading: string): string => {
      const start = deployErrors.indexOf(heading);
      expect(start, heading).toBeGreaterThan(-1);
      const rest = deployErrors.slice(start + heading.length);
      const next = rest.search(/\n#{2,3} /);
      return next === -1 ? rest : rest.slice(0, next);
    };
    expect(catalogSection("### exit 4:auth.probe_failed")).toContain("로그인 문제가 아니에요");
    expect(catalogSection("### exit 5:apps.list_unavailable")).toContain("앱이 없어진 게 아니에요");
    for (const heading of ["## exit 4 — auth required", "## exit 65 — auth required"]) {
      expect(catalogSection(heading), heading).toContain("`exit 4:auth.probe_failed`");
    }
    for (const heading of ["## exit 5 — resource not found", "## exit 67 — resource not found"]) {
      expect(catalogSection(heading), heading).toContain("`exit 5:apps.list_unavailable`");
    }

    // development 도 auth_ok=false 를 전부 로그인 필요로 읽지 않고, auth status 를 exit 코드로 갈라요.
    for (const code of [...PROBE_CODES, "axhub auth status --json", "exit 4"]) {
      expect(development, code).toContain(code);
    }
    expect(development).toContain("exit 0 이면 로그인된 거라 이어가고");
  });

  // #405·#387: `auth status` 가 답했어도 로그인 판정이 없으면 probe 실패와 같아요 — `/me` 가 429·5xx 였거나
  // (`api`·`rate_limited`·`timeout`) 닿지 못했어요(`other`). CLI(prep_outcome::is_auth_probe_failure)가 이 넷을
  // probe 3종과 함께 판정 없음으로 보니, 판정 없음 코드를 적은 줄은 일곱 코드를 다 적어야 로그인 안내로 새지 않아요.
  test("판정 없음 코드 목록은 /me 429·5xx·연결 실패 코드까지 함께 적어요 (#405·#387)", () => {
    const UNANSWERED_CODES = ["`api`", "`rate_limited`", "`timeout`", "`other`"];
    const lists: string[] = [];
    for (const file of readdirSync(join(REPO_ROOT, "skills"), { recursive: true, encoding: "utf8" })) {
      if (!file.endsWith(".md")) continue;
      for (const line of readRepo(join("skills", file)).split("\n")) {
        if (line.includes("probe_timeout")) lists.push(`${file}: ${line}`);
      }
    }
    // deploy·up·development 본문, deploy workflow-details·error-empathy-catalog 예요.
    expect(lists.length).toBeGreaterThanOrEqual(8);
    for (const entry of lists) {
      for (const code of UNANSWERED_CODES) {
        expect(entry, code).toContain(code);
      }
    }
    // preflight 의 `--version` 이 실패하면(`cli_runtime_error`) 로그인 판정은 resolve 의 `auth_` 코드가 해요.
    const details = readRepo("skills/deploy/references/workflow-details.md");
    const envelope = details.slice(details.indexOf("## Deploy-prep envelope"), details.indexOf("## Target reconciliation"));
    expect(envelope).toContain("`cli_runtime_error`");
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
    expect(development).toContain(CLAUDE.developmentSkill.desktopCodeMode);
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

    // bootstrap(옛 scaffold 통합판)은 carry-over 문단 없이 이미 된 단계만 건너뛰어요 — 공유 계약을 끌어오지 않아요.
    expect(bootstrap).not.toContain("../deploy/references/session-carryover.md");
    expect(bootstrap).toContain("이미 된 단계는 건너뛰어요");

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
    expect(entries).toHaveLength(2);

    // KTD6: 인라인 command 는 hooks/session-*.sh wrapper 로 추출됐어요 —
    // hooks.json 은 bare 위임만 갖고, 계약 본문은
    // wrapper 스크립트가 소유해요 (codex 훅 trust hash 재신뢰 스팸 방지 구조).
    // wrapper 파일명 목록은 fixture 의 SESSION_WRAPPER_SCRIPTS 가 단일 소유해요.
    entries.forEach((entry, index) => {
      expect(entry.type).toBe("command");
      expect(entry.shell).toBe("bash");
      expect(entry.command).toBe(CLAUDE.surface.hookWrapperCommand(SESSION_WRAPPER_SCRIPTS[index]));
    });

    // every SessionStart hook injects context invisibly: suppressed JSON only,
    // never plain echo stdout and never a user-facing systemMessage banner
    for (const wrapper of SESSION_WRAPPER_SCRIPTS) {
      const script = readRepo(`hooks/${wrapper}`);
      expect(script).toContain("suppressOutput");
      expect(script).toContain("additionalContext");
      // 역방향 계열 — fixture 값이어도 not.toContain 은 여기 명시적으로 남겨요.
      expect(script).not.toContain(CLAUDE.forbidden.hookBannerField);
      expect(script).not.toContain('echo "');
      // Windows: backslash plugin-root paths must be slash-normalized before
      // JSON interpolation, or C:\... produces invalid JSON escapes
      if (script.includes(CLAUDE.surface.hookPluginRootEnv)) {
        expect(script).toContain(CLAUDE.surface.hookSlashNormalizeMarker);
      }
    }

    // 2번째 entry: 플러그인 업데이트 재시작 확인 (restart-confirm)
    const restartConfirm = readRepo("hooks/session-restart-confirm.sh");
    expect(restartConfirm).toContain("AXHUB_NO_AUTO_UPDATE");
    expect(restartConfirm).toContain("no-auto-update");
    expect(restartConfirm).toContain(".plugin-update-restart");
    expect(restartConfirm).toContain("-mmin -10080");
    // AP-26: 훅이 직접 판정해요 — 프롬프트 md 없이 로드된 plugin.json 버전을 비교해요.
    expect(restartConfirm).toContain(".claude-plugin/plugin.json");
    expect(restartConfirm).not.toContain("plugin-restart-confirm-prompt.md");
    // dev 가드는 이 entry 에 적용하지 않아요 — marker 는 머신 전역이라
    // 어느 세션의 확인도 유효해요 (eng review #1)
    expect(restartConfirm).not.toContain("../../.git");
    // 바이너리 무접촉 — 적용 확인 뒤 marker 삭제만 해요
    expect(restartConfirm).not.toContain("command -v axhub");
  });

  // 첫 문장 고정 문구("기존 앱을 axhub에 가져올 준비를 확인할게요.")는 2026-11 import 개편 때 뺐어요.
  test("import frontmatter routes existing code here without preamble leaks", () => {
    const importSkill = readRepo("skills/import/SKILL.md");
    const importFrontmatter = importSkill.match(/^---\n([\s\S]*?)\n---/)?.[1];
    expect(importFrontmatter, "import SKILL frontmatter not found").not.toBeNull();
    const importDescription = importFrontmatter?.match(/^description:\s*'([^']*)'/m)?.[1];
    expect(importDescription, "import description not found in frontmatter").toBeDefined();
    expect(importDescription?.startsWith("다른 곳에서 만든 앱(이미 코드가 있는 폴더)을 axhub 에 맞게 고쳐서")).toBe(true);
    expect(importDescription).toContain("빈 폴더 새 앱은 bootstrap, 이미 올린 앱 재배포는 deploy 예요");
    expect(importDescription).not.toContain("스킬을 사용");
    expect(importDescription).not.toContain("사용하겠습니다");
    expect(importDescription).not.toContain("axhub:import 스킬");
  });
});
