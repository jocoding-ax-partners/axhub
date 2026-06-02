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

## Workflow

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

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
   HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"
   "$HELPER" migrate-plan --dir "${AXHUB_MIGRATE_DIR:-.}" --json
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

3. **manifest 초안 준비.** helper 의 `suggested_manifest` 를 `axhub.yaml` 초안으로 보여줘요. required env 는 이름과 scope 만 포함하고, 값 설정은 `axhub env set` 경로로 안내해요. 기존 `apphub.yaml` 이 있으면 읽기는 계속 되지만 새 파일은 `axhub.yaml` 로 만들어요.

4. **기존 mutation 경로 재사용.** 앱 등록, git 연결, env 값 저장, 배포는 위 CLI boundary contract 와 기존 consent 경로만 써요. helper 로 consent 를 우회하지 않아요. remote detect 는 `axhub apps detect --help` 가 성공할 때만 CLI 로 실행하고, local dir 은 GitHub repo/ref/path 로 push 된 뒤에만 remote preview 대상이 돼요. CLI 가 없으면 raw backend 호출 없이 manifest 초안과 다음 작업만 안내해요.

5. **결과 검증.** 배포가 끝나면 live URL, deployment id, 감지된 build/runtime env 구분, Dockerfile/compose/auto 중 선택된 ladder 를 보여줘요. 실패하면 deploy error empathy catalog 형식으로 원인·확인 방법·재시도 명령을 짧게 안내해요.

## NEVER

- NEVER secret 값을 `axhub.yaml`, 로그, 질문 옵션, TodoWrite 에 넣으면 안 돼요.
- NEVER low confidence 나 다중 후보를 조용히 배포하지 않아요.
- NEVER 앱 등록·git 연결·배포 consent 를 helper 로 우회하지 않아요.
- NEVER raw backend endpoint 를 curl 하거나 추측한 HTTP payload 로 detect/mutation 을 실행하지 않아요.
- NEVER 새 배포 경로를 만들면 안 돼요. 기존 CLI 경로를 재사용해요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
