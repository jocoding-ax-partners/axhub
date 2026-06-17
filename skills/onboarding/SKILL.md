---
name: onboarding
description: 'This skill should be used when the user is new to axhub, asks what to do first, requests setup/onboarding/getting started, or says a short first-run phrase. 이 스킬은 axhub 를 처음 쓰는 사람이 셋업/온보딩 전체 과정을 한 번에 진행하고 싶어할 때 사용해요. 다음 표현에서 활성화: "셋업해줘", "셋업 해줘", "처음인데", "처음 사용", "처음 써", "처음 쓰는데", "처음 쓰는데 뭐부터", "뭐부터 하면 돼", "뭐부터 하면 되나요", "어떻게 시작하면 돼", "어떻게 시작해", "온보딩", "온보딩해줘", "시작하기", "axhub 시작", "axhub 처음", "초기 셋업", "setup", "set up", "onboard", "onboarding", "getting started", "get started", "first time", 또는 첫 사용자 셋업 의도. axhub CLI 설치(install-cli)·로그인(auth)·node 환경 감지를 순서대로 안내하고, node 가 없으면 명시 확인 후 설치해요. 빈 폴더에서도 첫 앱 만들기(init)로 자동 연결하지 않고, Ready card 에서 ‘첫 앱 만들어줘’ 같은 다음 단계만 안내해요. 환경 진단(doctor)이나 새 앱 초기화(init)와 달리 처음 사용자의 순차 온보딩을 담당해요.'
examples:
  - utterance: "셋업해줘"
    intent: "onboard axhub first-time onboarding"
  - utterance: "처음인데 어떻게 시작해"
    intent: "onboard axhub first-time onboarding"
  - utterance: "axhub 처음 쓰는데 뭐부터 하면 돼?"
    intent: "onboard axhub first-time onboarding"
  - utterance: "온보딩"
    intent: "onboard axhub first-time onboarding"
  - utterance: "getting started"
    intent: "onboard axhub first-time onboarding"
  - utterance: "set up axhub"
    intent: "onboard axhub first-time onboarding"
  - utterance: "first time using axhub"
    intent: "onboard axhub first-time onboarding"
allows-dependency-execution: true
model: sonnet
---

# Onboarding (first-run vibe coding orchestrator)

Frontmatter `description` 은 trigger 어구 baseline 때문에 보수적으로 유지해요. 실제 온보딩 범위와
안전 계약은 이 본문을 authoritative source 로 봐요.

처음 axhub 를 쓰는 사람을 위한 **온보딩 단일 진입점**이에요. 사용자는 `온보딩`, `처음인데 뭐부터`,
`getting started` 한 마디만 하면 돼요. 내부에서는 gap 마다 적절한 공개 `axhub` 명령을 직접 불러요
(`axhub auth`, `axhub update`, `axhub apps git`). 빈 폴더여도 init 스킬로 위임하지 않고, 첫 앱 만들기는 Ready card 안내로만 남겨요. 사용자는 sibling
skill 이름이나 slash command 를 몰라도 온보딩을 끝낼 수 있어요.

onboarding 의 제품 계약은 `detect-first → 첫 gap 처리 → 재감지` 루프예요. 안전하게 자동화할 수 있는 gap 은
끝까지 닫고, 브라우저 승인·OS installer GUI·PATH reload 처럼 에이전트가 대신 완료할 수 없는 gap 은
`READY_WITH_USER_ACTION` 카드와 자연어 재개 phrase(`승인했어`, `온보딩 계속`, `다시 온보딩해줘`)를 남겨요.

**책임 경계 (단일 판정원).** gap 판정·실행은 CLI 가 소유해요. `axhub plugin-support onboarding-detect --json`
한 번이 모든 gap 의 single detection source 예요 — 개별 gap 마다 preflight 를 다시 부르지 않아요. SKILL 은
한국어 안내 카드·AskUserQuestion 결정점·사용자 행동 재개 phrase(device 승인, installer 동의, 새 터미널 reload)만
렌더해요. detect JSON 의 `first_gap` 이 순서의 source of truth 예요.

## 진행 상황 알림 (Progress Reporting)

각 단계를 시작할 때 친근한 한국어 한 줄로 지금 뭐 하는 중인지 알려줘요 — vibe coder 가 멈춘 게 아니라 진행 중인 걸 알 수 있게 해요. 형식은 `○○ 하는 중이에요…`, 끝나면 `○○ 됐어요` 처럼 한 줄로 확인해요.

- 사람이 알아들을 요약만 알려요 — secret·내부 id·raw 출력·schema 본문은 chat 에 넣지 않아요 (Visibility 규칙 그대로).
- TodoWrite 가 있으면 체크리스트로도 같이 보여주고, 없는 host 에서도 이 한 줄 알림은 늘 해요.
- onboarding 은 첫 gap 하나씩 처리하고 재감지하는 분기 흐름이라 `[N/전체]` 숫자는 안 붙이고, 지금 처리 중인 단계 이름만 알려요.

단계 이름 (announce 용 한국어):
- `환경 점검하는 중이에요` (CLI·로그인·GitHub·실행환경을 한 번에 감지)
- 발견된 gap 에 따라 한 줄씩: `axhub CLI 설치하는 중이에요` · `로그인 진행하는 중이에요` · `실행환경(node·git) 점검하는 중이에요` · `GitHub App 설치 확인하는 중이에요` · `필요한 패키지 설치하는 중이에요` · `axhub 도구 연결하는 중이에요`
- `준비 다 됐어요` (Ready card)

## Workflow

**한눈에 — 실행 흐름.** read-only 감지 → 첫 gap 하나 처리 → 재감지, gap 0 될 때까지 반복 → Ready card. 순서:
`0` TodoWrite(가용 시) → `1` D1 비대화형 가드 → `2` DETECT_ALL(gap 일괄 스캔) + `2.5` GitHub App surface → `3` Gap 상태머신(첫 gap 하나 처리 후 재감지; `4`~`9` 핸들러로 분기 — CLI·인증 / git·node / GitHub App / repo·app / 의존성 / doctor) → `9.5` axhub MCP 등록+OAuth 연동(user scope, best-effort) → `10` 모든 gap 해소 시 Ready card.

**버전 체크.** onboarding 은 별도 step 없이 맨 앞 `2` DETECT_ALL 이 `cli_too_old`/`has_update` 를 잡고 `4c` 가 CLI·플러그인 업데이트를 안내해요 — 다른 3 스킬의 `1a 버전 체크` 와 같은 역할이에요 (중복 네트워크 호출을 피해 DETECT_ALL 로 통합).

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `승인했어`, `온보딩 계속`, `다시 로그인해줘`, `배포해`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

0. **TodoWrite 진행 체크리스트 (있을 때만).**

   TodoWrite 도구가 현재 host 에 노출돼 있을 때만 호출해요. Claude Desktop 처럼 TodoWrite 가 없으면 호출하지 말고, fallback todo 메시지도 만들지 말고 조용히 진행해요. 도구 가용성·생략을 사용자에게 언급하지 않아요.

   ```typescript
   TodoWrite({ todos: [
     { content: "도구 확인",       status: "in_progress", activeForm: "도구 보는 중" },
     { content: "로그인 확인",     status: "pending",     activeForm: "로그인 보는 중" },
     { content: "런타임 확인",     status: "pending",     activeForm: "런타임 보는 중" },
     { content: "GitHub 연결",     status: "pending",     activeForm: "GitHub 보는 중" },
     { content: "앱·repo 준비",    status: "pending",     activeForm: "앱 준비 중" },
     { content: "의존성 확인",     status: "pending",     activeForm: "의존성 보는 중" },
     { content: "최종 점검",       status: "pending",     activeForm: "마무리 중" }
   ]})
   ```

   매 step 과 매 AskUserQuestion 답변 뒤에 전체 todos 배열로 다시 호출해서 끝난 항목은 `completed`, 진행 항목은 `in_progress` 로 갱신해요. 워크플로 종료 시점에는 미완료 todo 가 0 개여야 해요 (남으면 다음 SKILL 화면에 버그처럼 남아요).

1. **Non-interactive AskUserQuestion guard (D1).**

   이 SKILL 의 모든 AskUserQuestion 은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 을 건너뛰고 안전 기본값으로 진행해요. 이 모드의 최종 상태는 `SAFE_STOP_NONINTERACTIVE` 예요. install/update/auth/init/deps mutation 과 git/node system install 또는 version switch 는 자동 실행하지 않아요.

2. **DETECT_ALL(read-only) — CLI 한 번으로 모든 gap 을 봐요.**

   감지는 `axhub plugin-support onboarding-detect --json` 가 cross-platform 으로 한 번에 처리해요 (CLI/auth/git/node/manifest/github/deploy). 이 블록은 **Bash tool 로 실행만 하고, 명령 본문을 chat 에 출력하지 말아요** — 사용자에겐 "도구·로그인·환경을 한 번에 확인하고 있어요" 같은 한 줄만 보여줘요. 이 명령은 exit 0 고정 fail-open 이라 CLI 가 아직 없어도 안전해요 (`cli_present:false` 로 fail-soft).

   ```bash
   if command -v axhub >/dev/null 2>&1; then
     # detect 의 self-probe 가 프로세스 PATH/HOME 차이로 axhub 를 못 찾아
     # cli_present:false 를 거짓 보고하는 걸 막으려고, 해석된 절대경로를
     # AXHUB_BIN 으로 고정해요 (axhub 가 모든 self-probe 를 이 경로로 spawn).
     AXHUB_BIN="$(command -v axhub)"; export AXHUB_BIN
     DETECT_JSON=$(axhub plugin-support onboarding-detect --json 2>/dev/null)
     [ -n "$DETECT_JSON" ] || DETECT_JSON='{"cli_present":true,"first_gap":"doctor_gap","github":{"state":"unavailable","install_url":null}}'
   else
     DETECT_JSON='{"cli_present":false,"first_gap":"cli_missing","github":{"state":"unavailable","install_url":null}}'
   fi
   echo "$DETECT_JSON"
   ```

   `first_gap=helper_outdated` 분기는 없어요 — detect 는 `axhub` CLI 안에 들어 있어서 "CLI 는 되는데 detect 가 빈 출력" 케이스가 구조적으로 사라졌어요. 위 fallback 처럼 빈 출력은 `doctor_gap` 으로 낮춰서 끝단 점검(Step 9)으로 보내요.

**대표 안전 실패 흐름.** `cli_missing` 이면 preflight/doctor 를 먼저 부르지 말고 설치 안내와 재개 phrase 만 남겨요. `cli_old` 이면 "axhub CLI 가 오래됐어요. 업데이트한 뒤 다시 시도해 주세요" 처럼 원인·행동을 짧게 말하고, raw stderr 나 내부 subcommand 덤프는 보여주지 않아요. 두 경우 모두 detect-first 근거(`first_gap`)를 따른 뒤 멈추거나 재감지해요.

   출력 JSON 주요 field (spec §2.1 schema_version="onboarding-detect/v1"):

   - `first_gap` / `gaps`: 처리할 첫 gap (아래 state machine 순서). 이걸 그대로 따라요.
   - `cli_present` / `cli_version` / `cli_state` / `cli_on_path` / `cli_too_old` / `has_update` / `latest_version`
   - `auth_ok` / `auth_error_code`
   - `git_present` / `git_repo` / `git_commit` / `node_present` / `node_version` / `node_required` / `node_mismatch` / `manifest_present` / `lockfile_present` / `deps_missing` / `dir_empty`
   - `github`: `{state, installed_logins[], uninstalled_logins[], install_url, multiple_installed}`. `state` 는 `installed` / `mixed` / `uninstalled` / `empty` / `auth_error` / `unavailable` 중 하나예요. **`install_url` 은 GitHub 조회가 성공하면 (`installed`/`mixed`/`uninstalled`/`empty`) 설치 여부·계정 수와 무관하게 항상 채워져요** (계정이 0개여도 app-level 링크로 fallback) — ready card(Step 10)와 GitHub 안내(Step 6)에서 무조건 보여줘요. `state` 가 `auth_error`/`unavailable` 면 null 이고, `auth_error` 면 `unknown` 으로 넘기지 말고 "다시 로그인해줘" 로 안내해요.
   - `deploy_checked` / `deploy_verified`

2.5. **GitHub App 설치·계정 추가 surface — DETECT 직후 무조건 (branch-independent, 비차단).**

   Step 2 detect JSON 의 `github` 를 그대로 써요 (accounts list 재호출 안 해요). `github.install_url` 이 null 이 아니면 (`github.state` 가 `installed`/`mixed`/`uninstalled`/`empty`) **설치 여부·계정 수·`first_gap` 과 무관하게 항상** 이 블록을 먼저 실행한 뒤 Step 3 gap 라우팅으로 가요. 모든 onboarding 경로가 gap 처리 전에 이 지점을 지나서, 빈 폴더처럼 GitHub 단계를 건너뛰는 경로에서도 install_url 을 맨 앞에서 한 번은 보장해요.

   **(a) install_url 한 줄 무조건 표시.** "GitHub App 설치·계정 추가 링크: `<github.install_url>`. 이미 설치돼 있어도 다른 org/계정을 더 붙일 수 있어요." `github.installed_logins` 가 있으면 "이미 연결된 계정: `<login...>`" 도 덧붙여요. `installation_id` 등 internal 값은 echo 하지 말고 `login` + `install_url` 만 보여줘요. 링크는 안내만 하고 자동으로 열지 않아요.

   **(b) 이미 1개 이상 설치된 경우 (`github.state` 가 `installed`/`mixed`) 다른 계정 설치를 한 번 물어요 (actionable, 비차단).** `uninstalled`/`empty` 면 (b) 를 건너뛰어요 — 미설치 설치 제안은 Step 6 이 소유해서 중복 질문을 막아요.

   ```json
   {
     "questions": [{
       "question": "다른 org/계정에도 GitHub App 을 설치할래요?",
       "header": "GitHub App",
       "multiSelect": false,
       "options": [
         {"label": "아니요, 계속", "description": "설치를 더 하지 않고 Step 3 gap 처리로 그대로 이어가요 (비차단 기본값)"},
         {"label": "설치할래요", "description": "install_url 을 보여주고 브라우저를 열어요. '설치했어' 또는 '온보딩 계속' 이라고 말하면 Step 2 재감지를 한 번 해요"}
       ]
     }]
   }
   ```

   `설치할래요` 면 `github.install_url` 을 열고, 사용자가 "설치했어" 또는 "온보딩 계속" 이라고 말하면 Step 2 재감지를 한 번 해요. `아니요, 계속` 은 아무 mutation 없이 Step 3 으로 이어가요. `github.install_url` 이 null (`auth_error`/`unavailable`) 이면 이 블록 전체를 생략하고, `auth_error` 면 "다시 로그인해줘" 로 낮춰요.

   **D1 비대화형 가드.** subprocess 에서는 (b) AskUserQuestion 을 건너뛰고 안전 기본값(`아니요, 계속`)으로 진행하고, install/connect mutation 이나 브라우저 열기를 자동 실행하지 않아요. (a) 표시 줄은 그대로 출력해요.

3. **Gap State Machine — 첫 gap 하나만 처리하고 재감지해요.**

   **gap 순서의 single source of truth 는 detect 의 `first_gap` 이에요.** 아래 표는 gap→처리 owner 매핑 참고용 문서일 뿐이라, 순서가 어긋나 보이면 표를 재구현하지 말고 항상 `first_gap` 을 따라요. 모든 gap 흐름은 이 SKILL 본문 Step 4-9 에 인라인돼 있어요 — sibling skill 위임은 없어요 (빈 폴더도 init 으로 위임하지 않아요).

   ```text
   START
     ↓
  DETECT_ALL(read-only)  ← axhub plugin-support onboarding-detect --json (단일 판정원)
     ├─ cli_missing         → Step 4a installer 안내 (사용자 승인) → DETECT_ALL
     ├─ cli_path_missing    → Step 4b repair-path → DETECT_ALL
     ├─ cli_old             → Step 4c axhub update check/apply → DETECT_ALL
     ├─ auth_missing        → Step 4d axhub auth status/login → DETECT_ALL
     ├─ git_missing         → Step 5 install_git(승인) → DETECT_ALL
     ├─ node_missing        → Step 5 install_node(승인) → DETECT_ALL
     ├─ node_mismatch       → Step 5 fix_node(승인+nvm) → DETECT_ALL
     ├─ github_app_missing  → Step 6 install_url → DETECT_ALL
     ├─ existing_repo_gap   → Step 7a axhub apps git status/connect → DETECT_ALL
     ├─ no_manifest_empty   → Step 7b 첫 앱 안내(advisory) → VIBE_READY_CARD
     ├─ deps_missing        → Step 8 install_deps(승인+ignore-scripts) → DETECT_ALL
     ├─ deploy_unverified   → Step 9 axhub deploy verify → DETECT_ALL
     ├─ doctor_gap          → Step 9 preflight 안내 → DETECT_ALL
     └─ no_gap              → VIBE_READY_CARD
   ```

   | gap id | 감지 조건 (Step 2 JSON) | 처리 (인라인) | 완료 확인 |
   |--------|-----------|------------|-----------|
   | `cli_missing` | `cli_present=false` | Step 4a installer 안내 | 재감지 시 `cli_present=true` |
   | `cli_path_missing` | `cli_present=true` + `cli_on_path=false` (`cli_state=on_disk_not_on_path`) | Step 4b repair-path | repair 적용 후 새 터미널 또는 resolved path 로 재확인 |
   | `cli_old` | `cli_too_old=true` 또는 `has_update=true` | Step 4c `axhub update` | apply 후 version 재확인 |
   | `auth_missing` | `auth_ok=false` (`auth_error_code` 참고) | Step 4d `axhub auth` | device approval 후 재감지 green |
   | `git_missing` | `git_present=false` | Step 5 | 설치 후 `git_present=true` |
   | `node_missing` | `node_present=false` | Step 5 | 설치 후 `node_present=true` |
   | `node_mismatch` | `node_mismatch=true` | Step 5 | target version active |
   | `github_app_missing` | `github.state` 가 `uninstalled`/`empty` | Step 6 | install_url 완료 후 재감지 |
   | `existing_repo_gap` | `git_repo=true` + `git_commit=true` + `manifest_present=false` | Step 7a `axhub apps git` | app↔repo connect 완료 |
   | `no_manifest_empty` | `manifest_present=false` + `dir_empty=true` | Step 7b 첫 앱 안내 (advisory, init 위임 없음) | advisory 표시 후 Ready card |
   | `deps_missing` | `deps_missing=true` | Step 8 | lockfile install exit 0 |
   | `deploy_unverified` | `deploy_checked=true` + `deploy_verified=false` | Step 9 | live/running/deployed 확인 |
   | `doctor_gap` | 온보딩 끝 핵심 체크 fail | Step 9 | 핵심 green 또는 PATH reload 안내 |

4. **CLI gap 흐름 (cli_missing / cli_path_missing / cli_old / auth_missing — 인라인).**

   **4a. `cli_missing`.** CLI 가 아직 없어요. OS 에 맞는 설치 채널을 한 줄로 설명하고 설치 승인을 받아요 (installer 실행은 사용자 행동 필수 차단점). 설치 후 "설치했어" 또는 "온보딩 계속" 이라고 말하면 Step 2 재감지를 해요. raw installer stderr 는 chat 에 노출하지 않아요.

   ```json
   {
     "questions": [{
       "question": "axhub CLI 를 지금 설치할까요?",
       "header": "CLI 설치",
       "multiSelect": false,
       "options": [
         {"label": "설치", "description": "OS 에 맞는 공식 설치 스크립트를 안내하고 실행 승인을 받아요"},
         {"label": "나중에", "description": "설치하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   설치 채널 안내 (단일 공식 채널):
   - macOS / Linux: `curl -fsSL https://cli.axhub.ai/install.sh | sh`
   - Windows: `irm https://cli.axhub.ai/install.ps1 | iex`

   **4b. `cli_path_missing`.** CLI 는 디스크에 있는데 PATH 에 안 잡혀요 (`cli_state=on_disk_not_on_path`). CLI 가 자기 PATH 를 고쳐요 — shell rc 수정 + backup 은 `repair-path` 가 소유해요.

   ```bash
   axhub plugin-support repair-path --json
   ```

   JSON: `{repaired, already_present, disabled, shell_rc, backup_path}` (spec §2.5). `repaired:true` 면 "PATH 를 고쳐뒀어요. **새 터미널을 한 번 열고** '온보딩 계속' 이라고 말해 주세요" 로 안내해요 (새 터미널 reload 는 사용자 행동 필수). `already_present:true` 면 바로 재감지하고, `disabled:true` (`AXHUB_DISABLE_PATH_REPAIR`) 면 수동 PATH 추가 안내 한 줄을 보여줘요. shell rc 변경 동의가 차단점이라, 자동 수정 전에 한 줄로 알려요.

   **4c. `cli_old`.** `cli_too_old=true` 또는 `has_update=true` 면 업데이트를 물어요. 이 plugin 의 스킬들은 ax-hub-cli **v0.20.0 이상**(plugin-support 표면 포함)이 필요해요. update-summary 헬퍼는 폐기됐어요 — 공개 `axhub update` 를 직접 부르고 한국어 메시지는 이 SKILL 이 렌더해요.

   ```bash
   PLUGIN_VER=$(grep -o '"version"[^,]*' "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json" 2>/dev/null | head -1 | sed -E 's/.*"version"[^"]*"([^"]+)".*/\1/')
   axhub update check ${PLUGIN_VER:+--plugin-version "$PLUGIN_VER"} --json
   ```

   `--plugin-version` 은 CLI v0.21.0+ 에서 플러그인 최신 여부도 함께 판정해요. 구 CLI 가 이 플래그를 거부하면 (exit 64) `axhub update check --json` 으로 한 번 더 호출해 CLI-only 로 떨어져요.

   응답은 `{current, latest, has_update}` (CLI) 에 더해 `--plugin-version` 을 줬으면 `plugin: {current, latest, has_update}` 도 있어요. CLI `has_update:false` 면 "이미 최신 버전이에요" 한 줄로 넘어가고, `has_update:true` 면 현재/최신 버전 diff 카드를 한국어로 보여주고 적용을 물어요.

   ```json
   {
     "questions": [{
       "question": "axhub CLI 업데이트를 적용할까요?",
       "header": "CLI 업데이트",
       "multiSelect": false,
       "options": [
         {"label": "적용", "description": "axhub update apply 로 cosign 검증 후 CLI 를 교체해요"},
         {"label": "취소", "description": "지금은 업데이트하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   `적용` 이면 `axhub update apply --execute --yes --json` 을 실행해요 (apply 승인이 차단점). cosign 검증·self-replace 는 CLI 가 소유하고, 끝나면 재감지로 version 만 확인해요. apply exit code 로 갈라요 (판정은 CLI 가 함):

   - exit 0 (`applied:true`) → "업데이트했어요" 후 재감지.
   - exit 14 (digest mismatch — 변조 신호) / exit 66 `cosign_enforce_failed` → **하드 스톱**. "보안 검증에 실패했어요. 강제로 진행하지 말고 회사 IT/보안팀에 알려주세요. 지금 버전은 그대로 써도 돼요" 로 안내하고 멈춰요.
   - exit 15 (swap failed) → 자동 재시도하지 말고 "설치 상태 진단해줘" 로 안내해요.
   - exit 4 (미인증) → "다시 로그인해줘" 로 낮춰요.

   **플러그인 업데이트** 는 위 응답의 `plugin` 블록으로 봐요 (CLI v0.21.0+). `plugin.has_update:true` 면 끝단에 한 줄 advisory 로 알려요 — "axhub 플러그인 새 버전(`plugin.latest`)이 있어요. Claude Code 에서 `/plugin update` 로 받을 수 있어요." 플러그인 교체 자체는 marketplace `/plugin update` 가 담당하니 gap 으로 막지 않고 안내만 해요. `plugin` 블록이 없으면 (구 CLI) 생략해요.

   **4d. `auth_missing`.** `auth_ok=false` 면 로그인 gap 이에요. 공개 `axhub auth` 표면을 직접 써요.

   ```bash
   axhub auth status --json
   ```

   status 응답을 4-case 로 갈라요 (refresh→login 결정은 CLI 가 해요):
   - `user_email` 있음 → 이미 로그인된 거예요. identity 한 줄만 보여주고 다음 gap 으로 가요.
   - `code: token_expired` → 만료. 먼저 `axhub auth refresh --json` 으로 device flow 없이 갱신을 시도해요. 성공이면 바로 재감지하고, `invalid_grant` 면 아래 full login 으로 내려가요 (refresh 가 friction 0 이라 우선).
   - `code: not_logged_in` → 미인증. 아래 질문으로 `axhub auth login` device-flow 를 시작해요.
   - 그 외 `code:` → 자연어 복구 안내를 한 줄로 보여주고 재개 phrase 를 남겨요.

   `axhub auth login --no-browser --json` 은 device flow 라 `device_code_issued` JSON line 의 `verification_uri` (non-null 이면 `verification_uri_complete` 우선) + `user_code` 만 humanize 해서 보여줘요. internal `device_code` 값은 echo 금지예요. 사용자가 "승인했어" 라고 말하면 재감지해요 — 브라우저 device 승인이 차단점이에요.

   ```json
   {
     "questions": [{
       "question": "지금 로그인할까요?",
       "header": "로그인",
       "multiSelect": false,
       "options": [
         {"label": "로그인", "description": "axhub auth login 으로 브라우저 device 승인을 시작해요"},
         {"label": "나중에", "description": "로그인하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

5. **git/node 런타임 gap (인라인).**

   git 은 clone/remote/push 전제조건이라 init/github 전에 닫아요. 시스템 설치 승인이 차단점이에요.

   ```json
   {
     "questions": [{
       "question": "git 이 없어요. 지금 설치할까요?",
       "header": "git 설치",
       "multiSelect": false,
       "options": [
         {"label": "지금 설치", "description": "macOS 는 Xcode CLT, Windows 는 winget/scoop, Linux 는 OS 패키지 매니저를 써요"},
         {"label": "나중에", "description": "설치하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   git 설치 fallback:
   - macOS: `xcode-select --install` 또는 `brew install git`
   - Windows: `winget install Git.Git` 또는 `scoop install git`
   - Linux: `apt-get install -y git` / `dnf install -y git` / `pacman -S git`

   node 가 없으면 같은 패턴으로 물어요.

   ```json
   {
     "questions": [{
       "question": "node 가 없어요. 지금 설치할까요?",
       "header": "node 설치",
       "multiSelect": false,
       "options": [
         {"label": "지금 설치", "description": "있는 패키지 매니저로 설치해요. Unix 는 없으면 nvm, Windows 는 수동 안내로 내려가요"},
         {"label": "나중에", "description": "설치하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   node 권장 버전 mismatch (`node_mismatch=true`) 는 경고만 하지 말고 교정 제안을 해요.

   ```json
   {
     "questions": [{
       "question": "node 권장 버전으로 맞출까요?",
       "header": "node 버전",
       "multiSelect": false,
       "options": [
         {"label": "맞추기", "description": ".nvmrc 또는 package.json engines.node 기준으로 nvm install/use 를 시도해요"},
         {"label": "나중에", "description": "현재 버전으로 계속하고 READY_WITH_USER_ACTION 문구를 남겨요"}
       ]
     }]
   }
   ```

   node 설치/교정 fallback:
   - macOS: `brew install node` 또는 nvm `v0.40.1` 태그
   - Windows: `winget install OpenJS.NodeJS.LTS` 또는 `scoop install nodejs-lts`; nvm-windows GUI 자동화는 하지 않아요
   - Linux: OS 패키지 매니저 또는 nvm `v0.40.1` 태그
   - 모두 실패하면 `https://nodejs.org` LTS 링크와 `온보딩 계속` 재개 phrase 를 줘요

6. **GitHub App 안내 (`github_app_missing`) — install_url 은 무조건, 미설치면 설치까지 막아요 (gate).**

   Step 2 detect JSON 의 `github` 를 그대로 써요. 두 가지를 해요:

   **(a) install_url 무조건 표시 + 연결 안내 — 이미 설치돼 있어도 항상.** `github.install_url` 이 있으면 설치 여부와 무관하게 한 줄로 연결을 안내해요: "GitHub App 을 설치·연결하려면 여기로 가요: `<github.install_url>`. 이미 설치돼 있어도 다른 org/계정을 더 연결할 수 있어요." `github.installed_logins` 가 있으면 "이미 연결된 계정: `<login...>`" 도 덧붙여요. 링크는 안내만 하고 자동으로 열지 않아요.

   **(b) 미설치면 (`github.state` 가 `uninstalled`/`empty`) 설치까지 막아요 (gate).** 설치가 확인되기 전에는 Step 7 (기존 repo 연결) 으로 진행하지 않아요. 가능한 전진배치는 **계정레벨 GitHub App 설치(install_url)만**이에요. OAuth device-flow 인가는 connect 단계에 남아요. `github.state` 가 `auth_error` 면 install_url 을 못 읽으니 "다시 로그인해줘" 로 안내하고, 재로그인 후 재감지하면 링크가 다시 떠요.

   ```json
   {
     "questions": [{
       "question": "GitHub App 을 먼저 설치할까요?",
       "header": "GitHub App",
       "multiSelect": false,
       "options": [
         {"label": "설치", "description": "install_url 을 열어 계정레벨 GitHub App 설치를 먼저 끝내요"},
         {"label": "나중에", "description": "설치를 미루면 다음 단계로 넘어가지 않고 READY_WITH_USER_ACTION 으로 멈춰요. 설치 후 `온보딩 계속`"}
       ]
     }]
   }
   ```

   설치 선택 시 `github.install_url` 을 보여주고 브라우저를 열어요. 사용자가 "승인했어" 또는 "온보딩 계속" 이라고 말하면 Step 2 재감지를 한 번 해요. 브라우저 App 설치가 차단점이에요.

   **미설치 동안 진행 차단 (gate).** `github.state` 가 `uninstalled`/`empty` 인 동안에는 Step 7 로 advance 하지 않아요. 설치를 확인(재감지 결과 `installed`/`mixed`)했거나, 사용자가 "나중에" 로 명시적으로 미뤄 READY_WITH_USER_ACTION 으로 멈출 때까지 `github_app_missing` 이 first_gap 으로 남아요. "나중에" 면 install_url + 재개 phrase(`승인했어`/`온보딩 계속`)를 남기고 멈춰요 — 미설치 상태로 다음 단계로 advance 하지 않아요. 이미 설치돼 있으면(installed/mixed) 막지 않고 그대로 다음 gap 으로 가요.

7. **Repo/App gap (`existing_repo_gap` / `no_manifest_empty`).**

   **7a. 기존 repo (`existing_repo_gap`) — 인라인, init 위임 아님.** 기존 repo+커밋+manifest 없음이면 clone 충돌을 피하려고 init 으로 가지 않아요. 공개 `axhub apps git` 표면으로 capability ladder 를 진행해요.

   ```bash
   axhub apps git status --app "$APP_ID" --json
   ```

   `axhub apps git status` 출력은 `install_url` / `repo_full_name` / `branch` / `installation_id` / `installed_logins[]` 를 줘요. `install_url` 을 한 줄로 보여주고 (capability ladder 안내, `installation_id` 등 internal 값은 echo 금지), 연결을 물어요. connect 는 dry-run → 승인 → execute 순서로, OAuth 승인이 단계별 차단점이에요. **`$APP_ID` 가 아직 없으면** (앱이 backend 에 안 만들어진 기존-repo) status 를 못 부르니 앱부터 만들어야 해요 — onboarding 은 앱을 자동 생성하지 않으니 `첫 앱 만들어줘` 재개 phrase 를 남기고 READY_WITH_USER_ACTION 으로 멈춰요.

   ```json
   {
     "questions": [{
       "question": "기존 repo 를 axhub 앱에 연결할까요?",
       "header": "repo 연결",
       "multiSelect": false,
       "options": [
         {"label": "연결", "description": "axhub apps git connect 로 app↔repo 연결을 진행해요 (OAuth 승인 필요)"},
         {"label": "아니요", "description": "현재 repo 는 그대로 두고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   `연결` 이면 먼저 `axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --json` 으로 미리봐요 — `--execute` 없이 부르면 dry-run 이라 OAuth/installation 검증만 하고 mutate 하지 않아요. dry-run 결과의 `installation_id` + `repo_full_name` 이 채워지면 승인 후 `--execute` 로 연결해요. 설치 개수 gate·ambiguous owner 처리는 status 출력의 `installed_logins` 기준으로 판단해요.

   **7b. 빈 dir (`no_manifest_empty`) — 안내만, init 위임 없음.** 빈 dir 이고 manifest 가 없어도 onboarding 은 첫 앱 만들기를 자동으로 시작하지 않아요. AskUserQuestion 없이 "이 폴더는 비어 있어요. 첫 앱을 만들려면 `첫 앱 만들어줘` 라고 말해 주세요" 한 줄만 안내하고, 이 gap 은 재감지 루프를 돌지 않고 바로 Step 10 Ready card 로 가요 — `no_manifest_empty` 를 다시 first_gap 으로 받아 무한 루프 도는 걸 막아요. Ready card 는 `첫 앱 만들어줘` 를 다음 단계 phrase 로 보여줘요.

8. **Dependency gap (`deps_missing`).**

   onboarding 은 프로젝트 의존성 설치를 할 수 있지만 `allows-dependency-execution: true` 의 보안 계약을 지켜야 해요. 의존성 설치는 repo on disk 뒤, manifest+lockfile 있을 때만, 명시 확인 필수, D1 guard 필수, 모든 command 에 `--ignore-scripts` 필수예요. lockfile 없으면 package manager 선택을 묻지 말고 skip 해요. 시스템 설치 승인이 차단점이에요.

   ```json
   {
     "questions": [{
       "question": "의존성을 설치할까요?",
       "header": "의존성",
       "multiSelect": false,
       "options": [
         {"label": "설치", "description": "lockfile 기준으로 --ignore-scripts 를 붙여 설치해요"},
         {"label": "나중에", "description": "postinstall 자동 실행 없이 READY_WITH_USER_ACTION 으로 안내해요"}
       ]
     }]
   }
   ```

   허용 command:
   - `bun install --ignore-scripts`
   - `pnpm install --ignore-scripts`
   - `npm install --ignore-scripts`
   - `yarn install --ignore-scripts`

   `--ignore-scripts` 는 postinstall 자동 실행 금지의 핵심 가드예요. native module 이 이 때문에 미빌드되면 VIBE_READY 로 거짓 green 을 주지 말고 `READY_WITH_USER_ACTION` 으로 낮춰요.

9. **Doctor/deploy evidence gap (`doctor_gap` / `deploy_unverified`).**

   온보딩 끝에는 read-only preflight 로 PATH/auth/profile 핵심 점검을 한 번 봐요.

   ```bash
   axhub plugin-support preflight --json
   ```

   `auth_ok`/`cli_on_path` 등이 green 이면 진단 카드를 한국어로 보여주고, 막힌 항목이 있으면 next-phrase (`다시 로그인해줘` / 새 터미널 reload) 를 안내해요 (doctor_gap 은 차단점 없음). 이전 배포의 deployment id 가 있으면 공개 `axhub deploy verify` 로 성공을 확인해요 (deployment id 는 배포 출력에서 받아요).

   ```bash
   axhub deploy verify "$DEPLOYMENT_ID"
   ```

   exit 0 이면 terminal success + 접근 가능 URL 이 확인된 거라 live evidence 로 써요. 비-0 이면 아직 진행 중이거나 실패라, URL surface 만 있고 live evidence 가 없으면 `READY_WITH_USER_ACTION` 으로 낮춰요. **latest 재탐색 없이 받은 deployment id 만 verify 해요** (correlation 계약, spec §2.3).

9.5. **axhub MCP 등록 + OAuth 연동 (user scope, best-effort · 비차단).** 목표는 2단계예요: **(1) 서버 등록(add)** — 로컬 config mutation, **(2) OAuth 연동(authenticate)** — 브라우저 승인 차단점. **등록만 하면 도구가 안 떠요** — 원격 MCP 는 tenant-scoped OAuth 라 연동까지 끝나야 `mcp__axhub__*` 도구가 살아나요. Step 4d 의 CLI 로그인과 MCP OAuth 는 별개 자격이라 둘 다 필요해요. 이 블록은 Bash tool 로 실행만 하고 명령 본문은 chat 에 출력하지 않아요. 연동을 못 끝내도 onboarding 은 막지 않고 `READY_WITH_USER_ACTION` 으로 안내만 남겨요.

   **D1 가드.** subprocess(`claude -p`/CI/headless)에서는 브라우저 OAuth 를 할 수 없으니 add·authenticate 를 자동 실행하지 말고, 수동 명령 한 줄만 남기고 넘어가요.

   **(a) host 판정.** `claude` CLI 가 있으면 Claude Code 경로 (b–c) 로, 없으면 Claude Desktop/기타 host (d) 로 가요.

   **(b) 등록 (idempotent).** 이미 있으면 건너뛰고, 없으면 user scope HTTP 로 추가해요.

   ```bash
   if command -v claude >/dev/null 2>&1; then
     claude mcp get axhub >/dev/null 2>&1 \
       || claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp
   fi
   ```

   **(c) 연동 상태 검증 + OAuth 안내.** add 만으로는 미연동일 수 있어서 health check 로 실제 상태를 확인해요 — 이게 기존에 빠졌던 핵심이에요.

   ```bash
   command -v claude >/dev/null 2>&1 && claude mcp get axhub 2>&1 | grep -i status
   ```

   `Status` 줄로 갈라요:
   - `Connected` → "axhub MCP 연동 완료예요. `mcp__axhub__*` 도구를 쓸 수 있어요 (새 세션에서 보일 수 있어요)" 한 줄.
   - `Needs authentication` (또는 status 줄 없음 = 미연동) → OAuth 가 남았어요. "Claude Code 에서 `/mcp` 를 실행하고 목록에서 **axhub** 를 골라 브라우저 인증(OAuth)을 끝내 주세요" 로 안내해요. 브라우저 승인이 차단점이라, 끝나면 "MCP 연동했어" 또는 "온보딩 계속" 이라고 말하면 이 검증을 한 번 다시 해요.
   - `claude mcp get` 자체가 실패/미존재 → (b) add 를 한 번 더 시도하고, 그래도 안 되면 (d) 수동 안내로 낮춰요.

   **(d) Claude Desktop/기타 host fallback.** `claude` CLI 가 없으면 자동 등록을 못 해요. 한 줄 안내: "Claude Desktop 은 설정 → 커넥터에서 커스텀 커넥터로 `https://mcp.axhub.ai/mcp` 를 추가하고 로그인하면 연동돼요. Claude Code 면 `claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp` 로 등록한 뒤 `/mcp` 로 OAuth 인증하면 돼요." 자동으로 링크를 열거나 mutate 하지 않아요.

10. **Ready card.**

   모두 green 이면 `VIBE_READY` 카드로 끝내요.

   ```text
   axhub 온보딩 완료예요. [VIBE_READY]
     ✓ CLI v<CLI_VERSION>
     ✓ 로그인 <masked-email>
     ✓ git v<GIT_VERSION>
     ✓ node v<NODE_VERSION> (pm: <bun|pnpm|npm|yarn>)
     ✓ GitHub App 설치됨 — 다른 org/계정 추가: <install_url>
     ✓ 앱 <app-slug> 연결됨
     ✓ 첫 배포 live: <deployment-url>
     ✓ 점검 통과
     ✓ axhub MCP 연동됨 — `mcp__axhub__*` 사용 가능 (`claude mcp get axhub` 가 Connected 일 때만 ✓; 미연동이면 `/mcp` OAuth 안내로 남겨요)

   이제 바로 코딩하면 돼요.
   다음에 말할 수 있는 것: "첫 앱 만들어줘", "배포해", "로그 봐줘", "환경변수 추가해줘", "테이블 추천해줘"
   ```

   GitHub App 줄의 `<install_url>` 은 설치 여부와 무관하게 **항상** 보여줘요 (무조건). Step 2 detect JSON 의 `github.install_url` 을 그대로 채워요 (GitHub 조회가 성공하면 계정이 0개여도 app-level 링크로 항상 채워져요). 링크는 보여주기만 하고 자동으로 열지 않아요. `github.install_url` 이 null 인 경우(=`auth_error`/`unavailable`)에만 이 줄을 생략하고, `auth_error` 면 재로그인 안내로 낮춰요.

   degraded 상태는 명확히 표시해요.
   - `READY_WITH_USER_ACTION`: 외부 승인, OS installer GUI, PATH reload, native build 처럼 사용자가 해야 하는 행동만 남음
   - `SAFE_STOP_NONINTERACTIVE`: CI/headless 라 mutation 을 자동 실행하지 않음
   - `BLOCKED_UNSUPPORTED`: 안전한 OS/권한/패키지 매니저 경로가 없음

## NEVER

- NEVER preflight 를 CLI 확인 이전에 호출 — CLI 부재 상태로 fire 되면 무한 루프 위험이에요. detect 는 fail-open 이라 먼저 써요.
- NEVER `command -v axhub` 가 성공하는데 `cli_missing` 으로 재설치를 안내하지 말아요 — 그건 구 CLI 의 detect self-probe 오탐이에요. Step 2 의 `AXHUB_BIN` 절대경로 핀으로 재감지하면 present 로 잡히고, 그래도 false 면 CLI 를 present 로 간주하고 `auth_missing` 부터 이어가요.
- NEVER 사용자가 sibling skill 이름이나 slash command 를 알아야만 끝나는 안내를 만들지 말아요.
- NEVER 한 번에 여러 mutate gap 을 추측 실행하지 말아요. 항상 detect-first → 첫 gap 처리 → 재감지 루프예요.
- NEVER gap 마다 preflight/detect 를 중복 호출하지 말아요. `onboarding-detect` 한 번이 단일 판정원이에요.
- NEVER plugin update 를 온보딩 중 실행하지 말아요. `/plugin update` 는 새 세션이 필요해서 끝단 advisory 로만 보여줘요.
- NEVER GitHub OAuth device-flow 인가를 install_url 단계에서 전진배치한다고 쓰지 말아요. install_url 단계는 계정설치만이에요.
- NEVER 빈 폴더에서 init 스킬로 위임하거나 앱을 자동 생성하지 말아요. 첫 앱 만들기는 Ready card 안내(`첫 앱 만들어줘`)로만 남겨요.
- NEVER lockfile 없이 dependency install 을 실행하지 말아요.
- NEVER dependency install 에서 `--ignore-scripts` 를 빼지 말아요. postinstall 자동 실행 금지예요.
- NEVER subprocess(`claude -p`/CI/headless)에서 install/update/auth/init/deps mutation 이나 git/node system install/version switch 를 자동 실행하지 말아요.
- NEVER `VIBE_READY` 카드에 확인하지 않은 항목을 green 으로 표시하지 말아요.
- NEVER deploy verify 에 deployment id 없이 latest 를 재탐색하지 말아요 (correlation 계약).
- NEVER axhub MCP 를 add(등록)만 하고 연동 완료로 선언하지 말아요. `claude mcp get axhub` 의 `Status: Connected` 확인 전까지는 미연동이라 `/mcp` OAuth 안내를 남겨요.

## Additional Resources

- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
- `../init/SKILL.md` — bootstrap saga + 첫 deploy 포함 계약 source (참고용 — onboarding 은 더 이상 위임하지 않아요).
