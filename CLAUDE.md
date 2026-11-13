<!-- codegraph:start -->
# CodeGraph — Code Intelligence

The monorepo is indexed by CodeGraph (`codegraph_*` MCP tools, server `codegraph serve --mcp`). Consult it BEFORE editing, not during.

- **Before editing a function, class, or method**, run `codegraph_impact` (or `codegraph_callers`) on it and report the blast radius to the user.
- **"How does X work?"** — start with `codegraph_context`, then ONE `codegraph_explore` for the symbols it surfaces. Don't grep + read in a loop.
- **"How does X reach Y?"** — `codegraph_trace` from→to returns the whole path in one call.
- Symbol lookup: `codegraph_search`; one symbol's source: `codegraph_node`; directory listing: `codegraph_files`.
- Results look stale or "Not initialized"? Check `codegraph_status`; Read files the staleness banner lists.
- Index lives at the monorepo root `.codegraph/` (gitignored). One-time setup: `codegraph init --index` at the monorepo root. New worktrees get a copy automatically via the lefthook `post-checkout` hook.
<!-- codegraph:end -->

> **모노레포 안이다.** 이 앱은 `axhub-mono` 의 `clients/plugin/` 다.
> 브랜치·커밋·워크트리 규칙과 앱 목록은 레포 루트의
> [`CLAUDE.md`](../../CLAUDE.md) · [`README.md`](../../README.md) 를 따른다.
> 브랜치는 `plugin/<spec번호>-<슬러그>` — **앱 접두사가 없으면 Spec Kit 이
> 다른 앱의 번호를 집는다.** 도는 워크플로는 루트 `.github/workflows/` 의 것뿐이고,
> 이 폴더의 `.github/` 는 원본 보관용이라 실행되지 않는다.

# axhub plugin (diet 체제)

> **정책 기준 문서:** 에이전트 행동 규칙은 `docs/policy/agent-policy.md`, 개발·운영 규칙은 `docs/policy/dev-policy.md`, 사용자 공개 정책은 `POLICY.md` 가 기준이에요. 이 파일의 요약과 다르면 정책 문서를 따라요. 어긋남은 `tests/policy-parity.test.ts` 가 잡아요.

axhub plugin은 instruction-only diet 뒤 현재 **7 skill** (`setup` / `bootstrap` / `plugins` / `deploy` / `import` / `development` / `operate`)을 제공해요. `deploy` 는 옛 `up`(지금 폴더 그대로 올리기)·`diagnosis`(배포 실패 원인 진단)를 합친 스킬이에요. `setup` 은 옛 `onboarding`·`update` 를 합친 점검 스킬이에요 (CLI 설치·버전 업데이트·로그인·회사·git). `operate` 는 옛 `clarity` 의 새 이름이에요 — 이미 있는 앱과 회사의 운영 명령을 맡아요. `plugins`는 사용자 명시 결정으로 추가됐고 deploy_method=plugin인 일반 App(회사 플러그인)의 찾기·정확한 버전 받기·설치·올리기와 게시 심사 신청·내리기를 담당해요. 게시자 관리는 App Console, 승인은 Console Review로 연결하고 판정·실행은 ax-hub-cli(`axhub`)에 둬요.

이 instruction-only diet (단일 SKILL.md 본문 + 라이브 `--help` 디스커버리 + corpus 없는 frontmatter 라우팅 + 작은 N skill) 은 외부 prior art 와 정합해요 — Supabase 의 공식 agent-skills (https://github.com/supabase/agent-skills) 도 같은 패턴(소수 skill · `--help` 디스커버리 · corpus 없는 frontmatter 라우팅)을 채택했어요. 그래서 라우팅 품질은 외부 corpus 가 아니라 frontmatter `description`·`examples` 에 투자해요.

**codex 파생 번들 (AP-20):** 소스는 claude-first 단일 트리 하나이고, OpenAI Codex CLI 용 번들 `plugins/axhub-codex` 는 `scripts/build-plugin-bundle.ts --host codex` 가 파생 생성해요 — 치환 테이블(`CODEX_SUBSTITUTIONS`, longest-first) + `codex-overrides/` 스왑(README/POLICY codex 판) + 훅 변환(`shell` 키 제거·`commandWindows` 추가·상태 marker `-codex` suffix) + 매니페스트 name 재작성(axhub-codex) + description 재합성으로요. 파생 번들은 직접 수정하지 않고 소스·override 를 고쳐 재생성하며(`bun run plugin:bundle:all`), drift·FORBIDDEN·hash-pin 게이트는 `tests/codex-bundle.test.ts` 가 강제해요. codex 노출 경로는 `.agents/plugins/marketplace.json`(axhub-codex 단일 엔트리, version 생략) 이에요 — codex 는 `.agents` 를 읽으면 legacy `.claude-plugin` 을 완전히 가리고, Claude Code 는 `.agents` 를 읽지 않아요(실측). 그래서 Codex 카탈로그엔 codex 번들만, Claude 엔 `.claude-plugin` 의 axhub 만 떠요. 타 host 가 `.agents` 를 채택하는 순간 axhub 엔트리를 재병기해야 해요 — DP-8 분기 재검증 항목이에요.

## 자동 업데이트 hook

diet 가 제거한 hook 중 **auto-update SessionStart 훅 1개**만 `hooks/` 로 재도입했어요. AP-26 부터는 에이전트 프롬프트 없이 **훅 스크립트(`hooks/session-auto-update.sh`)가 `"async": true` 백그라운드 worker 로 직접 확인·적용**해요 — 사용자에게 묻지 않고, 세션을 막지 않고, 에이전트 명령·권한 팝업이 0개예요. Claude Code 가 `hooks/hooks.json` 을 자동 발견해요 — plugin.json 선언은 불필요해요.

- **실행 모델:** hooks.json entry 1 이 `"async": true, "timeout": 5` 예요. async 를 아는 호스트는 비차단으로 돌리고 실행 중 timeout 을 적용하지 않아요 (Claude Code hooks reference "Run hooks in the background"). 끝나면 stdout JSON 의 `additionalContext` 만 **같은 세션의 다음 턴**에 에이전트에게 전달돼요 — 사용자 화면 노출 없음. async 를 모르는 구 호스트는 동기 훅으로 취급해 5초에 취소하므로 세션 시작 지연이 5초를 넘지 않아요 (apply 는 atomic swap 이라 중간 취소가 안전). `-p` headless 세션은 teardown 때 async 훅을 kill 해요 — throttle 캐시는 lock 을 잡은 직후 이미 갱신돼서, 그 회차(구 호스트의 5초 취소도 같아요)는 잃고 24h 뒤 다음 주기에 재시도해요(수용). 훅 계열 중 유일하게 네트워크·바이너리 교체를 하는 예외이고, 다른 전경 훅은 여전히 네트워크 0 이에요.
- **gate (worker 첫 수 ms):** kill switch → dev 가드 → CLI 3-경로(AP-17: `command -v axhub` → `~/.axhub/bin-path` → `~/.axhub/bin/axhub`(.exe), 찾은 절대경로로 직접 실행) → `~/.axhub/cache/.plugin-update-check` mtime 24h throttle → `~/.axhub/cache/.auto-update.lock/` `mkdir` 원자 lock(30분 TTL 회수, 못 잡으면 캐시 미touch). 해당 없으면 출력 없이 exit 0.
- **dev 가드:** 소스 트리에만 있는 번들러 `${CLAUDE_PLUGIN_ROOT}/../../scripts/build-plugin-bundle.ts` 또는 `${CLAUDE_PLUGIN_ROOT}/scripts/build-plugin-bundle.ts` 가 있으면(`plugins/axhub`·`dist/axhub-plugin` in-place 로딩·레포 루트 직접 로딩) auto-update 를 조용히 skip 해요 — 메인테이너 세션 노이즈 방지. 모노레포에선 `.git` 이 레포·worktree 루트에만 있어 `.git` 으로는 판정하지 않아요. 번들은 `scripts/` 를 싣지 않아 end user 의 `~/.claude/plugins/cache/...` 경로는 무영향이에요.
- **판정:** `axhub update check --plugin-version <plugin.json version> --field-expr '[.current,.has_update,.latest,.disabled,.is_downgrade,.plugin.has_update,.plugin.latest]|map(tostring)|join("|")'` 1회로 `|` 구분 7필드를 받아 `IFS='|' read` 로 읽어요 (jq·sed 없음). 공백 구분은 CLI 가 disabled·feed 장애 때 내는 빈 `latest` 를 기본 IFS 가 접어 필드가 밀려요. `disabled` 면 `SKIP_DISABLED`, `latest` 가 비면(feed 장애) CLI 쪽만 `CHECK_FAILED reason=cli_feed` 로 남기고 플러그인 단계는 이어가요. `--field-expr` 를 모르는 구 CLI(exit 64)는 `--json` 원문에서 top-level 만 뽑아 CLI 만 올리고 플러그인은 다음 주기로 미뤄요. 실패·필드 부족은 `CHECK_FAILED` 로 log 하고 멈춰요 — 침묵을 최신으로 읽지 않아요.
- **CLI 업데이트:** `has_update && !disabled && !is_downgrade` 이고 halt marker 의 버전이 `latest` 와 다르면 `axhub update apply --execute --yes --json` 을 실행해요(즉시 반영, 판정은 exit 코드). exit 14/66(digest·cosign) → `~/.axhub/cache/.auto-update-halt`(`<latest>|<exit>`, 브리지 경유면 `<latest> via v0.44.2|<exit>`)를 쓰고 같은 버전은 재시도하지 않아요. 새 `latest` 가 나오면 halt 를 지우고 다시 시도해요.
- **서명 신원 브리지 (#453):** 현재 CLI 가 0.44.1 이하이고 `latest` 가 0.44.2 보다 높으면 목표는 먼저 0.44.2 예요. 이 CLI 들은 옛 레포 서명만 믿어 mono 가 서명한 최신을 exit 66 으로 막아요. 그래서 `AXHUB_UPDATE_FEED_URL=https://cli.axhub.ai/identity-bridge` 로 버전 포인터만 바꿔 두 서명을 다 믿는 0.44.2 를 받고(자산·SHA256·cosign 검증은 그대로), 같은 run 에서 최신으로 이어요. 포인터는 CLI 릴리스 워크플로가 매번 게시해요(`clients/cli/scripts/identity-bridge.sh`). 브리지만 받고 멈추면 log 에 `cli=<이전>-><0.44.2>` 를 덧붙이고 그 변경을 알려요.
- **apply timeout (#386):** 현재 CLI 가 0.46.10 미만이면 브리지·본 apply 둘 다 `AXHUB_TIMEOUT=10m` 을 붙여요 — 이 CLI 들은 전역 timeout(기본 30초)을 아카이브 다운로드 전체에 걸어 느린 회선에서 exit 10 이 반복돼요. 0.46.10 이상은 아카이브에 10분 하한이 있고 전역 timeout 을 늘리면 멈춤 감지만 길어져서 그대로예요.
- **플러그인 업데이트:** `plugin.has_update` 면 `claude plugin list` 로 scope 감지(기본 user) → `claude plugin marketplace update axhub`(실패해도 계속, log 에 `marketplace=failed`) → `claude plugin update axhub@axhub --scope <scope> --json` — **재시작해야 반영**돼요. exit 0 은 `updateOutcome` 이 `updated`·`up_to_date`·`skipped` 모두라서, `updated` 일 때만 marker(`~/.axhub/cache/.plugin-update-restart`, 내용=받은 버전|scope)를 기록하고 나머지는 `PLUGIN_SKIPPED reason=not_updated` 예요 — 낡은 카탈로그의 "already at the latest version" 으로 재시작 안내가 반복되지 않아요. `claude` 가 PATH 에 없으면 플러그인만 건너뛰고 log 에 남겨요.
- **알림:** 실제로 바뀐 게 있을 때만 worker 가 종료 직전 suppressed JSON 1회를 내요 — "axhub CLI 가 vA → vB 로 자동 업데이트됐어요" 또는 보안 검증 실패 한 줄("강제로 진행하지 말고 회사 IT·보안팀에 알려주세요"). 에이전트는 다음 응답 앞머리에 그 한 줄만 말하고 버전 확인 명령을 실행하지 않아요. 최신이면 출력이 없어요.
- **관측:** `~/.axhub/cache/auto-update.log` 에 run 당 1줄(`<UTC ts> host=<claude|codex> <RESULT> k=v…`, 200줄 상한). 결과 어휘: `UP_TO_DATE`·`UPDATED`·`CHECK_FAILED`·`APPLY_FAILED`·`SECURITY_HALT`·`SKIP_HALTED`·`SKIP_DISABLED`·`SKIP_DOWNGRADE`·`PLUGIN_UPDATED`·`PLUGIN_FAILED`·`PLUGIN_SKIPPED`.
- **재시작 확인 훅 (2번째 entry, 동기):** SessionStart 가 marker(7일 TTL, mtime)를 감지하면 `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 의 `version`(이 세션이 실제 로드한 버전) ≥ marker 버전이면 "플러그인 v<x> 적용을 확인했어요" 한 줄을 emit 하고 marker 를 삭제해요. 낮으면 "재시작하면 적용돼요" 한 줄만 emit 하고 marker 를 둬요. `claude plugin list` 는 실행하지 않아요(에이전트 명령 0). dev 가드는 이 entry 에 적용하지 않아요 — marker 는 머신 전역이라 어느 세션의 확인도 유효해요.
- **끄기:** `AXHUB_NO_AUTO_UPDATE=1` 또는 marker 파일 `~/.axhub/config/no-auto-update`(모든 훅 kill switch 는 env 에서 `AXHUB_` 를 뗀 소문자-하이픈 marker counterpart 를 가져요) — 둘 중 하나면 worker 는 캐시·lock·log 어느 것도 만들지 않고 완전히 건너뛰어요(안내 없음). 훅 bash 는 shell profile 을 소싱하지 않아 profile 에만 export 한 GUI 세션에선 env 를 못 볼 수 있어요 — 그 경우 marker 파일이 닿는 채널이에요. kill switch 는 이 2-계층이 계약이에요.
- **Windows:** hook 은 `"shell": "bash"` 로 고정했어요 — Windows 에선 Git Bash 로 돌고(없으면 silent PowerShell fallback 대신 깨끗이 skip), `bash`·`find`·`sed`·`awk`·`command -v`·`$HOME` 등 Git for Windows 번들 도구만 써요 (jq·node 같은 외부 의존 없음). 백그라운드 프로세스는 harness 가 관리하므로 스크립트 안에서 fork 하지 않아요.
- **codex 판:** codex 도 `async: true` command 훅을 지원해요 (developers.openai.com/codex/hooks). 차이는 셋 — codex 는 async 에도 timeout 을 적용하므로 파생 번들에선 `timeout` 키를 빼고(기본 600s), 사용자가 훅을 신뢰한 뒤에만 실행되며, 플러그인 갱신 명령 블록을 치환 테이블이 통째로 바꿔요(`codex plugin marketplace upgrade axhub` → 실패 시 `codex plugin add axhub-codex@axhub`, marker `-codex`). wrapper 본문 변경은 codex 재신뢰 없이 반영되므로 CHANGELOG 에 명시해요.
- best-effort·비차단 — 실패·구 CLI·네트워크 오류면 log 만 남기고 사용자의 작업을 막지 않아요. skill 들의 기존 `1a 버전 체크`(10분 TTL, 안내만)와 보완 관계예요.
- **수동 on-demand counterpart:** CLI 업데이트를 사용자가 직접 부르는 진입점은 `setup` skill (`skills/setup/SKILL.md`) 이에요 — 훅과 달리 24h throttle 없이 바로 `axhub update check --json` 으로 확인하고 새 버전이면 `axhub update apply --execute --yes --json` 으로 올려요. 플러그인은 스킬이 직접 갱신하지 않고, 쓰는 AI 앱의 플러그인 화면에서 업데이트하도록 안내해요.

## CLI 호출 표면

- skill 들은 흡수된 helper 표면을 `axhub plugin-support <cmd>` (hidden 그룹) 로 호출해요 (`operate` skill 은 예외 — preflight 확인 뒤 운영은 공개 표면만 탐색·실행, `deploy` skill 의 실패 진단은 공개 `axhub deploy status`/`deploy logs`/`deploy diagnose` 만 써요) — 예: `axhub plugin-support onboarding-detect`, `axhub plugin-support preflight`, `axhub plugin-support deploy-prep`. hidden 명령은 외부 무보증이지만 계약 parity 테스트 + 최소 CLI 버전 게이트로 plugin 과 동기화돼요.
- 사용자 가치가 있는 공개 표면은 `axhub deploy push --app <app>`, `axhub deploy verify <deployment-id> --app <app>`, `axhub deploy diagnose`, `axhub plugin publish <path>` 예요.

## 최소 CLI 버전 게이트

- bootstrap·deploy skill은 `plugin-support` 기능(preflight)으로 최소 표면 **0.21.3+**를 확인하고, bootstrap은 preflight `capabilities.self_hosted_git.git_policy_decision`, deploy는 `capabilities.deploy.push` 가 없으면 CLI 업데이트로 보내요. plugins는 version을 추측하지 않고 preflight 로 CLI 를 확인한 뒤 쓰는 명령마다 `--help` 를 먼저 읽어 거기 있는 인자만 써요. operate 는 preflight 로 CLI 를 확인하고, 실행 전 고른 명령의 `--help` 를 읽어 거기 있는 인자·플래그만 써요. CLI가 없거나 기능이 없으면 멈추고 setup으로 보내며 우회하지 않아요.
- **CLI 경로 해석 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — 부모 앱(Claude Desktop·VS Code·터미널 앱)이 물려준 낡은 PATH 때문에 설치된 CLI 를 못 찾는 상태가 macOS·Linux·Windows 모두에서 흔해요. 모든 skill 의 CLI 가드는 `command -v axhub` → 위치 파일 `~/.axhub/bin-path` → canonical `~/.axhub/bin/axhub`(.exe) 순으로 찾고, 디스크에 있으면 재설치·setup 으로 돌리지 않고 그 절대경로로 `plugin-support repair-path --json` 을 실행해 영속 PATH 를 고친 뒤 같은 세션은 반환된 `bin_path` 절대경로로 이어가요. 세 경로 모두에서 못 찾을 때만 setup 을 안내해요.

## 살아남은 quality gate

- `bun run lint:tone --strict` — 모든 한글 텍스트 해요체 0 err (금지: 합니다 / 입니다 / 드립니다 / 당신).
- frontmatter validity check — 7 skill의 SKILL.md frontmatter 유효성.
- 대표 여정 회귀 — 첫 셋업 → 앱 생성 → 배포 → 상태 확인 경로를 문서·skill 본문·fixture 계약으로 같은 방향에 맞춰요.
- `bun run plugin:bundle` — `.claude`, `.omx`, `node_modules`, 인덱스 DB 같은 개발 산출물이 빠진 clean local plugin bundle 을 만들어요. 로컬 Claude Code 검증은 repo 루트가 아니라 `dist/axhub-plugin` 을 써요.
- 실제 ax-hub-cli 구현/schema parity/release 는 이 plugin repo 범위 밖 follow-up 으로 남겨요.

## Release flow (commit-and-tag-version, 단순화 + 수동 미러)

plugin 릴리즈는 `commit-and-tag-version` 기반 3단계로 bump·tag 를 만들고(postbump 는 단순화 — codegen:version·release:check·5-binary build·bin/ add 전부 제거), 공개 레포 동기화와 GitHub release 는 mono 워크플로를 손으로 실행해요. tag push 로 도는 워크플로는 없어요. 기준 절차는 `docs/policy/dev-policy.md` DP-4 예요.

```bash
# step 1 — plugin/ 브랜치에서 bump + commit (tag 미생성)
bun run release
# step 2 — CHANGELOG narrative (해요체 1-3 문장) 추가 후 amend → PR 머지
git commit --amend --no-edit -a
# step 3 — 머지된 main 에서 plugin-vX.Y.Z tag 생성 + push
bun run release:tag
# step 4 — 공개 레포 main 으로 미러 (dry-run 확인 후 실제 실행, 로그에 `axhub: 머지 완료`)
gh workflow run repo-mirror.yml --ref plugin-vX.Y.Z -f target=plugin -f dry_run=true
gh workflow run repo-mirror.yml --ref plugin-vX.Y.Z -f target=plugin -f dry_run=false
# step 5 — 미러 머지 확인 뒤 mono GitHub release + Slack 공지
gh workflow run plugin-release.yml -f tag=plugin-vX.Y.Z
```

사용자는 공개 레포 main 을 읽으므로 step 4 전에는 아무에게도 나가지 않아요. 미러는 tag 를 넘기지 않아서 공개 레포에는 tag·GitHub release 가 생기지 않아요.

## deploy 성공 선언 규칙

- deploy 성공 선언은 CLI 의 최종 결과로만 해요 — `axhub deploy push --app <app> --execute --wait` 가 같은 배포 번호로 끝까지 기다린 결과, 또는 끝나지 않았거나(exit 6·10, `status` 가 `started`·`running`) 기다리다 끊겼을 때 다시 올리지 않고 이어 부르는 `axhub deploy verify <deployment-id> --app <app> --wait --wait-timeout 9m` 의 결과예요. latest 재탐색 경로는 금지이고, 확인하지 못한 항목은 됐다고 표시하지 않아요.
- **올리는 길은 CLI 가 골라요 (AP-23):** 저장 위치(Axhub 저장소·GitHub·정적 사이트)별 길·배포 번호 찾기·기다리기는 `axhub deploy push` 몫이에요(preflight `capabilities.deploy.push`, CLI 0.62.0+). 2026-11 전에는 스킬이 `apps get` 의 `deploy_method`·`git_backend` 로 selfhosted push·GitHub·static upload lane 을 직접 나눴어요. "지금 폴더 그대로"라고 해도 커밋해서 올리고(업로드 배포 없음, CLI 0.63.0), 저장소 없는 옛 앱은 Axhub 저장소로 옮길지 물어요. 앱 저장소에 안 붙은 로컬 소스 폴더는 `import` 로 넘겨요.
- **스테이징 옵트인 앱(AP-25):** 배포 결과의 `environment` 가 `staging` 이면 스테이징에만 반영됐고 운영은 심사 뒤라고 알리고 "운영 반영 완료" 라고 말하지 않아요. 심사 신청은 사용자가 요청할 때만 해요.

## Skill routing

이 repo의 공개 plugin surface는 `setup` / `bootstrap` / `plugins` / `deploy` / `import` / `development` / `operate` 일곱 스킬이에요.

Key routing rules:
- 처음 셋업·CLI 설치·버전 확인/업데이트·로그인·환경 점검 → `setup` (점검 결과 표로 끝내고 앱 생성·배포로 이어가지 않아요 — AP-8)
- 템플릿으로 새 앱 생성·첫 배포(코드 저장 위치는 회사 판정대로 Axhub 저장소·내 GitHub·회사 GitHub 조직) → `bootstrap`
- 회사 플러그인 마켓플레이스(deploy_method=plugin App) 찾기·정확한 버전 받기·Claude Code/Codex 설치·올리기와 게시 심사 신청·심사 상태·마켓플레이스 연결·내리기 → `plugins` (확인(preflight·`--tenant`, 올리기·연결·내리기는 llms.txt 「회사 플러그인 빠른 시작」·「회사 플러그인 마켓플레이스」 `curl -fsSL` 원문, `--help` 먼저) → 설치는 미리보기·위험 요약 → 한 번 묻고 `--execute --yes`, 올리기는 미리보기 → 권리 확인 질문 → `--attest-distribution-rights --execute --submit`(같은 `--idempotency-key`), 상태는 `plugin manage status` 의 `headline`·`next_action`, 내리기는 `plugin delist` 미리보기 → 묻고 실행. 업로드·심사 신청은 아직 설치 가능이 아니에요(`servable` 일 때만). 앱 배포는 deploy, 새 앱은 bootstrap, axhub 플러그인 자체 업데이트는 setup)
- 비어 있지 않은 기존 로컬 앱의 첫 axhub 연결·첫 배포 가져오기 → `import` (올릴 수 있나 → 기준 문서 `curl -fsSL` 원문 숙지 → 비교 → 안내와 확인 1회(올리기 / 그만두기) → 덩어리마다 커밋하며 고치기 → 다시 비교 → execute 한 번·`deploy verify --wait` → 결과 표. `.env` 값은 `--env-from-dotenv` 로 첫 배포 전 등록 — AP-30)
- 이미 axhub 에 있는 앱의 배포·배포 실패 원인 진단 → `deploy` (대상 → 기준 문서 `curl -fsSL` 원문 숙지 → 점검 → 맞추기(승인 카드 없음, 항목마다 커밋, `env set --from-dotenv`) → `deploy push` 미리보기 → `--execute --wait` 한 번 → 결과 표. 원인만 물으면 실패 진단만 하고 아무것도 바꾸지 않아요 — AP-4. 다른 배포 서비스를 말했으면 하지 않고 멈춰요 — AP-27)
- 이미 axhub 에 있는 앱의 기능 추가·수정(화면·검색·폼·회사 데이터(커넥터) 대시보드 등) → `development` (앱 확인 → 기준 문서 `curl -fsSL` 원문·`AGENTS.md`·llms.txt 기능 문서 숙지 → 회사 데이터는 쓸 때만 `axhub connectors mine`·`resources` 로 실제 구조 확인 → 계획 두세 줄 보여주고 바로 만들기(되돌리기 어려운 일만 물음 — AP-5) → 빌드·검사·로컬 확인 → 결과 표. 배포는 하지 않아요)
- 이미 axhub 에 있는 앱과 회사의 운영 명령(로그·앱 이름·아이콘 같은 앱 정보·앱 DB 조회·환경변수·롤백·삭제·절전·권한 확인/부여·내 커넥터·GitHub 다시 연결(첫 연결은 bootstrap)) → `operate` (확인(preflight·`--tenant`) → 바꾸는 일은 llms.txt 운영 문서 `curl -fsSL` 원문으로 뜻 확인·비슷한 일은 차이를 보여주고 물음 → `--json-schema --field-expr` 로 좁혀 명령 찾기·`--help` 먼저 → 조회는 바로, 바꾸는 명령은 미리보기 → 한 번 묻고 → `--execute`. 없는 기능은 "axhub 에 그 기능은 없어요". 배포·실패 원인은 deploy, 앱 코드는 development, 새 앱은 bootstrap, 설치·업데이트·로그인은 setup 에 넘겨요 — 2026-11 전 이름은 `clarity`)

**모든 트리거 전제 (AP-11):** 위 규칙은 axhub 맥락(대화의 axhub 언급·현재 폴더의 axhub 연결·직전 axhub 작업)이 있을 때만 유효해요. 맥락 없는 일반 발화("배포해"·"업데이트해줘"·"로그 보여줘")는 실행·안내로 밀어붙이지 않고 한 번 묻거나 종료하며 다른 axhub 스킬로 넘기지 않아요. bootstrap·import 는 preview-confirm 승인이 backstop 이라, deploy 는 `deploy-prep` 이 처음 올리는 폴더(`bootstrap_plan`)를 import·bootstrap 으로 돌려보내서 frontmatter 게이트로만 적용해요.

**앱 git backend 선판정 (AP-24):** fresh bootstrap은 read-only `axhub apps git-backend --tenant <tenant> --json`의 top-level `git_backend`만 판정 입력으로 써요(서버가 `decision` 을 주면 그 판정을 따르고 `--git-choice` 로 넘겨요 — spec 706). 기존 코드 import는 bootstrap 과 같은 회사 판정으로 고른 `--git-choice` 를 `plugin-support import` 에 넘기고 CLI 가 경로를 정해요(`selfhosted_repo_push`, static 포함). selfhosted는 device flow·GitHub App 설치 대사를 노출하지 않아요. 이미 있는 앱의 배포 길은 `deploy push` 가 골라요(AP-23) — deploy 는 빈 폴더만 `axhub repo clone` 하고 앱 저장소에 안 붙은 로컬 소스 폴더는 `import`로 넘기며 C1/Gitea API를 직접 호출하지 않아요.

**codex 질문 프로토콜·fail-closed (AP-12 codex 판):** codex 판은 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형 + 추천안 `(추천)` + 답→행동 매핑을 쓰고, 질문한 턴은 도구 호출 없이 끝내요. 네이티브 선택 카드(`[features] default_mode_request_user_input`)가 켜진 세션에서 **빈 답변은 미승인**이고 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요. 이 규칙은 두 채널이 겹쳐 소유해요 — AP-12 정책, 그리고 실행 스킬 본문의 첫 8,000B(본문은 8,000B에서 절단되니 규칙은 그 안에 둬요).

**진입 확인 (AP-12):** 새로 만드는 일(bootstrap·import·plugin 게시)은 interactive preview 승인 하나가 axhub 진입 확인을 겸해요. 이미 AXHub 에 있는 앱의 재배포(deploy)는 승인 카드 없이 **배포 전 맞추기(AP-29)** 를 해요 — preflight `context_url` 기준 문서와 `deploy check` 로 점검·로컬 빌드를 하고, 고칠 것은 항목마다 `배포 전 맞추기: <항목>` 으로 커밋하고, 새 설정값은 `env set --from-dotenv` 로 등록한 뒤 `deploy push` 로 올려요. 물을 것(안 되는 것의 대안·되돌릴 수 없는 서버 앱 전환 등)이 있을 때만 모아서 한 번에 물어요. headless 는 미리보기와 요약까지만 하고 바꾸지 않고 멈춰요.
