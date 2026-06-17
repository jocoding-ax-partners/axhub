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
allows-dependency-execution: true
model: sonnet
---

# Init

새 axhub 앱을 `axhub apps bootstrap` saga 로 한 번에 만들어요. backend app 생성 + GitHub repo 생성 + 첫 deploy 를 server-side 에서 처리하고, saga 응답의 `repo_full_name` 으로 현재 dir 에 git clone 해서 local + remote 둘 다 채워줘요. 기존 `axhub init` 호출은 `--from-template` 미구현 stub 라 SKILL 에서 호출하지 않아요.

## Vibe Coder Visibility Rules

이 SKILL 을 쓰는 사람은 대부분 개발 지식이 없어요. CLI 가 돌려주는 다음 field 는 **internal verification primitives** 예요. SKILL 안에서는 변수에 담아 주고받되, **raw 값을 사용자 chat 에 echo 하면 안 돼요**:

- `schema_version` (예: `bootstrap/v1`) — API 응답 검증용. raw 값 echo 금지.
- `items[].id`, `items[].folder_name`, `items[].name`, `items[].resource_tier` — `axhub apps templates list` 가 반환한 backend template registry. id 는 사용자 발화 매칭에만 쓰고, raw 목록 dump 금지.
- `bootstrap_id`, `status_url`, `stage`, `app_id`, `deployment_id`, `repo_full_name`, `error_code`, `error_message` — bootstrap saga 의 internal verification primitives. raw 값 echo 금지 (단 `repo_full_name` 은 마지막 단계에서 `git clone` URL 로 사용자에게 보여줘요).
- `request_id`, `idempotency_key`, `installation_id`, `device_code` — internal correlation/auth primitives. raw 값 echo 금지. **예외**: CLI GitHub device-flow 준비 단계에서 `event: device_code_issued` 를 emit 하면 `verification_uri` (또는 `verification_uri_complete`) + `user_code` 쌍은 humanize 해서 사용자에게 즉시 보여줘요. internal `device_code` 값 자체는 echo 금지.

대신 사용자에게는 한국어 한 줄로 진행 상황만 알려드려요. raw CLI JSON 이 디버깅에 필요한 환경은 `AXHUB_INIT_VERBOSE=1` 이 켜진 경우에만 echo 해요.

## 진행 상황 알림 (Progress Reporting)

각 단계를 시작할 때 친근한 한국어 한 줄로 지금 뭐 하는 중인지 알려줘요 — vibe coder 가 멈춘 게 아니라 진행 중인 걸 알 수 있게 해요. 형식은 `[현재/전체] ○○ 하는 중이에요…`, 끝나면 `○○ 됐어요` 처럼 한 줄로 확인해요.

- 사람이 알아들을 요약만 알려요 — secret·내부 id·raw 출력·schema 본문은 chat 에 넣지 않아요 (위 Visibility Rules 그대로).
- TodoWrite 가 있으면 체크리스트로도 같이 보여주고, 없는 host 에서도 이 한 줄 알림은 늘 해요.

단계 이름 (announce 용 한국어):
- `[1/7] axhub 점검하는 중이에요`
- `[2/7] 작업공간 확인하는 중이에요`
- `[3/7] 템플릿 고르는 중이에요`
- `[4/7] 앱 이름 정하는 중이에요`
- `[5/7] 미리보기 만드는 중이에요`
- `[6/7] 앱 만드는 중이에요` (GitHub 승인이 필요하면 그 대기도 한 줄로 알려요)
- `[7/7] 코드 받아서 정리하는 중이에요`

## Workflow

**한눈에 — 실행 순서.** step 라벨은 히스토리상 순서가 섞여 있으니, 실제 실행은 이 순서로 읽어요:
`1` CLI 가드 → `1a` 버전 체크(신버전 안내) → `0.5` 재진입 resume 확인 → `2` template registry → `2.5` GitHub App 게이트 → `3` template 선택 → `4` 앱 이름 → `5` bootstrap dry-run 미리보기 → `6` 확인 + execute(saga) → `7` repo clone → `8.5` 자동 연결 준비 → `8` 결과 안내 → `9` MCP 설치(선택). (`0` TodoWrite 는 가용 시 전 구간에 걸쳐 갱신.)

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Visible response contract:** when no pending resume state exists, the first visible chat sentence must be exactly "새 앱을 만들 수 있는 템플릿을 확인할게요." Step 0.5 에서 pending resume state 가 있으면 first visible chat sentence 는 이어서 할지 묻는 문장으로 시작해요.

**이 SKILL 이 앱 생성 전체를 담당해요.** "앱 만들어줘" 류 발화에 generic 한 스택/데이터-fetch 질문을 즉석에서 만들지 말고, 반드시 아래 template flow 를 따라요. 스택 선택은 Step 3 (backend template registry), 데이터 접근은 Step 8 의 `@ax-hub/sdk` 안내로 처리해요.

**같은 대화 맥락 이어받기 (carry-over).** 단, **이 대화에 구체 근거**(직전 `connector_query`/`connector_resources`/`row_list`/`table_list` 등 조회 도구의 실제 결과, 또는 이 대화의 온보딩 Ready card)가 보이면 그 맥락을 반영해도 돼요 — generic 즉석 질문을 새로 만드는 게 아니라 **이미 본 것만** 이어받는 거예요. 근거가 없으면 콜드(평소)로 가고, 리소스를 지어내지 않아요. 감지 휴리스틱·confabulation 가드·마찰 억제 범위·D1 가드는 `../deploy/references/session-carryover.md` 를 단일 계약으로 따라요. 조건부 ack("방금 본 `<리소스>` 데이터 반영할게요.")는 조회 근거가 있을 때만, 대화형(D1)에서만 보여줘요.

**대표 여정에서의 역할.** onboarding 이 `VIBE_READY` 또는 `READY_WITH_USER_ACTION` 으로 첫 셋업 상태를 정리한 뒤, 사용자가 "첫 앱 만들어줘" 류로 넘어오면 이 SKILL 이 앱 생성·repo 준비·첫 배포 결과 안내까지 이어요. 실패해도 raw JSON/stderr 를 보여주지 말고, 재개 phrase(`다시 만들어줘`, `다시 로그인해줘`, `설치했어`)로 같은 saga 를 이어갈 수 있게 남겨요.

0. **TodoWrite 진행 체크리스트 (있을 때만).**

   TodoWrite 도구가 현재 host 에 노출돼 있을 때만 호출해요. 없으면 호출하지 말고 조용히 진행해요.

   ```typescript
   TodoWrite({ todos: [
     { content: "CLI와 template registry 확인", status: "in_progress", activeForm: "CLI 확인 중" },
     { content: "template 선택", status: "pending", activeForm: "template 고르는 중" },
     { content: "앱 이름 입력", status: "pending", activeForm: "앱 이름 정하는 중" },
     { content: "bootstrap saga 실행 (app + repo + deploy)", status: "pending", activeForm: "bootstrap 진행 중" },
     { content: "git clone + 자동 연결 + 결과 안내", status: "pending", activeForm: "코드 가져오는 중" }
   ]})
   ```

   매 step 과 매 AskUserQuestion 답변 뒤에 전체 todos 배열로 다시 호출해서 끝난 항목은 `completed` 로 갱신해요. 종료 시점에는 미완료 todo 가 0 개여야 해요.

1. **CLI guard — axhub 존재 + preflight 동작 확인.**

   먼저 `axhub` CLI 가 PATH 에 있는지 보고, 있으면 `axhub plugin-support preflight --json` 이 동작하는지로 게이트해요. 버전 숫자를 직접 비교하지 않아요 — preflight 가 정상 JSON 을 주면 진행하고, `plugin-support` 가 unknown subcommand (clap usage error, exit 64) 면 CLI 가 오래된 거예요.

   ```bash
   if ! command -v axhub >/dev/null 2>&1; then
     echo "axhub CLI가 아직 없네요. 온보딩부터 진행할게요." >&2
     exit 0
   fi
   PREFLIGHT_JSON=$(axhub plugin-support preflight --json 2>/dev/null)
   PREFLIGHT_EXIT=$?
   if [ "$PREFLIGHT_EXIT" = "2" ] || [ -z "$PREFLIGHT_JSON" ]; then
     echo "axhub CLI가 오래됐어요. \`axhub update apply\`로 업데이트한 뒤 다시 시도해 주세요." >&2
     exit 0
   fi
   echo "$PREFLIGHT_JSON"
   ```

   세 갈래예요: (a) `command -v axhub` 없음 → 온보딩 안내 후 멈춰요. (b) CLI 는 있는데 `plugin-support` 가 clap usage error (exit 64) 거나 빈 출력 → "axhub CLI가 오래됐어요. `axhub update apply`로 업데이트한 뒤 다시 시도해 주세요" 안내 후 멈춰요 (최소 0.20.0 필요 — 숫자 비교는 안 하고 preflight 동작 여부로만 판정). (c) preflight JSON 정상 → `auth_ok` 등을 그대로 읽어 계속 진행해요. raw stderr 는 chat 에 노출하지 않아요.

1a. **버전 체크 (맨 처음, best-effort · 비차단 · 10분 TTL).** preflight 가 정상이면 본 작업 전에 axhub CLI·플러그인 새 버전이 있는지 한 번 가볍게 확인해요. 매 호출 네트워크를 피하려 10분 캐시하고, 실패·구 CLI 면 조용히 건너뛰어요 — 앱 생성을 막지 않아요.

   ```bash
   STAMP="${TMPDIR:-/tmp}/axhub-update-check.stamp"
   if [ -z "$(find "$STAMP" -mmin -10 2>/dev/null)" ]; then
     : > "$STAMP"
     PLUGIN_VER=$(grep -o '"version"[^,]*' "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json" 2>/dev/null | head -1 | sed -E 's/.*"version"[^"]*"([^"]+)".*/\1/')
     UPD=$(axhub update check ${PLUGIN_VER:+--plugin-version "$PLUGIN_VER"} --json 2>/dev/null)
   fi
   ```

   `UPD` 의 `has_update`(CLI) / `plugin.has_update`(플러그인) 중 하나라도 true 면 한 줄만 안내한 뒤 이어가요. 둘 다 false 거나 `UPD` 가 비면(캐시 hit·네트워크 실패·구 CLI) 아무것도 안 보여줘요.
   - CLI 새 버전: "axhub CLI 새 버전(`latest`)이 나왔어요 — '업데이트 해줘'라고 하면 적용할게요."
   - 플러그인 새 버전: "axhub 플러그인 새 버전(`plugin.latest`)이 있어요 — `/plugin update` 로 받을 수 있어요."

0.5. **재진입 resume state 를 먼저 확인해요 (proactive resume).**

   Step 1 의 CLI guard 통과 뒤, 새로 시작하기 전에 항상 repo-local saga state 를 확인해요. `.axhub/init-resume.json` 만 plugin-support 로 읽어요.

   ```bash
   axhub plugin-support init-resume route --json
   ```

   JSON: `{route(fresh|watch_status|resume_last), reason, state_stale, requires_status_authority, args{status_command, resume_command}}` (spec §2.2). route 가 `watch_status` 또는 `resume_last` 이고 `clone_done=false` 면 템플릿 목록을 보여주기 전에 먼저 이어서 할지 물어요. raw `bootstrap_id`, `idempotency_key`, repo, slug 값은 chat 에 echo 하지 않아요. slug 는 사용자에게 보이는 앱 별칭으로만 짧게 humanize 해요.

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

   비대화형/D1 guard 에서는 safe default `새로 시작` 으로 진행해요. `이어서 하기` 를 고르면 route enum 과 `args.*_command` 를 그대로 써요. SKILL 이 raw id 를 다시 조합하지 않아요:

   - `watch_status` → `args.status_command` 를 실행해요. 현재 shape 은 `axhub apps bootstrap-status "$BOOTSTRAP_ID" --watch --watch-timeout 9m --json` 예요.
   - `resume_last` → `args.resume_command` 가 base argv 예요. SKILL 은 그 base 를 그대로 쓰고, `$AXHUB_TENANT` env 가 set 돼 있을 때만 `--tenant "$AXHUB_TENANT"` 를 덧붙여요 (battle-tested composition — base 에 tenant 가 없어도 SKILL 이 합성). 합성 후 shape 은 `axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --tenant "$AXHUB_TENANT" --execute --resume-last --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json` 예요.
     - resume 명령이 `no pending github device flow` 로 실패하면 바로 막혔다고 하지 않아요. 먼저 `axhub github accounts list --json` 를 읽기 전용으로 재조회하고, 선택한 GitHub owner 의 installation 이 `installed=true` 또는 `installation_id` 로 확인될 때만 같은 template/name/slug/subdomain/github-owner/repo-name/idempotency-key 로 `--resume-last` 없는 `bootstrap --execute --watch --watch-timeout 9m` 복구를 한 번 실행해요. `device_code_pending` 이거나 owner 설치 확인이 안 되면 fresh execute 를 하지 않아요.
   - 상태가 깨졌거나 오래돼 `fresh` 를 주면 자연어로 "이전 기록을 찾지 못해서 새로 시작할게요." 라고 말하고 Step 2 로 가요.

**Tenant 선택 (axhub-tenant-picker:L1).** `axhub plugin-support tenant-resolve` 가 캐시(`.axhub/state/tenant.json`)/tenants list/preflight 로 tenant 를 결정해요 (exit 0 고정 fail-open). fence 간 env 는 휘발하므로 결정된 tenant 를 캐시에 영속화해서 다음 fence 가 re-read 해요. 명시 `AXHUB_TENANT` override 가 있으면 호출을 건너뛰어요.

```bash
# axhub-tenant-picker:L1 — thin resolver (위험 로직은 axhub plugin-support tenant-resolve 가 소유)
TENANT_CACHE=".axhub/state/tenant.json"
NEEDS_PICK="false"
CANDIDATES_JSON="[]"
if [ -z "${AXHUB_TENANT:-}" ]; then
  # 한 번 호출 → CLI 가 자기 JSON 에서 필드 추출(eval-safe @sh, Git Bash portable — 외부 파서 의존 없음).
  # AXHUB_TENANT/_NEEDS_PICK_RAW/CANDIDATES_JSON 와 비-TTY fallback 용 첫 후보(_FIRST_CANDIDATE)를 한 번에 받아요.
  eval "$(axhub plugin-support tenant-resolve --field-expr '"AXHUB_TENANT=" + (.tenant // "" | @sh), "_NEEDS_PICK_RAW=" + (.needs_pick // false | tostring | @sh), "CANDIDATES_JSON=" + ((.candidates // []) | tojson | @sh), "_FIRST_CANDIDATE=" + ((.candidates // [])[0].id // (.candidates // [])[0].slug // "" | @sh)' 2>/dev/null)"
  : "${AXHUB_TENANT:=}"
  : "${_NEEDS_PICK_RAW:=false}"
  : "${CANDIDATES_JSON:=[]}"
  : "${_FIRST_CANDIDATE:=}"
  if [ "$_NEEDS_PICK_RAW" = "true" ]; then
    if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
      # non-TTY: active fallback + 경고 (R4 fail-wrong guard) — 미리 추출한 첫 후보 사용
      AXHUB_TENANT="$_FIRST_CANDIDATE"
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

`AXHUB_TENANT` 가 비어 있으면 tenant 를 확정할 수 없어요 — preflight `auth_ok` 와 `current_team_id` 를 먼저 확인하고 `다시 로그인해줘` 라고 안내해요.

**Tenant picker (axhub-tenant-picker:L2).** `NEEDS_PICK=true` 이고 대화형 TTY 일 때만 실행해요. `CANDIDATES_JSON` 에서 후보 목록을 읽어 AskUserQuestion 으로 선택을 요청하고, 결과를 `.axhub/state/tenant.json` 에 `{tenant, source:"picker", ts}` 로 기록해요.

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
}
```

**Non-interactive AskUserQuestion guard (D1):** `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 환경에서는 L2 AskUserQuestion 을 건너뛰어요 — L1 블록이 이미 active fallback + 경고를 처리했어요. tenant picker 의 safe default 는 후보 목록의 첫 tenant 예요.

2. **Backend template registry 를 읽어요.**

   ```bash
   # tenant fence re-read — fence 간 env 휘발, tenant-resolve 가 cache-first 로 L1 영속값(.axhub/state/tenant.json)을 재취득
   AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
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

   `axhub apps templates list` 는 backend 명령이지 helper 가 아니에요 — 그대로 써요. `schema_version` 은 internal primitive 라 echo 하지 않아요. `items[]` 의 `id` 또는 built-in alias (`react` / `nextjs` / `astro`) 를 `--template` 인자로 써요.

   on exit 4 (auth 만료) → "다시 로그인해줘"라고 말하면 이어서 처리할 수 있다고 안내해요. on exit 8 (tenant 미해석) → `axhub profile current --json` 안내. 그 외 비정상 종료는 "설치 상태 진단해줘"라고 말하면 점검할 수 있다고 안내해요.

2.5. **GitHub App 설치 확인 — 미설치면 설치까지 막아요 (gate).**

   template 목록이 정상적으로 오면 로그인이 된 거예요. 이어서 GitHub App 설치 상태를 확인해요. 규칙은 두 가지예요: **install_url 은 어떤 상태에서도 무조건 보여줘요.** 그리고 **아무 GitHub 계정에도 설치가 안 됐으면(미설치) 설치·연결이 확인될 때까지 앱 만들기를 막아요.** 이미 어딘가 설치돼 있으면 막지 않고 바로 진행해요.

   왜 막냐면, OAuth device flow (Step 7a) 는 GitHub **사용자 인증**만 해요. repo 를 만들려면 AxHub GitHub App 을 계정/org 에 **설치**하는 별도 1회 단계가 필요한데, 이건 GitHub 웹 UI 에서 사람이 직접 눌러야 해요.

   ```bash
   axhub github accounts list --json   # data.accounts[]: login, installed, install_url
   ```

   먼저 출력 상태를 봐요. **확인할 수 없는 상태에서는 막지 않아요**:

   - 출력이 비었거나 JSON 파싱이 안 되면 (`unavailable`) — 설치 상태를 확인 못 하니 **막지 않고 그대로 진행**해요.
   - auth 에러 envelope 면 — 인증 만료라 "다시 로그인해줘" 로 안내하고 재로그인 후 이 단계를 다시 해요.
   - 정상 응답이면 아래로 진행해요.

   `data.accounts[]` 를 읽어서 **install_url 이 있으면 무조건 먼저 한 줄로 보여줘요 — 설치 여부와 무관하게 항상이에요.** `installation_id` 같은 internal 값은 echo 하지 말고 `login` 과 `install_url` 만 보여줘요.

   **마찰 억제(같은 대화):** 이 대화에서 온보딩이 이미 이 install-link 를 보여줬으면 재안내는 생략해도 돼요. 단 줄이는 건 **재안내뿐**이라, 설치 판정(`accounts list`)·owner-pick(2+ 설치 시 선택)·0-install gate 는 맥락과 무관하게 그대로 실행해요 — 마찰만 줄이고 gate 는 우회하지 않아요. 계약은 `../deploy/references/session-carryover.md`.

   - **install_url (항상 표시)**: "GitHub App 설치·계정 추가 링크: `<install_url>`"

   그 다음 `installed:true` 인 `login` 개수로 갈라요.

   **(A) 설치된 계정이 1개 이상 — 막지 않아요.** owner 를 정해요 (`ambiguous_installation` 예방). App 이 여러 계정/org 에 설치돼 있으면 `--github-owner` 없이는 `ambiguous_installation` (CLI exit 9) 으로 멈춰요:

   - **`AXHUB_GITHUB_OWNER` env 가 있으면** 그 값을 `$GITHUB_OWNER` 로 바로 써요 (질문 없이).
   - **정확히 1개**: 묻지 말고 그 `login` 을 `$GITHUB_OWNER` 로 자동으로 잡아요.
   - **2개 이상**: 아래 질문으로 한 번 물어서 고른 `login` 을 `$GITHUB_OWNER` 로 잡아요.

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

   옵션 label 은 `installed:true` 인 `login` 값으로 채워요 (UI 제한상 최대 3개). 비대화형/D1 guard 에서는 `AXHUB_GITHUB_OWNER` env 가 있으면 그 owner 로 진행하고, 없으면 묻지 않고 safe default `취소` 로 bootstrap 을 멈춰요.

   **(B) 설치된 계정이 0개로 확인됨 (`uninstalled`/`empty`) — 설치까지 막아요 (gate).** install_url 이 있으면 또렷이 다시 보여주고, 빈 목록이라 링크를 못 읽으면 "axhub 대시보드의 GitHub 연결 메뉴에서 AxHub 앱을 설치해요" 로 안내해요. 그런 다음 설치·연결을 요청해요:

   > GitHub App 이 아직 어떤 GitHub 계정에도 설치 안 됐어요. repo 를 만들려면 먼저 설치가 필요해요.
   > 1. 위 링크 `<install_url>` 를 브라우저에서 열어요.
   > 2. repo 를 만들 계정/org 을 고르고 저장소 접근을 승인해요.
   > 3. 끝나면 "설치했어" 라고 알려줘요.

   그리고 아래 질문으로 설치 완료를 기다려요. **설치가 확인되기 전에는 Step 3 (template 선택) 이후로 절대 진행하지 않아요.**

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

   `설치 완료` 를 고르면 `axhub github accounts list --json` 를 다시 읽어요. 이제 `installed:true` 가 보이면 (A) 로 가서 owner 를 정하고 진행해요. 아직 없으면 같은 install_url 을 한 번 더 보여주고 이 질문을 다시 띄워요. `취소` 면 "GitHub App 을 설치하면 '다시 만들어줘' 라고 말해 주세요. 이어서 만들게요." 로 멈춰요.

   **Non-interactive AskUserQuestion guard (D1):** subprocess 에서는 설치 브라우저 단계를 사람이 완료할 수 없어요. 이 gate 의 safe default 는 `취소` 라 bootstrap 을 시작하지 않고, install_url + 재개 phrase(`다시 만들어줘`)를 남기고 멈춰요.

## 템플릿 선택 가이드

이 가이드는 두 번째 registry 가 아니에요. 먼저 `axhub apps templates list --json` 로 backend 가 반환한 template 목록을 읽고, 그 안에 있는 alias / folder_name 에만 설명을 덧붙여요. 선택 값은 반드시 backend 가 반환한 `id` 또는 built-in alias (`react` / `nextjs` / `astro`) 여야 해요.

알 수 없는 새 template 이 backend 에서 오면 숨기지 않아요. 로컬 설명이 없는 항목은 backend `name` 과 `folder_name` 을 그대로 보여주고, "이름을 보고 고르면 돼요. 잘 모르겠으면 먼저 Next.js 추천을 봐요." 처럼 중립 안내만 덧붙여요.

| alias / folder | 이렇게 만들고 싶을 때 골라요 |
|---|---|
| `nextjs` (또는 `nextjs-axhub`) | 쇼핑몰, 예약, 결제, 로그인, 관리자 화면처럼 화면과 기능이 함께 있는 웹서비스를 만들 때 추천해요. |
| `astro` (또는 `astro-axhub`) | 회사 소개, 랜딩 페이지, 블로그, 문서처럼 글과 이미지 중심이고 자주 바뀌지 않는 사이트에 좋아요. |
| `react` (또는 `react-axhub`) | 로그인한 뒤 쓰는 설정 화면, 입력 폼, 관리 화면처럼 버튼을 눌러 내용이 자주 바뀌는 화면에 좋아요. |

backend 가 반환한 template 전체 목록은 먼저 텍스트로 보여줘요. structured AskUserQuestion 은 UI 제한에 맞춰 **최대 3개 선택지** 만 쓰고, 선택지는 모두 실제 backend template 이어야 해요. template picker 에는 `기타` / `Other` / `직접 고르기` / `취소` 같은 generic 선택지를 직접 넣지 말아요.

**Non-interactive AskUserQuestion guard (D1):** subprocess 에서는 AskUserQuestion 을 건너뛰고 안전한 기본값으로 진행해요 — template 선택은 `abort`, 앱 이름은 `abort`, bootstrap 실행 확인은 `취소`, GitHub owner 선택은 `취소`(단 `AXHUB_GITHUB_OWNER` env 가 있으면 그 owner), auto-connect 실행 확인은 `아니요`, resume offer 는 `새로 시작` 예요.

3. **template 을 선택해요.**

   먼저 위 가이드와 backend 가 반환한 template 전체 목록을 텍스트로 보여줘요. 사용자가 발화에 exact alias 또는 backend folder_name 을 이미 적었다면 (예: "nextjs 앱 만들어줘") AskUserQuestion 없이 그 alias 로 진행해요. id 가 없으면 structured AskUserQuestion 은 3개 이하 선택지만 쓰고, 각 option 은 실제 backend template 하나에 대응해야 해요.

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

   위 JSON 은 예시예요. 각 버튼은 backend 가 해당 alias 또는 folder 를 반환할 때만 보여줘요. backend 가 알려진 template 을 3개보다 많이 반환하면 먼저 추천할 실제 template 3개만 버튼으로 만들고, 나머지는 텍스트 목록에서 free-text 입력으로 고를 수 있게 둬요. 사용자가 free-text 로 답한 값이 backend 목록의 exact alias / folder_name / name 과 맞지 않으면 saga 를 시작하지 말고 다시 목록을 보여줘요. subprocess 에서는 자동 선택하지 않아요.

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
   AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --dry-run --json
   ```

   `$GITHUB_OWNER` 가 Step 2.5 에서 정해졌을 때만 `--github-owner` 를 붙여요. 응답 envelope 의 미리보기 카드 (template / slug / subdomain / repo_name / private/public) 를 사용자에게 한국어 한 줄씩 보여줘요. raw JSON dump 금지.

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
   AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
   axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --json
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --execute --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
   ```

   **에이전트도 terminal 까지 폴링해요.** `--watch-timeout` (또는 `--watch-interval`) 을 붙이면 explicit streaming override 라 CLI 가 비-TTY 에서도 terminal(saga 완료 / 실패) 까지 직접 폴링해요. 이 bash 는 Bash tool `timeout: 570000` (9.5분) 으로 호출하고, 9분 초과 시 CLI Timeout + resume hint 를 받으면 "아직 만드는 중이에요, 계속 확인할게요" 후 아래 bootstrap-status 를 한 번 더 호출해요:

   ```bash
   AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
   # idempotency-key 는 axhub 의 agent-safe mutation retry 계약(--help 명시) — 같은 key 라 execute 재호출은 backend 가 dedup 해서 같은 bootstrap_id 를 반환해요(새 saga side-effect 없음). 위 --watch 호출이 device flow + saga 진행을 처리하고, 이 호출은 bootstrap_id 한 줄만 추출해요.
   BOOTSTRAP_ID=$(axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" ${GITHUB_OWNER:+--github-owner "$GITHUB_OWNER"} --tenant "$AXHUB_TENANT" --execute --idempotency-key "$IDEMPOTENCY_KEY" --field-expr '.data.bootstrap_id // empty' 2>/dev/null || true)
   axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --bootstrap-id "$BOOTSTRAP_ID" --json
   axhub apps bootstrap-status "$BOOTSTRAP_ID" --tenant "$AXHUB_TENANT" --watch --watch-timeout 9m --json
   ```

   진행 중 매 ~30s 마다 한국어 한 줄로 narrate 해요 — "앱 만들고 있어요" / "GitHub repo 만들고 있어요" / "첫 배포 중이에요. 거의 다 왔어요".

   **GitHub 연결 필요 (CLI stdout 의 `device_code_issued` event 처리).** CLI 가 GitHub App 미설치 / installation 만료 / scope 부족 상태를 해결하려고 device flow 를 시작하면, backend saga 를 시작하기 전에 다음 JSON line 을 emit 해요:

   ```json
   {"event":"device_code_issued","data":{"verification_uri":"https://github.com/login/device","verification_uri_complete":null,"user_code":"XXXX-XXXX","expires_in":899}}
   ```

   이 event 가 나오면 `axhub plugin-support init-resume put ... --pending-device-flow true --json` 로 state 를 갱신한 뒤 즉시 사용자에게 한국어로 안내해요. raw JSON dump 금지 — `verification_uri` (또는 non-null 이면 `verification_uri_complete`), `user_code`, `expires_in` 만 humanize 해요:

   > GitHub 연결이 필요해요. 다음 단계로 진행해 주세요:
   >
   > 1. 브라우저에서 열기: `<verification_uri_complete 우선, 없으면 verification_uri>`
   > 2. 코드 입력: `<user_code>`
   > 3. axhub GitHub App 설치 승인
   >
   > 브라우저에서 승인한 다음 '승인했어' 라고 알려주세요. 제가 이어서 마무리할게요. (유효시간 약 `<expires_in/60>` 분)

   **대화형 TTY** 면 saga 가 polling 으로 install 완료를 기다리니 SKILL 은 다음 stage event 까지 narrate 만 계속해요. **에이전트 / 비-TTY** 면 CLI 가 emit 직후 fast-exit 하므로, challenge 를 보여준 뒤 브라우저 승인을 기다려요. 사용자가 승인 신호("승인했어" / "연결했어" / "됐어")를 주면 에이전트가 캐시된 device flow 를 `--resume-last` 로 직접 이어받아요:

   ```bash
   AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --tenant "$AXHUB_TENANT" --execute --resume-last --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
   ```

   **outstanding code 가 있는 동안 `--resume-last` 없이 fresh `bootstrap --execute` 를 다시 호출하지 말아요 — 새 code 를 발급해 이미 승인한 code 를 버려요.** resume 응답이 아직 `device_code_pending` 이면 "브라우저 승인이 아직 안 끝난 것 같아요. 승인 후 다시 알려주세요" 후 승인 신호를 받으면 한 번 더 resume 해요. resume 응답이 `no pending github device flow` 이면 `axhub github accounts list --json` 로 선택한 owner 설치가 확인될 때만 같은 idempotency key 로 아래 복구 명령을 한 번 실행해요:

   ```bash
   AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
   axhub github accounts list --json
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --github-owner "$GITHUB_OWNER" --repo-name "$APP_SLUG" --repo-private --tenant "$AXHUB_TENANT" --execute --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
   ```

   device code 가 만료(약 15분)됐으면 이 Step 의 fresh `--execute` 부터 새 challenge 를 발급해요. 상세 device flow 안내 패턴은 GitHub 연결 surface 설계를 따라요.

7. **응답에서 `repo_full_name` 을 꺼내 CWD 로 받아요.**

   ```bash
   # 완료된 saga 의 bootstrap-status 는 read-only — repo_full_name 한 줄만 CLI 가 추출
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

   항상 현재 dir (CWD) 에 코드를 받아요. 서브 dir 만들지 않아요. 이미 `.git` 이 있는 dir 은 안전을 위해 자동 clone 건너뛰고 수동 명령 안내를 한 줄로 보여줘요. clone 실패 시 `repo_full_name` 만 알려주고 수동 clone 안내를 보여줘요.

8.5. **clone 직후 자동 연결을 준비해요 (post-scaffold auto-connect).**

   clone 이 성공하면 먼저 resume state 에 `clone_done=true` 를 기록하고 clear 해요. 그 다음 로컬 실행 가능 여부를 scaffold-detect 로 감지해요.

   ```bash
   axhub plugin-support init-resume put --template "$TEMPLATE" --app-name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --idempotency-key "$IDEMPOTENCY_KEY" --bootstrap-id "$BOOTSTRAP_ID" --repo-full-name "$REPO" --clone-done true --json
   axhub plugin-support init-resume clear --json
   axhub plugin-support scaffold-detect --json
   ```

   `scaffold-detect` JSON: `schema_version="scaffold-detect/v1"`, `package_json_present/lockfile_present/manager/node_available/dev_script_present/can_install/can_start_dev/install_command/dev_command/reason` (spec §2.2). `package.json` 이 없거나 lockfile 이 없거나 dev script 가 없으면 로컬 미리보기는 건너뛰고 Step 8 로 가요. node 가 없으면 "앱 실행 준비를 도와드릴게요" 라고 말하고 onboarding 으로 넘겨요.

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

   비대화형/D1 guard 에서는 safe default `아니요` 로 넘어가요. `네, 실행까지` 면 scaffold-dev 를 실행해요. 내부 install 은 lockfile 이 있을 때만, 항상 `--ignore-scripts` 를 붙여 실행해요.

   ```bash
   axhub plugin-support scaffold-dev start --json
   ```

   `scaffold-dev` JSON: `schema_version="scaffold-dev/v1"`, `action/alive/ready/reason/pid/url/port/exit_code` (spec §2.2). 결과는 자연어로만 보여줘요:

   - 성공/이미 실행 중 → "로컬 미리보기도 떠 있어요." + `url` 이 있으면 보조로 안내
   - install/dev 실패 → "미리보기 자동 실행이 잠깐 안 됐어요. '다시 해줘' 하면 재시도할게요"
   - lockfile 없음 → "코드는 준비됐어요" + localhost 생략

8. **결과와 다음 액션을 안내해요.**

   saga 응답의 `app_id` / `deployment_id` / `repo_full_name` 을 humanize 해서 한국어 한 줄씩 보여줘요. **배포 공개 URL** 은 배포 성공 후 `axhub apps get` 으로 앱의 `access_url` 을 읽어서 hero 첫 줄로 보여줘요. URL 은 절대 합성하지 않아요.

   ```bash
   PUBLIC_URL="$(axhub apps get "$APP_ID" --no-input --field-expr '.access_url // .data.access_url // empty' 2>/dev/null || true)"
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

   (업데이트 알림은 Step 1a 에서 맨 앞에 처리하니 여기선 다시 안 해요.)

   확인된 공개 URL 이 없으면 "인터넷 배포가 시작됐어요. '방금 배포 어디까지 됐어?' 라고 물으면 이어서 확인할게요." 라고 낮춰 말해요.

   `error_code` 로 saga 가 실패했으면 다음 routing 을 써요:
   - `conflict` / `ambiguous_installation` (CLI exit 9) → 먼저 **install_url 을 한 줄로 보여주고**(무조건), 이어서 Step 2.5 의 owner 선택("어느 GitHub 계정에 저장소를 만들까요?")을 다시 띄워요. 고른 `login` 을 `$GITHUB_OWNER` 로 잡고 같은 idempotency key 로 Step 6 execute 를 `--github-owner "$GITHUB_OWNER"` 와 함께 한 번 다시 실행해요. 비대화형/D1 에서는 `AXHUB_GITHUB_OWNER` env 가 있으면 그 owner 로 재실행하고, 없으면 `취소` 로 멈춰요.
   - `github.installation_missing` / `github.repo_create_failed` → "GitHub 연결 다시 해줘"라고 말하면 이어서 처리할 수 있다고 안내
   - `validation.template_not_found` → Step 2 로 돌아가 다시 목록을 보여줘요
   - `validation.slug_collision` → Step 4 로 돌아가 새 이름을 받아요
   - `auth` (CLI exit 4) → "다시 로그인해줘"
   - `forbidden` / `tenant_scope` (CLI exit 12 / 8) → 권한 부족 안내 + workspace admin 문의
   - 그 외 → "설치 상태 진단해줘"

   앱이 정상 생성됐으면, 방금 만든 코드에서 필요한 테이블·환경변수를 추천받을지 AskUserQuestion 으로 한 번 물어봐요(비대화형/D1 guard 에서는 묻지 않고 safe default `아니요`).

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

   `네, 추천받기` 면 infer-tables-env 분석으로 넘어가요. `아니요` 면 그냥 마무리해요. infer-tables-env 분석은 scaffold 코드뿐 아니라 **이 대화에서 실제 조회한 리소스**(connector/table 결과가 컨텍스트에 있을 때)도 함께 보고 테이블·env·필요하면 템플릿을 추천해요. 조회 근거가 없으면 코드 기준으로만 추천하고 carry-over 를 주장하지 않아요 (`../deploy/references/session-carryover.md`).


## NEVER

- NEVER GitHub App 이 아무 계정에도 설치 안 된 상태(`github.state` = uninstalled/empty)에서 Step 3 이후로 진행하거나 bootstrap dry-run/execute 를 호출하지 않아요. Step 2.5 gate 에서 설치가 확인될 때까지 멈춰요.
- NEVER `axhub init` 또는 `axhub init --from-template` 을 호출하지 않아요. `--from-template` flag 가 미구현 stub 라 generic docker manifest 만 만들어져요. SKILL 은 `axhub apps bootstrap` saga 만 써요.
- NEVER `axhub apps create` 또는 `axhub deploy create` 를 직접 호출하지 않아요. bootstrap saga 가 server-side 에서 둘 다 처리해요.
- NEVER remote `templates.json` 또는 폐기된 fetch-template 을 source 로 쓰지 않아요. backend `axhub apps templates list` 만 source-of-truth 예요.
- NEVER subprocess (`$CI` / `$CLAUDE_NON_INTERACTIVE` / no TTY) 에서 template 또는 앱 이름을 임의로 고르지 않아요. safe default 가 `abort` 또는 `취소` 예요.
- NEVER `--execute` 를 `--dry-run` 미리보기 + 사용자 확인 없이 호출하지 않아요.
- NEVER auth 만료를 template 조회 실패로 오해하지 않아요. CLI auth 실패 (exit 4 / error_code `auth`) 는 "다시 로그인해줘".
- NEVER `bootstrap --execute` 호출 직후 별도 `axhub deploy create` 를 다시 부르지 않아요. saga 가 첫 deploy 까지 포함해요.
- NEVER saga stdout 에서 `event: device_code_issued` 가 나왔는데 `verification_uri` + `user_code` 를 즉시 보여주지 않고 silent 하게 narrate 만 반복하지 않아요.
- NEVER `repo_full_name` 응답이 비어 있는데 임의 URL 을 만들어 clone 시도하지 않아요.
- NEVER shell 에서 CLI 버전 숫자를 직접 파싱·비교하지 않아요. 버전 게이트는 `axhub plugin-support preflight --json` 이 동작하는지(정상 JSON vs clap usage error exit 64)로만 판정해요.

## Additional Resources

- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template (Step 8 에서 사용).
- `../deploy/references/session-carryover.md` — 같은 대화 조회·온보딩 맥락 carry-over·confabulation 가드·마찰 억제 단일 계약.
- `../onboarding/SKILL.md` — 빈 폴더 외 gap 처리·CLI 부재 라우팅 source.
