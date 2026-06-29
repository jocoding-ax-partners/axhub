---
name: deploy
description: '이 스킬은 사용자가 이미 axhub에 연결된 현재 브랜치를 axhub 라이브로 다시 배포하고 싶어할 때 사용해요. 비어 있지 않은 기존 로컬 앱을 처음 axhub에 등록·연결·첫 배포까지 가져오는 요청은 import 스킬이 담당해요. 다음 표현에서 활성화: "공개해", "내보내자", "띄워", "배포", "배포해", "배포해줘", "쏘자", "올려", "올리자", "터트려", "푸시한 거 띄워", "프로덕션", "프로덕션에 박아", "demo가 필요", "demo가 필요해", "deploy", "launch", "release", "rollout", "ship", 또는 이미 연결된 axhub 앱의 현재 브랜치를 라이브로 올리고 싶다는 의도. 안전한 배포 준비 확인, 라이브 profile/app 해석, AskUserQuestion preview card 를 통한 AskUserQuestion preview-confirm gate, exit-code 기반 복구 라우팅을 담당해요. 경계: 사용자가 배포 실패 원인 진단을 명시하면 diagnosis 가 맡고, 기존 로컬 앱의 첫 연결·첫 배포는 import 가 맡고, 이 스킬은 연결된 앱의 새 배포·재배포·검증만 맡아요.'
examples:
  - utterance: "paydrop 배포해"
    intent: "deploy current branch to axhub live"
  - utterance: "어쨌든 배포 미리보기 확인하고 진행해"
    intent: "deploy current branch to axhub live"
  - utterance: "ship paydrop"
    intent: "deploy current branch to axhub live"
  - utterance: "/axhub:deploy paydrop --commit abc123"
    intent: "deploy specific commit to axhub live"
  - utterance: "이전에 승인한 거 그대로 다시 배포해"
    intent: "deploy current branch to axhub live"
allows-dependency-execution: false
model: sonnet
---

# Deploy via axhub

Deploy a vibe coder's app to axhub with safety primitives. `axhub plugin-support` 명령들 (preview, recovery planning) 과 공개 `axhub deploy` 명령으로 진행해요. preview-confirm flow 없이 `axhub deploy create` 를 직접 호출하지 않아요.
명시적인 배포 실패 원인 진단 요청(예: "배포 실패 원인 진단해줘", "왜 배포가 죽었어")은 `diagnosis` 에 양보해요. 이 스킬은 원인 분석만 하려고 deploy/verify 를 다시 실행하지 않아요.

`import` 스킬과 경계를 나눠요. `import` 는 기존 로컬 앱의 첫 axhub 연결과 첫 배포 성공 증거를 담당하고, 이 스킬은 연결된 앱의 ordinary redeploy 를 담당해요. import 의 static terminal success(`active_release_id` + `verified` + `public_url`)는 import envelope 에서 검증하고, deploy 는 deployment-record 경로의 `axhub deploy verify <deployment-id>` 계약과 static redeploy 경로의 `active_release_id` + 공개 URL 확인 계약을 각각 지켜요.

## 절대 규칙 — 길어도 매 응답·최종 메시지에 먼저 적용해요

이 SKILL 은 길어요. 그래도 아래 둘은 **어떤 단계·어떤 모델에서도 우선**해요 (헤드리스 회귀의 핵심):

1. **헤드리스 = AskUserQuestion 0회.** `claude -p`·CI·`$CLAUDE_NON_INTERACTIVE`·TTY 없음이면 헤드리스예요. 헤드리스에선 preview 카드를 포함해 **어떤 AskUserQuestion 도 호출하지 않고** `--dry-run` safe default 로 바로 진행해요. (아래 "AUQ preview 카드 필수/우회 금지" 류 NEVER 규칙은 **대화형 `--execute` 에만** 적용돼요 — 헤드리스는 `--execute` 자체를 안 하니 AUQ 도 안 불러요.)
2. **최종 사용자 메시지 = 한국어 한 줄 요약 + 다음 행동.** 마지막 텍스트에 raw id(`app-123`)·exit 숫자(`exit 65`)·내부 이름(`plugin-support`·`deploy-prep`·`preflight`·`quality gate`·`in-flight`)·dry-run 내부 덤프를 **절대 넣지 않아요**. 디버그 요약 대신 감정+원인+다음 행동만 보여줘요 (상세 template: 아래 Vibe Coder Visibility Rules).

## Claude Desktop Natural-Language Path

When the user says a human deployment phrase such as `배포해줘`, `올려줘`, or `프로덕션에 띄워줘`, keep the visible conversation human:

- The first visible chat sentence must be exactly `배포 준비를 확인할게요.`
- For the initial Desktop preview, stop reading this skill after this section. Do not read the long workflow below until the user has approved the preview card.
- Before the Bash/tool call, make sure the command runs in the user-visible app folder. In Claude Desktop, if the active root and an added folder differ and the added folder is the only Vite/React app (`package.json` has `vite` + `react`/`react-dom`), run the command from that folder (`cd "<that folder>" && ...`). If multiple app folders are plausible, ask which folder to deploy and stop.
- If stdout says `axhub 매니페스트(axhub.yaml)가 없어요.`, do not initialize files from deploy. Stop with Korean guidance: non-empty existing app → `기존 앱 올려`, empty directory new template → `새 앱 만들어줘`.
- Immediately run one Bash/tool call with title `배포 준비 확인`: `axhub plugin-support deploy-preview-summary --user-utterance "<latest user sentence>"`.
- Copy that Korean stdout as the preview card and ask for explicit approval.
- After the user explicitly approves, continue into the canonical workflow below starting at Step 1.1 with the approval decision already captured. Use `deploy-prep`, git readiness/status-first checks, token gate, public `axhub deploy create --execute --json` when redeploy creation is actually required, and Step 5 verify. Do not insert a separate approved-run helper bridge between preview approval and the canonical workflow.
- Bind `DEPLOY_ID` only from an in-flight deployment id or public `axhub deploy create --execute --json` output. If no deployment id is present, do **not** declare success; say "배포 시작은 확인했지만 결과 확인 id 를 못 받았어요. '배포 상태 확인해줘'라고 말하면 이어서 볼게요." and stop.
- Confirm success only with `axhub deploy verify "$DEPLOY_ID"` (Step 5). Copy verify-derived Korean success/recovery text, not raw deploy-create stdout, as the final deploy result. Do not call this skill again after approval.
- Do not echo the user's phrase as a route conversion, such as `"배포해줘" → ...`.
- If a Bash/tool call is needed, use Korean titles only: `배포 준비 확인`, `배포 실행`, or `배포 상태 확인`.
- Before any destructive deploy, show only the Korean preview card (앱 / 환경 / 브랜치 / 커밋 / 예상 시간) and ask for explicit approval.
- If login is expired or missing, explain whether login is needed in Korean and ask before starting a login flow.

## Vibe Coder Visibility Rules

이 SKILL 을 쓰는 사람은 대부분 개발 지식이 없어요. 다음 field 는 **internal verification primitives** 예요. SKILL 안에서는 변수에 담아 주고받되, **raw 값을 사용자 chat 에 echo 하면 안 돼요**:

- `pending_action_id`, `pending_action_hash`, `command_argv`, `command_id`
- `retry_policy`, `idempotency_key`
- `exit_code`, `next_action`, `schema_version`, `stdout_json`, `stderr` (raw)
- `bootstrap_plan`, `required_steps`, `decision_inputs`
- `active_release_id`, `deploy_method`, `release_id`, `manifest` (static lane 내부 검증 primitive)

대신 사용자에게는 한국어 한 줄로 진행 상황만 알려드려요. 예시 templates:

| 시점 | 사용자 chat 한 줄 |
|------|-------------------|
| Step 1 first-run boundary | "이 폴더는 먼저 가져오기 또는 새 앱 만들기 흐름으로 이어가야 해요." |
| Step 1.5 git 저장 지점 준비 | "배포 전에 파일 저장 지점을 만들어두고 있어요." |
| Step 3 preview card | 5필드 한국어 카드만 (앱 / 환경 / 브랜치 / 커밋 / 예상 시간) |
| Step 4 approval → deploy | "배포 확인을 받았어요. 시작해요." |
| Step 5 verify | "배포 결과를 확인하고 있어요." |
| Step 6 exit 4 / 65 | "axhub 로그인이 만료됐어요. 다시 로그인할까요?" |
| Step 6 exit 64 | "다른 배포가 진행 중이라 지금은 못 올려요. 잠시 뒤에 다시 시도해요." |
| Step 6 exit 5 / 67 | "이 이름의 앱을 못 찾았어요. 비슷한 이름을 알려드릴게요." |

raw JSON 이 디버깅에 필요한 환경은 `AXHUB_DEPLOY_VERBOSE=1` 이 켜진 경우에만 echo 해요.

## 진행 상황 알림 (Progress Reporting)

각 단계를 시작할 때 친근한 한국어 한 줄로 지금 뭐 하는 중인지 알려줘요 — vibe coder 가 멈춘 게 아니라 진행 중인 걸 알 수 있게 해요. 형식은 `[현재/전체] ○○ 하는 중이에요…`, 끝나면 `○○ 됐어요` 처럼 한 줄로 확인해요.

- 사람이 알아들을 요약만 알려요 — secret·내부 id·raw 출력·schema 본문은 chat 에 넣지 않아요 (위 Visibility Rules 그대로).
- TodoWrite 가 있으면 체크리스트로도 같이 보여주고, 없는 host 에서도 이 한 줄 알림은 늘 해요.

단계 이름 (announce 용 한국어):
- `[1/5] axhub 점검하는 중이에요`
- `[2/5] 배포 대상 확인하는 중이에요` (routing·작업공간·deploy-prep)
- `[3/5] 미리보기 보여줄게요`
- `[4/5] 배포하는 중이에요`
- `[5/5] 배포 결과 확인하는 중이에요` (verify)

## Workflow

**한눈에 — 실행 순서.** step 라벨은 히스토리상 순서가 섞여 있으니, 실제 실행은 이 순서로 읽어요:
`1` CLI 가드 → `1a` 버전 체크(신버전 안내) → `1.1` deploy-prep(resolve+preflight) → `1.2` static 분기(deploy_method=static 이면 독립 lane) → `0` TodoWrite(1.1 결과로 도출) → `1.1b` first-run 경계(import/init 위임) → `1.5` git 저장 준비 → `1.6` in-flight 감지 → `1.7` status-first 게이트 → `2` preview 결정(headless) → `3` preview 카드 → `3.5` 토큰 게이트 → `4` deploy create(fallback) → `5` verify(성공 선언) → `6` 비-0 에러 라우팅 / `7` dry-run 트리거.

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**배포 직전 선택 점검(infer-tables-env 연계).** 배포 본 단계로 들어가기 전에, 코드에서 필요한 테이블·환경변수를 먼저 추천받을지 AskUserQuestion 으로 한 번 물어봐요. 비차단이라 헤드리스/비대화형에서는 묻지 않고 safe default(`아니요, 바로 배포`)로 그냥 배포를 이어가요. **development skill 이 같은 대화에서 이미 배포 준비 점검을 마쳤으면**(carry-over: `references/session-carryover.md` 의 "배포 준비 점검 완료" 근거가 이 대화에 보임) 이 질문을 건너뛰고 safe default 로 바로 배포를 이어가요 — 중복 질문 방지.

```json
{
  "questions": [{
    "question": "배포 전에 코드에서 필요한 테이블·환경변수를 먼저 추천받을래요?",
    "header": "사전 점검",
    "multiSelect": false,
    "options": [
      {"label": "아니요, 바로 배포", "description": "점검 건너뛰고 바로 배포를 이어가요"},
      {"label": "네, 먼저 추천받기", "description": "코드 분석으로 필요한 테이블·env 를 먼저 추천받아요"}
    ]
  }]
}
```

`네, 먼저 추천받기` 면 infer-tables-env 분석으로 넘어가 추천을 보여준 뒤 배포로 돌아와요. 어느 쪽이든 배포를 막지 않아요.

**Headless first rule.** `claude -p`, CI, `$CLAUDE_NON_INTERACTIVE`, or an unavailable/denied AskUserQuestion tool means headless mode 예요. In headless mode:

- Do not call AskUserQuestion. Do not render numbered choices and stop.
- Use the safe default immediately.
- For Step 3 preview, force `DEPLOY_DECISION=dry_run`.
- Continue with the Bash dry-run command path so the QA run sees real CLI/auth behavior without mutating production.

1. **CLI guard — axhub 존재 + preflight 동작 확인.**

   `axhub` CLI 가 PATH 에 있는지 먼저 보고, 있으면 `axhub plugin-support preflight --json` 이 동작하는지로 게이트해요. 버전 숫자를 직접 비교하지 않아요.

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

   세 갈래예요: (a) `command -v axhub` 없음 → 온보딩 안내 후 멈춰요. (b) CLI 는 있는데 `plugin-support` 가 clap usage error (exit 64) 거나 빈 출력 → "axhub CLI가 오래됐어요. `axhub update apply`로 업데이트한 뒤 다시 시도해 주세요" 안내 후 멈춰요 (최소 0.20.0 필요 — 숫자 비교는 안 하고 preflight 동작 여부로만 판정). (c) preflight JSON 정상 → 계속 진행해요. `auth_ok` 가 false 면 인증 상태를 설명하고 `다시 로그인해줘` 라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어 복구 안내를 붙여요. raw stderr 는 chat 에 노출하지 않아요.

1a. **버전 체크 (맨 처음, best-effort · 비차단 · 10분 TTL).** preflight 가 정상이면 본 배포 작업 전에 axhub CLI·플러그인 새 버전이 있는지 한 번 가볍게 확인해요. 매 호출 네트워크를 피하려 10분 캐시하고, 실패·구 CLI 면 조용히 건너뛰어요 — 배포를 막지 않아요.

   ```bash
   STAMP="${TMPDIR:-/tmp}/axhub-update-check.stamp"
   if [ -z "$(find "$STAMP" -mmin -10 2>/dev/null)" ]; then
     : > "$STAMP"
     PLUGIN_VER=$(grep -o '"version"[^,]*' "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json" 2>/dev/null | head -1 | sed -E 's/.*"version"[^"]*"([^"]+)".*/\1/')
     UPD=$(axhub update check ${PLUGIN_VER:+--plugin-version "$PLUGIN_VER"} --json 2>/dev/null)
   fi
   ```

   `UPD` 의 `has_update`(CLI) / `plugin.has_update`(플러그인) 중 하나라도 true 면 한 줄만 안내한 뒤 배포를 이어가요. 둘 다 false 거나 `UPD` 가 비면(캐시 hit·네트워크 실패·구 CLI) 아무것도 안 보여줘요.
   - CLI 새 버전: "axhub CLI 새 버전(`latest`)이 나왔어요 — '업데이트 해줘'라고 하면 적용할게요."
   - 플러그인 새 버전: "axhub 플러그인 새 버전(`plugin.latest`)이 있어요 — `/plugin update` 로 받을 수 있어요."

**Routing 게이트 (Step 1 직후, resolve 전에 실행).** 이 SKILL 은 `description:` 의 "배포"·"deploy"·"ship" 어구로 자동 선택돼서, 다른 배포 타깃(`vercel` 등)을 쓰려는 발화나 axhub 와 무관한 프로젝트에도 끌려올 수 있어요. 두 단계로 걸러요 — **(A) 명시 타깃 판별(LLM)** → **(B) axhub 맥락 게이트(route-decision)** — 둘 다 통과해야 deploy-prep 으로 가요.

**(A) 명시 타깃 판별 (named-target-wins — LLM 이 직접 판단, CLI 위임 금지).** 발화에 axhub 가 아닌 다른 배포 타깃이 **명시**돼 있으면 (예: `vercel`, `netlify`, `fly`/`fly.io`, `cloudflare`/`pages`, `render`, `railway`, `heroku`, `aws`/`amplify`, `gcp`, `azure`, `vps`, `github pages` 등) axhub 배포를 멈추고 양보해요. 한 줄만 안내하고 **deploy-prep·`axhub deploy create` 를 하나도 호출하지 말아요**: "다른 배포 타깃(`<target>`)을 쓰려는 것 같아서 axhub 배포는 건너뛸게요." 타깃 판별은 자연어 이해라 LLM 이 해요 — route-decision 은 이걸 판별하지 않아요. axhub 를 명시했거나(`axhub 에 배포`/`/axhub:deploy`·`/배포` 슬래시) 타깃 언급이 없으면 (B) 로 가요.

**(B) axhub 맥락 게이트 (route-decision).** 타깃을 안 가린 발화가 정말 axhub 배포 맥락인지 (`axhub.yaml` marker·로그인 상태) 확정해요. `EXPLICIT` 은 호출 모달리티예요 — `/deploy`·`/axhub:deploy`·`/배포` 슬래시면 `EXPLICIT=1`, 자연어면 `EXPLICIT=0`, 모호하면 `EXPLICIT=1`. `$ARGS` 에는 사용자 발화 원문을 그대로 담아요 (뒤 deploy-prep 의 resolve 가 slug 후보로 써요).

```bash
EXPLICIT_FLAG=""
[ "${EXPLICIT:-0}" = "1" ] && EXPLICIT_FLAG="--explicit"
# fail-open: 빈/실패면 axhub 로 진행해요 — 실제 배포는 뒤의 preview-confirm gate 가 막아요.
ROUTE_DECISION=$(axhub plugin-support route-decision --user-utterance "$ARGS" $EXPLICIT_FLAG --field-expr '.decision // "axhub"' 2>/dev/null || echo axhub)
```

`route-decision` 의 binding 계약은 `.decision` 단일이에요. 현재 CLI 는 marker/auth 기반이라 실제로 **`axhub` 또는 `ignore`** 만 내요 (NL 타깃 판별은 (A) 가 이미 했고, 이 호출은 keyword 판정을 하지 않아요). exit 0 고정 fail-open (빈 출력 → `axhub`, 뒤 preview-confirm 이 backstop). `ROUTE_DECISION` 값으로 분기해요. **`axhub` 일 때만 진행**해요:

- **`axhub`** → 정상 경로. 아래 Step 1.1 (deploy-prep) 로 계속 진행해요. **이 경로(axhub 확정)로 들어온 뒤에만**, 이 대화에서 온보딩/리소스 맥락이 보이면 중복 재설명을 줄이고 의도를 한 줄로 이어요 — route gate 통과 후에만 적용해서 다른 타깃(vercel 등) 핸드오프를 억제하지 않아요. 배포 결정·verify 경로는 그대로예요. 계약은 `references/session-carryover.md`.
- **그 외 (`ignore` 등 axhub 가 아닌 모든 값)** → axhub 배포 맥락인지 확실치 않아요 (`axhub.yaml` 없음 등). 아래 AskUserQuestion 으로 한 번 물어봐요. **물어보기 전에는 deploy-prep 을 호출하지 말아요.**

```json
{
  "question": "axhub 에 배포할까요, 아니면 다른 곳에 배포할까요?",
  "header": "배포 대상",
  "options": [
    {"label": "axhub 에 배포", "value": "axhub", "description": "axhub 라이브로 배포를 이어가요."},
    {"label": "여기 말고 다른 곳", "value": "other", "description": "axhub 배포를 멈춰요. 다른 배포 도구를 쓸게요."}
  ]
}
```

subprocess 에서는 질문을 건너뛰고 deploy 의 safe default ("여기 말고 다른 곳") 로 멈춰요.

**Tenant 선택 (axhub-tenant-picker:L1).** `axhub plugin-support tenant-resolve` 가 캐시(`.axhub/state/tenant.json`)/tenants list/preflight 로 tenant 를 결정해요 (exit 0 고정 fail-open). 명시 `AXHUB_TENANT` override 가 있으면 호출을 건너뛰어요.

```bash
# axhub-tenant-picker:L1
TENANT_CACHE=".axhub/state/tenant.json"
NEEDS_PICK="false"
CANDIDATES_JSON="[]"
if [ -z "${AXHUB_TENANT:-}" ]; then
  # tenant-resolve 한 번 호출 → CLI 가 자기 JSON 에서 필요한 field 를 eval 안전(@sh) 하게 뽑아줘요 (jq 의존 없음).
  eval "$(axhub plugin-support tenant-resolve --field-expr '"AXHUB_TENANT=" + (.tenant // "" | @sh), "_NEEDS_PICK_RAW=" + (.needs_pick // false | tostring | @sh), "CANDIDATES_JSON=" + (.candidates // [] | tojson | @sh), "_CAND_FIRST=" + (.candidates // [] | (.[0].id // .[0].slug // "") | @sh)' 2>/dev/null)"
  : "${AXHUB_TENANT:=}"; : "${_NEEDS_PICK_RAW:=false}"; : "${CANDIDATES_JSON:=[]}"; : "${_CAND_FIRST:=}"
  if [ "$_NEEDS_PICK_RAW" = "true" ]; then
    if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
      AXHUB_TENANT="$_CAND_FIRST"
      echo "여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant($AXHUB_TENANT)로 진행해요"
    else
      NEEDS_PICK="true"
    fi
  fi
fi
if [ -n "${AXHUB_TENANT:-}" ] && [ "$NEEDS_PICK" = "false" ]; then
  mkdir -p "$(dirname "$TENANT_CACHE")"
  printf '{"tenant":"%s","source":"resolved","ts":%s}\n' "$AXHUB_TENANT" "$(date +%s 2>/dev/null || echo '0')" > "$TENANT_CACHE"
fi
export AXHUB_TENANT
export NEEDS_PICK
export CANDIDATES_JSON
```

**Tenant picker (L2).** `NEEDS_PICK=true` 이고 대화형 TTY 일 때만 `CANDIDATES_JSON` 으로 AskUserQuestion 을 띄워 선택을 받고, `.axhub/state/tenant.json` 에 `{tenant, source:"picker", ts}` 로 기록해요. subprocess 에서는 L1 의 active fallback (후보 첫 tenant) 을 써요.

To deploy:

0. **TodoWrite 진행 체크리스트 — 실제 재배포 경로에서 도출해요 (고정 목록 붙여넣기 금지).** TodoWrite 가 host 에 있을 때만 호출해요. 항목은 Step 1.1 의 `deploy-prep` 결과에 따라 달라져요: git-connected 앱의 push 자동배포는 자기가 만들지 않은 배포를 **watch** (status-first), 이미 axhub 앱으로 연결된 non-git 앱은 명시 승인 후 재배포용 `deploy create` 를 실행해요. first-connect / app 등록 / manifest 초기화가 필요하면 `import` 또는 `init` 으로 양보하고 여기서 멈춰요. 상황을 읽고 그에 맞는 todos 를 써요. 참고 shape A — git-connected / status-first watch:

   ```typescript
   TodoWrite({ todos: [
     { content: "배포 상태 확인 (preflight)", status: "in_progress", activeForm: "배포 상태 확인하는 중" },
     { content: "최신 저장 지점 푸시 확인",     status: "pending",     activeForm: "푸시 상태 보는 중" },
     { content: "자동 시작된 배포 따라가기",     status: "pending",     activeForm: "배포 따라가는 중" },
     { content: "결과 확인 (verify)",          status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   참고 shape B — manual redeploy / non-git existing app:

   ```typescript
   TodoWrite({ todos: [
     { content: "토큰 확인 (preflight)",         status: "in_progress", activeForm: "토큰 확인하는 중" },
     { content: "앱 / 환경 / 브랜치 확정",         status: "pending",     activeForm: "앱 정보 정리하는 중" },
     { content: "git 저장 지점 확인",             status: "pending",     activeForm: "배포용 저장 지점 보는 중" },
     { content: "미리보기 카드 보여드리기",         status: "pending",     activeForm: "미리보기 준비하는 중" },
     { content: "동의 받고 재배포 시작",            status: "pending",     activeForm: "재배포 시작하는 중" },
     { content: "결과 확인 (verify)",             status: "pending",     activeForm: "확인하는 중" }
   ]})
   ```

   매 step 과 매 AskUserQuestion 답변 뒤에 전체 todos 배열로 다시 호출해서 갱신해요. 종료 시점에는 미완료 todo 가 0 개여야 해요. 이전 스킬 todo 가 화면에 남아 있으면 patch 하지 말고 위 배열 전체로 교체해요.

1.1. **Live resolve + preflight (parallel via deploy-prep).** Fetch authoritative `{profile, endpoint, app_id, app_slug, branch, commit_sha, commit_message, eta_sec}` AND preflight (`auth_ok`, `cli_too_old/new`) in one call:

   ```bash
   DEPLOY_PREP_JSON=$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --json)
   [ "${AXHUB_DEPLOY_VERBOSE:-0}" = "1" ] && echo "$DEPLOY_PREP_JSON"
   # 같은 호출 결과에서 Step 1.6/1.7/4 가 재사용할 field 를 CLI 가 eval 안전(@sh) 하게 한 번에 뽑아줘요 (재호출 금지·jq 의존 없음).
   eval "$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --field-expr '"IN_FLIGHT_ID=" + (.in_flight_deploy.id // "" | @sh), "IN_FLIGHT_COMMIT=" + (.in_flight_deploy.commit_sha // "" | @sh), "RESOLVE_COMMIT=" + (.resolve.commit_sha // "" | @sh), "GITHUB_CONNECTED=" + (.github_connected // false | tostring | @sh)' 2>/dev/null)"
   : "${IN_FLIGHT_ID:=}"; : "${IN_FLIGHT_COMMIT:=}"; : "${RESOLVE_COMMIT:=}"; : "${GITHUB_CONNECTED:=false}"
   ```

   위 두 번째 호출은 비용을 아끼려면 첫 호출 JSON 을 재활용해도 되지만, 계약상 같은 deploy-prep 결과를 Step 1.6/1.7/4 가 동일 값으로 써야 해요 (재호출 의미 불변 — 도는 배포·github 연결 상태가 두 경로에서 갈라지면 안 돼요). `IN_FLIGHT_ID` / `IN_FLIGHT_COMMIT` / `RESOLVE_COMMIT` / `GITHUB_CONNECTED` 는 Step 1.6 의 3-way 분기와 Step 1.7 status-first gate 가 그대로 재사용해요. `DEPLOY_PREP_JSON` envelope 자체는 quality_gate / bootstrap_plan / exit_code 분기에 계속 써요 (`.quality_gate.passed`, `.bootstrap_plan`, `.exit_code`, `.resolve.app_id` 등은 에이전트가 envelope 에서 직접 읽어요). JSON envelope: `{preflight, resolve, bootstrap_plan?, in_flight_deploy?, github_connected, quality_gate, exit_code}` (spec §2.3 deploy-prep 행 — 전 필드 binding).

   `.quality_gate.passed == false` 면 violations 를 먼저 보여주고 기본은 멈춰요. 대화형 모드에서만 아래 AskUserQuestion 으로 강제 진행을 허용해요. subprocess 에서는 "품질 게이트가 막았어요. 그래도 진행할까요?" 의 safe default `취소` 예요.

   ```json
   {
     "question": "품질 게이트가 막았어요. 그래도 진행할까요?",
     "header": "품질게이트",
     "options": [
       {"label": "취소", "description": "설정 불일치를 고친 뒤 다시 배포해요."},
       {"label": "강제로 진행", "description": "위험을 알고 현재 값으로 계속해요."}
     ]
   }
   ```

   `bootstrap_plan` 이 non-null 이거나 기존 `app_id` 를 resolve 못 하면 이 스킬에서 원격 생성·첫 연결을 계속하지 않아요. 비어 있지 않은 기존 앱이면 `import` 로, 빈 디렉토리 새 템플릿이면 `init` 으로 자연어 안내 후 멈춰요. `exit_code == 65` 면 auth recovery (Step 6). `exit_code == 64` 이고 quality_gate 가 막은 게 아니면 version-skew recovery (Step 6). `exit_code == 67` 이고 `bootstrap_plan` 이 null 이면 ambiguous resolve 로 다뤄요.

   Never use cached `app_id` for mutation. resolve 가 `app_id` 를 주면 기존 앱 배포라 git readiness → preview → approval-deploy 로 가요. resolve 가 ambiguity 면 사용자에게 disambiguate (slug list only, numeric id 는 숨김) 를 물어요. resolve 가 등록된 앱을 못 찾으면 preview card 로 가지 말고 `import` 또는 `init` 경계 안내로 멈춰요. deploy MUST NOT continue to the preview card while `branch` or `commit_sha` is empty.

1.1c. **타깃 앱 reconcile (stale manifest 슬러그 방지 — deploy-safety).** resolve 는 로컬 `axhub.yaml` 슬러그를 source 로 쓰는데, 그 슬러그가 **의도한 앱과 다를** 수 있어요 (예: 템플릿 기본 슬러그가 안 고쳐진 폴더, 같은 대화에서 다른 앱을 방금 만든 경우). resolve 가 `app_id` 를 준 기존-앱 경로에서 mutation 전에 타깃을 확정해요:
   - **의도 충돌 신호**가 있으면 — 같은 대화에서 사용자가 **다른 앱**을 만들었거나(carry-over `references/session-carryover.md`), 발화가 **다른 앱 이름**을 가리키거나, manifest 슬러그가 stale 의심이면 — resolve 결과를 그대로 신뢰하지 말고 "배포 대상이 `<app_slug>` 맞아요? 아니면 올바른 앱을 알려줘요" 로 한 번 확인해요.
   - 사용자가 **다른 앱**을 지정하면 그 앱 슬러그로 `axhub.yaml` 을 고친 뒤 **Step 1.1 deploy-prep 을 재실행**해 새 manifest 로 다시 resolve 해요 (axhub.yaml 수정 = deploy 영향 변경이라 Step 1.5 git 저장에 담겨요).
   - **headless/비대화형**(확인 불가)에서 의도 충돌 신호가 있으면 **dry-run 으로 강등**해요 — 불확실한 라이브 배포로 잘못된 앱을 건드리지 않아요.
   - 충돌 신호가 없으면 manifest resolve 가 정상 경로예요 (preview 카드 `① 앱` 이 최종 안전망).

1.2. **Static app 분기 (deploy_method auto-detect).** Step 1.1 resolve 가 기존 `app_id` 를 줬으면, 그 앱이 static-hosting 앱인지 먼저 봐요. static 앱은 deployment-record 파이프라인(git readiness·in-flight·preview-confirm·`deploy create`·verify)을 타지 않고 **독립 static lane** 으로 가요 — 자체 dry-run/`--execute` 안전장치와 `active_release_id` 성공 신호를 써요. (`APP_ID` / `APP_SLUG` / `AXHUB_TENANT` 는 Step 1.1 resolve·tenant 단계에서 이미 bind 됐어요.)

   ```bash
   # 백엔드 app GET 이 deploy_method 를 줘요 (CLI 가 raw JSON 을 field-expr 로 그대로 노출). 빈 값/미지원이면 일반 파이프라인으로 떨어져요 (비-static 앱을 가로채지 않음).
   DEPLOY_METHOD=$(axhub apps get "$APP_ID" --no-input --field-expr '.deploy_method // empty' 2>/dev/null || true)
   ```

   - `DEPLOY_METHOD` = `static` → 아래 static lane (1.2a~1.2e) 로 가고, 일반 Step 1.1b/1.5/1.6/1.7/2/3/3.5/4/5 는 건너뛰어요.
   - 그 외 (`docker` / `compose` / 빈 값 / 조회 실패) → 기존 파이프라인 그대로 (가로채지 않음). 혹시 비-static 앱이 static lane 으로 새도 backend 가 release create 를 `409 unsupported_for_static_app` 으로 막아요 (backstop).

1.2a. **Capability probe (static 전용).** static lane 은 `apps static deploy` 가 있는 최신 axhub 가 필요해요 — preflight 가 이 표면을 안 덮으니 `--help` 동작으로만 게이트하고 버전 숫자는 비교하지 않아요. (static 은 현재 beta 채널 기능이라, 아직 못 받은 CLI 에선 이 게이트가 업데이트를 안내하고 멈춰요 — 의도된 동작이에요.)

   ```bash
   if ! axhub apps static deploy --help >/dev/null 2>&1; then
     echo "정적 사이트 배포는 최신 axhub 가 필요해요. '업데이트 해줘'라고 하면 올린 뒤 다시 시도할게요." >&2
     exit 0
   fi
   ```

1.2b. **올릴 폴더(`--from-dir`) 결정.** 정적 빌드 출력 폴더를 정해요. 흔한 출력 폴더를 우선순위로 자동감지하고, 못 찾거나 모호하면 대화형에서 한 번 물어요 (헤드리스는 첫 후보, 없으면 1.2c dry-run 안내로 멈춰요).

   ```bash
   STATIC_DIR=""
   for d in dist build out public; do
     [ -d "$d" ] && { STATIC_DIR="$d"; break; }
   done
   ```

   자동감지 결과는 1.2c preview 카드에서 사용자가 확인·교체할 수 있어요.

1.2c. **Preview (dry-run 기본).** 실제 업로드 전에 dry-run 으로 미리보기를 만들어요. headless 면 dry-run 만 하고 AskUserQuestion 을 부르지 않아요.

   ```bash
   axhub apps static deploy --app "$APP_ID" --from-dir "$STATIC_DIR" --tenant "$AXHUB_TENANT" --dry-run
   ```

   CLI 가 주는 "would deploy N files (B bytes)" 를 한국어 카드로 humanize 해요 (raw dry-run 덤프 금지):

   ```
   다음 정적 사이트를 올릴게요:
   ① 앱:    <APP_SLUG>
   ② 폴더:  <STATIC_DIR>
   ③ 내용:  파일 N개 · 총 B
   ④ 과정:  release 생성 → 업로드 → finalize → activate

   진행할까요? [네 / 아니요 / 미리보기만]
   ```

   그 다음 structured AskUserQuestion 으로 확인받아요 (대화형만):

   ```json
   {
     "question": "이 정적 사이트를 올릴까요?",
     "header": "static 배포",
     "options": [
       {"label": "네, 올리기", "value": "approve", "description": "실제로 업로드하고 활성화해요"},
       {"label": "미리보기만", "value": "dry_run", "description": "실제 업로드 없이 확인만 해요"},
       {"label": "취소", "value": "abort", "description": "멈춰요"}
     ]
   }
   ```

1.2d. **Execute.** `approve` 면 실제 업로드해요. 진행(release 생성→업로드→finalize→activate)을 한국어 한 줄로 narrate 하고, 성공은 `active_release_id` 로 선언해요 — 이 lane 은 `deploy verify` 를 쓰지 않아요 (static 은 deployment-record 가 아니라 release 라 verify 가 404). `dry_run` 이면 1.2c 미리보기에서 멈추고, `abort` 면 "멈췄어요." 로 끝나요. headless 에선 `--execute` 를 하지 않고 dry-run 결과만 알려요.

   ```bash
   axhub apps static deploy --app "$APP_ID" --from-dir "$STATIC_DIR" --tenant "$AXHUB_TENANT" --execute
   ```

1.2e. **성공 + 공개 URL.** activate 가 성공하면(`active_release_id` 확인) 앱 공개 URL 을 읽어 hero 로 보여줘요 — URL 은 합성하지 않아요.

   ```bash
   PUBLIC_URL=$(axhub apps get "$APP_ID" --no-input --field-expr '.access_url // empty' 2>/dev/null || true)
   ```

   `PUBLIC_URL` 이 있으면 "🎉 정적 사이트가 올라갔어요: <PUBLIC_URL>", 없으면 "정적 사이트가 활성화됐어요. '주소 확인해줘'라고 하면 이어서 볼게요." 로 낮춰 말해요.

1.1b. **First-run route boundary.** `deploy` 는 first-run 원격 생성이나 manifest 초기화를 맡지 않아요. Step 1.1 이 기존 앱을 resolve 못 하거나 `bootstrap_plan` 을 반환하면 여기서 멈춰요.

   - 비어 있지 않은 기존 로컬 앱에 manifest 가 없거나 첫 axhub 연결이 필요한 경우: `import` 스킬로 이어가라고 안내해요.
   - 빈 디렉토리에서 새 앱 생성이 필요한 경우: `init` 스킬로 이어가라고 안내해요.
   - deploy 는 이미 axhub 앱이 resolve 된 ordinary redeploy 만 계속 진행해요.

   - 현재 폴더가 비어 있지 않은 기존 앱이면: "이 폴더는 먼저 axhub로 가져와야 해요. '기존 앱 올려'라고 말하면 가져오기 흐름으로 이어갈게요." 라고 안내하고 `import` 로 양보해요.
   - 빈 디렉토리에서 새 템플릿 앱을 만들려는 흐름이면: "새 앱은 템플릿부터 고르면 돼요. '새 앱 만들어줘'라고 말하면 이어갈게요." 라고 안내하고 `init` 으로 양보해요.
   - 이 단계에서 `apps create`, `deploy create`, `axhub init`, `plugin-support bootstrap --auto-chain` 을 호출하지 않아요.

   이 hard stop 뒤에는 Step 1.5/preview/deploy 로 이어가지 않아요.

1.5. **Git 저장 지점 준비** — resolve 가 `git_init_needed: true` OR `git_has_commit: false` OR `branch`/`commit_sha` 가 empty OR local deploy 영향 uncommitted 변경이 있으면, preview 를 아직 보여주지 말아요. explanatory copy / AskUserQuestion 전에, 전체 TodoWrite 목록을 local git readiness 체크리스트로 교체해요.

   Deploy MUST NOT show a preview card for an old `commit_sha` while the manifest that will make that deploy work is still uncommitted.

   ```typescript
   TodoWrite({ todos: [
     { content: "git 저장소 만들기",        status: "in_progress", activeForm: "git 저장소 만드는 중" },
     { content: "파일을 첫 저장 지점에 담기", status: "pending",     activeForm: "파일 담는 중" },
     { content: "첫 커밋 만들기",          status: "pending",     activeForm: "첫 커밋 만드는 중" },
     { content: "배포 정보 다시 확인하기",   status: "pending",     activeForm: "배포 정보 다시 보는 중" },
     { content: "미리보기 카드 보여드리기",  status: "pending",     activeForm: "미리보기 준비하는 중" }
   ]})
   ```

   그 다음 non-developer Korean 으로 설명해요:

   ```
   배포 전에 파일을 저장 지점에 한 번 담아둬야 해요.
   이렇게 해야 어떤 버전을 올릴지 정확히 알 수 있어요.
   지금은 아직 그 저장 지점이 없어서, 제가 자동으로 만들어드릴게요.
   ```

   그리고 물어요 (2-option):

   ```json
   {
     "question": "배포 전 저장 지점을 만들까요?",
     "header": "저장 지점",
     "options": [
       {"label": "지금 만들기", "value": "init_and_continue", "description": "현재 폴더에 저장 지점을 자동으로 만들고 배포를 이어가요."},
       {"label": "취소", "value": "abort", "description": "배포를 멈춰요."}
     ]
   }
   ```

   "지금 만들기" 면 local git 명령을 조용히 실행해요 (raw `git init`/`git add`/`git commit` 출력을 chat 에 echo 하지 말고 "저장 지점을 만들고 있어요." 한 줄). 그 다음 deploy-prep 을 다시 돌려 fresh `commit_sha` 를 받고 Step 1.6 부터 이어가요.

   ```bash
   if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
     git init >/dev/null 2>&1
   fi
   git add -A >/dev/null 2>&1
   git commit -m "init: axhub deploy baseline" >/dev/null 2>&1 || true
   git branch -M main >/dev/null 2>&1
   axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --json
   ```

   git stderr/stdout 은 `/dev/null` 로 보내요 — vibe coder chat 에 raw git output 이 노출되지 않게 막아요. `git commit` 이 실패하면 (no staged files / missing identity) `|| true` 로 넘어가고, 뒤따르는 deploy-prep 이 `branch`/`commit_sha` 가 비었다고 알려서 humanized 한 줄로 안내해요. 실패 시 "저장 지점을 만들지 못했어요. 잠시 뒤에 다시 시도해요." 로 멈춰요. fresh resolve 가 `branch` + `commit_sha` 를 줄 때까지 deploy 하지 말아요. "취소" 면 git 명령 없이 멈춰요. subprocess 에서는 safe default "취소" — never run `git init` automatically.

1.6. **In-flight deploy 감지 (배포 충돌 방지) — 3-way 분기.** `deploy-prep` 응답에 `.in_flight_deploy.id` 가 non-null 이면 이미 진행 중인 배포가 있어요. `in_flight_deploy.commit_sha` 와 `resolve.commit_sha` 비교로 3 가지 분기를 결정해요.

   **Ownership 추론 한계.** ownership 판별은 `commit_sha` 비교만 써요. mono-repo same-HEAD case 에서 본인/타인 구분 못 해요. 그래서 Step 1.6b copy 도 "가능성이 있어요" 로 약화해 false confidence 를 피해요.

   - **1.6a (same-commit)**: 두 commit_sha non-empty + 일치 — 본인 배포 중복 가능성. "이미 배포가 진행 중이에요." prompt.
   - **1.6b (cross-tenant)**: 두 commit_sha non-empty + 다름 — 다른 user 의 in-flight 가능성. "다른 사람이 같은 앱에 배포 중일 가능성이 있어요." prompt.
   - **1.6c (uncertain)**: 하나가 empty — uncertain. "배포 중인 게 있는데 누구 건지 확인 중이에요." prompt.

   ```bash
   # IN_FLIGHT_ID / IN_FLIGHT_COMMIT / RESOLVE_COMMIT 는 Step 1.1 의 deploy-prep eval 에서 이미 bind 됐어요 (재호출·재파싱 안 해요).
   if [ -n "$IN_FLIGHT_ID" ]; then
     if [ -z "$IN_FLIGHT_COMMIT" ] || [ -z "$RESOLVE_COMMIT" ]; then
       INFLIGHT_BRANCH="uncertain"     # → 1.6c
     elif [ "$IN_FLIGHT_COMMIT" = "$RESOLVE_COMMIT" ]; then
       INFLIGHT_BRANCH="same"          # → 1.6a
     else
       INFLIGHT_BRANCH="cross_tenant"  # → 1.6b
     fi
     if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
       exit 0  # non-interactive: safe default = abort (모든 분기 공통)
     fi
   fi
   ```

   세 분기 모두 3-option AskUserQuestion (해요체) 으로 물어요. 1.6a 는 "이미 배포가 진행 중이에요. 어떻게 할까요?", 1.6b 는 "다른 사람이 같은 앱에 배포 중일 가능성이 있어요. 어떻게 할까요?", 1.6c 는 "배포 중인 게 있는데 누구 건지 확인 중이에요. 어떻게 할까요?" — 옵션은 공통이에요:

   ```json
   {
     "header": "배포 충돌",
     "options": [
       {"label": "진행 중인 배포 보기", "value": "monitor", "description": "현재 진행 중인 배포 결과를 확인해요."},
       {"label": "새 배포 시작", "value": "force_new", "description": "진행 중인 배포와 별개로 지금 바로 새 배포를 올려요."},
       {"label": "취소", "value": "abort", "description": "배포를 멈춰요."}
     ]
   }
   ```

   1.6a 는 최근 60초 이내면 "진행 중인 배포 보기" default highlight, 60초 넘으면 "새 배포 시작". 1.6b / 1.6c 는 보수적으로 "취소" default highlight.

   - `monitor` → Step 5 verify-after-watch 로 바로 이동해 `$IN_FLIGHT_ID` 를 따라가요. 새 `deploy create` 안 해요.
   - `force_new` → Step 2 로 진행해요. exit 64 + `validation.deployment_in_progress` 가 나도 retry 하지 않아요 (Step 6).
   - `abort` → 멈춰요.

1.7. **Status-first gate (배포는 status 먼저 — `deploy create` 는 fallback).** push 가 자동배포를 트리거하는 환경(`deploy-prep` 의 `.github_connected: true`)에서는 preview/approval 로 가기 전에 **지금 돌고 있는 배포가 있는지 먼저 확인**해요. push 로 이미 시작된 배포가 있는데 새 `deploy create` 를 실행하면 exit 64 충돌이나 commit 불일치로 재시도 루프에 빠져요. 도는 배포가 있으면 그걸 따라가고(create 생략), 없을 때만 Step 2 이후 명시적 create 로 진행해요. 단, Step 1.6 에서 사용자가 `force_new` 를 골랐으면 그 선택을 존중해 이 gate 는 건너뛰고 Step 2 로 가요.

   ```bash
   # GITHUB_CONNECTED / STATUS_FIRST_ID(=IN_FLIGHT_ID) 는 Step 1.1 deploy-prep eval 에서 이미 bind 됐어요.
   STATUS_FIRST_ID="$IN_FLIGHT_ID"
   # github 연결 앱인데 in-flight 가 아직 안 보이면, push 자동배포 등록 시간을 잠깐 줘요 (interactive 만, 최대 ~15s).
   if [ -z "$STATUS_FIRST_ID" ] && [ "$GITHUB_CONNECTED" = "true" ] && [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then
     for _ in 1 2 3; do
       sleep 5
       # refresh 호출 결과에서 CLI 가 직접 in-flight id/commit + resolve commit 을 eval 안전하게 뽑아줘요 (jq 의존 없음).
       eval "$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --refresh-in-flight --field-expr '"STATUS_FIRST_ID=" + (.in_flight_deploy.id // "" | @sh), "_RF_IN_FLIGHT_COMMIT=" + (.in_flight_deploy.commit_sha // "" | @sh), "_RF_RESOLVE_COMMIT=" + (.resolve.commit_sha // "" | @sh)' 2>/dev/null)"
       : "${STATUS_FIRST_ID:=}"
       if [ -n "$STATUS_FIRST_ID" ]; then
         # 재사용될 Step 1.6 3-way 분기가 refresh 된 값을 쓰게 동기화해요.
         IN_FLIGHT_ID="$STATUS_FIRST_ID"; IN_FLIGHT_COMMIT="${_RF_IN_FLIGHT_COMMIT:-}"; RESOLVE_COMMIT="${_RF_RESOLVE_COMMIT:-}"; break
       fi
     done
   fi
   ```

   - `STATUS_FIRST_ID` non-empty → 이미 배포가 돌고 있어요. **Step 1.6 의 3-way 분기를 그대로 재사용**해요: same-commit 이면 본인 push 배포라 바로 Step 5 (`monitor`), cross-tenant / uncertain 이면 1.6b / 1.6c 로 물어요. 이 경로에서는 **새 `deploy create` 를 실행하지 않아요**.
   - `STATUS_FIRST_ID` empty → Step 2 이후 명시적 create 경로로 진행해요.

2. **Preview decision lane (headless).** Headless 에서는 preview card 를 보여준 뒤 "진행할까요?" 같은 대기형 문장을 출력하지 말아요. 내부 결정을 먼저 확정해요:

   ```bash
   AXHUB_HEADLESS=0
   if ! [ -t 1 ] || [ -n "${CI:-}" ] || [ -n "${CLAUDE_NON_INTERACTIVE:-}" ]; then
     AXHUB_HEADLESS=1
   fi
   DEPLOY_DECISION="${DEPLOY_DECISION:-}"
   if [ "$AXHUB_HEADLESS" = "1" ]; then
     DEPLOY_DECISION="dry_run"
     echo "비대화형이라 실제 배포 대신 dry-run 으로 CLI/auth 경로만 확인해요." >&2
   fi
   ```

   `DEPLOY_DECISION=dry_run` 이면 Step 4 에서 `--dry-run` 만 실행해요. `approve` 는 대화형 AskUserQuestion 승인 뒤에만 가능해요. Headless 에서는 외부 환경변수가 approve 를 미리 넣어도 dry-run 으로 덮어써요.

3. **Render preview card via AskUserQuestion.** AskUserQuestion is interactive-only; headless sessions use the dry-run path. The card MUST echo all five identity fields verbatim in Korean:

   ```
   다음을 실행할게요:
   ① 앱:    paydrop
   ② 환경:  production (https://axhub-api.jocodingax.ai)
   ③ 브랜치: main
   ④ 커밋:  a3f9c1b — "결제 페이지 버그 수정" (12분 전 푸시, you)
   ⑤ 예상:  약 3분 소요

   진행할까요? [네 / 아니요 / 미리보기만 (--dry-run)]
   ```

   `references/error-empathy-catalog.md` ("deploy-preview") 템플릿을 써요. 표시 slug 에 NFKC normalize 를 적용하고, NFKC 가 문자열을 바꾸면 경고를 보여줘요. 그 다음 structured AskUserQuestion 으로 물어요:

   ```json
   {
     "question": "진행할까요?",
     "header": "배포 확인",
     "options": [
       {"label": "네, 배포", "value": "approve", "description": "동의를 받고 실제 배포를 시작해요."},
       {"label": "미리보기만", "value": "dry_run", "description": "--dry-run 으로 실제 배포 없이 확인해요."},
       {"label": "취소", "value": "abort", "description": "배포를 멈춰요."}
     ]
   }
   ```

   `dry_run` 이면 Step 4 에 `--dry-run` 을 붙이고 Step 5 를 건너뛰어요. Headless 에서는 AskUserQuestion 을 부르지 말고 옵션 목록에서 멈추지도 말고, `DEPLOY_DECISION=dry_run` 을 직접 적용하고 Step 4 에 `--dry-run` 을 붙이고 Step 5 를 건너뛰어요.

3.5. **Token freshness gate.** Before running deploy, confirm the auth token is fresh. Skip when `AXHUB_AUTH_BG_REFRESH=0`.

   ```bash
   axhub plugin-support token-gate
   ```

   `token-gate` 는 stdout 이 없고 **exit code 가 계약**이에요: 0 fresh/authorized/disabled/fail-open, **65** inline probe UNAUTHORIZED → Step 6 recovery (spec §2.3). 30s mtime 폴링 (5s × 6) 후 timeout 시 `axhub auth status --json` 을 inline 으로 불러요. Test fixtures inject `AXHUB_TOKEN_PATH` / `AXHUB_GATE_FAKE_NOW` / `AXHUB_GATE_POLL_*`.

4. **On user approval**, run deploy. This Step 은 **fallback create 경로**예요 — Step 1.7 status-first 가 도는 배포를 못 찾았을 때만 도달해요.

   ```bash
   # tenant 캐시 재파싱 대신 CLI 가 자기 결과에서 tenant 를 뽑게 해요 (캐시 fallback, jq 의존 없음).
   if [ -z "${AXHUB_TENANT:-}" ]; then
     AXHUB_TENANT=$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)
   fi
   PROFILE_FLAG=()
   if [ -n "${PROFILE:-}" ] && [ "${PROFILE:-}" != "default" ]; then
     PROFILE_FLAG=(--profile "$PROFILE")
   fi
   AXHUB_STDERR_TMP=$(mktemp); AXHUB_STDOUT_TMP=$(mktemp)
   # --field-expr 로 stdout 에 deployment id 만 남겨요 (exit code 분기·stderr 는 그대로 — field-expr 는 stdout 만 바꿔요).
   if [ "${DEPLOY_DECISION:-approve}" = "dry_run" ]; then
     axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --dry-run --field-expr '.id // .deployment_id // empty' >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
   elif [ "${DEPLOY_DECISION:-approve}" = "abort" ]; then
     echo "배포를 멈춰요." >&2; rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"; exit 0
   else
     axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --execute --field-expr '.id // .deployment_id // empty' >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
   fi
   AXHUB_EXIT=$?
   # Format: "axhub-error-sub-key: 64:validation.deployment_in_progress"
   if [ $AXHUB_EXIT -eq 64 ] && grep -qE '^axhub-error-sub-key:.*64:validation\.deployment_in_progress' "$AXHUB_STDERR_TMP" 2>/dev/null; then
     # in-flight race: silent swallow raw stderr, re-fetch in-flight id for Step 5.
     # refresh 호출 결과에서 CLI 가 직접 in-flight id + commit 을 eval 안전하게 뽑아줘요 (jq 의존 없음).
     eval "$(axhub plugin-support deploy-prep --intent deploy --refresh-in-flight --field-expr '"IN_FLIGHT_ID=" + (.in_flight_deploy.id // "" | @sh), "COMMIT_SHA=" + (.in_flight_deploy.commit_sha // .resolve.commit_sha // "" | @sh)' 2>/dev/null)"
     : "${IN_FLIGHT_ID:=}"
     if [ -n "$IN_FLIGHT_ID" ]; then
       DEPLOY_ID="$IN_FLIGHT_ID"
     else
       echo "다른 배포가 진행 중이에요. 잠시 뒤에 다시 시도해요." >&2
       rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"; exit 0
     fi
  elif [ $AXHUB_EXIT -eq 0 ]; then
    # stdout 에는 --field-expr 가 남긴 deployment id 만 있어요 (dry-run·id 없음 시 빈 문자열 → 아래 guard 가 처리).
    DEPLOY_ID=$(cat "$AXHUB_STDOUT_TMP")
    if [ -z "$DEPLOY_ID" ]; then
      echo "배포 시작은 확인했지만 결과 확인 id 를 못 받았어요. '배포 상태 확인해줘'라고 말하면 이어서 볼게요." >&2
      rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"; exit 0
    fi
    echo "배포 결과를 확인하고 있어요." >&2
   else
     AXHUB_STDERR_CAPTURE=$(cat "$AXHUB_STDERR_TMP"); AXHUB_STDOUT_CAPTURE=$(cat "$AXHUB_STDOUT_TMP"); echo "배포를 시작하지 못했어요. 복구 방법을 확인하고 있어요." >&2
   fi
   rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"
   ```

   `DEPLOY_ID` 는 deploy create stdout JSON 의 `.id` (또는 in-flight race 시 재조회한 in-flight id) 예요. Step 5 verify 가 이 id 를 써요.

5. **Success declaration — verify once (배포 성공 선언은 verify 로만).** 배포가 끝났는지 / 성공인지 선언은 **오직 `axhub deploy verify <deployment-id>` 한 번**으로 해요. deployment id 는 Step 4 의 `DEPLOY_ID` (또는 Step 1.6/1.7 의 `$IN_FLIGHT_ID`) 예요. prose polling 절차나 `deploy status --watch` 기반 성공 주장은 하지 않아요.

   ```bash
   echo "배포 결과를 확인하고 있어요." >&2
   VERIFY_OUT=$(mktemp)
   axhub deploy verify "$DEPLOY_ID" > "$VERIFY_OUT" 2>&1
   VERIFY_EXIT=$?
   # raw 출력은 chat 에 cat 하지 않아요 — 에이전트가 $VERIFY_OUT 을 읽고 아래 분기의 한국어 문장만 보여줘요.
   ```

   `axhub deploy verify` 는 공개 명령이에요. terminal success state + 접근 가능 URL 을 확인해요 (spec §2.3, correlation 계약: **deployment id 인자 필수, latest 재탐색 금지**). 비-0 exit 는 절대 성공으로 선언하지 않아요. exit code 로 갈라요:

   - exit 0 (terminal success) → 단일 한국어 완료 요약을 보여줘요 (live URL 포함).
   - exit 6 (non-terminal — 아직 진행 중) → "빌드가 아직 진행 중이에요. '배포 상태 확인해줘'라고 말하면 이어서 볼 수 있어요." 로 안내하고 멈춰요.
   - exit 7 (terminal failure — 배포 실패) → "배포가 실패했어요." 후 Step 6 의 4-part 에러 카드로 라우팅해요.
   - exit 5 (unknown deployment id) → "그 배포를 못 찾았어요." 안내 후 멈춰요 (latest 재탐색 금지).
   - exit 4 (auth 만료, CLI-native) → Step 6 의 auth recovery (exit 4 / 65) 로 라우팅해요 ("axhub 로그인이 만료됐어요. 다시 로그인할까요?").

   `--dry-run` 경로에서는 verify 를 건너뛰어요 (실제 deploy 가 없어요). (업데이트 알림은 Step 1a 에서 맨 앞에 처리하니 여기선 다시 안 해요.)

**대표 verify 실패/진행 중 복구.** `verify` 가 exit 6 을 주면 진행 중으로만 말하고 성공을 선언하지 않아요. exit 7 이면 실패로 말하되 앱이 망가졌다고 단정하지 말고, 같은 deployment id 로 확인한 증거와 다음 행동(`배포 상태 확인해줘`, `로그 보여줘`, `다시 배포해줘`)만 제안해요. 두 경우 모두 raw verify 출력·exit jargon·latest 재탐색은 사용자에게 노출하지 않아요.

6. **On any non-zero exit**, route via `axhub plugin-support classify-exit "$EXIT" "$STDOUT"` (canonical router; 두 공간 다 처리: CLI-native 4/5/6 와 helper-output 65/67/68 을 classify-exit 가 65→4 / 67→5 / 68→6 으로 정규화해요 — normalize_helper_exit 계약 불변) 또는 `references/error-empathy-catalog.md` by exit code:
   - exit 64 + `validation.deployment_in_progress` → 4-part Korean copy: "다른 배포가 진행 중이에요. 앱은 안전해요. 5분만 기다리면 자동으로 다음 배포가 가능해요." Never retry. Offer to watch the in-flight deploy instead.
   - exit 9 + `subdomain_not_configured` (or stderr contains "subdomain") → backend precondition. `axhub apps update <slug> --subdomain <subdomain> --json` 는 별도 destructive mutation 이라 바로 실행하지 말고, subdomain 2..32자 제약 후보를 preview card 로 보여주고 승인 받아요. 승인 후 apps_update 를 단독 Bash 로 실행하고, 성공하면 같은 deploy preview 승인 맥락에서 Step 4 를 한 번만 재시도해요. 다시 exit 9 면 다음 precondition branch 로 라우팅해요.
   - exit 9/64/67 + `github.git_connection_required` / `git_connection_required` / `precondition_failed` (stderr contains "GitHub 저장소 연결" / "GitHub 연결이 먼저 필요해요") → deploy 에서 GitHub repo 생성·first push·`apps git connect` 를 진행하지 않아요. 비어 있지 않은 기존 앱의 첫 연결/복구가 필요하므로 "이 앱은 먼저 GitHub 연결을 가져오기 흐름에서 정리해야 해요. '기존 앱 가져와줘'라고 말하면 이어갈게요." 로 안내하고 멈춰요. raw `install_url`, repo 후보, 내부 앱 id 는 사용자 chat 에 노출하지 않아요.
   - exit 4 (CLI) / 65 (helper deploy-prep·preflight·token-gate) → token expired template + AskUserQuestion to run auth login
   - exit 5 (CLI) / 67 (helper deploy-prep) → resource not found + did-you-mean suggestion from apps list
   - exit 6 (CLI) / 68 (helper) → rate limit + Retry-After backoff
   - exit 1 → transport error; retry at most once for read paths, never for create

7. **Dry-run NL trigger** — if the user said "한번 해보기만", "리허설", "테스트로", "진짜 안 올리고", add `--dry-run` to Step 4 and skip Step 5 verify.

## NEVER

- NEVER let deploy create or initialize first-run app/import state. Missing app/manifest first-connect belongs to `import` or `init`.
- NEVER retry `axhub deploy create` on exit 64.
- NEVER drop `--json` (parsing relies on it).
- NEVER call `axhub deploy create --execute` without the AskUserQuestion preview decision.
- NEVER declare deploy success from `deploy status --watch` output or a prose polling loop. Success declaration is `axhub deploy verify <deployment-id>` run once.
- NEVER call `axhub deploy verify` without a deployment id (latest 재탐색 금지 — correlation 계약).
- NEVER static lane(deploy_method=static)에서 `axhub deploy verify` 를 호출하지 않아요 — static 은 deployment-record 가 아니라 release 라 verify 가 404 예요. static 성공은 `apps static deploy --execute` 의 `active_release_id`(activate)로 선언해요.
- NEVER 비-static 앱(deploy_method ≠ static)을 static lane 으로 보내지 않아요 — deploy_method 가 빈 값/미지원이면 일반 deployment-record 파이프라인으로 가요.
- NEVER static lane 에서 `apps static deploy --execute` 를 dry-run 미리보기 + AskUserQuestion 승인 없이 호출하지 않아요 (헤드리스 제외 — 헤드리스는 dry-run 만).
- NEVER change command semantics after approval (omitting `--execute`, changing `--app`, or `--commit`). Surface the typed reason in one jargon-free line and stop, or fall back to Step 1.7 status-first watch.
- NEVER instruct the user to run `axhub deploy create`, `axhub deploy verify`, or any deploy CLI command themselves. The agent runs deploy and verify itself inside this SKILL flow; if blocked, surface the typed reason in one jargon-free line and stop.
- NEVER run `deploy create` when Step 1.7 status-first already found an in-flight deploy for this app; route to verify instead.
- NEVER call `axhub deploy cancel` without explicit confirmation.
- NEVER infer `app_id` from `pwd` or git remote alone in the mutation path; always live resolve through deploy-prep.
- NEVER bypass the AskUserQuestion preview card on slash invocation; slash is explicit confirmation for the SKILL invocation, not for the destructive operation.
- NEVER insert the old approved-run helper bridge between preview approval and the canonical deploy workflow; approval must flow into `deploy-prep` / public `axhub deploy create --execute --json` / verify.

## Additional Resources

For exit-code → 4-part Korean error template (emotion + cause + action + button): `references/error-empathy-catalog.md`.
For same-conversation 조회·온보딩 맥락 carry-over·confabulation 가드·마찰 억제 단일 계약: `references/session-carryover.md`.
Deploy 보조 명령(list/cancel) 커버리지: `references/command-coverage.md`.
