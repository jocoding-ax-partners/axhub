import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { HOST_EXPECTATIONS } from "./fixtures/host-expectations";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");
// host 결합 기대값은 Claude lane fixture 를 참조해요 — codex 열은 후속 유닛이
// tests/fixtures/host-expectations.ts 에 추가해요.
const CLAUDE = HOST_EXPECTATIONS.claude;

describe("update Desktop UX contract", () => {
  test("plugin manifests stay concise — routing contracts live in skills and hooks", () => {
    const manifest = JSON.parse(readRepo(".claude-plugin/plugin.json")) as { description: string };
    const marketplace = JSON.parse(readRepo(".claude-plugin/marketplace.json")) as { plugins: Array<{ description: string }> };
    const descriptions = [manifest.description, marketplace.plugins[0]?.description ?? ""];

    for (const description of descriptions) {
      // Discover/Installed UI 에 그대로 노출되는 필드라 간결하게 유지해요.
      // update/import 라우팅 계약 본문은 skills/*/SKILL.md 와 hooks/ 가
      // 소유해요 — 아래 다른 테스트들이 그 표면을 잠가요.
      expect(description.length).toBeLessThanOrEqual(320);
      expect(description).toContain("ax-hub-cli");
      expect(description).toContain("onboarding/bootstrap/scaffold/plugins/deploy/up/import/development/diagnosis/clarity/update");
      expect(description).not.toContain("Import priority is strict");
      expect(description).not.toContain("Routing priority is strict");
      expect(description).not.toContain("현재 버전을 확인할게요");
      expect(description).not.toContain("기존 앱을 axhub에 가져올 준비를 확인할게요");
      expect(description).not.toContain(CLAUDE.surface.pluginListCommand);
    }
  });

  test("does not read plugin cache manifests in Claude Desktop", () => {
    const update = readRepo("skills/update/SKILL.md") + readRepo("skills/update/references/plugin-update.md") + readRepo("skills/update/references/post-update-continuation.md");

    expect(update).toContain("description: 'axhub 최신 확인, 버전 확인, 업데이트 전용 skill");
    expect(update).toContain("axhub가 진짜 최신인지 먼저 확인");
    expect(update).toContain('일반 UX 역할 문구나 "알아서 진행"만으로는 update 가 아니며');
    expect(update).toContain("freshness/update 단어가 없는 기존 앱 import·배포 요청은 import/deploy 로 양보해요");
    expect(update).not.toContain("명령어는 잘 몰라");
    expect(update).not.toContain("명령어를 잘 모르는");
    expect(update).toContain("일반 Code-mode script");
    expect(update).toContain(CLAUDE.updateSkill.autopilotSlashRef);
    expect(update).toContain('첫 visible assistant text는 정확히 "현재 버전을 확인할게요."');
    expect(update).toContain("사용자에게 보이는 첫 문장은 반드시 정확히 `현재 버전을 확인할게요.`");
    expect(update).toContain("app status, app creation, deployment 는 update 뒤에 이어서 처리해요");
    expect(update).toContain("내 앱들이 지금 어떤 상태인지도 알아서 봐줘");
    expect(update).toContain("새 재즈 댄스 수업 예약 앱 하나 만들어서 실제로 배포까지");
    expect(update).toContain("axhub가 진짜 최신인지 먼저 확인해주고, 내 앱들이 지금 어떤 상태인지도 알아서 봐줘");
    expect(update).toContain(CLAUDE.updateSkill.pluginCacheGuard);
    expect(update).toContain("스킬 호출 전 사전 안내 문장도 쓰지 않아요");
    expect(update).toContain("작업 디렉토리 밖이라 초보자에게 불필요한 읽기 권한 팝업이 떠요");
    expect(update).toContain(CLAUDE.updateSkill.exactListOnlyOutput);
    expect(update).toContain(CLAUDE.updateSkill.permissionCardExactList);
    expect(update).toContain(CLAUDE.updateSkill.compoundListProbeInlineCode);
    expect(update).toContain(CLAUDE.updateSkill.listRedirectInlineCode);
    expect(update).toContain(CLAUDE.updateSkill.listRedirectGrepInlineCode);
    expect(update).toContain("redirect");
    expect(update).toContain(CLAUDE.updateSkill.rewriteToBareListRule);
    expect(update).toContain("플러그인 버전, 설치 scope, 다음 CLI 확인을 영어 내부 로그처럼 chat 에 쓰지 말고");
    expect(update).toContain("필요한 경우 `현재 플러그인 버전을 확인했어요.` 라고만 말해요");
    expect(update).toContain("버전과 설치 위치를 같은 문장에 섞지 않아요");
    expect(update).toContain("설치 위치값은 업데이트 명령의 `--scope` 인자에만 쓰고 chat 에는 쓰지 않아요");
    expect(update).toContain("플러그인 확인 직후에는 `현재 플러그인 버전을 확인했어요.` 만 보여줘요");
    expect(update).toContain(CLAUDE.updateSkill.readFullListInternally);
    expect(update).toContain("파일 읽기 도구를 쓰지 않아요");
    expect(update).toContain(CLAUDE.updateSkill.singleListForVersion);
    expect(update).toContain(CLAUDE.updateSkill.listFailureFallback);
    expect(update).toContain("scope 원문, 영어 진행 로그는 사용자에게 말하지 않아요");
    expect(update).toContain("`現재 버전을 확인할게요.`, `現在 버전을 확인할게요.`");
    expect(update).toContain("한글 `현재`를 한자나 일본어 문자로 바꾸는 출력은 실패예요");
  });

  test("uses the newest enabled plugin entry when Claude lists duplicates", () => {
    const update = readRepo("skills/update/SKILL.md") + readRepo("skills/update/references/plugin-update.md") + readRepo("skills/update/references/post-update-continuation.md");

    expect(update).toContain(CLAUDE.updateSkill.duplicateListEntries);
    expect(update).toContain("enabled 항목 중 **가장 높은 semver** 를 `<PLUGIN_VERSION>` 으로 삼아요");
    expect(update).toContain("`local` → `project` → `user`");
    expect(update).toContain("낮은 버전이 함께 남아 있어도 사용자에게 중복 설치·scope 원문을 설명하지 않고");
    expect(update).toContain(CLAUDE.updateSkill.reListAfterSuccess);
    expect(update).toContain("확인된 받은 버전이 CLI 응답의 플러그인 최신 버전보다 높아도 최종 카드에는 확인된 받은 버전만 한국어 결과 줄로 써요");
    expect(update).toContain("낡은 중복 항목을 나열하지 않아요");
    expect(update).toContain("중복 설치 판정 알고리즘");
    expect(update).toContain("낮은 버전 block 이 남아 있어도 그것은 cleanup 대상이 아니며");
    expect(update).toContain("현재 확인한 최고 enabled 버전이 CLI 응답의 플러그인 최신 버전 이상이면 업데이트 필요처럼 보여도");
    expect(update).toContain("`local 1.8.2` 와 `user 1.8.0` 이 함께 있으면 현재 버전은 `1.8.2`");
    expect(update).toContain("`user 1.8.0 → 1.8.2` 같은 정리성 업데이트나 결과 카드를 만들지 않아요");
    expect(update).toContain(CLAUDE.updateSkill.noUpdateForLowerDup);
    expect(update).toContain("그 최고 버전을 가진 block 들 안에서만");
    expect(update).toContain(CLAUDE.updateSkill.noCleanupUpdateWhenLatest);
    expect(update).toContain(CLAUDE.updateSkill.neverJudgeByFirstStale);
    expect(update).toContain("NEVER 최고 enabled semver 가 이미 최신인데도 낮은 중복 scope 를 기준으로");
  });

  test("keeps plugin update confirmation text human-facing", () => {
    const update = readRepo("skills/update/SKILL.md") + readRepo("skills/update/references/plugin-update.md") + readRepo("skills/update/references/post-update-continuation.md");

    expect(update).toContain("플러그인: vX -> vY 받음 (재시작 필요)");
    expect(update).toContain(CLAUDE.restartNotice.applySentence);
    expect(update).toContain(CLAUDE.restartNotice.exactSayRule);
    expect(update).toContain(CLAUDE.restartNotice.exactFinalRule);
    expect(update).toContain("`앱을 재시작해 주세요`");
    expect(update).toContain("알 수 없는 로마자 단어를 만들지 않아요");
    expect(update).toContain("확인·비교 결과를 설명하는 영어 디버그 문장이나 raw 확인 줄은 쓰지 않아요");
    for (const leakedPhrase of ["PLUGIN_UPDATED_VERSION", "matching plugin.latest", "Confirmed:", "Confirmed", "plugin.latest"]) {
      expect(update).not.toContain(leakedPhrase);
    }
  });

  test("never dead-ends a Desktop plugin-only update at the slash-command panel", () => {
    const update = readRepo("skills/update/SKILL.md") + readRepo("skills/update/references/plugin-update.md");

    expect(update).toContain("CRITICAL Desktop plugin update is executable");
    expect(update).toContain("slash command 나 대화형 패널이 아니라 Desktop Bash tool 에서 직접 실행하는 비대화형 CLI 명령");
    expect(update).toContain(CLAUDE.updateSkill.noSlashFallbackAfterList);
    expect(update).toContain(CLAUDE.updateSkill.pluginOnlyFlow);
    expect(update).toContain("사용자가 명시적으로 `플러그인만` 업데이트하고 CLI 는 건드리지 말라고 했으면");
    expect(update).toContain("이 제외 요청을 이유로 플러그인 업데이트까지 멈추면 실패예요");
    expect(update).toContain(CLAUDE.updateSkill.marketplaceCmdInlineCode);
    expect(update).toContain("marketplace 새로고침이 실패해도 기존 cache 로 plugin update 를 계속 시도해 dead-end 를 만들지 않아요");
    expect(update).toContain(CLAUDE.updateSkill.marketplaceRefreshFirst);
    expect(update).toContain(CLAUDE.updateSkill.scopedUpdateDirect);
    expect(update).toContain("대화형 패널이라 직접 실행할 수 없다");
    expect(update).toContain("인터랙티브 터미널에서 직접 하도록 떠넘기지 말아요");
  });

  test("continues mixed app-status requests after the update boundary", () => {
    const update = readRepo("skills/update/SKILL.md") + readRepo("skills/update/references/plugin-update.md") + readRepo("skills/update/references/post-update-continuation.md");
    const clarity = readRepo("skills/clarity/SKILL.md") + readRepo("skills/clarity/references/execution-guardrails.md");
    // update 라우터 본문은 hooks/update-router.sh 로, SessionStart AP-14 폴백
    // 본문은 hooks/session-update-router-guard.sh 로 추출됐어요 (KTD6) — 계약
    // 검증 표면은 hooks.json(위임 wiring) + 두 스크립트의 합집합이에요.
    const hooks =
      readRepo("hooks/hooks.json") +
      "\n" +
      readRepo("hooks/update-router.sh") +
      "\n" +
      readRepo("hooks/session-update-router-guard.sh");
    const policy = readRepo("POLICY.md");
    const clarityFrontmatter = clarity.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";

    expect(update).toContain("버전 확인과 다른 axhub 운영 요청을 함께 말하면");
    expect(update).toContain("버전 확인/업데이트 결과를 먼저");
    expect(update).toContain("업데이트 확인은 끝났어요. 이어서 요청하신 작업을 계속할게요.");
    expect(update).toContain("`App list (axhub)`, `Deployment list (axhub)`, `Tenant recent deployments (axhub)`, `App get (axhub)`");
    expect(update).toContain("read 작업이어도 CLI 계약을 우선해요");
    expect(update).toContain("원문이 영어로 `then`, `and then`, `after that`, `help me understand` 를 써도");
    expect(update).toContain("업데이트 뒤 남은 요청을 버리지 않아요");
    expect(update).toContain("CRITICAL post-update app overview");
    expect(update).toContain("CRITICAL post-update GitHub reconnect/device-code");
    expect(update).toContain("업데이트 확인은 끝났어요. 이어서 GitHub 계정 연결을 확인할게요.");
    expect(update).toContain("axhub clarity GitHub device-flow 계약을 inline 으로 적용해요");
    expect(update).toContain("`/axhub:clarity` slash command 를 새로 호출하지 않아요");
    expect(update).toContain("failing skill badge 가 보이면 실패예요");
    expect(update).toContain("`axhub git_connection_status`");
    expect(update).toContain("`axhub github status`");
    expect(update).toContain("`AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link`");
    expect(update).toContain("`axhub github accounts list --json`");
    expect(update).toContain("Desktop-visible title/description 은 모두 정확히 `계정 인증 시작`");
    expect(update).toContain("Desktop-visible title/description 은 모두 정확히 `인증 확인`");
    expect(update).toContain("인증 URL: `https://github.com/login/device`");
    expect(update).toContain("입력 코드: <USER_CODE>");
    expect(update).toContain("[https://github.com/login/device](github.com/login/device)");
    expect(update).toContain("링크/자동링크 형태로 꾸미면 실패예요");
    expect(update).toContain("코드 노출 뒤 응답을 끝내지도 말아요");
    expect(update).toContain("After approving in the browser, rerun axhub github link (or axhub github accounts list --json)");
    expect(update).toContain("저장된 pending link 가 이어져요");
    expect(update).toContain("사용자에게 다음 요청처럼 떠넘기지 말고");
    expect(update).toContain("`계정 인증 시작` command 뒤에 이 `인증 확인` command 가 보이지 않고 assistant 응답이 끝나면 실패예요");
    expect(hooks).toContain("The assistant body must print exactly two normal chat lines with the URL in inline code");
    expect(hooks).toContain("never print the URL as a bare auto-link or Markdown link such as [https://github.com/login/device](github.com/login/device)");
    expect(hooks).toContain("Do not finish after showing the code");
    expect(hooks).toContain("continue to 인증 확인 in the same assistant turn");
    expect(hooks).toContain("배포 상태 확인해줘");
    expect(update).toContain("사용자가 `승인했어` 라고 다시 말하기를 기다리지 말고");
    expect(update).toContain("NEVER update 결과 뒤 GitHub 계정 재연결/device code 흐름에서");
    expect(update).toContain("`업데이트 확인은 끝났어요. 이어서 요청하신 앱 상태 확인을 계속할게요.` 를 말한 뒤에는 설치 확인·버전 확인·플러그인 확인을 다시 하지 않아요");
    expect(update).toContain(CLAUDE.updateSkill.noReprobeAfterBoundary);
    expect(update).toContain("`axhub apps --help`");
    expect(update).toContain("`axhub apps list --json`");
    expect(update).toContain("첫 overview 의 Desktop-visible Bash command 는 아래 두 개만 허용해요");
    expect(hooks).toContain(CLAUDE.hooksContract.visibleListExact);
    expect(update).toContain("현재 폴더명·대화 맥락·가장 최근 수정 앱 중 하나로 관련 앱을 식별했으면");
    expect(update).toContain("멈춰서 \"어느 앱을 더 볼까요?\"라고 묻지 않아요");
    expect(update).toContain("상세와 최근 배포 이력까지 같은 흐름에서 바로 확인해요");
    expect(update).toContain("관련 앱을 하나로 좁혔는데 `이 중 어느 앱의 배포 상태나 로그를 더 자세히 확인하고 싶으신가요?`");
    expect(update).toContain("`어느 앱을 볼까요?`");
    expect(update).toContain("`더 자세히 확인하고 싶은 앱을 말해 주세요` 같은 질문으로 끝나면 실패예요");
    expect(update).toContain("`axhub apps get <app> --json`");
    expect(update).toContain("`axhub deploy list --app <app> --json`");
    expect(update).toContain("`Deployment list (axhub)`, `App get (axhub)`, `Tenant recent deployments (axhub)`");
    expect(update).toContain("존재하지 않는 `axhub deployment list`");
    expect(update).toContain("업데이트 결과 카드 뒤에는 남은 요청을 이어서 처리하되, 이때도 앱 상태/배포 이력은 MCP/App 도구가 아니라 위의 CLI overview 흐름으로 실행해요");
    expect(update).toContain("명령 문자열 뒤에 공백 외 어떤 문자도 붙이지 않아요");
    expect(update).toContain("존재하지 않는 단수 명령 `axhub app list`");
    expect(update).toContain("`command -v axhub && axhub --version`");
    expect(update).toContain(CLAUDE.updateSkill.compoundListProbeInlineCode);
    expect(update).toContain(CLAUDE.updateSkill.listGrepShortInlineCode);
    expect(update).toContain("`axhub apps list --json 2>/dev/null | head -100`");
    expect(update).toContain("`axhub --help | head`");
    expect(update).toContain("`&&`");
    expect(update).toContain("`2>/dev/null`");
    expect(update).toContain("`bash -lc`");
    expect(update).toContain("사용자의 추가 프롬프트를 기다리지 말고 다음 적절한 axhub 흐름을 시작해요");
    expect(update).toContain("`앱 상태 조회`, `배포 상태 조회`, `최근 배포 조회`, `GitHub 연결 상태 확인` 같은 tool 제목이 떠올랐다면");
    expect(update).toContain("Task/Subagent/Agent/백그라운드 작업으로 우회하지 않아요");
    expect(update).toContain("NEVER Task/Subagent/Agent/백그라운드 작업으로 mixed request 의 남은 앱 상태 확인을 우회하지 말아요");
    expect(update).toContain("update 결과 뒤 같은 assistant 흐름에서 직접 이어가요");
    expect(update).toContain("사용자가 `앱 상태 확인해줘`, `배포해줘`, `새 앱 만들어줘` 같은 말을 다시 하지 않아도 돼요");
    expect(update).toContain("바로 다음 axhub 흐름으로 이어가요");
    expect(update).not.toContain("다음 동작은 `clarity`");
    expect(update).not.toContain("그 다음 작업은 `clarity`");
    expect(clarity).toContain("axhub CLI 운영 명령 브리지");
    expect(clarityFrontmatter).not.toContain("disable-model-invocation");
    expect(clarity).toContain("frontmatter description 라우팅으로 자연어에서도 이 스킬이 직접 받아요");
    expect(clarity).toContain("## Do not invoke / route guard");
    expect(clarity).toContain("첫 visible assistant text 를 정확히 `현재 버전을 확인할게요.`");
    expect(clarity).toContain("이미 `/axhub:clarity` 배지가 뜬 뒤 이 문서를 읽었다면");
    expect(clarity).toContain("`Using /axhub:clarity...` 같은 문장을 쓰거나 명령을 실행하지 말고");
    expect(clarity).toContain("내 앱들이 지금 어떤 상태인지도 알아서 봐줘");
    expect(clarity).toContain("`command -v axhub && axhub --version`");
    expect(clarity).toContain("`Checking axhub CLI installation and version`");
    expect(clarity).toContain("`MCP tools to check axhub status and your apps`");
    expect(clarityFrontmatter).toContain("app status overview");
    expect(clarityFrontmatter).toContain("app creation");
    expect(clarityFrontmatter).toContain("mixed freshness+status+create prompts");
    expect(clarityFrontmatter).toContain("최신/버전/update/latest/freshness checks");
    expect(clarityFrontmatter).toContain("GitHub 계정 재연결/device code");
    expect(clarity).not.toContain("내 앱들이 지금 어떤 상태인지 모르겠어");
    expect(clarityFrontmatter).not.toContain("내 앱들이 지금 어떤 상태인지도 알아서 봐줘");
    expect(clarity).not.toContain("operational-lookups.md");
    expect(clarity).not.toContain("show a read-only axhub overview");
    expect(clarity).not.toContain("is production healthy?");
    expect(hooks).toContain("UserPromptSubmit");
    expect(hooks).toContain("SessionStart");
    expect(hooks).toContain("AXHUB_NO_UPDATE_ROUTER");
    expect(hooks).toContain("axhub freshness/update");
    expect(hooks).toContain("hookSpecificOutput");
    expect(hooks).toContain("additionalContext");
    expect(hooks).toContain("suppressOutput");
    // hook output stays invisible to the user: context injection only, no banner
    // 역방향 계열 — fixture 값이어도 not.toContain 은 여기 명시적으로 남겨요.
    expect(hooks).not.toContain(CLAUDE.forbidden.hookBannerField);
    expect(hooks).toContain("Before Finding tools");
    expect(hooks).toContain("invoke the axhub update skill");
    expect(hooks).toContain("현재 버전을 확인할게요");
    expect(hooks).toContain("allowed visible commands are exactly axhub apps --help then axhub apps list --json");
    expect(hooks).toContain("GitHub reconnect/device-code after update");
    expect(hooks).toContain("axhub clarity device-flow contract inline");
    expect(hooks).toContain("do not invoke /axhub:clarity");
    expect(hooks).toContain("failing skill badge");
    expect(hooks).toContain("never run axhub git_connection_status");
    expect(hooks).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link");
    expect(hooks).toContain("axhub github accounts list --json");
    expect(hooks).toContain("shell tool titles and descriptions exactly 계정 인증 시작 and 인증 확인");
    expect(hooks).toContain("never wait for the user to say 승인했어");
    expect(hooks).toContain("Once the assistant says 업데이트 확인은 끝났어요");
    expect(hooks).toContain(CLAUDE.hooksContract.noReprobeContract);
    expect(hooks).toContain("If folder/current conversation/latest list clearly identifies a related app, do not ask which app");
    expect(hooks).toContain("continue with CLI-only axhub apps get <app> --json and axhub deploy list --app <app> --json before any summary");
    expect(hooks).toContain(CLAUDE.hooksContract.neverDesktopAppTools);
    expect(hooks).toContain("Never run nonexistent singular axhub app list");
    expect(hooks).toContain("Never run command -v axhub && axhub --version");
    expect(hooks).toContain(CLAUDE.hooksContract.compoundListProbePlain);
    expect(hooks).toContain(CLAUDE.hooksContract.listRedirectGrepPlain);
    expect(hooks).toContain("axhub deployment list");
    expect(hooks).toContain("axhub apps --help");
    expect(hooks).toContain("axhub apps list --json");
    expect(hooks).toContain("Never add sleep, &&, pipes, redirects");
    expect(hooks).toContain("Never add sleep, &&, pipes, redirects, head, tail, grep, sed, awk, bash -lc, sh -c, or 2>/dev/null");
    expect(hooks).toContain("update-first routing guard is active for Code mode");
    expect(hooks).not.toContain("axhub update check --plugin-version");
    expect(policy).toContain('"axhub가 진짜 최신인지 먼저 확인"');
    expect(policy).toContain("가장 먼저 `update` 스킬로 처리해요");
    expect(policy).toContain(CLAUDE.policy.noPreemptBySlash);
    expect(policy).toContain("Code-mode update router guard");
    expect(policy).toContain("SessionStart fallback");
    expect(policy).toContain("UserPromptSubmit match");
    expect(policy).toContain("존재하지 않는 `axhub app list`");
    expect(policy).toContain("plural `axhub apps` 표면");
    expect(policy).toContain(CLAUDE.policy.noReprobeInAppFlow);
    expect(policy).toContain("현재 폴더명·대화 맥락·가장 최근 목록으로 관련 앱이 하나로 좁혀지면");
    expect(policy).toContain("사용자에게 어느 앱을 볼지 묻지 말고");
    expect(policy).toContain("`axhub apps get <app> --json`, `axhub deploy list --app <app> --json`");
    expect(policy).toContain("존재하지 않는 `axhub deployment list`");
    expect(policy).toContain("`Tenant recent deployments`, `Deployment list`, `App list`, `App get`");
    expect(policy).toContain("`| head`, `2>/dev/null`, `grep`, `&&` 같은 shell 후처리는 붙이지 않아요");
  });
});
