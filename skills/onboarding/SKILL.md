---
name: onboarding
description: 'This skill should be used when the user is new to axhub, asks what to do first, requests setup/onboarding/getting started, or says a short first-run phrase. 이 스킬은 axhub 를 처음 쓰는 사람이 셋업/온보딩 전체 과정을 한 번에 진행하고 싶어할 때 사용해요. 다음 표현에서 활성화: "셋업해줘", "셋업 해줘", "처음인데", "처음 사용", "처음 써", "처음 쓰는데", "처음 쓰는데 뭐부터", "뭐부터 하면 돼", "뭐부터 하면 되나요", "어떻게 시작하면 돼", "어떻게 시작해", "온보딩", "온보딩해줘", "시작하기", "axhub 시작", "axhub 처음", "초기 셋업", "setup", "set up", "onboard", "onboarding", "getting started", "get started", "first time", 또는 첫 사용자 셋업 의도. axhub CLI 설치(install-cli)·로그인(auth)·node 환경 감지를 순서대로 안내하고, node 가 없으면 명시 확인 후 설치해요. 빈 폴더에서는 바로 템플릿을 묻지 말고 ‘첫 앱 만들래요?’를 먼저 물은 뒤, 사용자가 원하면 첫 앱 만들기(init)로 연결해요. 환경 진단(doctor)이나 새 앱 초기화(init)와 달리 처음 사용자의 순차 온보딩을 담당해요.'
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
multi-step: true
needs-preflight: false
allows-dependency-execution: true
model: sonnet
---

# Onboarding (first-run vibe coding orchestrator)

Frontmatter `description` 은 nl-lexicon trigger baseline 때문에 보수적으로 유지해요. 실제 온보딩 범위와
안전 계약은 이 본문을 authoritative source 로 봐요.

처음 axhub 를 쓰는 사람을 위한 **온보딩 단일 진입점**이에요. 사용자는 `온보딩`, `처음인데 뭐부터`,
`getting started` 한 마디만 하면 돼요. 내부에서는 기존 skill
(`install-cli`/`update`/`upgrade`/`auth`/`github`/`init`/`doctor`/`deploy`)을 위임하지만,
사용자는 sibling skill 이름이나 slash command 를 몰라도 온보딩을 끝낼 수 있어요.

onboarding 의 제품 계약은 `detect-first → 첫 gap 처리 → 재감지` 루프예요. 안전하게 자동화할 수 있는 gap 은
끝까지 닫고, 브라우저 승인·OS installer GUI·PATH reload 처럼 에이전트가 대신 완료할 수 없는 gap 은
`READY_WITH_USER_ACTION` 카드와 자연어 재개 phrase(`승인했어`, `온보딩 계속`, `다시 온보딩해줘`)를 남겨요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `승인했어`, `온보딩 계속`, `다시 로그인해줘`, `배포해`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

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

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **Non-interactive AskUserQuestion guard (D1).**

   이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 `tests/fixtures/ask-defaults/registry.json` 의 안전 기본값으로 진행해요. 이 모드의 최종 상태는 `SAFE_STOP_NONINTERACTIVE`예요. install/update/auth/init/deploy/deps mutation 과 git/node system install 또는 version switch 는 자동 실행하지 않아요.

2. **DETECT_ALL(read-only) — helper 한 번으로 모든 gap 을 봐요.**

   감지 로직은 `axhub-helpers onboarding-detect` 가 cross-platform 으로 한 번에 처리해요 (CLI/auth/git/node/manifest/github/deploy). 이 블록은 **Bash/PowerShell tool 로 실행만 하고, 스크립트나 명령 본문을 chat 에 출력하지 말아요** — 사용자에겐 "도구·로그인·환경을 한 번에 확인하고 있어요" 같은 한 줄만 보여줘요. preflight 와 같은 helper-pick 패턴이라 CLI 가 아직 없어도 안전해요 (helper 가 `cli_present:false` 로 fail-soft 해요). 단일 opaque 호출이라 예전 dual-platform DETECT 스크립트가 chat 으로 새던 문제도 사라져요.

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   DETECT_JSON=$("$HELPER" onboarding-detect --json 2>/dev/null)
   if [ -z "$DETECT_JSON" ]; then
     if command -v axhub >/dev/null 2>&1; then
       # axhub CLI 는 되는데 onboarding-detect 가 빈 출력 → helper 바이너리가 이
       # subcommand 보다 오래된 것. 작동하는 CLI 를 cli_missing 으로 오라우팅하지 말고
       # helper_outdated 로 표시해요.
       DETECT_JSON='{"cli_present":true,"first_gap":"helper_outdated","helper_outdated":true,"github":{"state":"unavailable","install_url":null}}'
     else
       DETECT_JSON='{"cli_present":false,"first_gap":"cli_missing","github":{"state":"unavailable","install_url":null}}'
     fi
   fi
   echo "$DETECT_JSON"
   ```

   fallback 분기: `axhub-helpers` 도 `axhub` 도 없으면 `cli_missing`. helper 가 없/오래됐지만 `axhub` 는 되면 `helper_outdated` 예요 (보통 plugin 이 binary+skill 을 함께 배포해서 안 생겨요). `first_gap=helper_outdated` 면 install-cli 로 가지 말고 "플러그인·도구가 오래된 것 같아요. `/plugin update` 후 새 세션에서 다시 시도해 주세요" 라고 안내하고 멈춰요. 출력 JSON 주요 field:

   - `first_gap` / `gaps`: 처리할 첫 gap (아래 state machine 순서). 이걸 그대로 따라요.
   - `cli_present` / `cli_version` / `cli_state` / `cli_on_path` / `cli_resolved_path` / `cli_too_old` / `has_update` / `latest_version`
   - `auth_ok` / `auth_error_code`
   - `git_present` / `git_repo` / `git_commit` / `node_present` / `node_version` / `node_required` / `node_mismatch` / `manifest_present` / `lockfile_present` / `deps_missing` / `dir_empty`
   - `github`: `{state, installed_logins[], uninstalled_logins[], install_url, multiple_installed}`. `state` 는 `installed` / `mixed` / `uninstalled` / `empty` / `auth_error` / `unavailable` 중 하나예요. **`install_url` 은 GitHub 조회가 성공하면 (`installed`/`mixed`/`uninstalled`/`empty`) 설치 여부·계정 수와 무관하게 항상 채워져요** (계정이 0개여도 app-level 링크로 fallback) — ready card(Step 10)와 GitHub 안내(Step 6)에서 무조건 보여줘요. `state` 가 `auth_error`/`unavailable` 면 null 이고, `auth_error` 면 `unknown` 으로 넘기지 말고 "다시 로그인해줘" 로 안내해요.
   - `deploy_checked` / `deploy_verified`

2.5. **GitHub App 설치·계정 추가 surface — DETECT 직후 무조건 (branch-independent, 비차단).**

   Step 2 helper JSON 의 `github` 를 그대로 써요 (accounts list 재호출 안 해요). `github.install_url` 이 null 이 아니면 (`github.state` 가 `installed`/`mixed`/`uninstalled`/`empty`) **설치 여부·계정 수·`first_gap` 과 무관하게 항상** 이 블록을 먼저 실행한 뒤 Step 3 gap 라우팅으로 가요. 모든 onboarding 경로가 gap 처리 전에 이 지점을 지나서, 빈 폴더(→ init) 처럼 GitHub 단계를 건너뛰는 경로에서도 install_url 을 맨 앞에서 한 번은 보장해요. 이게 Step 6(미설치 gap)·Step 10(ready card)의 조건부 노출에 의존하던 누락을 닫아요.

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

   `설치할래요` 면 `github.install_url` 을 열고, 사용자가 "설치했어" 또는 "온보딩 계속" 이라고 말하면 Step 2 재감지를 한 번 해요. `아니요, 계속` 은 아무 mutation 없이 Step 3 으로 이어가요. `github.install_url` 이 null (`github.state` 가 `auth_error`/`unavailable`, 조회 실패) 이면 이 블록 전체를 생략하고, `auth_error` 면 "다시 로그인해줘" 로 낮춰요.

   **D1 비대화형 가드.** `claude -p`/CI/headless 에서는 (b) AskUserQuestion 을 호출하지 말고 `tests/fixtures/ask-defaults/registry.json` 의 안전 기본값(`아니요, 계속`)으로 진행하고, install/connect mutation 이나 브라우저 열기를 자동 실행하지 않아요. (a) 표시 줄은 그대로 출력해요.

3. **Gap State Machine — 첫 gap 하나만 처리하고 재감지해요.**

   **gap 순서의 single source of truth 는 helper 의 `first_gap` 이에요.** 아래 ASCII tree 와 Step 3 상태표는 gap→처리 owner 매핑 참고용 문서일 뿐이라, 순서가 어긋나 보이면 트리·표를 재구현하지 말고 항상 `first_gap` 을 따라요.

   ```text
   START
     ↓
  DETECT_ALL(read-only)
     ├─ cli_missing         → Skill("axhub:install-cli") → DETECT_ALL
     ├─ cli_env_invalid     → AXHUB_BIN env 수정 안내(재설치 금지) → DETECT_ALL
     ├─ cli_path_missing    → Skill("axhub:repair") → DETECT_ALL
     ├─ cli_old             → update(explicit-confirm+cosign) → DETECT_ALL
     ├─ auth_missing        → Skill("axhub:auth") → DETECT_ALL
     ├─ git_missing         → install_git(explicit-confirm) → DETECT_ALL
     ├─ node_missing        → install_node(explicit-confirm) → DETECT_ALL
     ├─ node_mismatch       → fix_node(explicit-confirm+nvm) → DETECT_ALL
     ├─ github_app_missing  → install_url → DETECT_ALL
     ├─ existing_repo_gap   → Skill("axhub:github") guided onboarding/connect → DETECT_ALL
     ├─ no_manifest_empty   → Skill("axhub:init") saga(app+repo+deploy+clone) → DETECT_ALL
     ├─ deps_missing        → install_deps(explicit-confirm+ignore-scripts) → DETECT_ALL
     ├─ deploy_unverified   → status/watch or deploy(existingrepo only) → DETECT_ALL
     ├─ doctor_gap          → Skill("axhub:doctor") → DETECT_ALL
     └─ no_gap              → VIBE_READY_CARD
   ```

   상태 테이블 — 감지 조건은 Step 2 helper JSON field 기준이에요. 보통은 helper 가 준 `first_gap` 을 그대로 따라가면 돼요. 아래는 gap → 처리 owner 매핑이에요:

   | gap id | 감지 조건 (Step 2 JSON) | 처리 owner | 완료 확인 |
   |--------|-----------|------------|-----------|
   | `cli_missing` | `cli_present=false` (`cli_state≠axhub_bin_invalid`, 또는 DETECT_JSON 비어 fallback) | `install-cli` | 재감지 시 `cli_present=true` |
   | `cli_env_invalid` | `cli_present=false` + `cli_state=axhub_bin_invalid` (`cli_resolved_path` 가 잘못된 `AXHUB_BIN` 경로) | onboarding | `unset AXHUB_BIN` 또는 경로 수정 후 재감지 시 `cli_present=true` |
   | `cli_path_missing` | `cli_present=true` + `cli_on_path=false` (`cli_state=on_disk_not_on_path`) | `repair` | repair-path 적용 후 새 터미널 또는 resolved path 로 재확인 |
   | `cli_old` | `cli_too_old=true` 또는 `has_update=true` (`latest_version` 참고) | `update` | cosign apply 후 version 재확인 |
   | `auth_missing` | `auth_ok=false` (`auth_error_code` 참고) | `auth` | device approval/token import 후 재감지 green |
   | `git_missing` | `git_present=false` | onboarding | 설치 후 `git_present=true` |
   | `node_missing` | `node_present=false` | onboarding | 설치 후 `node_present=true` |
   | `node_mismatch` | `node_mismatch=true` (`node_version` vs `node_required`) | onboarding | target version active |
   | `github_app_missing` | `github.state` 가 `uninstalled`/`empty` (installed 계정 없음). `github.install_url` 로 안내 | onboarding | install_url 완료 후 재감지 |
   | `existing_repo_gap` | `git_repo=true` + `git_commit=true` + `manifest_present=false` | `github` | app↔repo connect 완료 |
   | `no_manifest_empty` | `manifest_present=false` + `dir_empty=true` | `init` | manifest+repo+deployment evidence 존재 |
   | `deps_missing` | `deps_missing=true` (lockfile+manifest 있고 node_modules 없음) | onboarding | lockfile install exit 0 |
   | `deploy_unverified` | `deploy_checked=true` + `deploy_verified=false` | onboarding/status/deploy | live/running/deployed 확인 |
   | `doctor_gap` | (helper 범위 밖) 온보딩 끝 doctor 핵심 체크 fail | `doctor` | doctor 핵심 green 또는 PATH reload 안내 |

   `cli_env_invalid` 면 install-cli 로 가지 말아요. `AXHUB_BIN` 환경변수가 spawn 할 수 없는 경로 (`cli_resolved_path` 값) 를 가리키는 상태라서, 재설치해도 해결되지 않아요. 사용자에게 해당 경로를 보여주고 `unset AXHUB_BIN` (셸 프로필에 export 가 있으면 그 줄 제거) 하거나 올바른 CLI 경로로 수정한 후 새 세션에서 다시 시도하라고 안내하고 READY_WITH_USER_ACTION 으로 멈춰요.

4. **CLI 버전 gap (`cli_old`).**

   CLI mismatch 또는 update available 은 Step 2 helper JSON 의 `cli_too_old=true` 또는
   `has_update=true` (`latest_version` 참고) 로 판단해요. 하나라도 업데이트 필요 신호면 먼저 물어요.

   ```json
   {
     "questions": [{
       "question": "axhub CLI 업데이트를 적용할까요?",
       "header": "CLI 업데이트",
       "multiSelect": false,
       "options": [
         {"label": "적용", "description": "update 스킬로 cosign 검증 후 CLI 를 교체해요"},
         {"label": "취소", "description": "지금은 업데이트하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   `적용` 선택 시 `Skill("axhub:update")` 로 위임해요. update 가 cosign 검증과 self-replace 를 소유해요. onboarding 은 돌아오면 `axhub --version` 만 재확인해요. 플러그인 업데이트는 `Skill("axhub:upgrade")` 내용을 참고하되 중간 적용하지 않아요. Claude Code `/plugin update` 는 새 세션이 필요하므로 끝단 advisory 로만 보여줘요.

5. **git/node 런타임 gap.**

   git 은 clone/remote/push 전제조건이라 init/github 전에 닫아요.

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

   node 가 없으면 기존 질문을 유지해요.

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

   node 권장 버전 mismatch 는 경고만 하지 말고 교정 제안을 해요.

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

6. **GitHub App 안내 — install_url 은 무조건, 미설치면 설치 제안 (`github_app_missing`).**

   Step 2 helper JSON 의 `github` 를 그대로 써요 (여기서 accounts list 를 다시 안 돌려도 돼요). 두 가지를 해요:

   **(a) install_url 무조건 표시 + 연결 안내 — 이미 설치돼 있어도 항상.** `github.install_url` 이 있으면 설치 여부와 무관하게 한 줄로 **연결을 안내**해요: "GitHub App 을 설치·연결하려면 여기로 가요: `<github.install_url>`. 이미 설치돼 있어도 다른 org/계정을 더 연결할 수 있어요." `github.installed_logins` 가 있으면 "이미 연결된 계정: `<login...>`" 도 덧붙여요. 링크는 안내만 하고 브라우저를 자동으로 열지는 않아요.

   **(b) 미설치면 (`github.state` 가 `uninstalled`/`empty`) 설치까지 막아요 (gate).** 설치가 확인되기 전에는 Step 7 (첫 앱 만들기, init 위임) 으로 진행하지 않아요. 가능한 전진배치는 **계정레벨 GitHub App 설치(install_url)만**이에요. OAuth device-flow 인가는 connect 단계에 남아요. `github.state` 가 `auth_error` 면 인증 만료로 install_url 을 못 읽으니 "다시 로그인해줘" 로 안내하고, 재로그인 후 재감지하면 링크가 다시 떠요.

   ```json
   {
     "questions": [{
       "question": "GitHub App 을 먼저 설치할까요?",
       "header": "GitHub App",
       "multiSelect": false,
       "options": [
         {"label": "설치", "description": "install_url 을 열어 계정레벨 GitHub App 설치를 먼저 끝내요"},
         {"label": "나중에", "description": "설치를 미루면 첫 앱 만들기로 넘어가지 않고 READY_WITH_USER_ACTION 으로 멈춰요. 설치 후 `온보딩 계속`"}
       ]
     }]
   }
   ```

   설치 선택 시 `github.install_url` 을 보여주고 브라우저를 열어요. 사용자가 "승인했어" 또는 "온보딩 계속" 이라고 말하면 Step 2 재감지를 한 번 해요. `apps git connect` OAuth device-flow 인가는 app id 가 생기는 init/github 단계에서 처리해요.

   **미설치 동안 진행 차단 (gate).** `github.state` 가 `uninstalled`/`empty` 인 동안에는 Step 7 (repo/app gap, 첫 앱 만들기) 로 advance 하지 않아요. 설치를 확인(재감지 결과 `github.state` 가 `installed`/`mixed`)했거나, 사용자가 "나중에" 로 명시적으로 미뤄 READY_WITH_USER_ACTION 으로 멈출 때까지 `github_app_missing` 이 first_gap 으로 남아요. "나중에" 면 install_url + 재개 phrase(`승인했어`/`온보딩 계속`)를 남기고 멈춰요 — 미설치 상태로 init 으로 위임하지 않아요. 이미 설치돼 있으면(installed/mixed) 막지 않고 그대로 다음 gap 으로 가요 (install_url 은 (a) 에서 추가 설치용으로 계속 보여줘요).

7. **Repo/App gap.**

   기존 repo+커밋+manifest 없음이면 init 으로 가지 말아요. clone 충돌을 피하려고 github guided onboarding/connect 로 라우팅해요.

   ```json
   {
     "questions": [{
       "question": "기존 repo 를 axhub 앱에 연결할까요?",
       "header": "repo 연결",
       "multiSelect": false,
       "options": [
         {"label": "연결", "description": "github 스킬로 앱 생성, remote 확인, 첫 push, app↔repo connect 를 진행해요"},
         {"label": "아니요", "description": "현재 repo 는 그대로 두고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   빈 dir 이고 manifest 가 없으면 첫 앱 만들기를 제안해요.

   ```json
   {
     "questions": [{
       "question": ‘첫 앱 만들래요?’,
       "header": "첫 앱",
       "multiSelect": false,
       "options": [
         {"label": "네", "description": "init saga 로 앱+repo+첫 배포+clone 을 진행해요"},
         {"label": "아니요", "description": "새 앱을 만들지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   `네` 선택 시 `Skill("axhub:init")` 로 위임해요. init 경로는 saga 가 이미 첫 배포를 포함해요. **init 경로는 saga 배포 URL surface 만 하고 재배포 X**예요. saga 가 deployment id/status 를 남기면 status/watch 로 확인해요. 기존 repo 경로에서만 `deploy` 위임을 제안해요.

8. **Dependency gap (`deps_missing`).**

   onboarding 은 프로젝트 의존성 설치를 할 수 있지만 `allows-dependency-execution: true` 의 보안 계약을 지켜야 해요. 의존성 설치는 repo on disk 뒤, manifest+lockfile 있을 때만, 명시 확인 필수, D1 guard 필수, 모든 command 에 `--ignore-scripts` 필수예요. lockfile 없으면 package manager 선택을 묻지 말고 skip 해요.

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

9. **Doctor/deploy evidence gap.**

   온보딩 끝에는 `Skill("axhub:doctor")` 로 PATH/helper/auth/profile 핵심 점검을 한 번 돌려요. init saga 의 deployment id/status 가 있으면 `axhub deploy status <DEPLOYMENT_ID> --app <APP_ID> --watch --watch-timeout <N> --json` 형태로 status/watch evidence 를 확인해요. URL surface 만 있고 live evidence 가 없으면 `READY_WITH_USER_ACTION` 으로 낮춰요.

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
     ✓ doctor 점검 통과

   이제 바로 코딩하면 돼요.
   다음에 말할 수 있는 것: "배포해", "로그 봐줘", "환경변수 추가해줘", "테이블 추천해줘"
   ```

   GitHub App 줄의 `<install_url>` 은 설치 여부와 무관하게 **항상** 보여줘요 (무조건). Step 2 helper JSON 의 `github.install_url` 을 그대로 채워요 (GitHub 조회가 성공하면 계정이 0개여도 app-level 링크로 항상 채워져요). 이미 설치된 사용자도 다른 org/계정을 더 붙일 수 있게 링크를 남기는 거예요. 링크는 보여주기만 하고 자동으로 열지 않아요. `github.install_url` 이 null 인 경우(=`github.state` 가 `auth_error`/`unavailable`, 조회 자체 실패)에만 이 줄을 생략하고, `auth_error` 면 재로그인 안내로 낮춰요.

   degraded 상태는 명확히 표시해요.
   - `READY_WITH_USER_ACTION`: 외부 승인, OS installer GUI, PATH reload, native build 처럼 사용자가 해야 하는 행동만 남음
   - `SAFE_STOP_NONINTERACTIVE`: CI/headless 라 mutation 을 자동 실행하지 않음
   - `BLOCKED_UNSUPPORTED`: 안전한 OS/권한/패키지 매니저 경로가 없음

## NEVER

- NEVER preflight 를 CLI 확인 이전에 호출 — CLI 부재 상태로 fire 되면 무한 루프 위험이에요.
- NEVER 사용자가 sibling skill 이름이나 slash command 를 알아야만 끝나는 안내를 만들지 말아요.
- NEVER 한 번에 여러 mutate gap 을 추측 실행하지 말아요. 항상 detect-first → 첫 gap 처리 → 재감지 루프예요.
- NEVER plugin update 를 온보딩 중 실행하지 말아요. `/plugin update` 는 새 세션이 필요해서 끝단 advisory 로만 보여줘요.
- NEVER GitHub OAuth device-flow 인가를 Phase B 에서 전진배치한다고 쓰지 말아요. Phase B 는 install_url 계정설치만이에요.
- NEVER init saga 뒤 deploy 를 재호출하지 말아요. init 경로는 saga URL/evidence surface, 재배포 X예요.
- NEVER lockfile 없이 dependency install 을 실행하지 말아요.
- NEVER dependency install 에서 `--ignore-scripts` 를 빼지 말아요. postinstall 자동 실행 금지예요.
- NEVER subprocess(`claude -p`/CI/headless)에서 install/update/auth/init/deploy/deps mutation 이나 git/node system install/version switch 를 자동 실행하지 말아요.
- NEVER `VIBE_READY` 카드에 확인하지 않은 항목을 green 으로 표시하지 말아요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
- `../install-cli/SKILL.md` — OS 감지 / race-check / post-verify 패턴 재사용 source.
- `../github/SKILL.md` — 기존 repo guided onboarding/connect 위임 source.
- `../init/SKILL.md` — bootstrap saga + 첫 deploy 포함 계약 source.
- `../update/SKILL.md` — CLI update + cosign 검증 source.
- `../upgrade/SKILL.md` — plugin advisory-only source.
