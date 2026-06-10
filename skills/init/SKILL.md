---
name: init
description: '이 스킬은 Claude Desktop에서 사용자가 "새 앱 만들어줘", "앱 만들어줘", "프로젝트 만들어줘"처럼 말할 때 axhub 템플릿 앱 생성을 담당해요. 사용자에게는 내부 작동 라벨을 말하지 말고 바로 템플릿 확인으로 시작해요. 일반 앱 브레인스토밍이나 임의 스택 질문으로 우회하지 말고, axhub template 선택 → 앱 이름 → 실행 승인 순서로 진행해요. 다음 표현에서 활성화: "새 앱 만들어줘", "새 앱 만들어", "앱 만들어줘", "결제 앱 만들어", "결제 앱 만들어줘", "빈 디렉토리", "프로젝트 만들어", "프로젝트 만들어줘", "프로젝트 초기화", "프로젝트 초기화 해줘", "프로젝트 초기화해줘", "초기화 해줘", "초기화해줘", "apphub.yaml 만들어", "apphub.yaml 만들어줘", "axhub.yaml 만들어", "axhub.yaml 만들어줘", "fastapi 앱", "FastAPI 앱 만들어줘", "next.js 앱", "Next.js 앱 만들어줘", "nextjs 앱", "init", "scaffold", 또는 빈 디렉토리에서 새 앱 시작 의도. `axhub apps bootstrap` saga 로 backend app + GitHub repo + 첫 deploy 를 한 번에 진행하고 `repo_full_name` 으로 현재 dir 에 git clone 해요.'
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
multi-step: true
needs-preflight: false
allows-dependency-execution: true
model: sonnet
---

# Init

새 axhub 앱을 `axhub apps bootstrap` saga 로 한 번에 만들어요. backend app 생성 + GitHub repo 생성 + 첫 deploy 를 server-side 에서 처리하고, saga 응답의 `repo_full_name` 으로 현재 dir 에 git clone 해서 local + remote 둘 다 채워줘요. 기존 `axhub init` 호출은 Rust v1.0.0-rc.1 에서 `--from-template` 미구현 stub 라 SKILL 에서 호출하지 않아요.

## Vibe Coder Visibility Rules

이 SKILL 을 쓰는 사람은 대부분 개발 지식이 없어요. CLI 와 helper 가 돌려주는 다음 field 는 **internal verification primitives** 예요. SKILL 안에서는 이 field 들을 변수에 담아 helper 와 주고받되, **raw 값을 사용자 chat 에 echo 하면 안 돼요** (deploy SKILL 의 동일 룰을 따라요):

- `schema_version` (예: `bootstrap/v1`) — API 응답 검증용. raw 값 echo 금지.
- `items[].id`, `items[].folder_name`, `items[].name`, `items[].resource_tier` — `axhub apps templates list` 가 반환한 backend template registry. id 는 사용자 발화 매칭에만 쓰고, raw 목록 dump 금지.
- `bootstrap_id`, `status_url`, `stage`, `app_id`, `deployment_id`, `repo_full_name`, `error_code`, `error_message` — bootstrap saga 의 internal verification primitives. raw 값 echo 금지 (단 `repo_full_name` 은 마지막 단계에서 `git clone` URL 로 사용자에게 보여줘요).
- `request_id`, `idempotency_key`, `installation_id`, `device_code` — internal correlation/auth primitives. raw 값 echo 금지. **예외**: CLI GitHub device-flow 준비 단계에서 `event: device_code_issued` 를 emit 하면 `verification_uri` (또는 `verification_uri_complete`) + `user_code` 쌍은 humanize 해서 사용자에게 즉시 보여줘요. 대화형 TTY 에서는 승인까지 polling 하고, 에이전트/비-TTY 에서는 event 직후 fast-exit 하므로 사용자는 안내를 못 보면 멈춘 줄 알아요. internal `device_code` 값 자체는 여전히 echo 금지.

대신 사용자에게는 한국어 한 줄로 진행 상황만 알려드려요. 예시:

| 시점 | 사용자 chat 한 줄 |
|------|-------------------|
| Step 1 CLI 존재 확인 | "axhub 도구가 있는지 보고 있어요." |
| Step 2 template 목록 조회 | "사용할 수 있는 템플릿을 확인하고 있어요." |
| Step 2.5 GitHub App 설치 확인 | "GitHub App 이 설치돼 있는지 봐요. 안 돼 있으면 설치 링크를 주고 기다려요." |
| Step 3 template 선택 | "어떤 종류 프로젝트를 만들지 골라요." |
| Step 4 앱 이름 입력 | "앱 이름을 정해요." |
| Step 5 bootstrap dry-run | "어떤 작업을 할지 미리 보여줘요." |
| Step 6 실행 확인 | "지금 만들고 배포까지 한 번에 진행해요." |
| Step 7 bootstrap execute + watch | "앱 만들고, GitHub repo 만들고, 첫 배포까지 진행 중이에요. 보통 2~5분 정도 걸려요." |
| Step 7a GitHub 연결 필요 (`device_code_issued` 발생 시) | "GitHub 연결이 필요해요. 브라우저에서 `{verification_uri}` 열고 코드 `{user_code}` 를 입력해 주세요. 대화형 TTY 에서는 승인하면 자동으로 진행되고, 에이전트 컨텍스트에서는 안내 후 멈춰요." |
| Step 8 git clone | "코드를 현재 폴더로 가져와요." |
| Step 9 결과 안내 | "끝났어요. 이렇게 시작하면 돼요." |

raw helper JSON 이 디버깅에 필요한 환경 (개발 검증) 은 `AXHUB_INIT_VERBOSE=1` 환경변수가 켜진 경우에만 echo 해요. 기본 흐름은 항상 한 줄 자연어로 진행해요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Visible response contract:** when no pending resume state exists, the first visible chat sentence must be exactly "새 앱을 만들 수 있는 템플릿을 확인할게요." Do not add any preface before that sentence. Step 0.5 에서 pending resume state 가 있으면 first visible chat sentence 는 이어서 할지 묻는 문장으로 시작해요. Claude Desktop badge already shows internal execution metadata separately; do not restate badge metadata in prose.

To start an axhub app:

**이 SKILL 이 앱 생성 전체를 담당해요.** "앱 만들어줘" / "그걸로 앱 만들고 싶어" 류 발화에 generic 한 스택/데이터-fetch 질문 (예: "어떤 스택으로 만들까요?", "데이터를 어떻게 가져올까요?") 을 즉석에서 만들지 말고, 반드시 아래 Step 0.5-8 template flow 를 따라요. 스택 선택은 Step 3 (backend template registry), 데이터 접근은 Step 8 의 `@ax-hub/sdk` 안내로 처리해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "CLI와 template registry 확인", status: "in_progress", activeForm: "CLI 확인 중" },
     { content: "template 선택", status: "pending", activeForm: "template 고르는 중" },
     { content: "앱 이름 입력", status: "pending", activeForm: "앱 이름 정하는 중" },
     { content: "bootstrap saga 실행 (app + repo + deploy)", status: "pending", activeForm: "bootstrap 진행 중" },
     { content: "git clone + 자동 연결 + 결과 안내", status: "pending", activeForm: "코드 가져오는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.


0.5. **재진입 resume state 를 먼저 확인해요 (proactive resume).**

   Step 1 로 새로 시작하기 전에 항상 repo-local state 를 확인해요. 이 state 는 품질 state 가 아니므로 품질 state helper 를 쓰지 않고, `.axhub/init-resume.json` 만 helper 로 읽어요.

   ```bash
   axhub-helpers init-resume route --json
   ```

   route 가 `watch_status` 또는 `resume_last` 이고 `clone_done=false` 면 템플릿 목록을 보여주기 전에 먼저 이어서 할지 물어요. raw `bootstrap_id`, `idempotency_key`, repo, slug 값은 chat 에 echo 하지 않아요. slug 는 사용자에게 보이는 앱 별칭으로만 짧게 humanize 해요.

   ```json
   {
     "question": "저번에 만들던 앱을 이어서 할까요?",
     "header": "이어서",
     "options": [
       {"label": "이어서 하기", "value": "resume", "description": "이전 생성 흐름을 계속해요"},
       {"label": "새로 시작", "value": "fresh", "description": "이전 기록은 두고 새 앱 생성을 시작해요"}
     ]
   }
   ```

   비대화형/D1 guard 에서는 safe default `새로 시작` 으로 진행해요. `이어서 하기` 를 고르면 helper 의 route enum 과 `args.*_command` 를 그대로 써요. SKILL 이 raw id 를 다시 조합하지 않아요:

   - `watch_status` → helper 의 `args.status_command` 를 실행해요. 현재 command shape 은 `axhub apps bootstrap-status "$BOOTSTRAP_ID" --watch --watch-timeout 9m --json` 예요.
   - `resume_last` → helper 의 `args.resume_command` 를 실행해요. 현재 command shape 은 `axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --tenant "$AXHUB_TENANT" --execute --resume-last --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json` 예요.
     - 만약 resume 명령이 `no pending github device flow` 로 실패하면 바로 사용자에게 막혔다고 하지 않아요. 먼저 `axhub github accounts list --json` 를 읽기 전용으로 재조회하고, 선택한 GitHub owner 의 installation 이 `installed=true` 또는 `installation_id` 로 확인될 때만 같은 template/name/slug/subdomain/github-owner/repo-name/idempotency-key 로 `--resume-last` 없는 `bootstrap --execute --watch --watch-timeout 9m` 복구를 한 번 실행해요. 이 분기는 이미 승인된 device flow 가 CLI cache 에 남지 않았지만 GitHub App 설치/연결은 완료된 Claude Desktop 상태를 복구하기 위한 것이며, `device_code_pending` 이거나 owner 설치 확인이 안 되면 fresh execute 를 하지 않아요.
   - 상태가 깨졌거나 오래돼 helper 가 `fresh` 를 주면 자연어로 "이전 기록을 찾지 못해서 새로 시작할게요." 라고 말하고 Step 1 로 가요.

1. **CLI 존재를 확인해요.**

   ```bash
   axhub --version
   ```

   실패하면 "CLI 설치가 필요해요. 설치 도와줘라고 말해 주세요"처럼 자연어로 짧게 안내하고 중단해요.

**Tenant 선택 (axhub-tenant-picker:L1).** axhub-helpers `tenant-resolve` 가 캐시(`.axhub/state/tenant.json`)/tenants list/preflight 로 tenant 를 결정해요. fence 간 env 는 휘발하므로 결정된 tenant 를 캐시에 영속화해서 다음 fence 가 re-read 해요. 명시 `AXHUB_TENANT` override 가 있으면 helper 를 건너뛰어요.

```bash
# axhub-tenant-picker:L1 — thin resolver (위험 로직은 Rust axhub-helpers tenant-resolve 가 소유)
TENANT_CACHE=".axhub/state/tenant.json"
NEEDS_PICK="false"
CANDIDATES_JSON="[]"
# Precedence 1: 명시 AXHUB_TENANT env override → helper 호출 skip
if [ -z "${AXHUB_TENANT:-}" ]; then
  HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
  if [ -n "$HELPER" ] && [ ! -x "$HELPER" ] && [ -x "${HELPER}.exe" ]; then HELPER="${HELPER}.exe"; fi
  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null || command -v axhub-helpers.exe 2>/dev/null)"
  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers* "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers*; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
  TENANT_JSON=$([ -n "$HELPER" ] && "$HELPER" tenant-resolve --json 2>/dev/null)
  [ -n "$TENANT_JSON" ] || TENANT_JSON='{}'
  AXHUB_TENANT=$(printf '%s' "$TENANT_JSON" | jq -r '.tenant // empty' 2>/dev/null || true)
  _NEEDS_PICK_RAW=$(printf '%s' "$TENANT_JSON" | jq -r '.needs_pick // false' 2>/dev/null || echo false)
  # no-loop: needs_pick 는 비어있지 않은 resolve 에서만 true; 빈/부재 helper → false (재프롬프트 안 함)
  if [ "$_NEEDS_PICK_RAW" = "true" ]; then
    CANDIDATES_JSON=$(printf '%s' "$TENANT_JSON" | jq -c '.candidates // []' 2>/dev/null || echo '[]')
    if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
      # non-TTY: active fallback + 경고 (R4 fail-wrong guard — bash 위치 필수)
      AXHUB_TENANT=$(printf '%s' "$CANDIDATES_JSON" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)
      echo "여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant($AXHUB_TENANT)로 진행해요"
    else
      NEEDS_PICK="true"
    fi
  fi
fi
# 결정된 tenant 영속화 (fence 간 source of truth) — needs_pick 대기 중엔 미기록(L2 가 기록)
if [ -n "${AXHUB_TENANT:-}" ] && [ "$NEEDS_PICK" = "false" ]; then
  mkdir -p "$(dirname "$TENANT_CACHE")"
  printf '{"tenant":"%s","source":"resolved","ts":%s}\n' "$AXHUB_TENANT" "$(date +%s 2>/dev/null || echo '0')" > "$TENANT_CACHE"
fi
export AXHUB_TENANT
export NEEDS_PICK
export CANDIDATES_JSON
```

`AXHUB_TENANT` 가 비어 있으면 tenant 를 확정할 수 없어요 — preflight `auth_ok` 와 `current_team_id` 를 먼저 확인하고 `다시 로그인해줘` 라고 안내해요. 구버전·부재 helper 면 빈 값 → active tenant 로 진행하고, picker 는 helper 업데이트 후 돌아와요.

**Tenant picker (axhub-tenant-picker:L2).** `NEEDS_PICK=true` 이고 대화형 TTY 일 때만 실행해요. `CANDIDATES_JSON` 에서 후보 목록을 읽어 AskUserQuestion 으로 사용자에게 선택을 요청해요. 선택 결과를 `.axhub/state/tenant.json` 에 `{tenant, source:"picker", ts}` 형태로 기록해요 (이후 fence 가 re-read 해서 상속).

```typescript
if (NEEDS_PICK === "true") {
  const candidates = JSON.parse(CANDIDATES_JSON);
  AskUserQuestion({
    questions: [{
      question: "어떤 tenant 로 진행할까요?",
      header: "Tenant",
      multiSelect: false,
      options: candidates.map((t: { id?: string; slug?: string; name?: string }) => ({
        label: t.name ?? t.slug ?? t.id ?? "unknown",
        description: `ID: ${t.id ?? t.slug}`,
      })),
    }],
  });
  // 선택된 tenant ID 를 .axhub/state/tenant.json 에 write-back
  // mkdir -p .axhub/state && echo '{"tenant":"<선택값>","source":"picker","ts":<epoch>}' > .axhub/state/tenant.json
}
```

AskUserQuestion 답변을 받은 뒤 선택된 tenant ID 를 `AXHUB_TENANT` 로 확정하고 `.axhub/state/tenant.json` 에 `{"tenant": "<id>", "source": "picker", "ts": <epoch>}` 를 기록해요. 이후 fence 가 이 파일을 re-read 해서 같은 tenant 를 재사용해요.

**Non-interactive AskUserQuestion guard (D1):** `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 환경에서는 L2 AskUserQuestion 을 건너뛰어요 — L1 블록이 이미 active fallback + 경고를 처리했어요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 `picker` 채널 참조.

2. **Backend template registry 를 읽어요.**

   ```bash
   # tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
   AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
   axhub apps templates list --tenant "$AXHUB_TENANT" --json
   ```

   응답 envelope shape:

   ```json
   {
     "schema_version": "...",
     "data": {
       "items": [
         {"id": "<uuid>", "folder_name": "react-axhub", "name": "React (Axhub)", "resource_tier": "small"},
         {"id": "<uuid>", "folder_name": "nextjs-axhub", "name": "Next.js (Axhub)", "resource_tier": "small"},
         {"id": "<uuid>", "folder_name": "astro-axhub", "name": "Astro (Axhub)", "resource_tier": "micro"}
       ]
     }
   }
   ```

   `schema_version` 은 응답 검증용 internal primitive 예요 — raw 값을 사용자 chat 에 echo 하지 않아요. `items[]` 의 `id` 또는 built-in alias (`react` / `nextjs` / `astro`) 를 `--template` 인자로 써요.

   on exit 4 (auth 만료, CLI `axhub apps templates list` — 옛 sysexits 65 아님) → "다시 로그인해줘"라고 말하면 이어서 처리할 수 있다고 안내해요. on exit 8 (tenant 미해석) → `axhub profile current --json` 안내. 그 외 비정상 종료는 "설치 상태 진단해줘"라고 말하면 점검할 수 있다고 안내해요.

2.5. **GitHub App 설치 확인 — 미설치면 설치까지 막아요 (gate).**

   template 목록이 정상적으로 오면 로그인이 된 거예요. 이어서 GitHub App 설치 상태를 확인해요. 규칙은 두 가지예요: **install_url 은 어떤 상태에서도 무조건 보여줘요.** 그리고 **아무 GitHub 계정에도 설치가 안 됐으면(미설치) 설치·연결이 확인될 때까지 앱 만들기를 막아요.** 이미 어딘가 설치돼 있으면 막지 않고 바로 진행하고, 다른 org 을 더 붙이고 싶으면 같은 install_url 로 추가해요.

   왜 막냐면, OAuth device flow (Step 7a) 는 GitHub **사용자 인증**만 해요. repo 를 만들려면 AxHub GitHub App 을 계정/org 에 **설치(저장소 접근 권한 부여)**하는 별도 1회 단계가 필요한데, 이건 GitHub 웹 UI 에서 사람이 직접 눌러야 해요. 설치 없이 진행하면 bootstrap 이 repo 생성에서 막혀서 사용자가 멈춘 줄 알아요.

   ```bash
   axhub github accounts list --json   # data.accounts[]: login, installed, install_url
   ```

   먼저 출력 상태를 봐요. **확인할 수 없는 상태에서는 막지 않아요** (helper 의 gap 계약과 동일 — `auth_ok` + `uninstalled`/`empty` 일 때만 막아요):

   - 출력이 비었거나(타임아웃) JSON 파싱이 안 되면 (`unavailable`) — 설치 상태를 확인 못 하니 **막지 않고 그대로 진행**해요. 일시적 CLI 문제로 앱 만들기를 멈추지 않아요.
   - auth 에러 envelope (`{"status":"error","error":{"code":"auth"}}`) 면 — 인증 만료라 "다시 로그인해줘" 로 안내하고 재로그인 후 이 단계를 다시 해요 (막지 않고 재인증으로 라우팅).
   - 정상 응답이면 아래로 진행해요.

   `data.accounts[]` 를 읽어서 **install_url 이 있으면 무조건 먼저 한 줄로 보여줘요 — 설치 여부와 무관하게 항상이에요.** `install_url` 은 아무 entry 에서나 읽어요 (전부 같아요). `installation_id` 같은 internal 값은 echo 하지 말고 `login` 과 `install_url` 만 보여줘요. 링크는 보여주기만 하고 자동으로 열지 않아요.

   - **install_url (항상 표시)**: "GitHub App 설치·계정 추가 링크: `<install_url>`" — 설치 여부와 무관하게 매번 보여줘요. 이미 설치된 사람도 다른 org/계정을 더 붙일 수 있게 항상 남겨요.

   그 다음 `installed:true` 인 `login` 개수로 갈라요.

   **(A) 설치된 계정이 1개 이상 (`github.state` = installed/mixed) — 막지 않아요.** "이미 설치된 계정: `<login...>` (다른 계정/org 은 위 링크로 더 붙일 수 있어요)" 를 한 줄 덧붙이고 owner 를 정해요 (`ambiguous_installation` 예방). bootstrap 은 App 이 **여러 계정/org 에 설치**돼 있으면 `--github-owner` 없이는 `ambiguous_installation` (CLI exit 9) 으로 멈춰요:

   - **`AXHUB_GITHUB_OWNER` env 가 있으면** 그 값을 `$GITHUB_OWNER` 로 바로 써요 (질문 없이 — 비대화형 agent/CI 경로, single-login auto-pick 과 동일).
   - **정확히 1개**: 묻지 말고 그 `login` 을 `$GITHUB_OWNER` 로 자동으로 잡아요.
   - **2개 이상**: 아래 질문으로 한 번 물어서 고른 `login` 을 `$GITHUB_OWNER` 로 잡아요. 그래야 Step 5/6 이 멈추지 않아요.

   ```json
   {
     "questions": [{
       "question": "어느 GitHub 계정에 저장소를 만들까요?",
       "header": "GitHub 계정",
       "multiSelect": false,
       "options": [
         {"label": "<login-1>", "description": "이 계정/org 에 비공개 repo 를 만들어요"},
         {"label": "<login-2>", "description": "이 계정/org 에 비공개 repo 를 만들어요"}
       ]
     }]
   }
   ```

   옵션 label 은 `installed:true` 인 `login` 값으로 채워요 (UI 제한상 최대 3개; 더 많으면 먼저 전체를 텍스트로 보여주고 3개만 버튼으로). `installation_id` 같은 internal 값은 옵션에 넣지 말고 `login` 만 보여줘요. 비대화형/D1 guard 에서는 `AXHUB_GITHUB_OWNER` env 가 있으면 그 owner 로 진행하고, 없으면 묻지 않고 safe default `취소` 로 bootstrap 을 멈춰요 — 설치가 여러 개인데 owner 를 못 정하면 임의 계정에 repo 를 만들지 않아요.

   **(B) 설치된 계정이 0개로 확인됨 (`uninstalled` = 계정은 있는데 전부 미설치 / `empty` = 보이는 계정 자체가 없음) — 설치까지 막아요 (gate).** 위 degraded-state(`unavailable`/auth 에러)는 여기 해당 안 돼요 — 그건 막지 않아요. install_url 이 있으면 또렷이 다시 보여주고, 빈 목록이라 링크를 못 읽으면 "axhub 대시보드의 GitHub 연결 메뉴에서 AxHub 앱을 설치해요" 로 안내해요. 그런 다음 설치·연결을 요청해요:

   > GitHub App 이 아직 어떤 GitHub 계정에도 설치 안 됐어요. repo 를 만들려면 먼저 설치가 필요해요.
   > 1. 위 링크 `<install_url>` 를 브라우저에서 열어요.
   > 2. repo 를 만들 계정/org 을 고르고 저장소 접근을 승인해요.
   > 3. 끝나면 "설치했어" 라고 알려줘요.

   그리고 아래 질문으로 설치 완료를 기다려요. **설치가 확인되기 전에는 Step 3 (template 선택) 이후로 절대 진행하지 않아요 — bootstrap dry-run/execute 도 시작하지 않아요.**

   ```json
   {
     "questions": [{
       "question": "GitHub App 설치를 끝냈을까요?",
       "header": "GitHub App",
       "multiSelect": false,
       "options": [
         {"label": "설치 완료", "description": "설치·연결을 끝냈으면 다시 확인하고 이어서 만들어요"},
         {"label": "취소", "description": "지금은 앱 만들기를 멈춰요"}
       ]
     }]
   }
   ```

   `설치 완료` 를 고르면 `axhub github accounts list --json` 를 다시 읽어요. 이제 `installed:true` 가 보이면 (A) 로 가서 owner 를 정하고 진행해요. 아직 `installed` 가 하나도 없으면 같은 install_url 을 한 번 더 보여주고 이 질문을 다시 띄워요 (설치가 확인될 때까지 반복). 재조회가 `unavailable`/auth 에러로 돌아오면 무한 차단하지 말고 위 degraded-state 규칙대로 진행하거나 재인증으로 라우팅해요. `취소` 면 "GitHub App 을 설치하면 '다시 만들어줘' 라고 말해 주세요. 이어서 만들게요." 로 멈춰요.

   **Non-interactive AskUserQuestion guard (D1):** `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 환경에서는 설치 브라우저 단계를 사람이 완료할 수 없어요. 이 gate 의 safe default 는 `취소` (registry `init` 채널) 라 bootstrap 을 시작하지 않고, install_url + 재개 phrase(`다시 만들어줘`)를 남기고 멈춰요. 미설치인데 owner 를 추측해 repo 를 만들지 않아요.

## 템플릿 선택 가이드

이 가이드는 두 번째 registry 가 아니에요. 먼저 `axhub apps templates list --json` 로 backend 가 반환한 template 목록을 읽고, 그 안에 있는 alias / folder_name 에만 설명을 덧붙여요. 선택 값은 반드시 backend 가 반환한 `id` 또는 built-in alias (`react` / `nextjs` / `astro`) 여야 해요.

알 수 없는 새 template 이 backend 에서 오면 숨기지 않아요. 로컬 설명이 없는 항목은 backend `name` 과 `folder_name` 을 그대로 보여주고, "이름을 보고 고르면 돼요. 잘 모르겠으면 먼저 Next.js 추천을 봐요." 처럼 중립 안내만 덧붙여요.

| alias / folder | 이렇게 만들고 싶을 때 골라요 |
|---|---|
| `nextjs` (또는 `nextjs-axhub`) | 쇼핑몰, 예약, 결제, 로그인, 관리자 화면처럼 화면과 기능이 함께 있는 웹서비스를 만들 때 추천해요. |
| `astro` (또는 `astro-axhub`) | 회사 소개, 랜딩 페이지, 블로그, 문서처럼 글과 이미지 중심이고 자주 바뀌지 않는 사이트에 좋아요. |
| `react` (또는 `react-axhub`) | 로그인한 뒤 쓰는 설정 화면, 입력 폼, 관리 화면처럼 버튼을 눌러 내용이 자주 바뀌는 화면에 좋아요. |

backend 가 반환한 template 전체 목록은 먼저 텍스트로 보여줘요. structured AskUserQuestion 은 UI 제한에 맞춰 **최대 3개 선택지** 만 쓰고, 선택지는 모두 실제 backend template 이어야 해요. Claude Desktop AskUserQuestion 은 skip/free-text 입력을 자동으로 붙여요. 그래서 template picker 에는 `기타` / `Other` / `직접 고르기` / `취소` 같은 generic 선택지를 직접 넣지 말아요. 알려진 alias 는 위 설명을 짧게 붙이고, 알 수 없는 항목은 backend `name` 과 `folder_name` 을 붙여요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — template 선택은 `abort`, 앱 이름은 `abort`, bootstrap 실행 확인은 `취소`, GitHub owner 선택은 `취소`(단 `AXHUB_GITHUB_OWNER` env 가 있으면 그 owner), auto-connect 실행 확인은 `아니요`, resume offer 는 `새로 시작` 예요.

3. **template 을 선택해요.**

   먼저 위 가이드와 backend 가 반환한 template 전체 목록을 텍스트로 보여줘요. 사용자가 발화에 exact alias 또는 backend folder_name 을 이미 적었다면 (예: "nextjs 앱 만들어줘") AskUserQuestion 없이 그 alias 로 진행해요. id 가 없으면 structured AskUserQuestion 은 3개 이하 선택지만 쓰고, 각 option 은 실제 backend template 하나에 대응해야 해요. 선택 label 과 template id/alias 매핑은 chat 에 노출하지 말고 SKILL 내부 상태로만 들고 있어요.

   ```json
   {
     "question": "어떤 템플릿으로 시작할까요?",
     "header": "템플릿",
     "options": [
       {"label": "Next.js 추천", "description": "쇼핑몰·예약·결제·로그인·관리자 화면"},
       {"label": "Vite + React", "description": "로그인 뒤 쓰는 설정·입력·관리 화면"},
       {"label": "Astro", "description": "회사 소개·랜딩 페이지·블로그·문서"}
     ]
   }
   ```

   위 JSON 은 예시예요. 각 버튼은 backend 가 해당 alias 또는 folder 를 반환할 때만 보여줘요. `Next.js 추천` 은 `nextjs` alias 또는 `nextjs-axhub` folder, `Vite + React` 는 `react` alias 또는 `react-axhub` folder, `Astro` 는 `astro` alias 또는 `astro-axhub` folder 로 매핑해요. backend 가 알려진 template 을 3개보다 많이 반환하면 먼저 추천할 실제 template 3개만 버튼으로 만들고, 나머지는 이미 보여준 텍스트 목록에서 free-text 입력으로 고를 수 있게 둬요. backend 가 알려진 template 을 반환하지 않으면 `직접 고르기` 같은 generic 버튼을 만들지 말고 실제 backend `name` 을 최대 3개 버튼 label 로 써요. 사용자가 free-text 로 답한 값이 backend 목록의 exact alias / folder_name / name 과 맞지 않으면 saga 를 시작하지 말고 다시 목록을 보여줘요. subprocess 에서는 자동 선택하지 않아요.

4. **앱 이름을 정해요.**

   `--name` 은 bootstrap saga 의 required 인자예요. 사용자 발화에서 앱 이름을 유추할 수 있으면 (예: "결제 앱 만들어줘" → "결제 앱") AskUserQuestion 없이 그대로 써요. 없으면 한 번 물어요.

   ```json
   {
     "question": "앱 이름 뭘로 할래요?",
     "header": "앱 이름",
     "options": [
       {"label": "지금 발화 기준 자동", "value": "auto_from_utterance", "description": "발화에서 유추한 이름을 그대로 써요"},
       {"label": "직접 입력", "value": "manual_name", "description": "원하는 이름을 한 번만 말해요"},
       {"label": "취소", "value": "abort", "description": "프로젝트를 만들지 않아요"}
     ]
   }
   ```

   `--slug` 는 자동 유도해요 (이름을 소문자화 + 공백 → 하이픈, 특수문자 제거). slug 가 backend 정책과 충돌하면 saga 가 `error_code` 로 알려주고 SKILL 이 다시 한 번 물어요.

5. **Bootstrap dry-run 으로 미리보기를 만들어요.**

   ```bash
   # tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
   AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --dry-run --json
   ```

   `$GITHUB_OWNER` 가 Step 2.5 에서 정해졌을 때만 `--github-owner` 를 붙여요 (설치 0개라 비어 있으면 생략). 응답 envelope 의 미리보기 카드 (template / slug / subdomain / repo_name / private/public / installation_id 후보) 를 사용자에게 한국어 한 줄씩 보여줘요. raw JSON dump 금지. `--dry-run` default 가 true 라 명시적으로 안 적어도 같지만, 가독성을 위해 명시해요.

6. **사용자 확인 + execute.**

   ```json
   {
     "question": "지금 만들고 배포까지 진행할까요?",
     "header": "앱 만들기",
     "options": [
       {"label": "진행", "value": "execute", "description": "backend app + GitHub repo + 첫 deploy 를 자동으로 진행해요"},
       {"label": "취소", "value": "취소", "description": "지금은 만들지 않아요"}
     ]
   }
   ```

   확인 받으면 saga 를 실행해요. 실행 직전 `.axhub/init-resume.json` 에 template/app_name/slug/subdomain/idempotency_key 를 먼저 저장해요:

   ```bash
   # tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
   AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
   axhub-helpers init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --json
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --execute --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
   ```

   Step 2.5 에서 owner 를 정했으면 `--github-owner "$GITHUB_OWNER"` 를 그대로 붙여요 (설치 여러 개일 때 `ambiguous_installation` 을 막아요). 비어 있으면 생략하고 device flow 에 맡겨요.

   **에이전트도 terminal 까지 폴링해요 (axhub-cli 0.15.3+).** bare `--watch` 는 비-TTY/에이전트 컨텍스트에서 단일 스냅샷으로 degrade 하지만, `--watch-timeout` (또는 `--watch-interval`) 을 붙이면 explicit streaming override 라 CLI 가 degrade 하지 않고 terminal(saga 완료 / 실패) 까지 직접 폴링해요. 그래서 saga 가 끝까지 진행돼 `repo_full_name` 을 받을 수 있어요 (snapshot 으로 끊기지 않아요). `--no-input` 같은 플래그는 따로 안 붙여도 돼요 (비-TTY 면 CLI 가 자동 감지). 이 bash 는 Bash tool `timeout: 570000` (9.5분) 으로 호출하고, 9분 초과 시 CLI Timeout + resume hint 를 받으면 "아직 만드는 중이에요, 계속 확인할게요" 후 아래 bootstrap-status 를 한 번 더 호출해요:

   ```bash
   # tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
   AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
   BOOTSTRAP_ID=$(echo "$ACCEPTED_JSON" | jq -r '.data.bootstrap_id')
   axhub-helpers init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --bootstrap-id "$BOOTSTRAP_ID" --json
   axhub apps bootstrap-status "$BOOTSTRAP_ID" --tenant "$AXHUB_TENANT" --watch --watch-timeout 9m --json
   ```

   진행 중 매 ~30s 마다 한국어 한 줄로 narrate 해요 — "앱 만들고 있어요" / "GitHub repo 만들고 있어요" / "첫 배포 중이에요. 거의 다 왔어요". 60s 이상 같은 stage 머무르면 "조용하네요, 계속 기다리고 있어요" 한 줄을 추가해요.

   **GitHub 연결 필요 (CLI stdout 의 `device_code_issued` event 처리).** CLI 가 GitHub App 미설치 / installation 만료 / scope 부족 상태를 해결하려고 device flow 를 시작하면, backend saga 를 시작하기 전에 다음 JSON line 을 emit 해요 (대화형 TTY 면 그 뒤 승인까지 polling 하고, 에이전트/비-TTY 면 event 직후 fast-exit 해요):

   ```json
   {"event":"device_code_issued","data":{"verification_uri":"https://github.com/login/device","verification_uri_complete":null,"user_code":"XXXX-XXXX","expires_in":899}}
   ```

   이 event 가 stdout 에서 나오면 `axhub-helpers init-resume put ... --pending-device-flow true --json` 로 state 를 갱신한 뒤 narrate 보다 우선해서 즉시 사용자에게 한국어로 안내해요. raw JSON dump 금지 — `verification_uri` (또는 `verification_uri_complete` 가 non-null 이면 그걸 우선), `user_code`, `expires_in` 만 humanize 해요:

   > GitHub 연결이 필요해요. 다음 단계로 진행해 주세요:
   >
   > 1. 브라우저에서 열기: `<verification_uri_complete 우선, 없으면 verification_uri>`
   > 2. 코드 입력: `<user_code>`
   > 3. axhub GitHub App 설치 승인
   >
   > 브라우저에서 승인한 다음 '승인했어' 라고 알려주세요. 제가 이어서 마무리할게요. (유효시간 약 `<expires_in/60>` 분)

   `verification_uri_complete` 가 있으면 코드가 자동 입력되니 2번을 생략해도 돼요. 그 다음은 컨텍스트에 따라 갈라져요 (axhub-cli 0.15.3+): **대화형 TTY** 면 saga 가 polling 으로 GitHub App install 완료를 기다리니까 SKILL 은 stdout 의 다음 stage event (예: `app_created`, `repo_created`) 가 도착할 때까지 narrate 만 계속해요. **에이전트 / 비-TTY 컨텍스트** 면 CLI 가 `device_code_issued` emit 직후 fast-exit 하므로, challenge 를 보여준 뒤 **사용자에게 명령을 치라고 떠넘기지 말고** 브라우저 승인을 기다려요: "브라우저에서 승인한 다음 '승인했어' 라고 알려주세요. 제가 이어서 마무리할게요." 사용자가 승인 신호("승인했어" / "연결했어" / "됐어")를 주면 에이전트가 캐시된 device flow 를 `--resume-last` 로 직접 이어받아요 (resume 명령을 사용자에게 출력하지 말아요):

   ```bash
   # tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
   AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --tenant "$AXHUB_TENANT" --execute --resume-last --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
   ```

   이 resume 호출은 캐시된 device code 로 token 교환을 마치고 같은 saga 를 terminal 까지 이어가요 (`--watch` 는 인증 완료 후 saga 폴링용이라 fast-exit 와 충돌하지 않아요). **outstanding code 가 있는 동안 `--resume-last` 없이 fresh `bootstrap --execute` 를 다시 호출하지 말아요 — 새 code 를 발급해 이미 승인한 code 를 버려요.** resume 응답이 아직 `device_code_pending` (`DEVICE_FLOW_PENDING`) 이면 "브라우저 승인이 아직 안 끝난 것 같아요. 승인 후 다시 알려주세요" 후 승인 신호를 받으면 한 번 더 resume 해요. resume 응답이 `no pending github device flow` 이면 Claude Desktop 이 이미 device approval 을 끝냈지만 CLI cache 가 비어 있는지 확인해야 해요: `axhub github accounts list --json` 로 선택한 owner 설치가 확인될 때만 같은 idempotency key 로 아래 복구 명령을 한 번 실행해요. owner 설치 확인이 안 되면 fresh execute 를 하지 말고 새 device flow 안내로 돌아가요.

   ```bash
   # tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
   AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
   axhub github accounts list --json
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --github-owner "$GITHUB_OWNER" --repo-name "$APP_SLUG" --repo-private --tenant "$AXHUB_TENANT" --execute --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
   ```

   device code 가 만료(약 15분)됐으면 이 Step 의 fresh `--execute` 부터 새 challenge 를 발급해요. backend 가 `github_relogin_required` (428) 를 주면 user GitHub 토큰 만료라, fresh `--execute` 로 새 device flow 를 발급해 같은 surface → resume 흐름으로 복구해요. 설계 + resume 계약은 `../github/SKILL.md` 의 OAuth device flow 섹션과 `docs/superpowers/specs/2026-05-25-github-device-flow-surface-design.md` 를 참조해요.

   상세한 device flow 안내 패턴은 `../github/SKILL.md` 의 OAuth device flow 섹션을 따라요.

7. **응답에서 `repo_full_name` 을 꺼내 CWD 로 받아요.**

   ```bash
   REPO=$(echo "$FINAL_JSON" | jq -r '.data.status.repo_full_name // empty')
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

   항상 현재 dir (CWD) 에 코드를 받아요. 서브 dir (`$APP_SLUG/`) 만들지 않아서 사용자가 "cd $APP_SLUG" 추가 단계 안 해도 돼요. `.claude` / `.omc` / `.codegraph` 같은 IDE/도구 메타 dir 은 untracked 로 자연스럽게 보존돼요 (`git reset --hard` 는 tracked 파일만 건드려요). 이미 `.git` 이 있는 dir 은 안전을 위해 자동 clone 건너뛰고 수동 명령 안내를 한 줄로 보여줘요. clone 실패 시 (권한 / network) `repo_full_name` 만 사용자에게 알려주고 수동 clone 안내를 보여줘요.


8.5. **Step 8.5 — clone 직후 자동 연결을 준비해요 (post-scaffold auto-connect).**

   clone 이 성공하면 먼저 resume state 에 `clone_done=true` 를 기록하고 clear 해요. 그 다음 infer-tables-env handoff 보다 먼저 로컬 실행 가능 여부를 helper 로 감지해요.

   ```bash
   axhub-helpers init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --bootstrap-id "$BOOTSTRAP_ID" --repo-full-name "$REPO" --clone-done true --json
   axhub-helpers init-resume clear --json
   axhub-helpers scaffold-detect --json
   ```

   `package.json` 이 없거나 lockfile 이 없거나 dev script 가 없으면 로컬 미리보기는 건너뛰고 Step 8 로 가요. node 가 없으면 "앱 실행 준비를 도와드릴게요" 라고 말하고 onboarding 으로 넘겨요. raw install/dev 명령은 기본 chat 에 보여주지 않아요.

   감지 결과가 실행 가능하면 한 번만 물어요.

   ```json
   {
     "question": "앱을 바로 실행해 볼까요?",
     "header": "앱 실행",
     "options": [
       {"label": "아니요", "value": "skip", "description": "배포 결과만 확인해요"},
       {"label": "네, 실행까지", "value": "start", "description": "의존성을 설치하고 로컬 미리보기를 띄워요"}
     ]
   }
   ```

   비대화형/D1 guard 에서는 safe default `아니요` 로 넘어가요. `네, 실행까지` 면 helper 를 실행해요. helper 내부 install 은 lockfile 이 있을 때만 실행하고 항상 `--ignore-scripts` 를 붙여 lifecycle script 실행을 막아요.

   ```bash
   axhub-helpers scaffold-dev start --json
   ```

   결과는 자연어로만 보여줘요:

   - 성공/이미 실행 중 → "로컬 미리보기도 떠 있어요." + localhost URL 이 있으면 보조로 안내
   - install/dev 실패 → "미리보기 자동 실행이 잠깐 안 됐어요. '다시 해줘' 하면 재시도할게요"
   - postinstall 이 필요한 경우 → "로컬 미리보기는 잠깐 준비가 더 필요해요"
   - lockfile 없음 → "코드는 준비됐어요" + localhost 생략

8. **결과와 다음 액션을 안내해요.**

   saga 응답의 `app_id` / `deployment_id` / `repo_full_name` 을 humanize 해서 한국어 한 줄씩 보여줘요. **배포 공개 URL(라이브 앱 주소)** 은 배포 성공 후 `axhub apps get` 으로 앱의 `access_url` 을 읽어서 hero 첫 줄로 보여줘요. 이건 `https://<subdomain>.<tenant_slug>.axhub.ai` 형식의 **실제 접속 주소**예요. URL 은 절대 합성하지 않아요.

   ```bash
   # 라이브 공개 URL = 앱의 access_url (axhub apps get). saga 의 app_id(또는 slug)로 조회해요.
   PUBLIC_URL="$(axhub apps get "$APP_ID" --json --no-input 2>/dev/null | jq -r '.access_url // .data.access_url // empty')"
   ```

   dry-run 의 subdomain 은 내부 힌트라 인터넷 주소처럼 조합하지 않아요. `access_url` 을 못 읽으면 아래 낮춤 문장으로 안내해요.

   ```
   🎉 인터넷에 올라갔어요: <confirmed-public-url>
   친구한테 바로 보여줄 수 있어요.

   로컬 미리보기가 성공했으면: 로컬 미리보기도 떠 있어요: <localhost-url>

   다음에 뭐 할까요?
   - 코드 고치고 "다시 배포해줘"
   - "방금 배포 어디까지 됐어?"
   - 데이터 읽기는 template 에 설치된 @ax-hub/sdk 를 써요.
   ```

   확인된 공개 URL 이 없으면 "인터넷 배포가 시작됐어요. '방금 배포 어디까지 됐어?' 라고 물으면 이어서 확인할게요." 라고 낮춰 말해요.

   `error_code` 로 saga 가 실패했으면 다음 routing 을 써요:
   - `conflict` / `ambiguous_installation` (CLI exit 9, GitHub App 이 여러 계정/org 에 설치돼 owner 가 모호함) → 먼저 **install_url 을 한 줄로 보여주고**(무조건), 이어서 **즉석에서 새 질문을 만들지 말고** Step 2.5 의 owner 선택("어느 GitHub 계정에 저장소를 만들까요?")을 다시 띄워요. 옵션은 에러 hint 의 `owner=installation_id` 목록 또는 `axhub github accounts list --json` 의 `installed:true` 인 `login` 들로 채워요 (raw `installation_id` 는 옵션에 echo 하지 말고 `login` 만). 고른 `login` 을 `$GITHUB_OWNER` 로 잡고 같은 idempotency key 로 Step 6 execute 를 `--github-owner "$GITHUB_OWNER"` 와 함께 한 번 다시 실행해요. 비대화형/D1 에서는 `AXHUB_GITHUB_OWNER` env 가 있으면 그 owner 로 바로 재실행하고, 없으면 `취소` 로 멈춰요(추측 금지). install_url 은 이미 설치된 경우에도 항상 같이 보여주되, 이 에러는 "설치 없음" 이 아니라 "설치가 여러 개라 owner 선택 필요" 라서 url 만으로는 안 풀려요 — owner 선택까지 이어가요.
   - `github.installation_missing` / `github.repo_create_failed` → "GitHub 연결 다시 해줘"라고 말하면 이어서 처리할 수 있다고 안내
   - `validation.template_not_found` → Step 2 로 돌아가 다시 목록을 보여줘요
   - `validation.slug_collision` → Step 4 로 돌아가 새 이름을 받아요
   - `auth` (CLI exit 4, auth 만료 — 옛 sysexits 65 아님) → "다시 로그인해줘"
   - `forbidden` / `tenant_scope` (CLI exit 12 / 8, 권한·scope 부족) → 사용자에게 권한 부족 안내 + workspace admin 문의
   - 그 외 → "설치 상태 진단해줘"

   앱이 정상 생성됐으면, 방금 만든 코드에서 필요한 테이블·환경변수를 추천받을지 AskUserQuestion 으로 한 번 물어봐요(비대화형/D1 guard 에서는 묻지 않고 safe default `아니요` 로 넘어가요).

   ```json
   {
     "questions": [{
       "question": "방금 만든 코드에서 필요한 테이블·환경변수를 추천받을래요?",
       "header": "사전 점검",
       "multiSelect": false,
       "options": [
         {"label": "아니요", "description": "지금은 넘어가요"},
         {"label": "네, 추천받기", "description": "코드 분석으로 필요한 테이블·env 를 추천받아요"}
       ]
     }]
   }
   ```

   `네, 추천받기` 면 infer-tables-env 분석으로 넘어가요. `아니요` 면 그냥 마무리해요.

9. **axhub MCP 도구를 설치해요 (선택, 비차단).**

   clone 된 repo 루트에 `.mcp.json` 을 설치해서 편집기·에이전트가 axhub MCP 도구를 쓸 수 있게 해요 — 로컬 코드 정적 검증(`validate` / `scan_sites`, `axhub-helpers mcp-serve` stdio) + 원격 SDK 지식·스키마(`axhub`, ax-mcp). 기존 `.mcp.json` 이 있으면 사용자 항목은 보존하고 axhub 두 항목만 추가·갱신해요(idempotent).

   ```bash
   # clone 된 앱 디렉터리에서 실행해요. 기존 사용자 mcpServers 항목은 보존돼요.
   axhub-helpers mcp-install
   ```

   helper 가 없거나 실패해도 **막지 않아요** — vendored packs floor 가 그대로 동작하니 "MCP 도구는 나중에 설치해도 돼요" 한 줄만 안내하고 넘어가요. 원격 MCP URL 은 `AXHUB_MCP_URL` env 로 override 할 수 있어요.

## NEVER

- NEVER GitHub App 이 아무 계정에도 설치 안 된 상태(`github.state` = uninstalled/empty)에서 Step 3 (template 선택) 이후로 진행하거나 bootstrap dry-run/execute 를 호출하지 않아요. Step 2.5 gate 에서 install_url 을 보여주고 설치가 확인(재조회 `installed:true`)될 때까지 멈춰요. 이미 설치된(installed/mixed) 경우는 막지 않고, install_url 은 추가 설치용으로 계속 보여줘요.
- NEVER `axhub init` 또는 `axhub init --from-template` 을 호출하지 않아요. Rust v1.0.0-rc.1 에서 `--from-template` flag 가 미구현 stub (`initcmd.rs` run() 미사용) 이라 호출해도 generic docker manifest 만 만들어져요. SKILL 은 `axhub apps bootstrap` saga 만 써요.
- NEVER `axhub apps create` 또는 `axhub deploy create` 를 직접 호출하지 않아요. bootstrap saga 가 server-side 에서 둘 다 처리해요.
- NEVER `axhub-helpers fetch-template` 또는 remote `templates.json` 을 source 로 쓰지 않아요. backend `axhub apps templates list` 만 source-of-truth 예요.
- NEVER subprocess (`$CI` / `$CLAUDE_NON_INTERACTIVE` / no TTY) 에서 template 또는 앱 이름을 임의로 고르지 않아요. registry safe_default 가 `abort` 또는 `취소` 예요.
- NEVER `--execute` 를 `--dry-run` 미리보기 + 사용자 확인 없이 호출하지 않아요. backend app + GitHub repo + deploy 가 한 번에 mutate 돼요.
- NEVER auth 만료를 template 조회 실패로 오해하지 않아요. CLI auth 실패 (exit 4 / error_code `auth`, 옛 sysexits 65 아님) 는 "다시 로그인해줘"라고 말하면 이어서 처리할 수 있다고 안내해요.
- NEVER `bootstrap --execute` 호출 직후 별도 `axhub deploy create` 를 다시 부르지 않아요. saga 가 첫 deploy 까지 포함해요.
- NEVER saga stdout 에서 `event: device_code_issued` 가 나왔는데 `verification_uri` + `user_code` 를 사용자에게 즉시 보여주지 않고 silent 하게 narrate 만 반복하지 않아요. saga 가 GitHub App install 승인을 기다리며 block 돼서 사용자는 SKILL 이 멈춘 줄 알아요. internal `device_code` raw 값은 여전히 echo 금지 — humanize 대상은 `verification_uri` + `user_code` + `expires_in` 만이에요.
- NEVER `repo_full_name` 응답이 비어 있는데 임의 URL 을 만들어 clone 시도하지 않아요. 응답이 비면 "설치 상태 진단해줘"라고 말하면 점검할 수 있다고 안내해요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template (Step 8 에서 사용).
- `../github/SKILL.md` — OAuth device flow surface 패턴 (Step 6 의 `device_code_issued` event 처리 기준).
