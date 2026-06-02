---
name: migrate
description: 이 스킬은 기존 앱 올려줘, 만든 앱 가져오기, migrate, import existing app, 기존 프로젝트 배포 의도에서 활성화해요.
examples:
  - utterance: "기존 앱 올려줘"
    intent: "migrate an existing app into axhub"
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
# 원격 GitHub repo 감지 preview. 읽기 전용이고 mutation 이 없어요.
axhub apps detect --repo "$OWNER_REPO" --json

# monorepo 나 특정 ref 감지 preview. --repo 대신 split form 도 가능해요.
axhub apps detect --owner "$OWNER" --repo-name "$REPO" --ref "$REF" --path "$APP_PATH" --json

# 앱 등록
axhub apps create --from-file axhub.yaml --yes --json

# GitHub 연결 dry-run 검증
axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --json

# GitHub 연결 mutation
axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json

# env 값 저장. 값은 stdin 으로만 넣어요.
printf '%s' "$VALUE" | axhub env set --app "$APP_ID" "$KEY" --secret --from-stdin --stage "$STAGE" --json

# 배포 시작. deploy skill 과 같은 consent-mint / status-first gate 뒤에만 실행해요.
axhub deploy create --app "$APP_ID" --branch "$BRANCH" --commit "$COMMIT_SHA" --json
```

`axhub apps detect` 계약은 아래처럼 고정해요.

- 입력은 GitHub repo source 만 받아요. `--repo owner/name` 또는 `--owner OWNER --repo-name NAME` 중 하나를 쓰고, monorepo 는 `--path`, 비기본 브랜치는 `--ref` 를 추가해요. local directory 는 helper `migrate-plan` 로만 pre-scan 해요.
- 성공 JSON 은 `data.detected_providers`, `data.framework`, `data.install`, `data.build`, `data.start`, `data.port`, `data.health_path`, `data.deploy_method`, `data.confidence`, `data.env_refs[]` 를 읽어요. unknown future field 는 무시해요.
- exit `0` 은 preview 성공이에요. exit `64` 는 owner/name 형식·필수 플래그 같은 CLI validation 실패예요. exit `4` 는 로그인 필요, `5` 는 repo/ref/path not found, `6` 은 rate limit, `7` 은 backend/API/detect 실패, `8` 은 tenant 권한 문제, `10` 은 timeout, `1` 은 local generic 실패로 다뤄요.
- non-zero detect 는 앱 등록·git 연결·배포를 시작하지 않고, error envelope 의 request_id/subcode/doc_url 이 있으면 그대로 보여줘요. retry 는 `6`, `10`, 또는 envelope 의 `retryable:true` 에서만 해요.

remote detect CLI 는 `axhub apps detect --help` 가 성공할 때만 써요. 없으면 backend detect endpoint 를 직접 curl 하지 않아요.

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

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

Windows 는 특정 shell 을 단정하지 않아요. Git Bash/MSYS bash 만 있으면 bash snippet 을 쓰고, bash 가 없고 PowerShell 만 있으면 PowerShell snippet 을 써요. 두 경로 모두 `bin/axhub-helpers.exe` 와 PATH 의 `axhub-helpers.exe` 를 먼저 확인해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "로컬 프로젝트 구조와 후보 앱을 감지해요", status: "in_progress", activeForm: "후보 앱을 감지하는 중" },
     { content: "가져올 앱과 감지 신뢰도를 확인해요", status: "pending", activeForm: "감지 결과를 확인하는 중" },
     { content: "axhub.yaml 초안과 필수 env 안내를 준비해요", status: "pending", activeForm: "manifest 초안을 준비하는 중" },
     { content: "기존 consent/CLI 경로로 앱 등록·git 연결·배포를 실행해요", status: "pending", activeForm: "배포 경로를 실행하는 중" },
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

2. **후보 선택과 confidence 확인.** 후보가 2개 이상이면 앱 하나를 고르게 해요. helper 의 confidence 는 local marker 기반 힌트예요. `0.80` 이상이면 확인 후 진행하고, `0.60..0.79` 는 수정 가능한 계획으로 보여줘요. `0.60` 미만이나 후보 모호함은 진행을 막고 `axhub.yaml` 또는 Dockerfile/compose 를 요청해요.

   remote detect CLI 가 있으면 앱 등록 전에 같은 repo/ref/path 로 preview 를 한 번 실행해요. `deploy_method=compose` 이면 compose file 이 start source 라서 `data.start` 누락을 차단하지 않아요. Dockerfile 이 선택되면 Dockerfile 이 start source 라서 추측 start command 를 만들지 않아요. auto/buildpack 계열에서 `data.start` 가 있으면 그 값을 그대로 manifest 후보에 반영하고, 없으면 사용자에게 start command 를 받거나 `manifest만` 으로 멈춰요.

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
         {"label": "계속", "description": "현재 감지 계획으로 consent 발급과 배포를 진행해요"},
         {"label": "manifest만", "description": "axhub.yaml 초안만 만들고 배포는 멈춰요"},
         {"label": "중단", "description": "파일과 원격 상태를 바꾸지 않아요"}
       ]
     }]
   }
   ```

3. **manifest 초안 준비.** helper 의 `suggested_manifest` 를 `axhub.yaml` 초안으로 보여줘요. 위 control contract 표에 있는 field 만 남기고, required env 는 이름과 scope 만 포함해요. 값 설정은 `axhub env set` 경로로 안내해요. 기존 `apphub.yaml` 이 있으면 읽기는 계속 되지만 새 파일은 `axhub.yaml` 로 만들어요.

4. **기존 mutation 경로 재사용.** 앱 등록, git 연결, env 값 저장, 배포는 위 CLI boundary contract 와 기존 consent 경로만 써요. helper 로 consent 를 우회하지 않아요. remote detect 는 `axhub apps detect --help` 가 성공할 때만 CLI 로 실행하고, local dir 은 GitHub repo/ref/path 로 push 된 뒤에만 remote preview 대상이 돼요. CLI 가 없으면 raw backend 호출 없이 manifest 초안과 다음 작업만 안내해요.

5. **결과 검증.** 배포가 끝나면 live URL, deployment id, 감지된 build/runtime env 구분, Dockerfile/compose/auto 중 선택된 ladder 를 보여줘요. 실패하면 deploy error empathy catalog 형식으로 원인·확인 방법·재시도 명령을 짧게 안내해요.

## NEVER

- NEVER secret 값을 `axhub.yaml`, 로그, 질문 옵션, TodoWrite 에 넣으면 안 돼요.
- NEVER low confidence 나 다중 후보를 조용히 배포하지 않아요.
- NEVER 앱 등록·git 연결·배포 consent 를 helper 로 우회하지 않아요.
- NEVER raw backend endpoint 를 curl 하거나 추측한 HTTP payload 로 detect/mutation 을 실행하지 않아요.
- NEVER 새 배포 경로를 만들면 안 돼요. 기존 CLI 경로를 재사용해요.
- NEVER `axhub.yaml` 에 백엔드 parser 가 모르는 field, secret value, 설명용 metadata 를 넣지 않아요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
