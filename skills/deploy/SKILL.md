---
name: deploy
description: '이 스킬은 사용자가 현재 브랜치를 axhub 라이브로 배포하고 싶어할 때 사용해요. 다음 표현에서 활성화: "공개해", "내보내자", "띄워", "배포", "배포해", "배포해줘", "쏘자", "올려", "올리자", "터트려", "푸시한 거 띄워", "프로덕션", "프로덕션에 박아", "demo가 필요", "demo가 필요해", "deploy", "launch", "release", "rollout", "ship", 또는 현재 브랜치를 axhub 라이브로 올리고 싶다는 모든 의도. 안전한 배포 준비 확인, 라이브 profile/app 해석, AskUserQuestion preview card 를 통한 AskUserQuestion preview-confirm gate, exit-code 기반 복구 라우팅을 담당해요.'
examples:
  - utterance: "paydrop 배포해"
    intent: "deploy current branch to axhub live"
  - utterance: "어쨌든 그냥 배포해, 묻지 말고 prod에 박아"
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

## Claude Desktop Natural-Language Path

When the user says a human deployment phrase such as `배포해줘`, `올려줘`, or `프로덕션에 띄워줘`, keep the visible conversation human:

- The first visible chat sentence must be exactly `배포 준비를 확인할게요.`
- For the initial Desktop preview, stop reading this skill after this section. Do not read the long workflow below until the user has approved the preview card.
- Before the Bash/tool call, make sure the command runs in the user-visible app folder. In Claude Desktop, if the active root and an added folder differ and the added folder is the only Vite/React app (`package.json` has `vite` + `react`/`react-dom`), run the command from that folder (`cd "<that folder>" && ...`). If multiple app folders are plausible, ask which folder to deploy and stop.
- If stdout says `axhub 매니페스트(axhub.yaml)가 없어요.`, render the local initialization choices (`React/Vite로 초기화`, `다른 템플릿 선택`, `취소`) and stop. Do not ask for deploy approval from this state.
- Immediately run one Bash/tool call with title `배포 준비 확인`: `axhub plugin-support deploy-preview-summary --user-utterance "<latest user sentence>"`.
- Copy that Korean stdout as the preview card and ask for explicit approval.
- After the user explicitly approves, continue into the canonical workflow below starting at Step 1.1 with the approval decision already captured. Use `deploy-prep`, bootstrap/git readiness/status-first checks, token gate, public `axhub deploy create --execute --json` when creation is actually required, and Step 5 verify. Do not insert a separate approved-run helper bridge between preview approval and the canonical workflow.
- Bind `DEPLOY_ID` only from a recorded bootstrap deploy id, an in-flight deployment id, or public `axhub deploy create --execute --json` output. If no deployment id is present, do **not** declare success; say "배포 시작은 확인했지만 결과 확인 id 를 못 받았어요. '배포 상태 확인해줘'라고 말하면 이어서 볼게요." and stop.
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

대신 사용자에게는 한국어 한 줄로 진행 상황만 알려드려요. 예시 templates:

| 시점 | 사용자 chat 한 줄 |
|------|-------------------|
| Step 1 첫 배포 / app 등록 | "처음 배포라 앱을 먼저 만들고 있어요." |
| Step 1.5 git 저장 지점 준비 | "배포 전에 파일 저장 지점을 만들어두고 있어요." |
| Step 3 preview card | 5필드 한국어 카드만 (앱 / 환경 / 브랜치 / 커밋 / 예상 시간) |
| Step 4 approval → deploy | "배포 확인을 받았어요. 시작해요." |
| Step 5 verify | "배포 결과를 확인하고 있어요." |
| Step 6 exit 4 / 65 | "axhub 로그인이 만료됐어요. 다시 로그인할까요?" |
| Step 6 exit 64 | "다른 배포가 진행 중이라 지금은 못 올려요. 잠시 뒤에 다시 시도해요." |
| Step 6 exit 5 / 67 | "이 이름의 앱을 못 찾았어요. 비슷한 이름을 알려드릴게요." |

raw JSON 이 디버깅에 필요한 환경은 `AXHUB_DEPLOY_VERBOSE=1` 이 켜진 경우에만 echo 해요.

## Workflow

**한눈에 — 실행 순서.** step 라벨은 히스토리상 순서가 섞여 있으니, 실제 실행은 이 순서로 읽어요:
`1` CLI 가드 → `1a` 버전 체크(신버전 안내) → `1.1` deploy-prep(resolve+preflight) → `0` TodoWrite(1.1 결과로 도출) → `1.1b` first-run 브리지 → `1.5` git 저장 준비 → `1.6` in-flight 감지 → `1.7` status-first 게이트 → `2` preview 결정(headless) → `3` preview 카드 → `3.5` 토큰 게이트 → `4` deploy create(fallback) → `5` verify(성공 선언) → `6` 비-0 에러 라우팅 / `7` dry-run 트리거.

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**배포 직전 선택 점검(infer-tables-env 연계).** 배포 본 단계로 들어가기 전에, 코드에서 필요한 테이블·환경변수를 먼저 추천받을지 AskUserQuestion 으로 한 번 물어봐요. 비차단이라 헤드리스/비대화형에서는 묻지 않고 safe default(`아니요, 바로 배포`)로 그냥 배포를 이어가요.

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
ROUTE_JSON=$(axhub plugin-support route-decision --user-utterance "$ARGS" $EXPLICIT_FLAG 2>/dev/null)
# fail-open: 빈 출력이면 axhub 로 진행해요 — 실제 배포는 뒤의 preview-confirm gate 가 막아요.
ROUTE_DECISION=$(printf '%s' "$ROUTE_JSON" | jq -r '.decision // "axhub"' 2>/dev/null || echo axhub)
echo "$ROUTE_JSON"
```

`route-decision` 의 binding 계약은 `.decision` 단일이에요. 현재 CLI 는 marker/auth 기반이라 실제로 **`axhub` 또는 `ignore`** 만 내요 (NL 타깃 판별은 (A) 가 이미 했고, 이 호출은 keyword 판정을 하지 않아요). exit 0 고정 fail-open (빈 출력 → `axhub`, 뒤 preview-confirm 이 backstop). `ROUTE_DECISION` 값으로 분기해요. **`axhub` 일 때만 진행**해요:

- **`axhub`** → 정상 경로. 아래 Step 1.1 (deploy-prep) 로 계속 진행해요.
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
  TENANT_JSON=$(axhub plugin-support tenant-resolve --json 2>/dev/null)
  [ -n "$TENANT_JSON" ] || TENANT_JSON='{}'
  AXHUB_TENANT=$(printf '%s' "$TENANT_JSON" | jq -r '.tenant // empty' 2>/dev/null || true)
  _NEEDS_PICK_RAW=$(printf '%s' "$TENANT_JSON" | jq -r '.needs_pick // false' 2>/dev/null || echo false)
  if [ "$_NEEDS_PICK_RAW" = "true" ]; then
    CANDIDATES_JSON=$(printf '%s' "$TENANT_JSON" | jq -c '.candidates // []' 2>/dev/null || echo '[]')
    if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
      AXHUB_TENANT=$(printf '%s' "$CANDIDATES_JSON" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)
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

0. **TodoWrite 진행 체크리스트 — 실제 배포 경로에서 도출해요 (고정 목록 붙여넣기 금지).** TodoWrite 가 host 에 있을 때만 호출해요. 항목은 Step 1.1 의 `deploy-prep` 결과에 따라 달라져요: git-connected 앱의 push 자동배포는 자기가 만들지 않은 배포를 **watch** (status-first), first-deploy / non-git 은 명시 승인 후 `deploy create` 를 실행해요. 상황을 읽고 그에 맞는 todos 를 써요. 참고 shape A — git-connected / status-first watch:

   ```typescript
   TodoWrite({ todos: [
     { content: "배포 상태 확인 (preflight)", status: "in_progress", activeForm: "배포 상태 확인하는 중" },
     { content: "최신 저장 지점 푸시 확인",     status: "pending",     activeForm: "푸시 상태 보는 중" },
     { content: "자동 시작된 배포 따라가기",     status: "pending",     activeForm: "배포 따라가는 중" },
     { content: "결과 확인 (verify)",          status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   참고 shape B — first deploy / non-git:

   ```typescript
   TodoWrite({ todos: [
     { content: "토큰 확인 (preflight)",         status: "in_progress", activeForm: "토큰 확인하는 중" },
     { content: "앱 / 환경 / 브랜치 확정",         status: "pending",     activeForm: "앱 정보 정리하는 중" },
     { content: "git 저장 지점 확인",             status: "pending",     activeForm: "배포용 저장 지점 보는 중" },
     { content: "미리보기 카드 보여드리기",         status: "pending",     activeForm: "미리보기 준비하는 중" },
     { content: "동의 받고 배포 시작",            status: "pending",     activeForm: "배포 시작하는 중" },
     { content: "결과 확인 (verify)",             status: "pending",     activeForm: "확인하는 중" }
   ]})
   ```

   매 step 과 매 AskUserQuestion 답변 뒤에 전체 todos 배열로 다시 호출해서 갱신해요. 종료 시점에는 미완료 todo 가 0 개여야 해요. 이전 스킬 todo 가 화면에 남아 있으면 patch 하지 말고 위 배열 전체로 교체해요.

1.1. **Live resolve + preflight (parallel via deploy-prep).** Fetch authoritative `{profile, endpoint, app_id, app_slug, branch, commit_sha, commit_message, eta_sec}` AND preflight (`auth_ok`, `cli_too_old/new`) in one call:

   ```bash
   DEPLOY_PREP_JSON=$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --json)
   echo "$DEPLOY_PREP_JSON"
   ```

   `DEPLOY_PREP_JSON` 은 Step 1.6 / Step 1.7 에서 `.in_flight_deploy` / `.github_connected` 를 jq 로 읽을 때 다시 써요. JSON envelope: `{preflight, resolve, bootstrap_plan?, in_flight_deploy?, github_connected, quality_gate, exit_code}` (spec §2.3 deploy-prep 행 — 전 필드 binding). `jq -r '.resolve.app_id'` 등으로 추출해요.

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

   `bootstrap_plan` 이 non-null 이면 first-deploy 경로 — Step 1.1b 로 가요. `exit_code == 65` 면 auth recovery (Step 6). `exit_code == 64` 이고 quality_gate 가 막은 게 아니면 version-skew recovery (Step 6). `exit_code == 67` 이고 `bootstrap_plan` 이 null 이면 ambiguous resolve 로 다뤄요.

   Never use cached `app_id` for mutation. resolve 가 `app_id` 를 주면 기존 앱 배포라 `bootstrap apps_create` 를 돌리지 않고 git readiness → preview → approval-deploy 로 가요. resolve 가 ambiguity 면 사용자에게 disambiguate (slug list + numeric id) 를 물어요. resolve 가 등록된 앱을 못 찾고 `axhub.yaml`/`apphub.yaml` 이 있으면 Step 1.1b first-run bridge 로 가요. deploy MUST NOT continue to the preview card while `branch` or `commit_sha` is empty.

1.1b. **First-run bootstrap plan/record bridge.** Step 1.1 이 기존 `app_id` 를 resolve 못 했을 때만 써요. first-run remote mutation 전에 Rust FSM 에 다음 안전 step 을 물어요:

   ```bash
   axhub plugin-support bootstrap --auto-chain --json
   ```

   이 출력을 bootstrap state 의 source of truth 로 다뤄요 (BootstrapOutput: `state`(14 enum), `next_action`, `command[]`, `pending_action_id`, `pending_action_hash`, `idempotency_key`, `retry_policy`, `reason`, `next_steps[]` — spec §2.3). `template_required` / `git_init_required` / `first_commit_required` / `subdomain_collision` / `backend_contract_missing_defaults` / `idempotency_unavailable` 면 그 user-decision state 에서 멈추고 humanized 한 줄 + 가장 안전한 다음 명령을 보여줘요. `next_action: apps_create` 또는 `deploy_create` 면 `command` / `pending_action_id` / `pending_action_hash` / `retry_policy` 를 **내부 변수에만 bind** 하고 raw 값은 echo 하지 말고, "처음 배포라 앱을 먼저 만들고 있어요." 한 줄만 보여주고 preview-confirmed execution 으로 가요. helper 는 planner/recorder 일 뿐 mutate 승인이 아니에요. `deploy_create` 가 여기서 실행·record 됐으면 Step 4 에서 두 번째 `deploy_create` 를 돌리지 말고 Step 5 verify 로 점프해요.

   **Desktop hard-stop for `template_required` / `manifest_missing`:** `bootstrap --auto-chain --json` 이 `state: "template_required"` 또는 `reason: "manifest_missing"` 를 주면, 더 context/파일 검사를 하지 말고 `apps bootstrap` / `apps create` / `deploy create` 도 부르지 말고 즉시 AskUserQuestion 한 번만 렌더해요:

   ```json
   {
     "question": "axhub 매니페스트(axhub.yaml)가 없어요. Vite React 앱으로 어떻게 초기화할까요?",
     "header": "초기화",
     "options": [
       {"label": "React/Vite로 초기화", "description": "현재 Vite 앱 기준으로 로컬 axhub.yaml만 만들고 배포 미리보기로 돌아가요."},
       {"label": "다른 템플릿 선택", "description": "템플릿 이름을 받아 새 앱 생성 흐름으로 넘어가요."},
       {"label": "취소", "description": "로컬 파일과 원격 리소스를 만들지 않고 멈춰요."}
     ]
   }
   ```

   subprocess 에서는 `취소`. `React/Vite로 초기화` 면 local manifest init 만 하고 deploy Step 1.1 로 돌아가 normal preview 를 보여줘요:

   ```bash
   APP_NAME="$(basename "$PWD" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9-' '-' | sed 's/^-//;s/-$//')"
   [ -n "$APP_NAME" ] || APP_NAME="axhub-app"
   axhub init --framework react-vite --target auto --app "$APP_NAME" --no-git --json
   axhub manifest validate --file axhub.yaml --json
   ```

   `axhub init --from-template` 같은 폐기 flag 나 손으로 쓴 YAML 을 만들지 말아요; `axhub init --framework react-vite` 가 manifest-only source of truth 예요. local write/validate 가 실패하면 reason 을 보여주고 remote 전에 멈춰요. 성공하면 Step 1.1 을 다시 돌려 새 `axhub.yaml` 을 source of truth 로 만든 뒤 preview card 를 보여줘요. local init 이 deploy 영향 uncommitted 변경을 만들었으면 (`git status --porcelain` for axhub.yaml/package/lock/vite config/index.html/src) Step 1.5 로 먼저 가서 fresh commit 에 담아요.

   returned destructive `axhub ... --json` 명령은 preview confirmation 뒤에만 top-level Bash 로 실행하고, 결과를 같은 pending metadata 로 FSM 에 record 해요 — `pending_action_id`/`pending_action_hash`/`command_argv`/`exit_code`/`stdout_json`/`stderr` 는 record JSON envelope 안에만 두고 user-facing chat 에는 절대 안 보여요:

   ```bash
   cat > /tmp/axhub-bootstrap-record.json <<JSON
   {
     "schema_version": "bootstrap-record/v1",
     "pending_action_id": "$PENDING_ACTION_ID",
     "pending_action_hash": "$PENDING_ACTION_HASH",
     "command_argv": $COMMAND_ARGV_JSON,
     "exit_code": $EXIT_CODE,
     "stdout_json": $STDOUT_JSON,
     "stderr": "$STDERR_JSON_ESCAPED"
   }
   JSON
   axhub plugin-support bootstrap --record "$NEXT_ACTION" --json < /tmp/axhub-bootstrap-record.json
   ```

   retry create 는 helper 출력이 idempotency key + retry policy 를 명시할 때만 해요. `no_retry_without_confirmed_idempotency` 나 `idempotency_unavailable` 이면 retry 하지 말고 typed stop 을 보여줘요.

1.5. **Git 저장 지점 준비** — resolve 가 `git_init_needed: true` OR `git_has_commit: false` OR `branch`/`commit_sha` 가 empty OR local manifest/bootstrap step 이 deploy 영향 uncommitted 변경을 만들었으면, preview 를 아직 보여주지 말아요. explanatory copy / AskUserQuestion 전에, 전체 TodoWrite 목록을 local git readiness 체크리스트로 교체해요.

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
   IN_FLIGHT_ID=$(echo "$DEPLOY_PREP_JSON" | jq -r '.in_flight_deploy.id // ""')
   if [ -n "$IN_FLIGHT_ID" ]; then
     IN_FLIGHT_COMMIT=$(echo "$DEPLOY_PREP_JSON" | jq -r '.in_flight_deploy.commit_sha // ""')
     RESOLVE_COMMIT=$(echo "$DEPLOY_PREP_JSON" | jq -r '.resolve.commit_sha // ""')
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
   GITHUB_CONNECTED=$(echo "$DEPLOY_PREP_JSON" | jq -r '.github_connected // false')
   STATUS_FIRST_ID=$(echo "$DEPLOY_PREP_JSON" | jq -r '.in_flight_deploy.id // ""')
   # github 연결 앱인데 in-flight 가 아직 안 보이면, push 자동배포 등록 시간을 잠깐 줘요 (interactive 만, 최대 ~15s).
   if [ -z "$STATUS_FIRST_ID" ] && [ "$GITHUB_CONNECTED" = "true" ] && [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then
     for _ in 1 2 3; do
       sleep 5
       REFRESH_JSON=$(axhub plugin-support deploy-prep --intent deploy --user-utterance "$ARGS" --refresh-in-flight --json 2>/dev/null || echo '{}')
       STATUS_FIRST_ID=$(echo "$REFRESH_JSON" | jq -r '.in_flight_deploy.id // ""')
       if [ -n "$STATUS_FIRST_ID" ]; then
         DEPLOY_PREP_JSON="$REFRESH_JSON"; IN_FLIGHT_ID="$STATUS_FIRST_ID"; break
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
   ① 앱:    paydrop (id=42)
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

4. **On user approval**, run deploy. Step 1.1b 가 이미 `deploy_create` 를 실행·record 했으면 이 Step 을 돌리지 말아요 (같은 pending bootstrap action 을 double-submit 금지). 이 Step 은 **fallback create 경로**예요 — Step 1.7 status-first 가 도는 배포를 못 찾았을 때만 도달해요.

   ```bash
   AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
   PROFILE_FLAG=()
   if [ -n "${PROFILE:-}" ] && [ "${PROFILE:-}" != "default" ]; then
     PROFILE_FLAG=(--profile "$PROFILE")
   fi
   AXHUB_STDERR_TMP=$(mktemp); AXHUB_STDOUT_TMP=$(mktemp)
   if [ "${DEPLOY_DECISION:-approve}" = "dry_run" ]; then
     axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --dry-run --json >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
   elif [ "${DEPLOY_DECISION:-approve}" = "abort" ]; then
     echo "배포를 멈춰요." >&2; rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"; exit 0
   else
     axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --tenant "$AXHUB_TENANT" --execute --json >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
   fi
   AXHUB_EXIT=$?
   # Format: "axhub-error-sub-key: 64:validation.deployment_in_progress"
   if [ $AXHUB_EXIT -eq 64 ] && grep -qE '^axhub-error-sub-key:.*64:validation\.deployment_in_progress' "$AXHUB_STDERR_TMP" 2>/dev/null; then
     # in-flight race: silent swallow raw stderr, re-fetch in-flight id for Step 5.
     REFRESH_JSON=$(axhub plugin-support deploy-prep --intent deploy --refresh-in-flight --json 2>/dev/null || echo '{}')
     IN_FLIGHT_ID=$(echo "$REFRESH_JSON" | jq -r '.in_flight_deploy.id // ""')
     if [ -n "$IN_FLIGHT_ID" ]; then
       DEPLOY_ID="$IN_FLIGHT_ID"
       COMMIT_SHA=$(echo "$REFRESH_JSON" | jq -r '.in_flight_deploy.commit_sha // .resolve.commit_sha // empty')
     else
       echo "다른 배포가 진행 중이에요. 잠시 뒤에 다시 시도해요." >&2
       rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"; exit 0
     fi
  elif [ $AXHUB_EXIT -eq 0 ]; then
    DEPLOY_ID=$(jq -r '.id // .deployment_id // empty' "$AXHUB_STDOUT_TMP")
    if [ -z "$DEPLOY_ID" ]; then
      echo "배포 시작은 확인했지만 결과 확인 id 를 못 받았어요. '배포 상태 확인해줘'라고 말하면 이어서 볼게요." >&2
      rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"; exit 0
    fi
    echo "배포 결과를 확인하고 있어요." >&2
   else
     cat "$AXHUB_STDERR_TMP" >&2; cat "$AXHUB_STDOUT_TMP"
   fi
   rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"
   ```

   `DEPLOY_ID` 는 deploy create stdout JSON 의 `.id` (또는 in-flight race 시 재조회한 in-flight id) 예요. Step 5 verify 가 이 id 를 써요.

5. **Success declaration — verify once (배포 성공 선언은 verify 로만).** 배포가 끝났는지 / 성공인지 선언은 **오직 `axhub deploy verify <deployment-id>` 한 번**으로 해요. deployment id 는 Step 4 의 `DEPLOY_ID` (또는 Step 1.6/1.7 의 `$IN_FLIGHT_ID`, Step 1.1b 의 recorded deploy id) 예요. prose polling 절차나 `deploy status --watch` 기반 성공 주장은 하지 않아요.

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
   - exit 9/64/67 + `github.git_connection_required` / `git_connection_required` / `precondition_failed` (stderr contains "GitHub 저장소 연결" / "GitHub 연결이 먼저 필요해요") → "지금 GitHub repo 연결 진행할까요?" 를 묻지 말고 직접 GitHub connection block 을 보여줘요:

     ```bash
     axhub apps git status --app "$APP_ID" --json
     ```

     그 출력의 첫 `install_url` 을 `GitHub 연결 링크: <install_url>` 로 보여줘요. repo 가 아직 없으면 `GitHub repo 만들기: https://github.com/new?name=$APP_SLUG` 도 context 로 보여줘요. 그 다음 `axhub apps git` guided setup/connect 로 라우팅해요 — repo create, remote add, first push, connect 승인은 거기서 소유해요.

     ```bash
     axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json
     ```

     위 명령을 사용자의 다음 수동 명령으로 제시하지 말아요. guided ladder 가 repo visibility 를 검증하고 명시 승인을 받은 뒤에만 실행되는 최종 명령이에요. 계정이 이미 설치돼 있고 원하는 repo 가 `axhub apps git status` 에 보이면 repo 가 준비됐다고 알리고 바로 approved-connect 로 라우팅해요.
   - exit 4 (CLI) / 65 (helper deploy-prep·preflight·token-gate) → token expired template + AskUserQuestion to run auth login
   - exit 5 (CLI) / 67 (helper deploy-prep) → resource not found + did-you-mean suggestion from apps list
   - exit 6 (CLI) / 68 (helper) → rate limit + Retry-After backoff
   - exit 1 → transport error; retry at most once for read paths, never for create

7. **Dry-run NL trigger** — if the user said "한번 해보기만", "리허설", "테스트로", "진짜 안 올리고", add `--dry-run` to Step 4 and skip Step 5 verify.

## v0.2.0 command coverage polish

### deploy list

Read-only deployment browsing:

```bash
axhub deploy list --app "$APP_ID" --json
```

pagination 이 보이면 첫 페이지만 보여주고 follow-up 을 제안해요 (긴 목록 dump 금지).

### deploy cancel

Cancel is a mutation. Preview the in-progress deployment first (app id/slug, deployment id, branch/commit, current status, expected effect), confirm approval, then run:

```bash
axhub deploy cancel "$DEPLOYMENT_ID" --app "$APP_ID" --execute --json
```

After cancellation, run a read-only status check and summarize the terminal state.

## NEVER

- NEVER treat `axhub plugin-support bootstrap --auto-chain --json` as approval; it is only a plan/record FSM.
- NEVER retry `apps_create` or `deploy_create` unless bootstrap returns a confirmed idempotency key and retry policy that allows retry.
- NEVER skip `bootstrap --record` after a returned top-level destructive command finishes; pending action correlation is the audit trail.
- NEVER retry `axhub deploy create` on exit 64.
- NEVER drop `--json` (parsing relies on it).
- NEVER call `axhub deploy create --execute` without the AskUserQuestion preview decision, except when bootstrap has already returned a recorded pending action to execute exactly once.
- NEVER declare deploy success from `deploy status --watch` output or a prose polling loop. Success declaration is `axhub deploy verify <deployment-id>` run once.
- NEVER call `axhub deploy verify` without a deployment id (latest 재탐색 금지 — correlation 계약).
- NEVER change command semantics after approval (omitting `--execute`, changing `--app`, or `--commit`). Surface the typed reason in one jargon-free line and stop, or fall back to Step 1.7 status-first watch.
- NEVER instruct the user to run `axhub deploy create`, `axhub deploy verify`, or any deploy CLI command themselves. The agent runs deploy and verify itself inside this SKILL flow; if blocked, surface the typed reason in one jargon-free line and stop.
- NEVER run `deploy create` when Step 1.7 status-first already found an in-flight deploy for this app; route to verify instead.
- NEVER call `axhub deploy cancel` without explicit confirmation.
- NEVER infer `app_id` from `pwd` or git remote alone in the mutation path; always live resolve through deploy-prep.
- NEVER bypass the AskUserQuestion preview card on slash invocation; slash is explicit confirmation for the SKILL invocation, not for the destructive operation.
- NEVER insert the old approved-run helper bridge between preview approval and the canonical deploy workflow; approval must flow into `deploy-prep` / bootstrap / public `axhub deploy create --execute --json` / verify.

## Additional Resources

For Korean trigger lexicon (informal, honorific, demo-context variants): `references/nl-lexicon.md`.
For exit-code → 4-part Korean error template (emotion + cause + action + button): `references/error-empathy-catalog.md`.
