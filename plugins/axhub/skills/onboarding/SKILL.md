---
name: onboarding
description: 'Use when the user is new to axhub or asks for first setup/onboarding/getting started. 이 스킬은 "셋업해줘", "처음인데", "처음 쓰는데 뭐부터", "온보딩", "시작하기", "axhub 시작", "초기 셋업", "setup", "onboard", "getting started", "first time" 같은 첫 사용자 셋업 의도를 담당해요. axhub CLI 설치, 로그인, git/node, GitHub App, 앱 연결, 의존성, 최종 Ready card 를 detect-first 로 안내하되 빈 폴더에서 bootstrap 을 자동 실행하지 않아요. 이 트리거들은 axhub 맥락(발화의 axhub 언급·대화의 직전 axhub 작업)이 있을 때만 유효해요. 일반 프로젝트 셋업이나 다른 도구 온보딩 발화에는 이 스킬을 쓰지 않아요.'
examples:
  - utterance: "셋업해줘"
    intent: "onboard axhub first-time setup"
  - utterance: "처음인데 뭐부터 하면 돼?"
    intent: "onboard axhub first-time setup"
  - utterance: "온보딩"
    intent: "onboard axhub first-time setup"
  - utterance: "getting started"
    intent: "onboard axhub first-time setup"
allows-dependency-execution: true
model: sonnet
---

# Onboarding (first-run setup router)

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

처음 axhub 를 쓰는 사람을 위한 단일 진입점이에요. 사용자는 `온보딩`, `처음인데 뭐부터`, `getting started` 처럼 말하면 되고, 이 스킬은 CLI/auth/runtime/GitHub/repo/deps/MCP 준비를 한 gap 씩 닫아요. 환경 진단만 원하면 doctor/diagnosis 가 맞고, 새 앱 생성을 명시하면 bootstrap 이 맞아요. onboarding 은 빈 폴더에서도 자동 bootstrap 을 시작하지 않고 Ready card 에서 `첫 앱 만들어줘` 를 다음 말로 안내해요.

## Reference Loading

이 top-level 파일은 routing, safety, exact command anchors 를 보존하는 compact contract 예요. 세부 UX 는 detect 결과가 해당 branch 를 요구할 때만 아래 reference 를 읽어요.

- [`references/gap-state-machine.md`](references/gap-state-machine.md): `first_gap` 라우팅, gap별 완료 기준, repo/app/doctor/deploy evidence 흐름.
- [`references/install-channels-and-auth.md`](references/install-channels-and-auth.md): CLI 설치·PATH repair·update·auth, git/node 설치와 version 교정.
- [`references/github-app.md`](references/github-app.md): GitHub App install URL visibility, 다른 계정 추가 질문, 미설치 gate.
- [`references/dependency-install.md`](references/dependency-install.md): lockfile-only dependency install, `--ignore-scripts`, native build downgrade.
- [`references/mcp-ready-card.md`](references/mcp-ready-card.md): AI 활용 기록 옵트인, MCP add/auth distinction, `VIBE_READY`, `READY_WITH_USER_ACTION`, `SAFE_STOP_NONINTERACTIVE` card variants.

References 는 이 스킬의 일부예요. 명령 의미를 바꾸지 말고, top-level invariant 와 reference detail 이 충돌하면 top-level safety invariant 를 우선해요.

## Core Contract

1. **Single source of truth.** 모든 gap 판정은 `axhub plugin-support onboarding-detect --json` 한 번에서 온 JSON 이 source of truth 예요. `first_gap` 이 처리 순서를 결정해요. gap 마다 preflight 를 다시 돌려 순서를 추측하지 않아요.
2. **Detect-first loop.** `detect -> first_gap 하나 처리 -> 재감지` 를 반복해요. 한 번에 여러 mutate gap 을 실행하지 않아요. Claude Desktop 의 OAuth device flow 는 CLI 자동 브라우저 오픈/자동 polling 경로를 쓰고, 브라우저 실행 실패·만료·권한 거부 같은 fallback 에서만 `READY_WITH_USER_ACTION` 으로 멈춰요. OS installer GUI, PATH reload, GitHub App install, MCP OAuth 는 여전히 사용자 action gate 예요.
3. **Headless safety.** 순수 subprocess/headless/CI 에서는 AskUserQuestion 을 생략하고 safe defaults 로 멈춰요. install/update/auth/bootstrap/deps mutation, git/node system install, node version switch, browser open, MCP OAuth 를 자동 실행하지 않아요. 최종 상태는 `SAFE_STOP_NONINTERACTIVE` 예요.
4. **No automatic bootstrap.** 빈 폴더나 manifest 없는 폴더를 발견해도 bootstrap skill 로 위임하거나 앱을 자동 생성하지 않아요. `no_manifest_empty` 는 안내 후 Ready card 로 가고, 다음 말은 `첫 앱 만들어줘` 예요.
5. **GitHub App visibility.** detect JSON 의 `github.install_url` 이 null 이 아니면 설치 여부·계정 수·`first_gap` 과 무관하게 한 번은 보여줘요. `github.state` 가 `uninstalled`/`empty` 면 설치 확인 전 Step 7 repo/app 연결로 넘어가지 않아요.
6. **Dependency safety.** 의존성 설치는 manifest 와 lockfile 이 있을 때만, 명시 확인 뒤, 해당 lockfile 의 package manager 로만 실행해요. 모든 install command 는 반드시 `--ignore-scripts` 를 붙여요. lockfile 이 없으면 설치하지 않아요.
7. **MCP truth.** `claude mcp add` 는 등록일 뿐이에요. `claude mcp get axhub` 가 `Status: Connected` 를 보여주기 전까지 `mcp__axhub__*` 가 연결됐다고 말하지 않아요. 새로 add 한 세션에서는 `/mcp` OAuth 를 안내하지 말고 재시작 handoff 로 넘겨요 — `/mcp` OAuth 안내는 이전 세션에서 등록된 경우(resume 포함)에만 해요.
8. **Ready card honesty.** 확인하지 않은 항목은 green check 로 표시하지 않아요. 가능한 종료 상태는 `VIBE_READY`, `READY_WITH_USER_ACTION`, `SAFE_STOP_NONINTERACTIVE`, `BLOCKED_UNSUPPORTED` 예요.
9. **Telemetry opt-in.** AI 활용 기록(`axhub axrouter` — 내 Claude Code 프롬프트·응답·툴콜을 팀 워크스페이스로 보내는 수집 기능)은 무엇이 수집되는지 설명하고 물어본 뒤 사용자가 켜기를 고를 때만 켜요 — 동의 없이 켜지 않아요. 거절하면 같은 온보딩에서 다시 묻지 않고, headless 에서는 묻지도 켜지도 않아요. 미지원 워크스페이스·구 CLI 면 조용히 건너뛰어요.
10. **axhub 맥락 게이트.** 발화에 axhub 언급이 없고 대화에도 axhub 맥락(직전 axhub 작업)이 없으면, detect 를 시작하기 전에 "axhub 셋업을 말하는 거예요?" 를 한 번만 물어요. 아니라는 답이면 이 스킬을 종료하고 다른 axhub skill 로 넘기지 않아요. headless 에서는 묻지 않고 멈춰요.

## Progress

각 단계 시작에는 사용자가 멈춘 것으로 오해하지 않게 한국어 한 줄만 말해요. raw JSON, secret, internal id, full stderr 는 chat 에 넣지 않아요.
사용자에게 보이는 문장과 Bash/tool call 제목은 한국어로만 써요. `first_gap`, `gaps`, `cli_state`, `auth_error_code` 같은 detect 필드명이나 enum 값은 내부 라우팅용으로만 읽고, chat 에 그대로 출력하지 않아요. "빈 폴더라 자동으로 만들지 않았어요"처럼 사람 말로 바꿔요.

사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL 로만 써요. GitHub App 설치 URL, OAuth/device-flow URL, 앱 URL 모두 Markdown URL 링크 문법 없이 그대로 보여줘요. `[https://...](https://...)`, `[열기](https://...)`, `<https://...>` 처럼 URL 을 괄호나 label 로 감싸면 실패예요.

- `환경 점검하는 중이에요`
- `axhub CLI 설치하는 중이에요`
- `로그인 진행하는 중이에요`
- `실행환경(node·git) 점검하는 중이에요`
- `GitHub App 설치 확인하는 중이에요`
- `필요한 패키지 설치하는 중이에요`
- `AI 활용 기록 설정 확인하는 중이에요`
- `axhub 도구 연결하는 중이에요`
- `준비 다 됐어요`

TodoWrite 가 host 에 있으면 checklist 를 갱신해요. 없으면 언급하지 말고 자연어 진행 알림만 사용해요.

## Workflow

### 0. Non-interactive guard

첫 AskUserQuestion 또는 mutation 전에 대화형 여부를 판단해요. 다음 중 하나면 D1 safe-stop mode 예요: stdout 이 TTY 가 아님, `CI` 가 있음, `CLAUDE_NON_INTERACTIVE` 가 있음, `claude -p` 같은 subprocess/headless 호출임. 이 모드에서는 사용자 확인이 필요한 action 을 실행하지 않고 manual next phrase 와 `SAFE_STOP_NONINTERACTIVE` card 로 끝내요.

### 1. DETECT_ALL(read-only)

항상 먼저 한 번 감지해요. 이 block 은 Bash tool 로 실행만 하고 명령 본문을 사용자에게 출력하지 않아요.

```bash
if command -v axhub >/dev/null 2>&1; then
  AXHUB_BIN="$(command -v axhub)"; export AXHUB_BIN
  DETECT_JSON=$(axhub plugin-support onboarding-detect --json 2>/dev/null)
  [ -n "$DETECT_JSON" ] || DETECT_JSON='{"cli_present":true,"first_gap":"doctor_gap","github":{"state":"unavailable","install_url":null}}'
elif AXHUB_BIN_LOC="$(cat "$HOME/.axhub/bin-path" 2>/dev/null)" && [ -n "$AXHUB_BIN_LOC" ] && [ -f "$AXHUB_BIN_LOC" ]; then
  DETECT_JSON="{\"cli_present\":true,\"cli_on_path\":false,\"cli_state\":\"on_disk_not_on_path\",\"cli_resolved_path\":\"$AXHUB_BIN_LOC\",\"first_gap\":\"cli_path_missing\",\"github\":{\"state\":\"unavailable\",\"install_url\":null}}"
elif [ -f "$HOME/.axhub/bin/axhub" ] || [ -f "$HOME/.axhub/bin/axhub.exe" ]; then
  DETECT_JSON='{"cli_present":true,"cli_on_path":false,"cli_state":"on_disk_not_on_path","first_gap":"cli_path_missing","github":{"state":"unavailable","install_url":null}}'
else
  DETECT_JSON='{"cli_present":false,"first_gap":"cli_missing","github":{"state":"unavailable","install_url":null}}'
fi
echo "$DETECT_JSON"
```

`AXHUB_BIN` 은 PATH/HOME 차이 때문에 detect self-probe 가 현재 shell 의 axhub 를 못 찾는 오탐을 줄이기 위한 pin 이에요. `command -v axhub` 는 실패했지만 location 파일(`~/.axhub/bin-path` — CLI 0.24.8+ 가 자기 설치 위치를 기록)이 가리키는 파일이나 canonical install dir(`~/.axhub/bin/axhub` 또는 `.exe`)에 파일이 있으면 재설치가 아니라 `cli_path_missing` 이에요 — 새 세션이 부모 앱의 stale PATH 를 물려받아 `command -v` 가 계속 실패해도 절대 재설치를 권하지 않아요. location 파일 덕에 CARGO_HOME 등 커스텀 설치 위치도 인식돼요. 이 branch 에서는 detect 를 부르거나 `AXHUB_BIN` 을 export 하지 않아요. 열린 세션이 PATH 를 못 읽는 상태라 detect 가 `cli_on_path:true` 로 오보하거나 같은 gap 을 반복할 수 있기 때문이에요.

주요 필드는 `first_gap`, `gaps`, `cli_present`, `cli_version`, `cli_state`, `cli_on_path`, `cli_too_old`, `has_update`, `latest_version`, `auth_ok`, `auth_error_code`, `git_present`, `git_repo`, `git_commit`, `node_present`, `node_version`, `node_required`, `node_mismatch`, `manifest_present`, `lockfile_present`, `deps_missing`, `dir_empty`, `github`, `deploy_checked`, `deploy_verified` 예요. 이 이름들은 parsing 전용이고 사용자-facing 문장·표·도구 제목에는 노출하지 않아요.

### 2. GitHub App surface

DETECT 직후 `github.install_url` 이 있으면 항상 한 줄로 보여줘요. 이미 설치되어 있어도 다른 org/계정을 더 연결할 수 있다는 말을 붙여요. `installed_logins` 는 login 만 보여주고 `installation_id` 같은 internal 값은 보여주지 않아요. 자세한 질문과 gate 는 [`references/github-app.md`](references/github-app.md)를 읽어요.

### 3. first_gap router

`first_gap` 만 처리하고 재감지해요. 아래 table 은 owner map 이고, 순서는 detect JSON 이 정해요.

| `first_gap` | Handler |
| --- | --- |
| `cli_missing` | CLI install approval. Load [`references/install-channels-and-auth.md`](references/install-channels-and-auth.md). |
| `cli_path_missing` | detect 가 준 `cli_resolved_path`(없으면 canonical `"$HOME/.axhub/bin/axhub"`, Windows 는 `.exe`)로 `plugin-support repair-path --json` 실행 (bare `axhub` 는 이 상태에서 127); continue THIS session via the JSON `bin_path` absolute command; new terminal is next-session advice. |
| `cli_old` | `axhub update check` / `axhub update apply --execute --yes --json`; load install reference. |
| `auth_missing` | `axhub auth status`, refresh, or device login; load install reference. |
| `git_missing` | git install approval; load install reference. |
| `node_missing` | node install approval; load install reference. |
| `node_mismatch` | nvm/package-manager version correction approval; load install reference. |
| `github_app_missing` | GitHub App install gate; load [`references/github-app.md`](references/github-app.md). |
| `existing_repo_gap` | Existing repo app connection via `axhub apps git`; load gap-state reference and GitHub reference. |
| `no_manifest_empty` | No bootstrap. Show advisory and go to Ready card with `첫 앱 만들어줘`. |
| `deps_missing` | Lockfile-only install with `--ignore-scripts`; load [`references/dependency-install.md`](references/dependency-install.md). |
| `deploy_unverified` | Verify only known deployment id and app scope with `axhub deploy verify "$DEPLOYMENT_ID" --app "$APP_ID_OR_SLUG"`. |
| `doctor_gap` | Final read-only `axhub plugin-support preflight --json` and recovery phrase. |
| `no_gap` | Ready card. |

If a handler needs a prompt but D1 safe-stop mode is active, do not execute the mutation. Return `SAFE_STOP_NONINTERACTIVE` with the exact manual command or natural phrase.

`cli_path_missing` 은 CLI 가 디스크에 있는데 현재 셸 PATH 에 없는 상태예요. 이미 열린 세션의 PATH 는 밖에서 못 고치므로(OS 설계), repair-path 뒤에 `command -v axhub` 재감지를 반복하지 말고(무한 루프 방지) **repair-path JSON 의 `bin_path` 절대경로로 남은 온보딩 명령을 그대로 이어가요** (예: `"<bin_path>" auth status --json`). detect 의 `cli_resolved_path` 도 같은 절대경로예요. 남은 gap 재감지가 필요하면 `"<bin_path>" plugin-support onboarding-detect --json` 으로 실행하고, 결과에 `cli_path_missing`/`cli_on_path:false` 가 다시 보여도 이미 처리된 것으로 간주하고 다음 gap 으로 넘어가요. `bin_path` 가 없는 구 CLI 면 기존대로 `READY_WITH_USER_ACTION` 으로 "PATH 준비됐어요. 새 터미널을 열고 거기서 Claude 를 실행해 온보딩을 다시 불러 주세요" 라고 안내해요. 새 터미널·VS Code 앱 재시작 안내는 마무리 카드에 보조 문구로 한 번만 붙여요.

### 4. Telemetry opt-in, MCP and Ready card

After gaps are green, load [`references/mcp-ready-card.md`](references/mcp-ready-card.md) and finish in order: AI 활용 기록 옵트인 질문 → optional MCP registration in user scope → 최종 카드. 마무리 진입 시 "마지막 단계예요 — AI 활용 기록(선택)과 axhub 도구 연동을 정리하고, 필요하면 재시작 한 번으로 끝나요." 예고 한 줄을 먼저 말해요. 원칙은 재시작 최대 1회 · 카드 1장 · 질문은 옵트인 1개예요. Never claim MCP connected until `claude mcp get axhub` says `Status: Connected`.

새로 `claude mcp add` 를 실행한 세션에는 서버가 로드되지 않아요 — marker(`~/.axhub/cache/.onboarding-mcp-restart`)를 쓰고 Restart Handoff Card(`READY_WITH_USER_ACTION`)로 종료해요. 재시작 후에는 SessionStart hook 이 marker 를 감지해 새 세션이 마무리를 먼저 제안하고, `VIBE_READY` 를 출력할 때 marker 를 삭제해요. 세부 분기는 reference 의 Claude Code Path / Resume After Restart 섹션이 소유해요.

Finish with one honest card:

- `VIBE_READY`: verified green enough to start coding.
- `READY_WITH_USER_ACTION`: only external user action remains.
- `SAFE_STOP_NONINTERACTIVE`: headless/subprocess mode avoided mutation.
- `BLOCKED_UNSUPPORTED`: no safe OS/package-manager/permission path exists.

## NEVER

- NEVER call preflight before CLI detection; `onboarding-detect --json` is the fail-open first step.
- NEVER treat `command -v axhub` success as `cli_missing`; pin `AXHUB_BIN` and continue from the real detect state.
- NEVER treat `command -v axhub` failure as `cli_missing` when `~/.axhub/bin/axhub` or `~/.axhub/bin/axhub.exe` exists; route to `cli_path_missing` instead.
- NEVER `first_gap`, `gaps`, `cli_state`, `auth_error_code` 같은 detect 필드명·enum 값을 사용자에게 그대로 말하지 말아요.
- NEVER call detect via bare `axhub` or export `AXHUB_BIN` in the on-disk-not-on-PATH branch before repair-path; it can hide the PATH gap. After the repair-path pivot, re-detect via the `bin_path` absolute command and treat a repeated `cli_path_missing` as already handled.
- NEVER loop re-detect in the same session after repair-path if `command -v axhub` still fails; continue via the repair-path `bin_path` absolute command (new terminal is next-session advice; old CLI without `bin_path` falls back to the new-terminal stop).
- NEVER require the user to know sibling skill names or slash commands to finish onboarding.
- NEVER run multiple mutate gaps from one detect result. Always detect-first -> first_gap -> re-detect.
- NEVER run plugin update during onboarding; mention `/plugin update` as advisory only.
- NEVER move GitHub OAuth device-flow into the install_url stage; install_url is account-level App installation.
- NEVER 빈 폴더에서 bootstrap 스킬로 위임하거나 앱을 자동 생성하지 말아요.
- NEVER dependency install without a lockfile.
- NEVER omit `--ignore-scripts` from dependency install.
- NEVER 묻지 않고 또는 headless 에서 AI 활용 기록 수집(`axhub axrouter monitor`)을 켜지 말아요; 거절한 사용자에게 같은 온보딩에서 다시 묻지 말아요.
- NEVER subprocess(`claude -p`/CI/headless)에서 install/update/auth/bootstrap/deps mutation 이나 git/node system install/version switch 를 자동 실행하지 말아요.
- NEVER mark unchecked items green in `VIBE_READY`.
- NEVER run deploy verify without the concrete deployment id and app scope from the deploy output; no latest re-search.
- NEVER claim axhub MCP is connected after add only; require `claude mcp get axhub` connected status.
- NEVER `claude mcp add` 를 실행한 그 세션에서 `/mcp` OAuth 완료나 `mcp__axhub__*` 도구 활성화를 안내하지 말아요 — marker 를 쓰고 Restart Handoff Card 로 종료해요.
- NEVER `VIBE_READY` 출력 후 marker(`~/.axhub/cache/.onboarding-mcp-restart`)를 남기지 말아요.

## Additional Resources

- `../deploy/references/error-empathy-catalog.md` — Korean exit-code response shape.
- `../bootstrap/SKILL.md` — bootstrap saga source for explicit first-app creation; onboarding does not delegate to it automatically.
