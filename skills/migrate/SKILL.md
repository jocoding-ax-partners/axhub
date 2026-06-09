---
name: migrate
description: 이 스킬은 기존 앱 올려줘, 이 프로젝트 axhub로 옮길 수 있어?, 만든 앱 가져오기, migrate, import existing app, 기존 프로젝트 배포 의도에서 활성화해요. Claude Desktop 자연어에서는 로컬 서버 점검이나 일반 배포 조언으로 우회하지 않고 AXHub 가져오기 상태를 먼저 확인해요.
examples:
  - utterance: "기존 앱 올려줘"
    intent: "migrate an existing app into axhub"
  - utterance: "이 프로젝트 axhub로 옮길 수 있어?"
    intent: "check whether this project is already importable or already on axhub"
  - utterance: "이 Next.js 프로젝트 axhub 로 가져와"
    intent: "migrate an existing app into axhub"
  - utterance: "migrate this repo"
    intent: "migrate an existing app into axhub"
  - utterance: "import existing app"
    intent: "migrate an existing app into axhub"
  - utterance: "이미 만든 앱 배포해줘"
    intent: "migrate an existing app into axhub"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# 기존 앱 가져오기

이미 만든 웹 앱을 axhub 앱으로 등록하고, 감지 결과를 확인한 뒤 기존 배포 경로로 올려요. 로컬 감지는 helper 가 하고, 원격 mutation 과 배포는 검증된 axhub CLI 경계만 재사용해요.

## CLI boundary contract

이 스킬은 backend URL 을 직접 호출하지 않아요. remote 감지나 mutation 이 필요하면 먼저 axhub CLI surface 로 가능한지 확인하고, CLI 가 없으면 raw endpoint 로 추측하지 말고 `axhub.yaml` 초안까지만 만들어요.

현재 고정된 실행 가능 CLI 계약은 아래예요.

```bash
# 로컬 디렉터리 감지: helper migrate-plan/local pre-scan 과 axhub.yaml preview 를 기본으로 써요.
"$HELPER" migrate-plan --dir "${AXHUB_MIGRATE_DIR:-.}" --json

# 원격 GitHub repo 감지: v0.17.4 CLI read-only surface 만 사용해요.
axhub apps detect --repo "$OWNER_REPO" --ref "$REF" --path "$APP_PATH" --json
axhub apps detect --owner "$OWNER" --repo-name "$REPO" --ref "$REF" --path "$APP_PATH" --json

# 앱 등록. `axhub.yaml` 은 deploy manifest 전용이라 apps create 입력으로 쓰지 않아요.
axhub apps create --name "$APP_NAME" --slug "$APP_SLUG" --deploy-method "$DEPLOY_METHOD" --json
# 또는 별도 JSON app-create DTO 를 준비했으면
axhub apps create --from-file app-create.json --json

# GitHub 연결 dry-run 검증
axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --json

# GitHub 연결 mutation
axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json

# env 값 저장. 값은 stdin 으로만 넣어요.
printf '%s' "$VALUE" | axhub env set --app "$APP_ID" "$KEY" --secret --from-stdin --stage "$STAGE" --json

# 선택형 auth 전환용 OAuth client 생성. raw backend curl 대신 CLI 만 써요.
# 실행 전 approval context 은 action=auth_oauth_client_create, app_id=$APP_ID,
# context={name,type,redirect_uris,scopes,grant_types} 와 정확히 맞춰요.
axhub auth oauth client create --app "$APP_ID" --name web --type confidential --redirect-uri "$CALLBACK_URL" --scope openid --scope profile --scope email --grant-type authorization_code --execute --json

# 배포 시작. deploy skill 과 같은 preview confirmation / status-first gate 뒤에만 실행해요.
axhub deploy create --app "$APP_ID" --commit "$COMMIT_SHA" --execute --json
```

`axhub apps detect` 는 v0.17.4 의 현재 read-only remote 감지 계약이에요. 이 스킬 계약은 아래처럼 나뉘어요.

- 로컬 입력은 `AXHUB_MIGRATE_DIR` 또는 현재 디렉터리를 helper `--dir` 로 넘기고, monorepo 하위 앱은 `--app-path` 만 추가해요.
- 원격 GitHub repo 입력은 `axhub apps detect --repo "$OWNER_REPO"` 또는 `--owner "$OWNER" --repo-name "$REPO"` 를 사용해요. `--ref` 와 `--path` 는 사용자가 준 경우에만 추가하고, raw backend endpoint 로 추측하지 않아요.
- 성공 JSON 은 `data.detected_providers`, `data.framework`, `data.install`, `data.build`, `data.start`, `data.port`, `data.health_path`, `data.deploy_method`, `data.confidence`, `data.env_refs[]` 를 읽어요. unknown future field 는 무시해요.
- exit `0` 은 preview 성공이에요. exit `64` 는 local path 형식·필수 플래그 또는 remote repo/ref/path validation 실패예요. exit `1` 은 generic 실패로 다뤄요.
- non-zero detect/scan 은 앱 등록·git 연결·배포를 시작하지 않고, helper 또는 CLI error envelope 의 request_id/subcode/doc_url 이 있으면 그대로 보여줘요.

remote detect 는 현재 CLI 로만 써요. `axhub apps detect --help` 에 없는 flag 를 만들거나 raw backend detect endpoint 를 직접 curl 하지 않아요.

현재 production 감지는 아래 패턴까지 기대해요.

- Compose: `docker-compose.yml`, `docker-compose.yaml`, `compose.yml`, `compose.yaml`
- Dockerfile: repo root 또는 선택한 `--path` 아래의 `Dockerfile`
- Node: package start/build script, Next.js, Nuxt, SvelteKit, Remix
- Python: FastAPI, Django, Flask
- Go: 기본 `net/http`, Gin, Fiber, Echo, Chi
- Ruby: Sinatra, Ruby on Rails
- Java: Maven, Gradle
- Rust: Cargo 기반 웹 앱

## axhub.yaml control contract

자동 감지가 맞지 않거나 배포가 health/start/compose 단계에서 흔들리면 raw backend endpoint 가 아니라 `axhub.yaml` 로 제어해요. 새로 쓰는 canonical 파일은 항상 `axhub.yaml` 이에요. 백엔드 parser 는 8KB 를 넘는 파일을 거부하고, KnownFields 검사가 켜져 있어서 아래 표에 없는 YAML field 는 파싱 실패예요.

| 제어 대상 | field | 값 / 제약 | 쓰는 상황 |
| --- | --- | --- | --- |
| schema | `version` | `axhub/v1` 권장 | 에이전트가 생성한 manifest 임을 명확히 해요 |
| 표시 이름 | `name` | 문자열 | UI 표시 label 힌트가 필요할 때만 써요 |
| 런타임 port | `runtime.port` | `1..65535` container listen port | 프로세스가 3000/8000/8080 등에서 뜨는데 probe/service 가 다른 port 를 볼 때 써요 |
| health path | `runtime.health_path` | HTTP path 문자열 | `/health`, `/api/healthz` 처럼 `/` 가 404/302 를 주는 앱에서 써요 |
| build 전략 | `build.strategy` | `auto` 또는 `pinned` | 자동 감지를 쓰면 `auto`, command 를 계약으로 고정하면 `pinned` 를 써요 |
| framework hint | `build.framework` | `node`, `python`, `go`, `rust`, `ruby`, `java`, `kotlin` | generated Dockerfile base image 와 default command 힌트가 필요할 때만 써요 |
| install command | `build.install` | shell command 문자열 | lockfile/package manager 가 자동 감지와 다를 때 써요 |
| build command | `build.build` | shell command 문자열 | 빌드 산출물 위치나 test skip 정책을 고정할 때 써요 |
| start command | `build.start` | shell command 문자열 | start command 가 없거나 `PORT` bind 를 직접 보장해야 할 때 써요 |
| Dockerfile path | `build.dockerfile` | repo 안 path | `Dockerfile.prod`, `deploy/Dockerfile` 같은 non-root path 를 쓸 때 써요 |
| deploy method | `build.deploy_method` | `docker` 또는 `compose` | compose 로 배포해야 하면 `compose` 로 고정해요 |
| compose file | `build.compose_file` | repo 안 path | `compose.yaml`, `compose.yml`, `docker-compose.yaml` 등 실제 파일명을 고정해요 |
| env names | `env.required[]`, `env.optional[]` | `name`, `scope: build/runtime/both` | secret 값 없이 필요한 env 이름과 쓰임새만 기록해요 |
| CI commands | `ci.commands[]` | 최대 10개 | 배포 전 lightweight check 를 고정할 때만 써요 |
| CI timeout | `ci.timeout` | `1..600` 초 | CI command budget 을 조정할 때만 써요 |

Framework hint 는 검증용 enum 이 아니라 generated image/command 힌트예요. Node/Next/Nuxt/SvelteKit/Remix 는 `node`, Django/Flask/FastAPI 는 `python`, Gin/Fiber/Echo/Chi 는 `go`, Rails/Sinatra 는 `ruby`, Maven 은 `java`, Gradle Java 는 `java`, Gradle Kotlin/KTS 는 `kotlin` 을 우선 써요. Dockerfile 이나 compose 가 이미 정확하면 `build.framework` 를 억지로 넣지 않아도 돼요.

에이전트는 아래 순서로 manifest 를 최소화해요.

1. 자동 감지가 성공하면 `axhub.yaml` 을 새로 쓰지 않아요.
2. port/path 문제만 있으면 `runtime` 섹션만 추가해요.
3. start command 가 불명확하면 `build.strategy: pinned` 와 `build.start` 를 추가해요.
4. Dockerfile/compose 파일명이 문제면 `build.dockerfile` 또는 `build.deploy_method: compose` + `build.compose_file` 만 추가해요.
5. env 는 이름과 scope 만 넣고, 값은 `axhub env set --from-stdin` 으로 따로 저장해요.

가장 작은 port/health override 예시는 아래예요.

```yaml
version: axhub/v1
runtime:
  port: 3000
  health_path: /
```

Next.js 처럼 command 를 고정해야 할 때는 아래처럼 써요.

```yaml
version: axhub/v1
runtime:
  port: 3000
  health_path: /
build:
  strategy: pinned
  framework: node
  install: "npm ci"
  build: "npm run build"
  start: "npm run start"
```

compose 파일명을 제어해야 하면 아래처럼 써요.

```yaml
version: axhub/v1
build:
  deploy_method: compose
  compose_file: compose.yaml
```

Gradle Java 앱의 generated Dockerfile command 를 고정해야 하면 아래처럼 써요.

```yaml
version: axhub/v1
runtime:
  port: 8080
  health_path: /
build:
  strategy: pinned
  framework: java
  build: "if [ -f gradlew ]; then chmod +x gradlew && ./gradlew clean build -x test; else gradle clean build -x test; fi"
  start: "java $JAVA_OPTS -jar build/libs/*jar"
```

env 는 값 없이 이름과 scope 만 커밋해요.

```yaml
version: axhub/v1
env:
  required:
    - name: DATABASE_URL
      scope: runtime
  optional:
    - name: NEXT_PUBLIC_API_URL
      scope: build
```

문자열에 `:`, `#`, `{}`, `$` 같은 YAML 에 민감한 문자가 있으면 quote 해요. multi-line command, secret value, 설명용 key 는 넣지 않아요. 특히 `source`, `priority`, `max_size`, `unknown_field`, `database`, `registry`, `resources` 같은 백엔드에 없는 field 를 manifest 안에 넣으면 안 돼요.

배포 실패를 보고 제어할 때는 아래처럼 매핑해요.

- `connection refused`, `readiness probe failed`, `container port mismatch` → `runtime.port` 를 실제 listen port 로 고정해요.
- `/` 가 404/302 이고 앱은 정상 기동 → `runtime.health_path` 를 200 을 주는 path 로 고정해요.
- `railpack did not produce a start command`, `missing start` → `build.strategy: pinned` + `build.start` 를 추가해요.
- compose 파일을 못 찾음 → `build.deploy_method: compose` + 실제 `build.compose_file` 을 추가해요.
- private package, DB, OAuth, external API 가 필요함 → `env.required[]` 이름만 추가하고 값은 CLI env 경로로 받아요.
- worker-only, non-HTTP 앱, native system package 가 많은 앱은 `axhub.yaml` 만으로 보장하지 말고 Dockerfile/compose 계약을 먼저 요청해요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

## Claude Desktop Natural-Language Path

For ordinary Claude Desktop prompts such as `이 프로젝트 axhub로 옮길 수 있어?`, do this short path before reading the long migration playbook.

1. The first visible chat sentence must be exactly `가져오기 상태를 확인할게요.`
2. Use exactly one Bash tool with the title `가져오기 상태 확인`.
3. Run `axhub-helpers migrate-summary --user-utterance "<latest user sentence>"`.
4. Copy the Korean stdout as the answer.

Do not inspect local server state, package scripts, git release state, QA result files, previous deployment failures, or plugin source files for this prompt. Do not show command names, raw JSON fields, raw deploy status fields, local server evidence, route labels, slash commands, skill names, ToolSearch narration, emoji, or English tool-title fragments. App registration, GitHub connection, env writes, and deployment require a Korean preview and explicit approval before execution.

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"
if [ ! -x "$HELPER" ] && [ -x "${HELPER}.exe" ]; then HELPER="${HELPER}.exe"; fi
if [ ! -x "$HELPER" ]; then HELPER="$(command -v axhub-helpers 2>/dev/null || command -v axhub-helpers.exe 2>/dev/null || printf '%s\n' axhub-helpers)"; fi
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

Git Bash/MSYS bash 가 없는 Windows 에서는 PowerShell 로 같은 helper 계약을 실행해요.

```powershell
$PluginRoot = if ($env:CLAUDE_PLUGIN_ROOT) { $env:CLAUDE_PLUGIN_ROOT } else { "." }
$Helper = Join-Path $PluginRoot "bin/axhub-helpers.exe"
if (-not (Test-Path $Helper)) { $Helper = Join-Path $PluginRoot "bin/axhub-helpers" }
if (-not (Test-Path $Helper)) {
  $Cmd = Get-Command axhub-helpers.exe -ErrorAction SilentlyContinue
  if (-not $Cmd) { $Cmd = Get-Command axhub-helpers -ErrorAction SilentlyContinue }
  $Helper = if ($Cmd) { $Cmd.Source } else { "axhub-helpers" }
}
$PreflightJson = & $Helper preflight --json 2>$null
if (-not $PreflightJson) { $PreflightJson = "{}" }
$PreflightJson
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

Windows 는 특정 shell 을 단정하지 않아요. Git Bash/MSYS bash 만 있으면 bash snippet 을 쓰고, bash 가 없고 PowerShell 만 있으면 PowerShell snippet 을 써요. 두 경로 모두 `bin/axhub-helpers.exe` 와 PATH 의 `axhub-helpers.exe` 를 먼저 확인해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "로컬 프로젝트 구조와 후보 앱을 감지해요", status: "in_progress", activeForm: "후보 앱을 감지하는 중" },
     { content: "가져올 앱과 감지 신뢰도를 확인해요", status: "pending", activeForm: "감지 결과를 확인하는 중" },
     { content: "axhub.yaml 초안과 필수 env 안내를 준비해요", status: "pending", activeForm: "manifest 초안을 준비하는 중" },
     { content: "기존 approval/CLI 경로로 앱 등록·git 연결·배포를 실행해요", status: "pending", activeForm: "배포 경로를 실행하는 중" },
     { content: "선택형 auth 전환이 필요한지 확인해요", status: "pending", activeForm: "auth 전환 여부를 확인하는 중" },
     { content: "라이브 URL 과 다음 수정 포인트를 안내해요", status: "pending", activeForm: "결과를 정리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **로컬 light pre-scan.** 현재 디렉터리 또는 사용자가 지정한 경로를 helper 로 감지해요. helper 결과는 후보·힌트·env 이름만 다루고, secret 값은 절대 출력하지 않아요.

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"
   if [ ! -x "$HELPER" ] && [ -x "${HELPER}.exe" ]; then HELPER="${HELPER}.exe"; fi
   if [ ! -x "$HELPER" ]; then HELPER="$(command -v axhub-helpers 2>/dev/null || command -v axhub-helpers.exe 2>/dev/null || printf '%s\n' axhub-helpers)"; fi
   "$HELPER" migrate-plan --dir "${AXHUB_MIGRATE_DIR:-.}" --json
   ```

   Git Bash/MSYS bash 가 없는 Windows 에서는 PowerShell 로 같은 pre-scan 을 실행해요.

   ```powershell
   $PluginRoot = if ($env:CLAUDE_PLUGIN_ROOT) { $env:CLAUDE_PLUGIN_ROOT } else { "." }
   $Helper = Join-Path $PluginRoot "bin/axhub-helpers.exe"
   if (-not (Test-Path $Helper)) { $Helper = Join-Path $PluginRoot "bin/axhub-helpers" }
   if (-not (Test-Path $Helper)) {
     $Cmd = Get-Command axhub-helpers.exe -ErrorAction SilentlyContinue
     if (-not $Cmd) { $Cmd = Get-Command axhub-helpers -ErrorAction SilentlyContinue }
     $Helper = if ($Cmd) { $Cmd.Source } else { "axhub-helpers" }
   }
   $MigrateDir = if ($env:AXHUB_MIGRATE_DIR) { $env:AXHUB_MIGRATE_DIR } else { "." }
   & $Helper migrate-plan --dir $MigrateDir --json
   ```

2. **후보 선택과 confidence 확인.** 후보가 2개 이상이면 앱 하나를 고르게 해요. helper 의 confidence 와 `planning` field 를 같이 읽어요. 기본은 simple flow 유지예요. `0.80` 이상이고 hard-stop reason 이 없으면 기존 preview-first migrate 로 가요. confidence 가 낮거나 후보 모호함이 남아 있으면 serial `spec_only` planning 으로 승격해요. hard-stop reason 이 있거나 복잡 조건이 보이면 full `discover → planner → architect → critic → reviewer` consensus 로 승격해요.

   planning 이 필요한 경우 영속화 root 는 기본적으로 repo-local `.axhub/` 예요. 이때 `.axhub/spec` 은 앱별 승인 target-state planning state 이고, `.axhub/plan` 은 run별 stage ledger · approval · receipt 저장소예요. 상위 경로에 `.axhub-workspace` marker 가 있고 `shared_planning: true` 면 그 workspace root 의 `.axhub/` 로 확장해요. marker 가 malformed 하거나 unreadable 이면 parent marker 로 넘어가지 말고 repo-local 로 fail-closed 해요.

   다중 후보에서는 먼저 `APP_PATH` 를 고정한 뒤 planning persistence 를 실행해요. simple flow 에서는 wave 나 consensus jargon 을 앞단 UI 에 끌고 오지 않아요.

   ```bash
   "$HELPER" migrate-plan --dir "${AXHUB_MIGRATE_DIR:-.}" --app-path "$APP_PATH" --persist-planning --json
   ```

   ```powershell
   & $Helper migrate-plan --dir $MigrateDir --app-path $env:APP_PATH --persist-planning --json
   ```

   `spec_only` 는 항상 serial 이고 approval 전에는 `.axhub/spec/apps/<app_key>/latest.json` 을 갱신하지 않아요. full consensus 에서만 같은 app 안의 독립 unit 에 한해 conditional wave 병렬화를 허용해요. multi-app wave 는 v1 에서 금지예요. write target 충돌, dependency cycle, stage order 위반, app_key mismatch, independence proof 부족이 보이면 무조건 serial fallback 으로 내려가요.

   로컬 디렉터리는 앱 등록 전에 helper `migrate-plan` 과 manifest preview 를 실행해요. 원격 GitHub repo/ref/path 는 먼저 `axhub apps detect` read-only 결과를 보여줘요. 이 두 결과를 한 카드에 섞지 말고 `remote_deploy_detect` 와 `local_sdk_conversion_detect` 두 preview plane 으로 분리해요. `deploy_method=compose` 이면 compose file 이 start source 라서 `data.start` 누락을 차단하지 않아요. Dockerfile 이 선택되면 Dockerfile 이 start source 라서 추측 start command 를 만들지 않아요. auto/buildpack 계열에서 `data.start` 가 있으면 그 값을 그대로 manifest 후보에 반영하고, 없으면 사용자에게 start command 를 받거나 `manifest만` 으로 멈춰요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 질문 별 safe_default.

   ```json
   {
     "questions": [{
       "question": "어느 앱을 가져올까요?",
       "header": "앱 선택",
       "multiSelect": false,
       "options": [
         {"label": "첫 후보", "description": "가장 높은 confidence 후보 하나만 가져와요"},
         {"label": "직접 선택", "description": "후보 목록에서 다른 앱 경로를 고르게 해요"},
         {"label": "중단", "description": "후보가 모호하면 변경 없이 멈춰요"}
       ]
     }]
   }
   ```

   ```json
   {
     "questions": [{
       "question": "감지 계획으로 배포할까요?",
       "header": "계획 확인",
       "multiSelect": false,
       "options": [
         {"label": "계속", "description": "현재 감지 계획으로 preview 확인과 배포를 진행해요"},
         {"label": "manifest만", "description": "axhub.yaml 초안만 만들고 배포는 멈춰요"},
         {"label": "중단", "description": "파일과 원격 상태를 바꾸지 않아요"}
       ]
     }]
   }
   ```
2.5. **preview plane 분리 + 승인 액션 분리.** `remote_deploy_detect` 는 원격 저장소 기준 build/start/port/health/deploy_method/env preview 예요. `local_sdk_conversion_detect` 는 helper 의 `sdk_conversion` 후보 기준 언어별 wrapper target, dependency hint, data 후보, auth 후보, risk, hard-stop reason preview 예요. 이 둘은 이름과 표기 순서를 고정해서 같이 보여줘요.

   승인 액션은 아래처럼 분리해서 다뤄요. 하나의 “계속” 승인으로 로컬 patch 와 원격 mutation 을 같이 넘기지 않아요.
   - `sdk_wrapper_generate`: language SDK wrapper 생성 preview/실행
   - `data_patch_plan`: data 후보 파일 계획만 보여줘요
   - `auth_patch_plan`: auth 후보 파일 계획만 보여줘요
   - `auth_oauth_client_create`: OAuth client 생성 preview/실행
   - `app_create`: 앱 등록 preview/실행
   - `git_connect`: GitHub 연결 preview/실행
   - `env_set`: env 저장 preview/실행
   - `deploy_create`: 배포 시작 preview/실행

   hard-stop reason 이 하나라도 있으면 `sdk_wrapper_generate`, `data_patch_plan`, `auth_patch_plan` 은 preview-only 로 낮추고 local patch 실행을 막아요. 원격 mutation approval 도 helper 가 자동으로 묶지 말고 action 별로 따로 유지해요.

3. **manifest 초안 준비.** helper 의 `suggested_manifest` 를 `axhub.yaml` 초안으로 보여줘요. 후보를 직접 골랐으면 같은 helper 를 `--app-path "$APP_PATH"` 로 한 번 더 실행해서 선택한 후보 기준 manifest 를 다시 만들어요. 위 control contract 표에 있는 field 만 남기고, required env 는 선택한 후보 앱 안에서 발견된 이름과 scope 만 포함해요. 값 설정은 `axhub env set` 경로로 안내해요. 기존 `apphub.yaml` 이 있으면 읽기는 계속 되지만 새 파일은 `axhub.yaml` 로 만들어요.

   ```bash
   "$HELPER" migrate-plan --dir "${AXHUB_MIGRATE_DIR:-.}" --app-path "$APP_PATH" --json
   ```

   PowerShell 만 있는 Windows 에서도 같은 선택값을 넘겨요.

   ```powershell
   & $Helper migrate-plan --dir $MigrateDir --app-path $env:APP_PATH --json
   ```

   monorepo 후보를 선택했으면 선택한 `APP_PATH` 를 모든 preview 와 manifest 판단에 계속 묶어요. `APP_PATH` 가 `.` 이 아니면 아래 중 하나로 repo root 기준 path 를 명시할 수 있을 때만 mutation 으로 넘어가요.

   - Dockerfile 배포: `build.dockerfile: <APP_PATH>/Dockerfile`
   - Compose 배포: `build.deploy_method: compose` + `build.compose_file: <APP_PATH>/<compose-file>`
   - Buildpack/pinned command 배포: `build.strategy: pinned` + `build.install`/`build.build`/`build.start` 안에서 `cd <APP_PATH> && ...` 로 working directory 를 고정

   선택한 `APP_PATH` 를 backend 가 이해하는 위 field 로 고정할 수 없으면 앱 등록·git 연결·배포를 시작하지 않고 `manifest만` 으로 멈춰요. `source`, `app_path`, `priority` 같은 백엔드에 없는 field 를 만들어 넣으면 안 돼요.

4. **기존 mutation 경로 재사용.** 앱 등록, git 연결, env 값 저장, 배포는 위 CLI boundary contract 와 기존 approval 경로만 써요. helper 로 preview/approval 을 우회하지 않아요. `axhub apps git connect --execute` 는 GitHub 연결 승인 카드가 있어야만 실행해요. remote detect 는 v0.17.4 `axhub apps detect` read-only 로만 쓰고, mutation 은 preview+approval 뒤의 current CLI 명령으로만 진행해요.

5. **결과 검증.** 배포가 끝나면 live URL, deployment id, 감지된 build/runtime env 구분, Dockerfile/compose/auto 중 선택된 ladder 를 보여줘요. 실패하면 deploy error empathy catalog 형식으로 원인·확인 방법·재시도 명령을 짧게 안내해요.

6. **선택형 Auth Migration.** 사용자가 원할 때만 기존 앱의 로그인 흐름을 AxHub OAuth/OIDC 로 바꾸는 절차를 진행해요. 기본 migrate 는 배포 완료가 끝이에요. 이 단계는 사용자 앱 auth 파일 변경을 다루는 절차이고, AxHub backend/gateway 코드나 Data Gateway/DB query refactor 는 범위가 아니에요.

   먼저 아래 항목을 찾아서 변경 계획과 파일 목록을 보여줘요. 계획을 보여주기 전에는 기존 auth 코드를 조용히 삭제하거나 덮어쓰지 않아요.

   - login/logout/callback route
   - middleware/guard
   - `current_user`, `request.user`, context user 같은 현재 사용자 접근 경계
   - user model 과 session/cookie 설정
   - NextAuth/Auth.js, Passport, Clerk, Supabase, Devise, OmniAuth, Django auth, Flask-Login, Rails session, custom JWT/session 같은 auth library
   - OAuth/OIDC env 이름

   변경 계획에는 아래 내용을 포함해요.

   - 현재 auth 방식과 AxHub OAuth/OIDC target 방식
   - 바꿀 후보 파일 목록
   - 필요한 callback URL
   - 필요한 env 이름: `AXHUB_OAUTH_CLIENT_ID`, `AXHUB_OAUTH_CLIENT_SECRET`, `AXHUB_OAUTH_ISSUER`, `AXHUB_OAUTH_REDIRECT_URI`
   - rollback 방법
   - 실행할 로컬/e2e 검증 명령

   OAuth client 는 raw backend endpoint 를 curl 하지 않고 axhub CLI 로만 만들어요. 실행 전 callback URL, scopes, grant types 를 preview 로 보여주고 명시 확인을 받아요. 그 뒤 아래 CLI 명령을 한 번만 실행해요.

   ```bash
   axhub auth oauth client create \
     --app "$APP_ID" \
     --name web \
     --type confidential \
     --redirect-uri "$CALLBACK_URL" \
     --scope openid \
     --scope profile \
     --scope email \
     --grant-type authorization_code \
     --execute \
     --json
   ```

   `client_secret` 은 한 번만 표시되는 secret 이에요. `axhub.yaml`, 로그, 질문 옵션, TodoWrite, PR 본문에 남기지 않아요. 앱 env 로 저장해야 하면 값을 stdout 에 다시 출력하지 말고 stdin 으로만 넘겨요.

   ```bash
   printf '%s' "$AXHUB_OAUTH_CLIENT_SECRET" | axhub env set --app "$APP_ID" AXHUB_OAUTH_CLIENT_SECRET --secret --from-stdin --stage "$STAGE" --json
   ```

   Auth 변경 후에는 최소한 login redirect, callback 처리, logout, 보호 route 접근 제어, 기존 fallback/rollback 가능성, secret 미커밋 여부를 확인해요.

## NEVER

- NEVER secret 값을 `axhub.yaml`, 로그, 질문 옵션, TodoWrite 에 넣으면 안 돼요.
- NEVER low confidence 나 다중 후보를 조용히 배포하지 않아요.
- NEVER 앱 등록·git 연결·배포 approval 을 helper 로 우회하지 않아요.
- NEVER raw backend endpoint 를 curl 하거나 추측한 HTTP payload 로 detect/mutation 을 실행하지 않아요.
- NEVER 새 배포 경로를 만들면 안 돼요. 기존 CLI 경로를 재사용해요.
- NEVER `axhub.yaml` 에 백엔드 parser 가 모르는 field, secret value, 설명용 metadata 를 넣지 않아요.
- NEVER AxHub backend/gateway 코드를 auth migration 단계에서 수정하지 않아요.
- NEVER Data Gateway/DB query refactor 를 auth migration 단계에 섞지 않아요.
- NEVER password hash, session secret, OAuth client secret 을 manifest/log/TodoWrite/PR 본문에 출력하지 않아요.
- NEVER 기존 auth 코드를 조용히 삭제하지 않아요. 먼저 계획과 변경 파일 목록을 보여줘요.
- NEVER raw backend endpoint 를 curl 해서 OAuth client 를 만들지 않아요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
