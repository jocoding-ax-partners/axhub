---
name: init
description: '이 스킬은 사용자가 "새 앱 만들어줘", "앱 만들어줘", "프로젝트 만들어줘"처럼 빈 디렉토리에서 새 axhub 템플릿 앱을 만들고 싶을 때 axhub 템플릿 앱 생성을 담당해요. 비어 있지 않은 기존 로컬 앱을 axhub에 가져오거나 첫 연결·첫 배포까지 올리려는 요청은 import 스킬이 담당해요. 내부 작동 라벨을 말하지 말고 바로 템플릿 확인으로 시작하고, 일반 앱 브레인스토밍이나 임의 스택 질문으로 우회하지 말고 axhub template 선택 → 앱 이름 → 실행 승인 순서로 진행해요. 활성화 예: "새 앱 만들어줘", "앱 만들어줘", "결제 앱 만들어", "프로젝트 만들어", "프로젝트 초기화해줘", "초기화해줘", "fastapi 앱", "Next.js 앱 만들어줘", "init", "scaffold", 또는 빈 디렉토리에서 새 앱 시작 의도. axhub apps bootstrap saga 로 backend app + GitHub repo + 첫 deploy 를 한 번에 진행하고 repo_full_name 으로 현재 dir 에 git clone 해요.'
examples:
  - utterance: "새 앱 만들어줘"
    intent: "scaffold new axhub app"
  - utterance: "결제 앱 만들어"
    intent: "scaffold new axhub app"
  - utterance: "결제 앱 만들어줘"
    intent: "scaffold new axhub app"
  - utterance: "프로젝트 초기화해줘"
    intent: "scaffold new axhub app"
  - utterance: "init"
    intent: "scaffold new axhub app"
  - utterance: "scaffold"
    intent: "scaffold new axhub app"
  - utterance: "빈 디렉토리"
    intent: "scaffold new axhub app"
  - utterance: "그걸로 앱 만들고 싶어"
    intent: "scaffold new axhub app"
  - utterance: "이제 앱 만들어줘"
    intent: "scaffold new axhub app"
  - utterance: "그거로 앱 만들어줘"
    intent: "scaffold new axhub app"
allows-dependency-execution: true
model: sonnet
---

# Init

새 앱을 만들 수 있는 템플릿을 확인할게요.

빈 디렉토리에서 새 axhub 템플릿 앱을 만드는 스킬이에요. 비어 있지 않은 기존 앱을 axhub에 가져오거나 이미 있는 로컬 앱을 첫 연결·첫 배포까지 올리려는 요청은 `import` 스킬로 보내요.

이 스킬의 creation path 는 하나뿐이에요: backend `axhub apps bootstrap` saga. saga 가 backend app, GitHub repo, 첫 deploy 를 server-side 에서 처리하고, 성공 후 `repo_full_name` 으로 현재 디렉토리에 코드를 받아요. `axhub init`, `axhub apps create`, `axhub deploy create` 로 우회하지 않아요.

## 대표 여정에서의 역할

온보딩이 준비 상태가 된 뒤, `init` 은 새 앱 생성, GitHub repo 준비, 첫 배포 결과 안내를 맡아요.

## 기존 앱 가져오기와 분리

`init` 은 빈 디렉토리 새 앱 생성만 담당해요. 비어 있지 않은 폴더, 이미 만든 로컬 앱, "이 앱 올려" 요청은 `import` 스킬로 보내요.

## 같은 대화 맥락 이어받기

이미 본 것만 이어받아요. infer-tables-env 분석은 scaffold 코드뿐 아니라 같은 대화에서 실제로 조회한 리소스 근거까지 함께 봐요. 근거가 없으면 리소스를 지어내지 않아요, carry-over 를 주장하지 않아요. install-link 를 보여줬으면 재안내는 생략할 수 있지만 0-install gate 는 맥락과 무관하게 그대로 실행해요.

## Load These References

- `references/resume-and-tenant.md`: Step 0.5 resume state, device-flow resume recovery, tenant resolve/picker, fence 간 cache 재조회가 필요할 때 읽어요.
- `references/templates-and-github.md`: template registry 표시, template/app-name 선택, GitHub App installation/account gate, multi-owner picker, non-interactive defaults 가 필요할 때 읽어요.
- `references/bootstrap-and-local.md`: bootstrap dry-run/execute/watch, GitHub device-code event 처리, repo clone/current-dir safety, manifest slug correction, scaffold dependency/local preview 가 필요할 때 읽어요.
- `references/errors-and-followups.md`: long error routing, result card, optional MCP/setup/follow-up guidance, carry-over wording 이 필요할 때 읽어요.
- `../deploy/references/session-carryover.md`: 같은 대화의 조회·온보딩 근거를 이어받을 때만 읽어요. 근거가 없으면 리소스·테이블·앱 요구사항을 지어내지 않아요.
- `../deploy/references/error-empathy-catalog.md`: exit-code 안내를 사람 말투로 바꿀 때 읽어요.

Top-level 은 실행 순서와 안전 anchor 만 유지해요. 위 reference 는 필요한 단계에서만 열고, CLI command semantics 는 여기 적힌 core command shape 를 바꾸지 않아요.

## Visibility

사용자는 대부분 개발 지식이 없어요. CLI JSON 의 raw primitive 는 변수로만 다루고 chat 에 echo 하지 않아요.

- Echo 금지: `schema_version`, template `id`, `folder_name`, `resource_tier`, `bootstrap_id`, `status_url`, `stage`, `app_id`, `deployment_id`, `error_code`, `error_message`, `request_id`, `idempotency_key`, `installation_id`, `device_code`.
- 예외: GitHub device-flow event 가 나오면 `verification_uri` 또는 `verification_uri_complete`, `user_code`, 대략적인 만료 시간은 즉시 humanize 해서 보여줘요.
- `repo_full_name` 은 clone/manual remote 안내에 필요한 경우에만 보여줘요.
- `AXHUB_INIT_VERBOSE=1` 이 켜진 디버깅 환경 외에는 raw JSON/stderr 를 chat 에 dump 하지 않아요.

각 단계는 한 줄로만 진행 상황을 알려요: `[1/7] axhub 점검하는 중이에요`, `[2/7] 작업공간 확인하는 중이에요`, `[3/7] 템플릿 고르는 중이에요`, `[4/7] 앱 이름 정하는 중이에요`, `[5/7] 미리보기 만드는 중이에요`, `[6/7] 앱 만드는 중이에요`, `[7/7] 코드 받아서 정리하는 중이에요`.

## Workflow

실제 순서:

1. CLI guard: `axhub` 존재와 `axhub plugin-support preflight --json` 동작 확인.
2. Resume/tenant: pending `.axhub/init-resume.json` 이 있으면 먼저 이어서 할지 묻고, tenant 를 확정해요.
3. Template registry: `axhub apps templates list --tenant "$AXHUB_TENANT" --json`.
4. GitHub App gate: `axhub github accounts list --json` 로 install_url 표시, installed account 확인, owner 확정.
5. Template + app name: backend registry 에 있는 값만 고르고, 앱 이름이 없으면 물어요.
6. Dry-run preview: `axhub apps bootstrap ... --dry-run --json`.
7. Execute saga: 사용자 확인 후 `axhub apps bootstrap ... --execute --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json`.
8. Clone/current dir: completed saga 에서 `repo_full_name` 을 읽고 현재 dir 에 remote fetch/reset 해요.
9. Manifest/local: `axhub.yaml` slug 를 새 앱 slug 로 보정·검증하고, 선택 시 로컬 미리보기를 준비해요.
10. Result/follow-up: 확인된 `access_url` 이 있으면 보여주고, 없으면 배포 진행 중으로 낮춰 말해요.

Slash command, skill name, route label 은 사용자에게 말하지 않아요. Desktop 사용자에게는 `다시 로그인해줘`, `설치했어`, `다시 만들어줘`, `방금 배포 어디까지 됐어?` 같은 자연어를 안내해요.

## Scope Boundary

`init` 은 빈 디렉토리에서 새 app 을 만드는 bootstrap 전용이에요.

- 현재 폴더가 비어 있지 않고 사용자가 "이 앱 올려", "이 폴더 axhub에 올려", "기존 앱 가져와"처럼 말하면 `import` 스킬로 넘겨요.
- 일반 앱 아이디어를 새로 브레인스토밍하지 않아요. template 은 backend registry 에서 고르고, 데이터 접근은 생성된 template 의 `@ax-hub/sdk` 안내나 실제 조회 근거가 있을 때만 이어받아요.
- 같은 대화에서 실제 조회 결과나 onboarding Ready card 가 있으면 그 근거만 반영해요. 근거가 없으면 콜드 start 로 진행해요.

## Core Commands

### 1. CLI Guard

처음에는 CLI 존재와 최소 plugin-support surface 만 확인해요. 버전 숫자를 직접 비교하지 않아요.

```bash
if ! command -v axhub >/dev/null 2>&1; then
  echo "axhub CLI가 아직 없네요. 온보딩부터 진행할게요." >&2
  exit 0
fi
PREFLIGHT_JSON=$(axhub plugin-support preflight --json 2>/dev/null)
PREFLIGHT_EXIT=$?
if [ "$PREFLIGHT_EXIT" = "2" ] || [ -z "$PREFLIGHT_JSON" ]; then
  echo "axhub CLI가 오래됐어요. `axhub update apply`로 업데이트한 뒤 다시 시도해 주세요." >&2
  exit 0
fi
echo "$PREFLIGHT_JSON"
```

세 갈래예요: CLI 없음이면 onboarding 안내 후 stop, `plugin-support` unknown/빈 출력이면 update 안내 후 stop, 정상 JSON 이면 `auth_ok` 등을 읽고 계속해요. raw stderr 는 보여주지 않아요.

preflight 통과 후 update check 는 best-effort 10분 TTL 로만 해요. 실패·캐시 hit·구 CLI 는 조용히 건너뛰고 앱 생성을 막지 않아요.

```bash
STAMP="${TMPDIR:-/tmp}/axhub-update-check.stamp"
if [ -z "$(find "$STAMP" -mmin -10 2>/dev/null)" ]; then
  : > "$STAMP"
  PLUGIN_VER=$(grep -o '"version"[^,]*' "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json" 2>/dev/null | head -1 | sed -E 's/.*"version"[^"]*"([^"]+)".*/\1/')
  UPD=$(axhub update check ${PLUGIN_VER:+--plugin-version "$PLUGIN_VER"} --json 2>/dev/null)
fi
```

### 2. Resume And Tenant

CLI guard 통과 뒤에는 템플릿 목록보다 먼저 resume route 를 확인해요.

```bash
axhub plugin-support init-resume route --json
```

`watch_status` 또는 `resume_last` 이고 `clone_done=false` 면 이어서 할지 물어요. 비대화형/D1 guard 는 safe default `새로 시작` 이에요. 세부 resume/device-flow recovery 와 tenant picker 는 `references/resume-and-tenant.md` 를 읽어요.

tenant 는 cache-first resolver 로 확정해요. fence 간 env 는 휘발하므로 새 fence 에서 다시 읽어요.

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
export AXHUB_TENANT
```

`AXHUB_TENANT` 가 끝까지 비면 preflight `auth_ok` 와 `current_team_id` 를 확인하고 `다시 로그인해줘` 로 안내해요.

### 3. Template Registry

Backend registry 가 source of truth 예요.

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
axhub apps templates list --tenant "$AXHUB_TENANT" --json
```

사용자에게는 backend 가 반환한 template 전체 목록을 사람이 읽을 수 있게 보여줘요. 선택 값은 반드시 반환된 `id`, `folder_name`, 또는 built-in alias (`react`, `nextjs`, `astro`) 중 하나예요. registry 설명과 AskUserQuestion shape 는 `references/templates-and-github.md` 를 읽어요.

### 4. GitHub App Gate

Template 목록이 정상으로 오면 GitHub App 계정 상태를 확인해요.

```bash
axhub github accounts list --json
```

`install_url` 이 있으면 설치 여부와 무관하게 한 줄로 보여줘요. 설치된 계정이 0개로 확인되면 설치가 확인될 때까지 Step 5 이후로 진행하지 않아요. 설치된 계정이 1개면 자동 owner 로 쓰고, 2개 이상이면 owner 를 고르게 해요. subprocess/no TTY 에서는 `AXHUB_GITHUB_OWNER` 가 있을 때만 진행하고, 없으면 safe default `취소` 로 bootstrap 을 시작하지 않아요.

확인할 수 없는 상태(빈 출력, JSON parse 불가)는 막지 않고 진행해요. auth 에러는 `다시 로그인해줘` 로 안내해요. 자세한 gate loop 는 `references/templates-and-github.md` 를 읽어요.

### 5. Template And App Name

이미 발화에 exact alias/folder/name 이 있고 registry 와 맞으면 질문 없이 써요. 맞지 않으면 registry 목록을 다시 보여주고 다시 물어요.

비대화형/D1 guard 에서는 template 과 앱 이름을 임의로 고르지 않아요. safe default 는 `abort` 또는 `취소` 예요.

앱 이름이 발화에서 유추되면 그대로 쓰고, 없으면 한 번 물어요. `--slug` 는 이름을 기반으로 자동 유도하되 backend 정책 충돌은 saga error 를 보고 한 번 더 받아요.

### 6. Dry-Run Preview

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --dry-run --json
```

Dry-run envelope 에서 template, slug, subdomain, repo name, private/public 같은 preview 만 한국어로 보여줘요. raw JSON dump 금지. 확인을 받기 전에는 execute 를 호출하지 않아요.

### 7. Execute Bootstrap Saga

사용자가 진행을 확인하면 resume state 를 먼저 저장하고 saga 를 실행해요.

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --json
axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --execute --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
```

Bash/tool timeout 은 9.5분 이상으로 잡아요. CLI timeout 뒤 resume hint 가 있으면 terminal status 를 한 번 더 확인해요.

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
BOOTSTRAP_ID=$(axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --execute --idempotency-key "$IDEMPOTENCY_KEY" --field-expr '.data.bootstrap_id // empty' 2>/dev/null || true)
axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --bootstrap-id "$BOOTSTRAP_ID" --json
axhub apps bootstrap-status "$BOOTSTRAP_ID" --tenant "$AXHUB_TENANT" --watch --watch-timeout 9m --json
```

`device_code_issued` event 가 나오면 즉시 사용자가 승인할 수 있게 URL/code 를 보여주고, outstanding code 가 있는 동안 `--resume-last` 없이 fresh `bootstrap --execute` 를 다시 호출하지 않아요. 자세한 device-flow handling 은 `references/bootstrap-and-local.md` 를 읽어요.

### 8. Clone Into Current Directory

완료된 saga 에서 repo 를 읽고 현재 dir 을 채워요. 서브디렉토리를 만들지 않아요.

```bash
REPO=$(axhub apps bootstrap-status "$BOOTSTRAP_ID" --tenant "$AXHUB_TENANT" --field-expr '.data.status.repo_full_name // empty' 2>/dev/null || true)
if [ -z "$REPO" ]; then
  echo '{"systemMessage":"GitHub repo 정보가 응답에 없어요. 설치 상태 진단해줘라고 말하면 이어서 점검할 수 있어요."}'
  exit 65
fi
if [ -d .git ]; then
  echo "{\"systemMessage\":\"현재 dir 에 이미 .git 이 있어요. 자동 clone 건너뛸게요. 수동으로 origin 을 붙이려면: git remote add origin https://github.com/${REPO}.git && git fetch origin && git checkout -b main origin/main\"}"
else
  git init -q -b main
  git remote add origin "https://github.com/${REPO}.git"
  git fetch origin --quiet --depth=1
  DEFAULT_BRANCH=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo main)
  git reset --hard "origin/$DEFAULT_BRANCH"
  git branch --set-upstream-to="origin/$DEFAULT_BRANCH" "$DEFAULT_BRANCH" 2>/dev/null || true
fi
```

이미 `.git` 이 있으면 안전을 위해 자동 clone 을 건너뛰고 수동 remote 안내만 해요. clone 실패 시 임의 URL 을 합성하지 않아요.

### 9. Manifest Slug Correction And Local Preview

Clone 이 성공하면 먼저 `axhub.yaml` 의 앱 slug 를 `$APP_SLUG` 와 맞춰요. 템플릿 기본 slug 가 남으면 나중에 deploy resolve 가 다른 앱을 가리킬 수 있어요.

```bash
axhub manifest --json
axhub manifest validate --file axhub.yaml --json
axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --bootstrap-id "$BOOTSTRAP_ID" --repo-full-name "$REPO" --clone-done true --json
axhub plugin-support init-resume clear --json
axhub plugin-support scaffold-detect --json
```

`scaffold-detect` 가 package/lockfile/dev script/node 를 확인해요. 실행 가능하고 사용자가 원할 때만 local preview 를 시작해요. 내부 install 은 lockfile 이 있을 때만, 항상 `--ignore-scripts` 를 붙여요.

```bash
axhub plugin-support scaffold-dev start --json
```

Subprocess/no TTY 의 local preview safe default 는 `아니요` 예요. 자세한 dependency/local preview guard 는 `references/bootstrap-and-local.md` 를 읽어요.

### 10. Result

공개 URL 은 절대 합성하지 않아요. 배포 성공 후 앱의 `access_url` 을 읽어서 확인된 값만 보여줘요.

```bash
PUBLIC_URL="$(axhub apps get "$APP_ID" --no-input --field-expr '.access_url // .data.access_url // empty' 2>/dev/null || true)"
```

URL 이 있으면 첫 줄에 보여줘요. 없으면 "인터넷 배포가 시작됐어요. '방금 배포 어디까지 됐어?' 라고 물으면 이어서 확인할게요." 로 낮춰 말해요. 자세한 result card, optional MCP/setup, infer-tables-env follow-up 은 `references/errors-and-followups.md` 를 읽어요.

## Non-Interactive Defaults

Subprocess, CI, `CLAUDE_NON_INTERACTIVE`, no TTY 에서는 사람 선택을 임의로 대신하지 않아요.

- Resume offer: `새로 시작`
- Tenant picker: resolver 가 준 첫 후보 fallback 또는 로그인 안내
- GitHub owner: `AXHUB_GITHUB_OWNER` 가 있으면 사용, 없으면 `취소`
- Template 선택: `abort`
- App name: `abort`
- Bootstrap execute 확인: `취소`
- Auto-connect/local preview: `아니요`
- GitHub App install browser gate: `취소` 후 install_url 과 `다시 만들어줘` 재개 phrase 안내

## NEVER

- NEVER GitHub App 이 아무 계정에도 설치 안 된 상태에서 Step 5 이후로 진행하거나 bootstrap dry-run/execute 를 호출하지 않아요.
- NEVER `axhub init` 또는 `axhub init --from-template` 을 호출하지 않아요. 이 SKILL 은 `axhub apps bootstrap` saga 만 써요.
- NEVER `axhub apps create` 를 직접 호출하지 않아요. bootstrap saga 가 server-side 에서 app 생성을 처리해요.
- NEVER `axhub deploy create` 를 직접 호출하지 않아요. bootstrap saga 가 첫 deploy 까지 포함해요.
- NEVER remote `templates.json` 또는 폐기된 fetch-template 을 source 로 쓰지 않아요. backend `axhub apps templates list` 만 source of truth 예요.
- NEVER subprocess/headless 에서 template 또는 앱 이름을 임의로 고르지 않아요. safe default 는 `abort` 또는 `취소` 예요.
- NEVER `--execute` 를 `--dry-run` 미리보기와 사용자 확인 없이 호출하지 않아요.
- NEVER auth 만료를 template 조회 실패로 오해하지 않아요. CLI auth 실패(exit 4 또는 `auth`)는 `다시 로그인해줘` 로 안내해요.
- NEVER `device_code_issued` event 가 나왔는데 `verification_uri` + `user_code` 를 즉시 보여주지 않고 silent polling 만 하지 않아요.
- NEVER `repo_full_name` 이 비어 있는데 임의 URL 을 만들어 clone 시도하지 않아요.
- NEVER shell 에서 CLI 버전 숫자를 직접 파싱·비교하지 않아요. gate 는 `axhub plugin-support preflight --json` 동작 여부예요.
