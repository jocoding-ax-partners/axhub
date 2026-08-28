---
name: onboarding
description: 'Use when the user is new to axhub or asks for first setup/onboarding/getting started. 이 스킬은 "셋업해줘", "처음인데", "처음 쓰는데 뭐부터", "온보딩", "시작하기", "axhub 시작", "초기 셋업", "setup", "onboard", "getting started", "first time" 같은 첫 사용자 셋업 의도를 담당해요. axhub CLI 설치, 로그인, git/node, 앱 backend에 필요한 저장소 계정 준비, 의존성, 최종 Ready card를 detect-first로 안내하되 빈 폴더에서 bootstrap을 자동 실행하지 않아요. 현재 앱이 tenant-default selfhosted면 GitHub 로그인/App 설치 단계를 노출하지 않아요. 이 트리거들은 axhub 맥락이 있을 때만 유효해요.'
allows-dependency-execution: true
model: sonnet
---
> 이 본문이 중간에 끊겨 보이면 설치 경로의 이 SKILL.md 원문을 열어 전체 절차를 확인한 뒤 진행해요.


# Onboarding (first-run setup router)

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

처음 axhub를 쓰는 사람을 위한 단일 진입점이에요. CLI/auth/runtime/repo/deps 준비를 한 gap씩 닫되, 저장소 provider 단계는 현재 앱의 `git_backend`를 먼저 확인해요. 환경 진단만 원하면 diagnosis, 새 앱 생성을 명시하면 bootstrap이 맞아요. onboarding은 빈 폴더에서도 자동 bootstrap을 시작하지 않고 Ready card에서 `첫 앱 만들어줘`를 다음 말로 안내해요.

## Codex 첫 세션 안내

- 첫 axhub 명령에서 네트워크 접근 승인을 한 번 물어요 — 허용해야 axhub 백엔드에 닿아요.
- 시작 시 훅 신뢰를 묻는데, 신뢰하지 않으면 자동 업데이트·라우팅 가드가 조용히 꺼져요. `/hooks` 에서 언제든 다시 켤 수 있어요.
- 선택 카드로 답하고 싶으면 `~/.codex/config.toml` 의 `[features]` 에 `default_mode_request_user_input = true` 한 줄을 더하면 돼요. 켠 경우 빈 답변은 미승인으로 처리돼요. 설정을 대신 바꾸지는 않아요.

## Reference Loading

이 top-level 파일은 routing, safety, exact command anchors 를 보존하는 compact contract 예요. 세부 UX 는 detect 결과가 해당 branch 를 요구할 때만 아래 reference 를 읽어요.

- [`references/gap-state-machine.md`](references/gap-state-machine.md): `first_gap` 라우팅, gap별 완료 기준, repo/app/doctor/deploy evidence 흐름.
- [`references/install-channels-and-auth.md`](references/install-channels-and-auth.md): CLI 설치·PATH repair·update·auth, git/node 설치와 version 교정.
- [`references/github-app.md`](references/github-app.md): backend gate가 GitHub를 고른 경우에만 계정 연동, install URL, 다른 계정 추가 질문, 미설치 gate.
- [`references/dependency-install.md`](references/dependency-install.md): lockfile-only dependency install, `--ignore-scripts`, native build downgrade.
- [`references/ready-card.md`](references/ready-card.md): AI 활용 기록 옵트인, `VIBE_READY`, `READY_WITH_USER_ACTION`, `SAFE_STOP_NONINTERACTIVE` card variants.

References 는 이 스킬의 일부예요. 명령 의미를 바꾸지 말고, top-level invariant 와 reference detail 이 충돌하면 top-level safety invariant 를 우선해요.

## Core Contract

1. **Single source of truth.** gap 순서는 `axhub plugin-support onboarding-detect --json`을 써요. backend는 app target이 있으면 `axhub apps get <app> --json`, 없으면 `axhub apps git-backend --tenant <tenant> --json`의 top-level `git_backend`만 써요. Gitea API·C1 HTTP·remote URL은 보지 않아요.
2. **Detect-first loop.** `detect -> backend 판정 -> first_gap 하나 처리 -> 재감지`를 반복해요. selfhosted app/tenant에서는 provider 전용 gaps만 non-applicable로 건너뛰고 다음 non-provider gap을 처리해요.
3. **Headless safety.** 순수 subprocess/headless/CI에서는 명시 텍스트 승인을 생략하고 safe defaults로 멈춰요. install/update/auth/bootstrap/deps mutation, git/node system install, node version switch, browser open을 자동 실행하지 않아요.
4. **No automatic bootstrap.** 빈 폴더나 manifest 없는 폴더를 발견해도 bootstrap을 자동 실행하지 않아요. `no_manifest_empty`는 안내 후 Ready card로 가요.
5. **Repository provider visibility.** `git_backend.backend=selfhosted`면 detect JSON의 provider install URL·설치 상태를 보여주지 않고 계정 연동/App 설치 질문을 만들지 않아요. `git_backend.backend=github` 또는 `git_backend.source=legacy_github`만 기존 visibility와 gate를 유지해요.
6. **Dependency safety.** 의존성 설치는 manifest와 lockfile이 있을 때만, 명시 확인 뒤, 해당 lockfile의 package manager로만 실행해요. 모든 install command는 `--ignore-scripts`를 붙여요.
7. **Ready card honesty.** 확인하지 않은 항목은 green check로 표시하지 않아요. 내부 종료 상태 enum은 사용자 문장에 출력하지 말아요.
8. **Telemetry opt-in.** AI 활용 기록은 무엇이 수집되는지 설명하고 물어본 뒤 사용자가 켜기를 고를 때만 켜요 — 동의 없이 켜지 않아요. 거절하면 같은 온보딩에서 다시 묻지 않고, headless에서는 묻지도 켜지도 않아요.
9. **axhub 맥락 게이트.** axhub 맥락이 없으면 detect 전에 "axhub 셋업을 말하는 거예요?"를 한 번만 물어요. 아니라는 답이면 종료하고 다른 axhub skill로 넘기지 않아요.

## Progress

각 단계 시작에는 사용자가 멈춘 것으로 오해하지 않게 한국어 한 줄만 말해요. raw JSON, secret, internal id, full stderr 는 chat 에 넣지 않아요.
사용자에게 보이는 문장과 Bash/tool call 제목은 한국어로만 써요. `first_gap`, `gaps`, `cli_state`, `auth_error_code` 같은 detect 필드명이나 enum 값은 내부 라우팅용으로만 읽고, chat 에 그대로 출력하지 않아요. "빈 폴더라 자동으로 만들지 않았어요"처럼 사람 말로 바꿔요.

사용자에게 보이는 모든 URL은 평문 `https://...` 절대 URL로만 써요. 앱 URL과, GitHub branch에서만 필요한 install/device URL을 Markdown 링크로 감싸지 않아요.

- `환경 점검하는 중이에요`
- `axhub CLI 설치하는 중이에요`
- `로그인 진행하는 중이에요`
- `실행환경(node·git) 점검하는 중이에요`
- GitHub branch만 `GitHub 계정 연동 확인하는 중이에요`, `GitHub App 설치 확인하는 중이에요`
- `필요한 패키지 설치하는 중이에요`
- `AI 활용 기록 설정 확인하는 중이에요`
- `준비 다 됐어요`

update_plan 가 host 에 있으면 checklist 를 갱신해요. 없으면 언급하지 말고 자연어 진행 알림만 사용해요.

## Workflow

### 0. Non-interactive guard

첫 명시 텍스트 승인 또는 mutation 전에 대화형 여부를 판단해요. 다음 중 하나면 D1 safe-stop mode 예요: stdout 이 TTY 가 아님, `CI` 가 있음, `CODEX_NON_INTERACTIVE` 가 있음, `codex exec` 같은 subprocess/headless 호출임. 이 모드에서는 사용자 확인이 필요한 action 을 실행하지 않고 manual next phrase 와 `SAFE_STOP_NONINTERACTIVE` card 로 끝내요.

### 1. DETECT_ALL(read-only)

항상 먼저 한 번 감지해요. Codex Desktop 이 첫 읽기 전용 점검을 위험한 복합 셸처럼 보이지 않게, 아래 probe 는 **각각 별도 Bash tool call의 한 명령만** 실행해요. 명령 본문과 경로는 사용자 chat 에 출력하지 않아요.

1. 정확히 `command -v axhub` 를 실행해요.
2. 성공하면 반환된 한 줄이 절대경로인지 확인하고, 그 값을 그대로 인용한 `"<axhub 절대경로>" plugin-support onboarding-detect --json` 을 별도 call 로 실행해요. 반환된 경로를 변수에 넣거나 bare `axhub` 로 다시 찾지 않아요.
3. `command -v axhub` 가 실패하면 정확히 `cat "$HOME/.axhub/bin-path"` 를 별도 call 로 실행해요. 한 줄 경로가 있으면 `test -f "<반환된 절대경로>"` 를 별도 call 로 확인해요.
4. location 파일도 없거나 가리킨 파일이 없으면 `test -f "$HOME/.axhub/bin/axhub"` 와 `test -f "$HOME/.axhub/bin/axhub.exe"` 를 각각 필요할 때만 별도 call 로 확인해요.
5. PATH 에서 찾은 바이너리의 detect 출력이 비었으면 내부적으로 `doctor_gap` fallback 으로 처리해요. PATH 밖 파일만 확인됐으면 내부적으로 `cli_path_missing`, 어디에도 없으면 `cli_missing` 으로 처리해요. fallback JSON 을 만들기 위한 `echo` 명령은 실행하지 않아요.

이 단계의 command string 에 `if`/`elif`, `$()`, `&&`/`||`, `;`, pipe, redirect, 변수 대입을 넣지 않아요. Desktop 에 `Contains shell syntax ... cannot be statically analyzed` 권한 카드가 뜨는 명령은 실패예요. 반환된 절대경로를 직접 실행하는 방식이 기존 `AXHUB_BIN` pin 역할을 대신해 PATH/HOME 차이의 오탐을 막아요. `command -v axhub` 는 실패했지만 location 파일(`~/.axhub/bin-path` — CLI 0.24.8+ 가 자기 설치 위치를 기록)이 가리키는 파일이나 canonical install dir(`~/.axhub/bin/axhub` 또는 `.exe`)에 파일이 있으면 재설치가 아니라 `cli_path_missing` 이에요 — 새 세션이 부모 앱의 stale PATH 를 물려받아도 재설치를 권하지 않아요. location 파일 덕에 CARGO_HOME 등 커스텀 설치 위치도 인식돼요. 이 branch 에서는 detect 를 부르지 않아요. 열린 세션이 PATH 를 못 읽는 상태라 같은 gap 을 반복할 수 있기 때문이에요.

주요 detect 필드는 parsing 전용이고 사용자-facing 문장·표·도구 제목에는 노출하지 않아요.

### 2. Git backend gate
DETECT의 `first_gap`이 `cli_missing`·`cli_path_missing`·`cli_old`·`auth_missing` 중 하나면 이 단계의 나머지를 건너뛰고 4절의 해당 handler를 먼저 실행한 뒤 DETECT_ALL로 돌아와요. `cli_path_missing` 복구 후에는 반환된 absolute `bin_path`를 계속 사용해요. CLI·auth gap을 닫기 전에는 preflight와 backend read를 절대 실행하지 않아요.


DETECT 뒤 `axhub plugin-support preflight --json`을 한 번 실행해요. `capabilities.self_hosted_git.apps_git_backend`, `capabilities.self_hosted_git.app_git_backend`, `capabilities.self_hosted_git.git_setup`이 모두 `true`여야 하며, 누락·malformed/false면 `axhub CLI를 최신 버전으로 업데이트해 주세요.`라고 안내하고 멈춰요.

app target이 있으면 첫 command, fresh tenant면 둘째를 Tool 제목 `앱 설정 확인`으로 한 번 실행해요. 확인된 literal tenant만 써요.

```bash
axhub apps get <app> --json
axhub apps git-backend --tenant <tenant> --json
```

top-level `git_backend.backend`·`git_backend.source`만 읽어요. `git_backend.backend=selfhosted`면 `github.install_url`을 숨기고 `github_link_missing`·`github_app_missing`을 처리하지 않아요. `references/github-app.md`를 건너뛰며 계정 로그인·App 설치 대사를 0회로 유지해요. tenant source는 `tenant|platform_default`예요.

read command 실패나 missing/malformed `git_backend`만 provider 단계 전에 중단 사유예요. app row를 먼저 만들거나 GitHub를 platform default로 추측하거나 tenant C1/Gitea를 직접 호출하지 않아요.

### 3. Repository provider surface

두 command 중 하나가 `git_backend.backend=github` 또는 app `git_backend.source=legacy_github`를 반환한 경우에만 기존 GitHub surface를 사용해요. DETECT의 `github.install_url`을 한 줄로 보여주고 설치 gate 앞에 `axhub github accounts list --json`을 한 번 돌려요. exit 4 + `github_relogin_required`면 `github_link_missing`으로 처리해요.

selfhosted이고 CLI/auth gap이 닫혔으면 session당 한 번 `axhub git setup --json`으로 account·endpoint별 helper·30일 PAT를 준비해요. marker로 재실행을 막고 GitHub 질문은 0회로 유지해요.

### 4. first_gap router

`first_gap`만 처리하고 재감지해요. CLI는 gap이 없으면 `first_gap:null`과 빈 `gaps`를 반환하며 이는 `no_gap`과 같은 완료예요. `doctor_gap`은 detect 출력이 비었을 때만 쓰는 플러그인 합성 값이에요. `github_link_missing`도 합성 값이라 GitHub branch의 3단계 연동 확인에서만 생기고 `github_app_missing`보다 먼저 처리해요. selfhosted branch는 둘 다 non-applicable이에요.

| `first_gap` | Handler |
| --- | --- |
| `cli_missing` | CLI install approval. Load [`references/install-channels-and-auth.md`](references/install-channels-and-auth.md). |
| `cli_path_missing` | detect 가 준 `cli_resolved_path`(없으면 canonical `"$HOME/.axhub/bin/axhub"`, Windows 는 `.exe`)로 `plugin-support repair-path --json` 실행 (bare `axhub` 는 이 상태에서 127); continue THIS session via the JSON `bin_path` absolute command; new terminal is next-session advice. |
| `cli_old` | `axhub update check` / `axhub update apply --execute --yes --json`; load install reference. |
| `auth_missing` | `axhub auth status`, refresh, or device login; load install reference. |
| `git_missing` | git install approval; load install reference. |
| `node_missing` | node install approval; load install reference. |
| `node_mismatch` | nvm/package-manager version correction approval; load install reference. |
| `github_link_missing` | GitHub 계정 연동 gate; load [`references/github-app.md`](references/github-app.md). |
| `github_app_missing` | GitHub App install gate; load [`references/github-app.md`](references/github-app.md). |
| `existing_repo_gap` | Existing repo app connection via `axhub apps git`; load gap-state reference and GitHub reference. |
| `no_manifest_empty` | No bootstrap. Show advisory and go to Ready card with `첫 앱 만들어줘`. |
| `deps_missing` | Lockfile-only install with `--ignore-scripts`; load [`references/dependency-install.md`](references/dependency-install.md). |
| `deploy_unverified` | Verify only known deployment id and app scope with `axhub deploy verify "$DEPLOYMENT_ID" --app "$APP_ID_OR_SLUG"`. |
| `doctor_gap` | Final read-only `axhub plugin-support preflight --json` and recovery phrase. |
| `no_gap` | Ready card. `first_gap` null/부재 + 빈 `gaps` 도 이 행으로 처리해요. |

If a handler needs a prompt but D1 safe-stop mode is active, do not execute the mutation. Return `SAFE_STOP_NONINTERACTIVE` with the exact manual command or natural phrase.

`cli_path_missing` 은 CLI 가 디스크에 있는데 현재 셸 PATH 에 없는 상태예요. 이미 열린 세션의 PATH 는 밖에서 못 고치므로(OS 설계), repair-path 뒤에 `command -v axhub` 재감지를 반복하지 말고(무한 루프 방지) **repair-path JSON 의 `bin_path` 절대경로로 남은 온보딩 명령을 그대로 이어가요** (예: `"<bin_path>" auth status --json`). detect 의 `cli_resolved_path` 도 같은 절대경로예요. 남은 gap 재감지가 필요하면 `"<bin_path>" plugin-support onboarding-detect --json` 으로 실행하고, 결과에 `cli_path_missing`/`cli_on_path:false` 가 다시 보여도 이미 처리된 것으로 간주하고 다음 gap 으로 넘어가요. `bin_path` 가 없는 구 CLI 면 기존대로 `READY_WITH_USER_ACTION` 으로 "PATH 준비됐어요. 새 터미널을 열고 거기서 Codex 를 실행해 온보딩을 다시 불러 주세요" 라고 안내해요. 새 터미널·VS Code 앱 재시작 안내는 마무리 카드에 보조 문구로 한 번만 붙여요.

### 5. Telemetry opt-in and Ready card

After gaps are green, load [`references/ready-card.md`](references/ready-card.md) and finish in order: AI 활용 기록 옵트인 질문 → 최종 카드. 마무리 진입 시 "마지막 단계예요 — AI 활용 기록(선택)만 정하면 끝나요." 예고 한 줄을 먼저 말해요. 원칙은 카드 1장 · 질문은 옵트인 1개예요. AI 활용 기록을 이번에 켰으면 적용은 Codex 재시작 후라는 점을 카드 한 줄로만 반영해요 — 별도 재시작 handoff 는 없어요.

Finish with one honest card:

- `VIBE_READY`: verified green enough to start coding.
- `READY_WITH_USER_ACTION`: only external user action remains.
- `SAFE_STOP_NONINTERACTIVE`: headless/subprocess mode avoided mutation.
- `BLOCKED_UNSUPPORTED`: no safe OS/package-manager/permission path exists.

위 상태 이름은 내부 판정용이에요. 최종 카드에는 enum 자체나 `[READY_WITH_USER_ACTION]` 같은 표식을 쓰지 말고, 현재 상태와 사용자가 할 다음 행동만 자연스러운 한국어로 보여줘요.

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
- NEVER move GitHub OAuth device-flow into the install_url stage; install_url is account-level App installation. 계정 연동(`github link`)과 App 설치(install_url)는 끝까지 별개 단계예요.
- NEVER 계정 연동을 영구적이라고 말하거나(`다시 묻지 않아요` 류) 승인 완료를 채팅으로 알려 달라고 요구하지 말아요.
- NEVER 빈 폴더에서 bootstrap 스킬로 위임하거나 앱을 자동 생성하지 말아요.
- NEVER dependency install without a lockfile.
- NEVER omit `--ignore-scripts` from dependency install.
- NEVER 묻지 않고 또는 headless 에서 AI 활용 기록 수집(`axhub axrouter monitor`)을 켜지 말아요; 거절한 사용자에게 같은 온보딩에서 다시 묻지 말아요.
- NEVER subprocess(`codex exec`/CI/headless)에서 install/update/auth/bootstrap/deps mutation 이나 git/node system install/version switch 를 자동 실행하지 말아요.
- NEVER mark unchecked items green in `VIBE_READY`.
- NEVER run deploy verify without the concrete deployment id and app scope from the deploy output; no latest re-search.
- NEVER MCP 서버 등록(`codex mcp add`)이나 MCP OAuth 인증을 안내·실행하지 말아요 — 온보딩은 CLI 준비까지만 담당해요.

## Additional Resources

- `../deploy/references/error-empathy-catalog.md` — Korean exit-code response shape.
- `../bootstrap/SKILL.md` — bootstrap saga source for explicit first-app creation; onboarding does not delegate to it automatically.
