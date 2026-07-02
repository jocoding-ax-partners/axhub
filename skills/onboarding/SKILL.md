---
name: onboarding
description: 'Use when the user is new to axhub or asks for first setup/onboarding/getting started. 이 스킬은 "셋업해줘", "처음인데", "처음 쓰는데 뭐부터", "온보딩", "시작하기", "axhub 시작", "초기 셋업", "setup", "onboard", "getting started", "first time" 같은 첫 사용자 셋업 의도를 담당해요. axhub CLI 설치, 로그인, git/node, GitHub App, 앱 연결, 의존성, 최종 Ready card 를 detect-first 로 안내하되 빈 폴더에서 init 을 자동 실행하지 않아요.'
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

처음 axhub 를 쓰는 사람을 위한 단일 진입점이에요. 사용자는 `온보딩`, `처음인데 뭐부터`, `getting started` 처럼 말하면 되고, 이 스킬은 CLI/auth/runtime/GitHub/repo/deps/MCP 준비를 한 gap 씩 닫아요. 환경 진단만 원하면 doctor/diagnosis 가 맞고, 새 앱 생성을 명시하면 init 이 맞아요. onboarding 은 빈 폴더에서도 자동 init 을 시작하지 않고 Ready card 에서 `첫 앱 만들어줘` 를 다음 말로 안내해요.

## Reference Loading

이 top-level 파일은 routing, safety, exact command anchors 를 보존하는 compact contract 예요. 세부 UX 는 detect 결과가 해당 branch 를 요구할 때만 아래 reference 를 읽어요.

- [`references/gap-state-machine.md`](references/gap-state-machine.md): `first_gap` 라우팅, gap별 완료 기준, repo/app/doctor/deploy evidence 흐름.
- [`references/install-channels-and-auth.md`](references/install-channels-and-auth.md): CLI 설치·PATH repair·update·auth, git/node 설치와 version 교정.
- [`references/github-app.md`](references/github-app.md): GitHub App install URL visibility, 다른 계정 추가 질문, 미설치 gate.
- [`references/dependency-install.md`](references/dependency-install.md): lockfile-only dependency install, `--ignore-scripts`, native build downgrade.
- [`references/mcp-ready-card.md`](references/mcp-ready-card.md): MCP add/auth distinction, `VIBE_READY`, `READY_WITH_USER_ACTION`, `SAFE_STOP_NONINTERACTIVE` card variants.

References 는 이 스킬의 일부예요. 명령 의미를 바꾸지 말고, top-level invariant 와 reference detail 이 충돌하면 top-level safety invariant 를 우선해요.

## Core Contract

1. **Single source of truth.** 모든 gap 판정은 `axhub plugin-support onboarding-detect --json` 한 번에서 온 JSON 이 source of truth 예요. `first_gap` 이 처리 순서를 결정해요. gap 마다 preflight 를 다시 돌려 순서를 추측하지 않아요.
2. **Detect-first loop.** `detect -> first_gap 하나 처리 -> 재감지` 를 반복해요. 한 번에 여러 mutate gap 을 실행하지 않아요. 사용자가 해야 하는 브라우저 승인, OS installer GUI, PATH reload, OAuth 는 `READY_WITH_USER_ACTION` 으로 멈추고 `승인했어`, `온보딩 계속`, `다시 온보딩해줘` 같은 자연어 재개 phrase 를 남겨요.
3. **Headless safety.** subprocess/headless/CI 에서는 AskUserQuestion 을 생략하고 safe defaults 로 멈춰요. install/update/auth/init/deps mutation, git/node system install, node version switch, browser open, MCP OAuth 를 자동 실행하지 않아요. 최종 상태는 `SAFE_STOP_NONINTERACTIVE` 예요.
4. **No automatic init.** 빈 폴더나 manifest 없는 폴더를 발견해도 init skill 로 위임하거나 앱을 자동 생성하지 않아요. `no_manifest_empty` 는 안내 후 Ready card 로 가고, 다음 말은 `첫 앱 만들어줘` 예요.
5. **GitHub App visibility.** detect JSON 의 `github.install_url` 이 null 이 아니면 설치 여부·계정 수·`first_gap` 과 무관하게 한 번은 보여줘요. `github.state` 가 `uninstalled`/`empty` 면 설치 확인 전 Step 7 repo/app 연결로 넘어가지 않아요.
6. **Dependency safety.** 의존성 설치는 manifest 와 lockfile 이 있을 때만, 명시 확인 뒤, 해당 lockfile 의 package manager 로만 실행해요. 모든 install command 는 반드시 `--ignore-scripts` 를 붙여요. lockfile 이 없으면 설치하지 않아요.
7. **MCP truth.** `claude mcp add` 는 등록일 뿐이에요. `claude mcp get axhub` 가 `Status: Connected` 를 보여주기 전까지 `mcp__axhub__*` 가 연결됐다고 말하지 말고 `/mcp` OAuth 안내로 남겨요.
8. **Ready card honesty.** 확인하지 않은 항목은 green check 로 표시하지 않아요. 가능한 종료 상태는 `VIBE_READY`, `READY_WITH_USER_ACTION`, `SAFE_STOP_NONINTERACTIVE`, `BLOCKED_UNSUPPORTED` 예요.

## Progress

각 단계 시작에는 사용자가 멈춘 것으로 오해하지 않게 한국어 한 줄만 말해요. raw JSON, secret, internal id, full stderr 는 chat 에 넣지 않아요.

- `환경 점검하는 중이에요`
- `axhub CLI 설치하는 중이에요`
- `로그인 진행하는 중이에요`
- `실행환경(node·git) 점검하는 중이에요`
- `GitHub App 설치 확인하는 중이에요`
- `필요한 패키지 설치하는 중이에요`
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
elif [ -f "$HOME/.axhub/bin/axhub" ] || [ -f "$HOME/.axhub/bin/axhub.exe" ]; then
  DETECT_JSON='{"cli_present":true,"cli_on_path":false,"cli_state":"on_disk_not_on_path","first_gap":"cli_path_missing","github":{"state":"unavailable","install_url":null}}'
else
  DETECT_JSON='{"cli_present":false,"first_gap":"cli_missing","github":{"state":"unavailable","install_url":null}}'
fi
echo "$DETECT_JSON"
```

`AXHUB_BIN` 은 PATH/HOME 차이 때문에 detect self-probe 가 현재 shell 의 axhub 를 못 찾는 오탐을 줄이기 위한 pin 이에요. `command -v axhub` 는 실패했지만 canonical install dir(`~/.axhub/bin/axhub` 또는 `.exe`)에 파일이 있으면 재설치가 아니라 `cli_path_missing` 이에요. 이 branch 에서는 detect 를 부르거나 `AXHUB_BIN` 을 export 하지 않아요. 열린 세션이 PATH 를 못 읽는 상태라 detect 가 `cli_on_path:true` 로 오보하거나 같은 gap 을 반복할 수 있기 때문이에요.

주요 필드는 `first_gap`, `gaps`, `cli_present`, `cli_version`, `cli_state`, `cli_on_path`, `cli_too_old`, `has_update`, `latest_version`, `auth_ok`, `auth_error_code`, `git_present`, `git_repo`, `git_commit`, `node_present`, `node_version`, `node_required`, `node_mismatch`, `manifest_present`, `lockfile_present`, `deps_missing`, `dir_empty`, `github`, `deploy_checked`, `deploy_verified` 예요.

### 2. GitHub App surface

DETECT 직후 `github.install_url` 이 있으면 항상 한 줄로 보여줘요. 이미 설치되어 있어도 다른 org/계정을 더 연결할 수 있다는 말을 붙여요. `installed_logins` 는 login 만 보여주고 `installation_id` 같은 internal 값은 보여주지 않아요. 자세한 질문과 gate 는 [`references/github-app.md`](references/github-app.md)를 읽어요.

### 3. first_gap router

`first_gap` 만 처리하고 재감지해요. 아래 table 은 owner map 이고, 순서는 detect JSON 이 정해요.

| `first_gap` | Handler |
| --- | --- |
| `cli_missing` | CLI install approval. Load [`references/install-channels-and-auth.md`](references/install-channels-and-auth.md). |
| `cli_path_missing` | `axhub plugin-support repair-path --json`; then user terminal reload or re-detect. |
| `cli_old` | `axhub update check` / `axhub update apply --execute --yes --json`; load install reference. |
| `auth_missing` | `axhub auth status`, refresh, or device login; load install reference. |
| `git_missing` | git install approval; load install reference. |
| `node_missing` | node install approval; load install reference. |
| `node_mismatch` | nvm/package-manager version correction approval; load install reference. |
| `github_app_missing` | GitHub App install gate; load [`references/github-app.md`](references/github-app.md). |
| `existing_repo_gap` | Existing repo app connection via `axhub apps git`; load gap-state reference and GitHub reference. |
| `no_manifest_empty` | No init. Show advisory and go to Ready card with `첫 앱 만들어줘`. |
| `deps_missing` | Lockfile-only install with `--ignore-scripts`; load [`references/dependency-install.md`](references/dependency-install.md). |
| `deploy_unverified` | Verify only known deployment id and app scope with `axhub deploy verify "$DEPLOYMENT_ID" --app "$APP_ID_OR_SLUG"`. |
| `doctor_gap` | Final read-only `axhub plugin-support preflight --json` and recovery phrase. |
| `no_gap` | Ready card. |

If a handler needs a prompt but D1 safe-stop mode is active, do not execute the mutation. Return `SAFE_STOP_NONINTERACTIVE` with the exact manual command or natural phrase.

`cli_path_missing` 은 CLI 가 디스크에 있는데 현재 셸 PATH 에 없는 상태예요. repair-path 뒤에도 `command -v axhub` 가 실패하면 무한 루프 방지를 위해 재감지를 반복하지 말고, `READY_WITH_USER_ACTION` 으로 "PATH 준비됐어요. 새 터미널을 열고 거기서 Claude 를 실행해 온보딩을 다시 불러 주세요" 라고 안내해요. 같은 터미널에서 Claude 만 재실행하면 stale 환경이라 같은 gap 이 반복될 수 있어요.

### 4. MCP and Ready card

After gaps are green, optionally register axhub MCP in user scope and verify authentication status. Load [`references/mcp-ready-card.md`](references/mcp-ready-card.md) before doing this step. Never claim MCP connected until `claude mcp get axhub` says `Status: Connected`.

Finish with one honest card:

- `VIBE_READY`: verified green enough to start coding.
- `READY_WITH_USER_ACTION`: only external user action remains.
- `SAFE_STOP_NONINTERACTIVE`: headless/subprocess mode avoided mutation.
- `BLOCKED_UNSUPPORTED`: no safe OS/package-manager/permission path exists.

## NEVER

- NEVER call preflight before CLI detection; `onboarding-detect --json` is the fail-open first step.
- NEVER treat `command -v axhub` success as `cli_missing`; pin `AXHUB_BIN` and continue from the real detect state.
- NEVER treat `command -v axhub` failure as `cli_missing` when `~/.axhub/bin/axhub` or `~/.axhub/bin/axhub.exe` exists; route to `cli_path_missing` instead.
- NEVER call detect or export `AXHUB_BIN` in the on-disk-not-on-PATH branch; it can hide the PATH gap.
- NEVER loop re-detect in the same session after repair-path if `command -v axhub` still fails; tell the user to open a new terminal.
- NEVER require the user to know sibling skill names or slash commands to finish onboarding.
- NEVER run multiple mutate gaps from one detect result. Always detect-first -> first_gap -> re-detect.
- NEVER run plugin update during onboarding; mention `/plugin update` as advisory only.
- NEVER move GitHub OAuth device-flow into the install_url stage; install_url is account-level App installation.
- NEVER 빈 폴더에서 init 스킬로 위임하거나 앱을 자동 생성하지 말아요.
- NEVER dependency install without a lockfile.
- NEVER omit `--ignore-scripts` from dependency install.
- NEVER subprocess(`claude -p`/CI/headless)에서 install/update/auth/init/deps mutation 이나 git/node system install/version switch 를 자동 실행하지 말아요.
- NEVER mark unchecked items green in `VIBE_READY`.
- NEVER run deploy verify without the concrete deployment id and app scope from the deploy output; no latest re-search.
- NEVER claim axhub MCP is connected after add only; require `claude mcp get axhub` connected status.

## Additional Resources

- `../deploy/references/error-empathy-catalog.md` — Korean exit-code response shape.
- `../init/SKILL.md` — bootstrap saga source for explicit first-app creation; onboarding does not delegate to it automatically.
