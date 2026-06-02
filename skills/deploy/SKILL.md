---
name: deploy
description: '이 스킬은 사용자가 현재 브랜치를 axhub 라이브로 배포하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "공개해", "내보내자", "띄워", "배포", "배포해", "배포해줘", "쏘자", "올려", "올리자", "터트려", "푸시한 거 띄워", "프로덕션", "프로덕션에 박아", "demo가 필요", "demo가 필요해", "deploy", "launch", "release", "rollout", "ship", 또는 현재 브랜치를 axhub 라이브로 push 하고 싶다는 모든 의도. axhub deploy create 를 안전 가드와 함께 트리거: 라이브 profile/app 해석, AskUserQuestion preview card 를 통한 HMAC consent gate, exit-code 기반 복구 라우팅.'
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
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Deploy via axhub

Deploy a vibe coder's app to axhub with safety primitives. Use the adapter `axhub-helpers` (auto on PATH while plugin is enabled) for live resolution and consent management. Do not call `axhub deploy create` directly without going through the helper flow.

## Vibe Coder Visibility Rules

이 SKILL 을 쓰는 사람은 대부분 개발 지식이 없어요. helper 가 돌려주는 다음 field 는 **internal verification primitives** 예요. PreToolUse hook 가 동의 토큰 검증에 쓰고, retry / record FSM 도 같은 값으로 동작해요. 그래서 SKILL 안에서는 이 field 들을 변수에 담아 helper 와 주고받되, **raw 값을 사용자 chat 에 echo 하면 안 돼요**:

- `binding_hash`, `pending_action_id`, `pending_action_hash`, `command_argv`, `command_id`
- `consent_binding`, `synthesized_by_helper`, `retry_policy`, `idempotency_key`
- `exit_code`, `next_action`, `schema_version`, `stdout_json`, `stderr` (raw)
- `bootstrap_plan`, `required_steps`, `decision_inputs`

대신 사용자에게는 한국어 한 줄로 진행 상황만 알려드려요. 예시 templates:

| 시점 | 사용자 chat 한 줄 |
|------|-------------------|
| Step 1 첫 배포 / app 등록 | "처음 배포라 앱을 먼저 만들고 있어요." |
| Step 1.5 git 저장 지점 준비 | "배포 전에 파일 저장 지점을 만들어두고 있어요." |
| Step 3 preview card | 5필드 한국어 카드만 (앱 / 환경 / 브랜치 / 커밋 / 예상 시간) |
| Step 4 consent → deploy | "배포 동의를 받았어요. 시작해요." |
| Step 5 watch | "빌드 시작했어요. 약 3분 뒤에 결과 알려드릴게요." |
| Step 6 exit 4 / 65 | "axhub 로그인이 만료됐어요. 다시 로그인할까요?" |
| Step 6 exit 64 | "다른 배포가 진행 중이라 지금은 못 올려요. 잠시 뒤에 다시 시도해요." |
| Step 6 exit 5 / 67 | "이 이름의 앱을 못 찾았어요. 비슷한 이름을 알려드릴게요." |

raw helper JSON 이 디버깅에 필요한 환경 (개발 검증) 은 `AXHUB_DEPLOY_VERBOSE=1` 환경변수가 켜진 경우에만 echo 해요. 기본 흐름은 항상 한 줄 자연어로 진행해요.

## Workflow

<!--
phase markers (Phase 0 baseline naming — keep aligned with
crates/axhub-helpers/src/telemetry.rs::record_phase_marker):
  - phase marker: step_0_preflight
  - phase marker: step_1_resolve
  - phase marker: step_1_1_bootstrap_plan
  - phase marker: step_2_preview_card
  - phase marker: step_3_consent
  - phase marker: step_4_deploy_create
  - phase marker: step_5_watch
runtime impact 0 — comments only.
-->

**CLAUDE_PLUGIN_ROOT 자동 확인.** 모든 helper 호출 전에 `CLAUDE_PLUGIN_ROOT` 를 자동으로 채워요. Claude Code 가 env 를 전달하지 않은 세션에서는 `CLAUDE_SKILL_DIR` 로 plugin root 를 계산하고, 그래도 없으면 PATH 의 `axhub-helpers` / `axhub-helpers.exe` 위치에서 root 를 역산해요. 성공하면 조용히 `PATH` 에 plugin `bin/` 을 앞에 붙이고, 사용자에게 절대경로 우회 안내를 시키지 않아요.

```bash
if [ -z "${CLAUDE_PLUGIN_ROOT:-}" ]; then
  if [ -n "${CLAUDE_SKILL_DIR:-}" ] && [ -d "${CLAUDE_SKILL_DIR}/../.." ]; then
    export CLAUDE_PLUGIN_ROOT="$(cd "${CLAUDE_SKILL_DIR}/../.." && pwd)"
  elif HELPER_FROM_PATH="$(command -v axhub-helpers 2>/dev/null)"; then
    export CLAUDE_PLUGIN_ROOT="$(cd "$(dirname "$HELPER_FROM_PATH")/.." && pwd)"
  elif [ -x "./bin/axhub-helpers" ]; then
    export CLAUDE_PLUGIN_ROOT="$(pwd)"
  fi
fi
if [ -n "${CLAUDE_PLUGIN_ROOT:-}" ]; then
  export PATH="${CLAUDE_PLUGIN_ROOT}/bin:${PATH}"
fi
```

Windows PowerShell 에서는 같은 규칙을 아래처럼 적용해요. native Windows 는 `.exe` helper 를 명시해요.

```powershell
if (-not $env:CLAUDE_PLUGIN_ROOT) {
  if ($env:CLAUDE_SKILL_DIR -and (Test-Path (Join-Path $env:CLAUDE_SKILL_DIR "..\.."))) {
    $env:CLAUDE_PLUGIN_ROOT = (Resolve-Path (Join-Path $env:CLAUDE_SKILL_DIR "..\..")).Path
  } elseif ($cmd = Get-Command axhub-helpers.exe -ErrorAction SilentlyContinue) {
    $env:CLAUDE_PLUGIN_ROOT = (Resolve-Path (Join-Path (Split-Path $cmd.Source -Parent) "..")).Path
  } elseif (Test-Path ".\bin\axhub-helpers.exe") {
    $env:CLAUDE_PLUGIN_ROOT = (Get-Location).Path
  } else {
    $AxhubCand = Get-ChildItem -Path (Join-Path $env:USERPROFILE ".claude\plugins\cache\axhub\axhub\*\bin\axhub-helpers.exe") -ErrorAction SilentlyContinue |
      Where-Object { $_.Directory.Parent.Name -match '^\d+\.\d+\.\d+$' } |
      Sort-Object { [version]$_.Directory.Parent.Name } | Select-Object -Last 1
    if ($AxhubCand) { $env:CLAUDE_PLUGIN_ROOT = $AxhubCand.Directory.Parent.FullName }
  }
}
if ($env:CLAUDE_PLUGIN_ROOT) {
  $env:PATH = (Join-Path $env:CLAUDE_PLUGIN_ROOT "bin") + [IO.Path]::PathSeparator + $env:PATH
}
```

**Routing 게이트 (Step 0 — auth/resolve 전에 실행).** 이 SKILL 은 `description:` 프론트매터의 "배포"·"deploy"·"ship" 같은 어구로도 자동 선택돼서, axhub 와 무관한 프로젝트나 다른 배포 타깃(`vercel` 등)을 쓰려는 발화에도 끌려올 수 있어요. 그래서 **인증·resolve 를 하기 전에** 공유 routing-decision 함수(`route-decision`)를 한 번 호출해서 정말 axhub 배포가 맞는지 먼저 확정해요. 이 함수는 hook 과 **똑같은** 결정 로직이라 두 레이어가 어긋나지 않아요 (named-target-wins 일관성).

`EXPLICIT` 은 호출 모달리티예요. 이 SKILL 이 `/deploy`, `/axhub:deploy`, 또는 한글 alias `/배포` **슬래시 명령**으로 호출됐으면 `EXPLICIT=1`, 자연어 skill-selection("배포해", "vercel에 배포해")으로 왔으면 `EXPLICIT=0` 으로 둬요. 슬래시면 leading `/deploy`·`/배포` 토큰이 `$ARGS` 에 안 남을 수 있어서(command 가 `$ARGUMENTS` 만 넘겨요) 모델이 직접 신호를 줘야 해요. 확실하지 않으면 `EXPLICIT=1` 로 둬요 (explicit 으로 간주 — 명시 의도를 막지 않아요). `$ARGS` 에는 app slug 만이 아니라 사용자 발화 원문을 그대로 담아서 `vercel` 같은 타깃 키워드가 살아 있게 해요.

```bash
echo '[deploy:Step 0 routing-gate] entered' >&2
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
EXPLICIT_FLAG=""
[ "${EXPLICIT:-0}" = "1" ] && EXPLICIT_FLAG="--explicit"
ROUTE_JSON=$("$HELPER" route-decision --user-utterance "$ARGS" $EXPLICIT_FLAG 2>/dev/null)
# fail-open: 빈 출력(헬퍼 자체가 없음)이면 axhub 로 진행해요 — SKILL 은 이미 선택된 상태고,
# 실제 배포는 뒤의 AskUserQuestion preview card + HMAC consent gate 가 막아요(zero 피해).
ROUTE_DECISION=$(printf '%s' "$ROUTE_JSON" | jq -r '.decision // "axhub"' 2>/dev/null || echo axhub)
echo "[deploy:Step 0 routing-gate] decision=$ROUTE_DECISION" >&2
echo "$ROUTE_JSON"
```

`ROUTE_DECISION` 값으로 분기해요. **`axhub` 일 때만 axhub 배포를 진행**해요:

- **`axhub`** → 정상 경로. 아래 Preflight 부터 Step 1 (deploy-prep) 로 계속 진행해요.
- **`yield`** → 사용자가 다른 배포 타깃(예: `vercel`/`netlify`/`cloudflare`/`fly`/`render`/`railway`)을 명시했어요 (marker 가 있어도 named-target-wins). axhub 배포를 멈추고 disambiguation 질문 없이 한 줄로 "다른 배포 타깃을 쓰려는 것 같아서 axhub 배포는 건너뛸게요." 만 안내한 뒤 일반 흐름에 양보해요. **Preflight·deploy-prep·consent-mint·`axhub deploy create` 를 하나도 호출하지 말아요.**
- **`ignore`** (marker 없음 + 무명시) / **`ask`** (axhub 와 다른 타깃 둘 다 명시) → axhub 인지 확실하지 않아요. 아래 AskUserQuestion 으로 한 번 물어봐요. 사용자가 "axhub 에 배포" 를 고르면 그때 Preflight 부터 이어가고, "여기 말고 다른 곳" 을 고르면 axhub 배포를 멈춰요. **물어보기 전에는 auth/resolve/consent 를 호출하지 말아요.**

```json
{
  "question": "axhub 에 배포할까요, 아니면 다른 곳에 배포할까요?",
  "header": "배포 대상",
  "options": [
    {
      "label": "axhub 에 배포",
      "value": "axhub",
      "description": "axhub 라이브로 배포를 이어가요."
    },
    {
      "label": "여기 말고 다른 곳",
      "value": "other",
      "description": "axhub 배포를 멈춰요. 다른 배포 도구를 쓸게요."
    }
  ]
}
```

이 게이트의 AskUserQuestion 도 아래 **Non-interactive AskUserQuestion guard (D1)** 를 따라요. subprocess(`claude -p` / CI / `$CLAUDE_NON_INTERACTIVE`) 에서는 질문을 건너뛰고 `tests/fixtures/ask-defaults/registry.json` 의 deploy 채널 safe default ("여기 말고 다른 곳" — axhub 배포 안 함) 로 멈춰요. once-per-project grace 경고는 prompt-route hook 소유라 여기서 다시 띄우지 않아요 (게이트는 매번 block — 의도적 이중 노출).

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

**Command lane.** POSIX/Git Bash/WSL 은 `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers` 를 쓰고, Windows PowerShell 은 `& "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe"` 를 써요. JSON stdin 이 필요한 helper 호출은 PowerShell 에서 `ConvertTo-Json -Compress | & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" <subcommand>` 형태로 실행해요. Bash 배열 예시는 Windows 에서 그대로 붙여넣지 말고 PowerShell 배열 (`$ProfileArgs = @("--profile", $env:PROFILE)`) 로 바꿔요.

To deploy:

0. **Render TodoWrite checklist — derive it from the actual deploy path (don't paste a fixed list).** Call TodoWrite at workflow start so the vibe coder sees real-time progress. The items depend on what `deploy-prep` returns in Step 1: a git-connected app whose push auto-deploys **watches** a deploy it did not create (status-first), while a first-deploy or non-git app mints consent and runs `deploy create`. Read the situation, then write the todos that match it — the two blocks below are reference shapes, not a script to paste. Reference shape A — git-connected / status-first watch:

   ```typescript
   TodoWrite({ todos: [
     { content: "배포 상태 확인 (preflight)", status: "in_progress", activeForm: "배포 상태 확인하는 중" },
     { content: "최신 저장 지점 푸시 확인",     status: "pending",     activeForm: "푸시 상태 보는 중" },
     { content: "자동 시작된 배포 따라가기",     status: "pending",     activeForm: "배포 따라가는 중" },
     { content: "결과 안내",                  status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   Reference shape B — first deploy / non-git (explicit create after status-first finds nothing):

   ```typescript
   TodoWrite({ todos: [
     { content: "토큰 확인 (preflight)",         status: "in_progress", activeForm: "토큰 확인하는 중" },
     { content: "앱 / 환경 / 브랜치 확정",         status: "pending",     activeForm: "앱 정보 정리하는 중" },
     { content: "git 저장 지점 확인",             status: "pending",     activeForm: "배포용 저장 지점 보는 중" },
     { content: "미리보기 카드 보여드리기",         status: "pending",     activeForm: "미리보기 준비하는 중" },
     { content: "동의 받고 배포 시작",            status: "pending",     activeForm: "배포 시작하는 중" },
     { content: "빌드 모니터 (~3분)",             status: "pending",     activeForm: "빌드 진행 보는 중" },
     { content: "결과 안내",                     status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.
   TodoWrite 상태는 Claude Code 세션 안에서 이어질 수 있어요. 그래서 이 스킬을 시작할 때는 기존 todo 에 항목을 하나씩 더하거나 일부만 고치지 말고, 위 배열 전체로 교체해요. 이전 스킬 todo 가 화면에 남아 있으면 Step 1 전에 deploy 목록만 보이도록 다시 호출해요.

1. **Live resolve + preflight (parallel via deploy-prep).** Fetch authoritative `{profile, endpoint, app_id, app_slug, branch, commit_sha, commit_message, eta_sec}` AND preflight (`auth_ok`, `cli_too_old/new`) in one helper call. Phase 1 runs preflight + resolve in parallel via `std::thread::scope`, so Step 2 (re-preflight) and Step 1.2 (re-resolve) below are skipped on the default path:

   ```bash
   echo '[deploy:Step 1 deploy-prep] entered' >&2
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   DEPLOY_PREP_JSON=$("$HELPER" deploy-prep --intent deploy --user-utterance "$ARGS" --json)
   echo "$DEPLOY_PREP_JSON"
   ```

   `DEPLOY_PREP_JSON` 변수는 Step 1.6 / Step 3.6 에서 `.in_flight_deploy` 필드를 jq 로 읽을 때 다시 사용해요.

   The JSON envelope contains `{preflight, resolve, bootstrap_plan?, quality_gate?, exit_code}`. Use `jq -r '.resolve.app_id'` and friends to extract fields. If `.quality_gate.passed == false`, show the violations first and stop by default. 대화형 모드에서만 아래 AskUserQuestion 으로 위험한 강제 진행을 허용해요. subprocess / CI 에서는 `tests/fixtures/ask-defaults/registry.json` 의 `quality_gate.abort_or_proceed` 와 deploy 질문 기본값을 따라 `취소`예요.

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

   If `bootstrap_plan` is non-null, this is a first-deploy path — fall through to Step 1.1 below. If `exit_code == 65`, surface auth recovery (Step 6 path). If `exit_code == 64` and `quality_gate.passed` is not false, surface version-skew recovery. If `exit_code == 67` AND `bootstrap_plan` is null, treat as ambiguous resolve.

   **Backwards-compat fallback (1 release cycle):** when `AXHUB_DEPLOY_PREP=0` is set, the helper exits silently with no JSON — Step 1 falls through to the legacy `resolve` call below, and Step 1.2 / Step 2 re-runs are not skipped:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   if [[ "${AXHUB_DEPLOY_PREP:-1}" == "0" ]]; then
     echo '[deploy:Step 1 resolve legacy] entered' >&2
     "$HELPER" resolve --intent deploy --user-utterance "$ARGS" --json
   fi
   ```

   Never use cached `app_id` for mutation. If resolve returns an `app_id`, this is an existing app deploy: do **not** run `bootstrap apps_create`, and continue with git readiness, preview, and the normal consent-deploy path. If resolve returns ambiguity, ask the user to disambiguate (slug list with numeric IDs). If resolve cannot identify a registered app and the project has an `axhub.yaml`/`apphub.yaml`, enter the first-run bootstrap bridge below. The resolve JSON also includes `git_repo`, `git_has_commit`, and `git_init_needed`; deploy MUST NOT continue to the preview card while `branch` or `commit_sha` is empty.

1.1. **First-run bootstrap plan/record bridge (Sprint 3).** Use this only when Step 1 did not resolve an existing `app_id`. Before any first-run remote mutation, ask the Rust FSM for the next safe step:

   ```bash
   echo '[deploy:Step 1 bootstrap-plan] entered' >&2
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   "$HELPER" bootstrap --auto-chain --json
   ```

   Treat this output as the source of truth for Sprint 3 bootstrap state. If it returns `template_required`, `git_init_required`, `first_commit_required`, `subdomain_collision`, `backend_contract_missing_defaults`, or `idempotency_unavailable`, stop at that user-decision state and surface a humanized one-line reason plus the safest next command (jargon-free per Vibe Coder Visibility Rules). If it returns `next_action: apps_create` or `next_action: deploy_create`, **internally bind** `command`, `binding_hash`, `pending_action_id`, `pending_action_hash`, `retry_policy`, and `consent_binding` into shell variables (PreToolUse hook consumes them for consent verification + retry policy enforcement) but **do not echo their raw values to the user chat** — those are internal verification primitives. Show the user a single humanized line such as "처음 배포라 앱을 먼저 만들고 있어요." and proceed to consent + execution. The helper is only a planner/recorder here; it must not be treated as approval to mutate. If `deploy_create` is executed and recorded here, do not mint or run a second `deploy_create` in Step 4; jump to Step 5 status-chain with the recorded deployment id.

   Execute returned destructive `axhub ... --json` commands only as top-level Bash after the consent path runs (consent token mint via `consent-mint`). Then record the observed result back into the FSM with the same pending metadata — keep `pending_action_id` / `pending_action_hash` / `command_argv` / `exit_code` / `stdout_json` / `stderr` strictly inside the record JSON envelope, never as user-facing chat text:

   ```bash
   echo '[deploy:Step 1 bootstrap-record] entered' >&2
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
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
   "$HELPER" bootstrap --record "$NEXT_ACTION" --json < /tmp/axhub-bootstrap-record.json
   ```

   S3B retry ownership lives in this skill because this skill runs the top-level command. Retry a create only when helper output explicitly provides an idempotency key and a retry policy that allows it. If the helper says `no_retry_without_confirmed_idempotency` or returns `idempotency_unavailable`, do not retry; show the typed stop.

1.2. **Fresh resolve after local/bootstrap state changes (legacy fallback only).** Phase 1 default path skips this — `deploy-prep` already covers it. This block runs only when `AXHUB_DEPLOY_PREP=0` is set, or when Step 1.5 (git-init) materially changed local commit identity since the deploy-prep call:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   if [[ "${AXHUB_DEPLOY_PREP:-1}" == "0" ]] || [[ "${AXHUB_RESOLVE_AFTER_GIT_INIT:-0}" == "1" ]]; then
     echo '[deploy:Step 1 resolve refresh] entered' >&2
     "$HELPER" resolve --intent deploy --user-utterance "$ARGS" --json
   fi
   ```

   Never use cached `app_id` for mutation. If resolve still returns ambiguity, ask the user to disambiguate (slug list with numeric IDs). Deploy MUST NOT continue to the preview card while `branch` or `commit_sha` is empty.

1.5. **Git 저장 지점 준비** — if resolve returns `git_init_needed: true` OR `git_has_commit: false` OR either `branch`/`commit_sha` is empty, do not show the deploy preview yet. Before showing any explanatory copy or AskUserQuestion, replace the full TodoWrite list with the local git readiness checklist. Do not render this plan as a markdown checklist; Claude Code TodoWrite is the progress UI for every 3+ step branch.

   ```typescript
   TodoWrite({ todos: [
     { content: "git 저장소 만들기",        status: "in_progress", activeForm: "git 저장소 만드는 중" },
     { content: "파일을 첫 저장 지점에 담기", status: "pending",     activeForm: "파일 담는 중" },
     { content: "첫 커밋 만들기",          status: "pending",     activeForm: "첫 커밋 만드는 중" },
     { content: "배포 정보 다시 확인하기",   status: "pending",     activeForm: "배포 정보 다시 보는 중" },
     { content: "미리보기 카드 보여드리기",  status: "pending",     activeForm: "미리보기 준비하는 중" }
   ]})
   ```

   Then explain in non-developer Korean (jargon-free):

   ```
   배포 전에 파일을 저장 지점에 한 번 담아둬야 해요.
   이렇게 해야 어떤 버전을 올릴지 정확히 알 수 있어요.
   지금은 아직 그 저장 지점이 없어서, 제가 자동으로 만들어드릴게요.
   ```

   Then ask (2-option humanized prompt — vibe coder 친화):

   ```json
   {
     "question": "배포 전 저장 지점을 만들까요?",
     "header": "저장 지점",
     "options": [
       {
         "label": "지금 만들기",
         "value": "init_and_continue",
         "description": "현재 폴더에 저장 지점을 자동으로 만들고 배포를 이어가요."
       },
       {
         "label": "취소",
         "value": "abort",
         "description": "배포를 멈춰요."
       }
     ]
   }
   ```

   If the user chooses "지금 만들기", run the local git commands silently (do not echo the raw `git init` / `git add` / `git commit` command output to the chat — surface a one-line "저장 지점을 만들고 있어요." instead). Then re-run resolve and continue from Step 2. Keep the git readiness TodoWrite list on screen and update statuses as each command finishes. 이 TodoWrite 호출도 기존 목록을 기준으로 patch 하지 말고 전체 교체로 실행해요. If another skill or stale todo list appears, replace the whole list again instead of patching individual items. 이전 스킬 todo 를 섞으면 사용자가 지금 흐름을 잘못 이해해요.

   ```bash
   echo '[deploy:Step 1.5 git-init] entered' >&2
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
     git init >/dev/null 2>&1
   fi
   git add -A >/dev/null 2>&1
   git commit -m "init: axhub deploy baseline" >/dev/null 2>&1 || true
   git branch -M main >/dev/null 2>&1
   "$HELPER" resolve --intent deploy --user-utterance "$ARGS" --json
   ```

   git stderr 와 stdout 은 모두 `/dev/null` 로 보내요 — vibe coder chat 에 raw git output 이 노출되지 않게 구조적으로 막아요. `git commit` 이 실패하면 (no staged files / missing git identity) `|| true` 로 다음 줄로 넘어가고, 뒤따르는 resolve 호출이 `branch` / `commit_sha` 가 비어 있다고 알려서 humanized 한 줄로 사용자에게 안내해요. `AXHUB_DEPLOY_VERBOSE=1` 환경변수가 켜진 세션에서는 Visibility Rules 가 verbose lane 으로 전환되어 raw 출력이 다시 보여요 (개발 검증용).

   If `git commit` fails because there are no staged files or git identity is missing, stop before deploy and surface a humanized one-line reason ("저장 지점을 만들지 못했어요. 잠시 뒤에 다시 시도해요." 같은 한 줄). 내부 git stderr 는 user chat 에 직접 echo 하지 마요. Do not mint deploy consent until a fresh resolve returns both `branch` and `commit_sha`.
   If the user chooses "취소", stop deploy without running any git command. In non-interactive mode (subprocess / CI / `claude -p`), use the registry safe default "취소" — never run `git init` automatically in headless context.

1.6. **In-flight deploy 감지 (배포 충돌 방지) — 3-way 분기.** `deploy-prep` 응답에 `.in_flight_deploy.id` 가 non-null 이면 이미 진행 중인 배포가 있어요. `in_flight_deploy.commit_sha` 와 `resolve.commit_sha` 비교로 3 가지 sub-step (1.6a / 1.6b / 1.6c) 중 어느 분기로 진입할지 결정해요.

   **Ownership 추론 한계 (issue #87).** 현재 ownership 판별은 `commit_sha` 비교만 사용해요. mono-repo team 의 same-HEAD case (다른 사람이 본인 HEAD 와 동일 commit 으로 push) 에서 본인 / 다른 사람 구분 못 해요 — Step 1.6a (same-commit) 가 다른 user 의 in-flight 를 본인 deploy 로 routing 할 수 있어요. 정식 fix 는 backend `BackendDeployment.owner_user_id` field 도착 후 진행해요 (별 RFC, Phase 2). 그래서 Step 1.6b copy 도 "가능성이 있어요" 로 약화해서 false confidence 회피해요.

   - **Step 1.6a (same-commit)**: 두 commit_sha 모두 non-empty 이고 일치 — 본인 배포 중복 가능성 (또는 mono-repo same-HEAD edge). 기존 "이미 배포가 진행 중이에요." prompt.
   - **Step 1.6b (cross-tenant)**: 두 commit_sha 모두 non-empty 이고 다름 — 다른 user 의 in-flight 가능성. "다른 사람이 같은 앱에 배포 중일 가능성이 있어요." prompt.
   - **Step 1.6c (uncertain)**: 둘 중 하나가 empty (commit_sha missing) — uncertain state. "배포 중인 게 있는데 누구 건지 확인 중이에요." prompt (silent misidentification 차단).

   ```bash
   echo '[deploy:Step 1.6 in-flight-check] entered' >&2
   IN_FLIGHT_ID=$(echo "$DEPLOY_PREP_JSON" | jq -r '.in_flight_deploy.id // ""')
   if [ -n "$IN_FLIGHT_ID" ]; then
     IN_FLIGHT_COMMIT=$(echo "$DEPLOY_PREP_JSON" | jq -r '.in_flight_deploy.commit_sha // ""')
     RESOLVE_COMMIT=$(echo "$DEPLOY_PREP_JSON" | jq -r '.resolve.commit_sha // ""')
     CREATED_AT=$(echo "$DEPLOY_PREP_JSON" | jq -r '.in_flight_deploy.created_at // ""')
     NOW_SEC=$(date +%s)
     CREATED_SEC=$(date -d "$CREATED_AT" +%s 2>/dev/null || date -j -f '%Y-%m-%dT%H:%M:%SZ' "$CREATED_AT" +%s 2>/dev/null || echo 0)
     DELTA=$((NOW_SEC - CREATED_SEC))
     # 3-way 분기 결정
     if [ -z "$IN_FLIGHT_COMMIT" ] || [ -z "$RESOLVE_COMMIT" ]; then
       INFLIGHT_BRANCH="uncertain"  # → Step 1.6c
     elif [ "$IN_FLIGHT_COMMIT" = "$RESOLVE_COMMIT" ]; then
       INFLIGHT_BRANCH="same"  # → Step 1.6a
     else
       INFLIGHT_BRANCH="cross_tenant"  # → Step 1.6b
     fi
     # non-interactive: safe default = abort (모든 분기 공통)
     if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
       echo "[deploy:Step 1.6 $INFLIGHT_BRANCH] non-interactive → abort" >&2
       exit 0
     fi
   fi
   ```

   1.6a — Step 1.6a (same-commit). AskUserQuestion JSON (해요체, 3-option). 최근 60초 이내 (`DELTA` ≤ 60) 면 "진행 중인 배포 보기" default highlight, 60초 넘으면 "새 배포 시작" default highlight.

   ```json
   {
     "question": "이미 배포가 진행 중이에요. 어떻게 할까요?",
     "header": "배포 충돌",
     "options": [
       {
         "label": "진행 중인 배포 보기",
         "value": "monitor",
         "description": "현재 진행 중인 배포 상태를 실시간으로 확인해요."
       },
       {
         "label": "새 배포 시작",
         "value": "force_new",
         "description": "진행 중인 배포와 별개로 지금 바로 새 배포를 올려요."
       },
       {
         "label": "취소",
         "value": "abort",
         "description": "배포를 멈춰요."
       }
     ]
   }
   ```

   1.6b — Step 1.6b (cross-tenant). 다른 사용자 의 in-flight 라 더 보수적이에요. default highlight 는 "취소".

   ```json
   {
     "question": "다른 사람이 같은 앱에 배포 중일 가능성이 있어요. 어떻게 할까요?",
     "header": "배포 충돌",
     "options": [
       {
         "label": "진행 중인 배포 보기",
         "value": "monitor",
         "description": "다른 사용자 의 배포 결과를 실시간으로 확인해요."
       },
       {
         "label": "새 배포 시작",
         "value": "force_new",
         "description": "다른 사용자 의 배포와 별개로 지금 바로 새 배포를 올려요."
       },
       {
         "label": "취소",
         "value": "abort",
         "description": "배포를 멈춰요."
       }
     ]
   }
   ```

   1.6c — Step 1.6c (uncertain). commit_sha missing → 누구 배포인지 판단 불가. silent misidentification 차단 위해 explicit uncertainty surface. default highlight 는 "취소".

   ```json
   {
     "question": "배포 중인 게 있는데 누구 건지 확인 중이에요. 어떻게 할까요?",
     "header": "배포 충돌",
     "options": [
       {
         "label": "진행 중인 배포 보기",
         "value": "monitor",
         "description": "진행 중인 배포 결과를 일단 지켜봐요."
       },
       {
         "label": "새 배포 시작",
         "value": "force_new",
         "description": "확인 안 되는 채로 지금 바로 새 배포를 올려요."
       },
       {
         "label": "취소",
         "value": "abort",
         "description": "배포를 멈춰요."
       }
     ]
   }
   ```

   - `monitor` 선택 시: Step 5 status-chain 으로 바로 이동해 `$IN_FLIGHT_ID` 를 watch 해요. 새 `deploy create` 는 실행하지 않아요.
   - `force_new` 선택 시: Step 2 로 진행해요. exit 64 + `validation.deployment_in_progress` 에러가 나도 retry 하지 않아요 (Step 6 라우팅 따름).
   - `abort` 선택 시: 배포를 멈춰요. consent 를 발급하지 않아요.

1.7. **Status-first gate (배포는 status 먼저 — `deploy create` 는 fallback).** push 가 자동배포를 트리거하는 환경(`deploy-prep` 의 `.github_connected: true`)에서는 preview/consent 로 가기 전에 **지금 돌고 있는 배포가 있는지 먼저 확인**해요. push 로 이미 시작된 배포가 있는데 새 `deploy create` 를 실행하면, exit 64 충돌이나 consent/commit 불일치 deny 로 이어져서 토큰 재발급·우회 루프에 빠져요. 도는 배포가 있으면 그걸 따라가고(create 생략), 없을 때만 Step 2 이후 명시적 create 로 진행해요 — 이게 "status 보고 배포가 아니면 그제서야 진짜 create" 예요. 단, Step 1.6 이 이미 in-flight 를 처리했으면 (특히 사용자가 거기서 `force_new` 를 골랐으면) 그 선택을 존중해서 이 gate 는 건너뛰고 Step 2 로 진행해요 — 같은 in-flight 를 다시 watch 로 되돌리지 않아요.

   ```bash
   echo '[deploy:Step 1.7 status-first] entered' >&2
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   GITHUB_CONNECTED=$(echo "$DEPLOY_PREP_JSON" | jq -r '.github_connected // false')
   STATUS_FIRST_ID=$(echo "$DEPLOY_PREP_JSON" | jq -r '.in_flight_deploy.id // ""')
   # github 연결 앱인데 in-flight 가 아직 안 보이면, push 자동배포가 backend 에 등록될 시간을 잠깐 줘요.
   # interactive 에서만 짧게 폴링해요 (최대 ~15s, 5s × 3). non-interactive 는 추가 대기 없이 캐시값만 써요.
   if [ -z "$STATUS_FIRST_ID" ] && [ "$GITHUB_CONNECTED" = "true" ] && [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then
     for _ in 1 2 3; do
       sleep 5
       REFRESH_JSON=$("$HELPER" deploy-prep --intent deploy --user-utterance "$ARGS" --refresh-in-flight --json 2>/dev/null || echo '{}')
       STATUS_FIRST_ID=$(echo "$REFRESH_JSON" | jq -r '.in_flight_deploy.id // ""')
       if [ -n "$STATUS_FIRST_ID" ]; then
         DEPLOY_PREP_JSON="$REFRESH_JSON"
         IN_FLIGHT_ID="$STATUS_FIRST_ID"
         break
       fi
     done
   fi
   ```

   - `STATUS_FIRST_ID` 가 non-empty → 이미 배포가 돌고 있어요. **여기서 Step 1.6 의 3-way 분기를 그대로 재사용**해요: `.in_flight_deploy.commit_sha` 와 `.resolve.commit_sha` 를 비교해서 same-commit 이면 본인 push 배포라 바로 Step 5 watch (`monitor`) 로 가고, cross-tenant / uncertain 이면 1.6b / 1.6c AskUserQuestion 으로 사용자에게 물어요. 이 경로에서는 **consent-mint 도 `deploy create` 도 실행하지 않아요** — 남의 배포를 말없이 본인 것처럼 따라가지 않으려고 same-commit 일 때만 자동 watch 예요.
   - `STATUS_FIRST_ID` 가 empty (도는 배포 없음) → Step 2 이후 명시적 create 경로로 진행해요. github 연결이 아니거나, 폴링 동안에도 자동배포가 안 잡힌 경우예요.
   - hook 이 `deploy create` 를 deny 하면 (consent/binding 불일치) flag 를 빼거나 wrapper 로 우회하지 말고, 사유를 한 줄로 알린 뒤 멈추거나 이 status-first watch 로 돌아와요 (NEVER 절 참조).

2. **Pre-flight version check (legacy fallback only).** Phase 1 default path skips this — `deploy-prep` already returned the preflight envelope as `.preflight` in Step 1's JSON. Use the cached value: read `cli_too_old`, `cli_too_new`, `auth_ok` directly via `jq`. The block below is the legacy fallback path that fires only when `AXHUB_DEPLOY_PREP=0` is set:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   if [[ "${AXHUB_DEPLOY_PREP:-1}" == "0" ]]; then
     echo '[deploy:Step 2 preflight legacy] entered' >&2
     "$HELPER" preflight --json
   fi
   ```

   On `cli_too_old: true`, halt and surface the corresponding entry from `references/error-empathy-catalog.md` ("version-skew"). Do not proceed.

   On `cli_too_new: true`, run the dismiss bridge below. The user can suppress the prompt for the current CLI version range by storing `ignore_too_new_until` in the helper preferences file.

2.5. **cli_too_new dismiss bridge (Phase 3.5 B-11).** Read user preference, decide to halt / proceed / prompt:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   if [[ "$CLI_TOO_NEW" == "true" ]]; then
     IGNORE_UNTIL=$("$HELPER" config get ignore_too_new_until --json 2>/dev/null | jq -r '.value // ""')
     CLI_VER=$(echo "$PREFLIGHT_JSON" | jq -r '.cli_version // ""')
     # Skip prompt if user previously dismissed at this CLI_VER or higher.
     if [[ -n "$IGNORE_UNTIL" && "$IGNORE_UNTIL" == "$CLI_VER" ]]; then
       echo '[deploy:Step 2.5 cli_too_new] dismissed via preference' >&2
     else
       # AskUserQuestion: 3 options — continue / explain upgrade / dismiss permanently for this version.
       case "${CLI_TOO_NEW_ANSWER:-continue}" in
         dismiss)
           "$HELPER" config set ignore_too_new_until "$CLI_VER"
           ;;
         explain)
           echo "업그레이드 안내: docs/migrate-rust.md 또는 axhub-helpers update 를 확인해요." >&2
           exit 64
           ;;
         continue|*)
           # 안전 기본값: 이번 세션만 진행하고 preferences 는 바꾸지 않아요.
           ;;
       esac
     fi
   fi
   ```

   AskUserQuestion JSON envelope (jargon-free 자연어):

   ```json
   {
     "question": "axhub CLI 가 더 최신 버전인데 계속할까요?",
     "header": "버전 확인",
     "options": [
       {
         "label": "계속해요",
         "value": "continue",
         "description": "이번 배포만 그대로 진행해요. 다음 세션에는 다시 물어요."
       },
       {
         "label": "업그레이드 안내",
         "value": "explain",
         "description": "axhub 최신 버전으로 올리는 방법을 보여줘요."
       },
       {
         "label": "이 버전부터는 묻지 마요",
         "value": "dismiss",
         "description": "지금 버전을 기억해 둬서, 같은 버전 동안에는 이 안내를 다시 띄우지 않아요."
       }
     ]
   }
   ```

   Non-interactive (`! [ -t 1 ]` / CI / `$CLAUDE_NON_INTERACTIVE`) registry default = "continue" (안전한 기본값, drift catch 는 review 책임). `AXHUB_CLI_TOO_NEW_DISMISS=0` 환경에서는 helper config_get 가 항상 None 을 반환해서 매번 prompt 가 떠요 (kill switch).

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — Step 3 preview → `--dry-run` (가장 안전해요), Step 6 exit-65 → `abort` (subprocess 자동 로그인 안 해요).

3. **Render preview card via AskUserQuestion**. The card MUST echo all five identity fields verbatim in Korean:

   ```
   다음을 실행할게요:
   ① 앱:    paydrop (id=42)
   ② 환경:  production (https://axhub-api.jocodingax.ai)
   ③ 브랜치: main
   ④ 커밋:  a3f9c1b — "결제 페이지 버그 수정" (12분 전 푸시, you)
   ⑤ 예상:  약 3분 소요

   진행할까요? [네 / 아니요 / 미리보기만 (--dry-run)]
   ```

   Use the template in `references/error-empathy-catalog.md` ("deploy-preview"). Apply NFKC normalize to displayed slug; if NFKC altered the string, surface a warning.

   Then ask with structured AskUserQuestion JSON:

   ```json
   {
     "question": "진행할까요?",
     "header": "배포 확인",
     "options": [
       {
         "label": "네, 배포",
         "value": "approve",
         "description": "동의를 받고 실제 배포를 시작해요."
       },
       {
         "label": "미리보기만",
         "value": "dry_run",
         "description": "--dry-run 으로 실제 배포 없이 확인해요."
       },
       {
         "label": "취소",
         "value": "abort",
         "description": "배포를 멈춰요."
       }
     ]
   }
   ```

   If the user chooses `dry_run`, add `--dry-run` to Step 4 and skip Step 5. If the user chooses `abort`, stop without minting consent.

3.5. **Token freshness gate (Phase 3.5 B-08).** Before minting consent, confirm that the auth token is fresh — SessionStart may have fired `auth-refresh-bg` in the background while the user reviewed the preview card. Skip when `AXHUB_AUTH_BG_REFRESH=0`.

   ```bash
   echo '[deploy:Step 3.5 token-freshness] entered' >&2
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   "$HELPER" token-gate
   ```

   `axhub-helpers token-gate` (sh/ps1-absorption Phase 1.1, T3) captures `now - 30 s` locally as the freshness anchor (matches `.plan` §3.4), polls token mtime up to 30 s (5 s × 6 iter), and calls `axhub auth status --json` inline on timeout. UNAUTHORIZED → exit 65 routes to Step 6 recovery. Pre-Phase 1.1 SKILL invocations through `bash hooks/token-freshness-gate.sh` still work — the shim delegates to the Rust binary — but new flows should call the helper directly. The Rust subcommand uses `std::fs::metadata().modified()` so the GNU vs BSD `stat` flag matrix disappears; cross-platform parity (Windows previously missing this gate entirely) comes for free. Test fixtures inject `AXHUB_TOKEN_PATH` / `AXHUB_GATE_FAKE_NOW` / `AXHUB_GATE_POLL_*` to exercise the gate without a live OAuth flow.

3.6. **토큰 freshness 폴링 중 신규 webhook 감지 (`--refresh-in-flight`).** Step 3.5 폴링 대기 중에 새 webhook 이 도착해 in_flight 상태가 바뀔 수 있어요. `AXHUB_REFRESH_IN_FLIGHT=1` 이거나 `--refresh-in-flight` 플래그가 있으면, 폴링 종료 직후 `deploy-prep` 을 재조회해요.

   ```bash
   echo '[deploy:Step 3.6 refresh-in-flight] entered' >&2
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   if [ "${AXHUB_REFRESH_IN_FLIGHT:-0}" = "1" ]; then
     REFRESH_JSON=$("$HELPER" deploy-prep --intent deploy --user-utterance "$ARGS" --refresh-in-flight --json 2>/dev/null || echo '{}')
     NEW_IN_FLIGHT=$(echo "$REFRESH_JSON" | jq -r '.in_flight_deploy.id // ""')
     if [ -n "$NEW_IN_FLIGHT" ]; then
       DEPLOY_PREP_JSON="$REFRESH_JSON"
       IN_FLIGHT_ID="$NEW_IN_FLIGHT"
       echo '[deploy:Step 3.6] in-flight detected → re-route to Step 1.6 logic' >&2
     fi
   fi
   ```

   in_flight 가 발견되면 Step 1.6 의 3-way 분기 (1.6a / 1.6b / 1.6c) logic 을 동일하게 적용해요. `IN_FLIGHT_COMMIT` vs `RESOLVE_COMMIT` 비교 후 same-commit / cross-tenant / uncertain 분기 선택 → 해당 AskUserQuestion → `monitor` (Step 5 watch) / `force_new` (Step 4 계속) / `abort` (중단). non-interactive 환경에서는 건너뛰어요 (`AXHUB_REFRESH_IN_FLIGHT` 기본값 0 → no-op).

4. **On user approval**, mint a consent token and run deploy. Run this step only when Step 1.1 did not already execute and record `deploy_create`; never double-submit a deploy for the same pending bootstrap action. 이 Step 은 **fallback create 경로**예요 — Step 1.7 status-first 가 도는 배포를 못 찾았을 때만 도달해요.

   ```bash
   echo '[deploy:Step 4 consent-deploy] entered' >&2
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   CONSENT_PROFILE=""
   PROFILE_FLAG=()
   if [ -n "${PROFILE:-}" ] && [ "${PROFILE:-}" != "default" ]; then
     CONSENT_PROFILE="$PROFILE"
     PROFILE_FLAG=(--profile "$PROFILE")
   fi
   cat <<JSON | "$HELPER" consent-mint
   {"tool_call_id":"pending","action":"deploy_create","app_id":"${APP_ID}","profile":"${CONSENT_PROFILE}","branch":"","commit_sha":"${COMMIT_SHA}","context":{}}
   JSON

   AXHUB_STDERR_TMP=$(mktemp)
   AXHUB_STDOUT_TMP=$(mktemp)
   axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --commit "$COMMIT_SHA" --execute --json >"$AXHUB_STDOUT_TMP" 2>"$AXHUB_STDERR_TMP"
   AXHUB_EXIT=$?
   # Format: "axhub-error-sub-key: 64:validation.deployment_in_progress" (main.rs:1845, quality_gate.rs:15)
   if [ $AXHUB_EXIT -eq 64 ] && grep -qE '^axhub-error-sub-key:.*64:validation\.deployment_in_progress' "$AXHUB_STDERR_TMP" 2>/dev/null; then
     # in-flight race: silent swallow raw stderr, then re-fetch in-flight id + commit + app_slug for Step 5 watch and Step 8 cache consistency.
     REFRESH_JSON=$("$HELPER" deploy-prep --intent deploy --refresh-in-flight --json 2>/dev/null || echo '{}')
     IN_FLIGHT_ID=$(echo "$REFRESH_JSON" | jq -r '.in_flight_deploy.id // ""')
     if [ -n "$IN_FLIGHT_ID" ]; then
       DEPLOY_ID="$IN_FLIGHT_ID"
       # Pull fresh commit_sha + app_slug so Step 8 statusline cache reflects the actually-running deploy (issue #81 C5).
       COMMIT_SHA=$(echo "$REFRESH_JSON" | jq -r '.in_flight_deploy.commit_sha // .resolve.commit_sha // empty')
       APP_SLUG=$(echo "$REFRESH_JSON" | jq -r '.resolve.app_slug // empty')
       echo "[deploy:Step 4 swallow-and-watch] routing to in-flight deploy $DEPLOY_ID" >&2
     else
       echo "다른 배포가 진행 중이에요. 잠시 뒤에 다시 시도해요." >&2
       rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"
       exit 0
     fi
   elif [ $AXHUB_EXIT -eq 0 ]; then
     # Happy path: extract deploy id + app slug from stdout JSON so Step 5 watch + Step 8 cache have non-empty values (issue #81 C1).
     DEPLOY_ID=$(jq -r '.id // .deployment_id // empty' "$AXHUB_STDOUT_TMP")
     APP_SLUG=$(jq -r '.app_slug // empty' "$AXHUB_STDOUT_TMP" 2>/dev/null)
     cat "$AXHUB_STDOUT_TMP"
   else
     cat "$AXHUB_STDERR_TMP" >&2
     cat "$AXHUB_STDOUT_TMP"
   fi
   rm -f "$AXHUB_STDERR_TMP" "$AXHUB_STDOUT_TMP"
   ```

   Windows PowerShell 에서는 같은 selective stderr filter 를 아래처럼 적용해요.

   ```powershell
   $AxhubStderrTmp = New-TemporaryFile
   $AxhubStdoutTmp = New-TemporaryFile
   & axhub deploy create --app $env:APP_ID @ProfileFlag --commit $env:COMMIT_SHA --execute --json 1>$AxhubStdoutTmp.FullName 2>$AxhubStderrTmp.FullName
   $AxhubExit = $LASTEXITCODE
   # Format: "axhub-error-sub-key: 64:validation.deployment_in_progress" (main.rs:1845, quality_gate.rs:15)
   if ($AxhubExit -eq 64 -and (Select-String -Path $AxhubStderrTmp.FullName -Pattern '^axhub-error-sub-key:.*64:validation\.deployment_in_progress' -Quiet)) {
     # in-flight race: silent swallow raw stderr, then re-fetch in-flight id + commit + app_slug for Step 5 watch and Step 8 cache consistency.
     $RefreshJson = & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" deploy-prep --intent deploy --refresh-in-flight --json 2>$null
     if (-not $RefreshJson) { $RefreshJson = '{}' }
     $Refresh = $RefreshJson | ConvertFrom-Json -ErrorAction SilentlyContinue
     $InFlightId = $Refresh.in_flight_deploy.id
     if ($InFlightId) {
       $DeployId = "$InFlightId"  # Use local scope, not $env:DEPLOY_ID (issue #81 C9)
       # Pull fresh commit_sha + app_slug so Step 8 statusline cache reflects the actually-running deploy (issue #81 C5).
       $CommitShaFresh = $Refresh.in_flight_deploy.commit_sha
       if (-not $CommitShaFresh) { $CommitShaFresh = $Refresh.resolve.commit_sha }
       $AppSlugFresh = $Refresh.resolve.app_slug
       [Console]::Error.WriteLine("[deploy:Step 4 swallow-and-watch] routing to in-flight deploy $InFlightId")
     } else {
       [Console]::Error.WriteLine("다른 배포가 진행 중이에요. 잠시 뒤에 다시 시도해요.")
       Remove-Item $AxhubStderrTmp.FullName -Force
       Remove-Item $AxhubStdoutTmp.FullName -Force
       exit 0
     }
   } elseif ($AxhubExit -eq 0) {
     # Happy path: extract deploy id + app slug from stdout JSON so Step 5 watch + Step 8 cache have non-empty values (issue #81 C1).
     $DeployOutput = Get-Content $AxhubStdoutTmp.FullName -Raw | ConvertFrom-Json -ErrorAction SilentlyContinue
     if ($DeployOutput) {
       $DeployId = if ($DeployOutput.id) { $DeployOutput.id } else { $DeployOutput.deployment_id }
       $AppSlugFresh = $DeployOutput.app_slug
     }
     Get-Content $AxhubStdoutTmp.FullName | Write-Output
   } else {
     [Console]::Error.WriteLine((Get-Content $AxhubStderrTmp.FullName -Raw))
     Get-Content $AxhubStdoutTmp.FullName | Write-Output
   }
   Remove-Item $AxhubStderrTmp.FullName -Force
   Remove-Item $AxhubStdoutTmp.FullName -Force
   ```

   The next Bash tool call id is created by Claude after consent-mint runs, so never invent `${NEXT_BASH_TOOL_CALL_ID}`, never set a fake `CLAUDE_SESSION_ID`, and never clear the real session env just to mint consent. `tool_call_id:"pending"` explicitly mints a short-lived pending token; the PreToolUse hook claims it once only when action/app/profile/commit/context all match. If the token is absent, already used, expired, or non-matching, the command is blocked. This avoids POSIX-only session-unset commands and keeps the flow portable across macOS/Linux/Windows Claude Code environments.

5. **Post-deploy chain** — capture `.id` from the deploy create JSON, then auto-follow until terminal:

   ```bash
   echo '[deploy:Step 5 status-chain] entered' >&2
   DEPLOY_LIST_JSON=$(axhub deploy list --app "$APP_ID" --json 2>/dev/null || echo '{"items":[]}')
   if [ "$(echo "$DEPLOY_LIST_JSON" | jq '(.items // .) | length')" -eq 0 ]; then
     echo '{"systemMessage":"배포 이력이 없어요. 먼저 /axhub:deploy 로 배포 후 다시 호출하세요."}'
     exit 0
   fi
   ```

   시작 시점에 "빌드 중이에요. 완료될 때까지 확인할게요 (보통 2~5분)." 한 줄을 먼저 보여주고, terminal status 까지 따라가요:

   ```bash
   axhub deploy status "$DEPLOY_ID" --app "$APP_ID" --watch --watch-timeout 9m --json
   ```

   **에이전트도 terminal 까지 폴링해요 (axhub-cli 0.15.3+).** bare `--watch` 는 agent context(비-TTY / `--no-input`)에서 single-snapshot 으로 degrade 하지만, `--watch-timeout` (또는 `--watch-interval`) 을 붙이면 explicit streaming override 라 CLI 가 degrade 하지 않고 terminal status(`succeeded` / `failed` / `cancelled` / `rolled_back`) 까지 직접 폴링하면서 NDJSON `stage_transition` 을 emit 해요. 그래서 SKILL 이 따로 bash polling loop 를 돌릴 필요가 없어요 — terminal 판정도 CLI 가 해요. 이 bash 는 Bash tool `timeout: 570000` (9.5분, `--watch-timeout 9m` 보다 약간 큼) 으로 호출해요. 사람 TTY 에서도 같은 명령이 스트림으로 watch 돼요.

   - **terminal 도달**: CLI 가 폴링을 끝내고 exit 해요. Step 6 exit-code 라우팅 + 성공/실패 안내로 완료를 한 번에 알려줘요. 사용자가 "아직도 진행 중이야?" 하고 다시 안 물어도 돼요.
   - **9분 timeout** (CLI 가 Timeout error + `axhub deploy watch ... resume` hint 를 줘요): 완료를 선언하지 말아요. "빌드가 아직 진행 중이에요 (9분+ 째). 계속 확인할게요." 한 줄을 보여주고 위 명령을 한 번 더 재호출해요 (총 2회 = 최대 ~19분). 2회 후에도 terminal 이 아니면 "빌드가 예상보다 길어요. `/axhub:status` 로 이어서 확인할 수 있어요." 로 안내하고 멈춰요.

   raw NDJSON / JSON dump 금지 — 진행은 NDJSON `stage_transition` 을 humanize 하고, terminal 시 단일 한국어 요약만 보여줘요.

   **watch-narration 은 interactive TTY 전용이에요.** 사람이 TTY 로 watch 할 때만 ~30s 마다 humanized Korean progress 를 렌더해요 ("1분 경과, 빌드 중이에요 (정상)", `references/recovery-flows.md` "watch-narration"). 에이전트는 terminal 도달 시 단일 완료 요약만 보여줘요.

6. **On any non-zero exit**, route via `axhub-helpers classify-exit "$EXIT" "$STDOUT"` (spec 004 Fork-A — canonical router; 두 공간 다 처리: Step 5 `deploy status --watch` 는 CLI-native 4/5/6, Step 1 `deploy-prep` 는 helper-output 65/67/68 을 내고, classify-exit 가 65→4 / 67→5 / 68→6 으로 정규화해요) 또는 `references/error-empathy-catalog.md` by exit code:
   - exit 64 + `validation.deployment_in_progress` → 4-part Korean copy: "다른 배포가 진행 중이에요. 앱은 안전해요. 5분만 기다리면 자동으로 다음 배포가 가능해요." Never retry. Offer to watch the in-flight deploy instead.
   - exit 64/67 + `github.git_connection_required`, `github.git_connection_not_found`, `git_connection_required`, or CLI stderr containing "GitHub 저장소 연결" → do not ask "지금 GitHub repo 연결 진행할까요?" and do not ask the user to invoke `/axhub:github`. Immediately show a direct GitHub connection block:

     ```bash
     echo '[deploy:Step 6 github-link] entered' >&2
     axhub apps git status --app "$APP_ID" --json
     ```

     Render the first `install_url` from that output as `GitHub 연결 링크: <install_url>` so the user can grant repo access directly. If the repo itself does not exist yet, also show `GitHub repo 만들기: https://github.com/new?name=$APP_SLUG` as context only. Then route into `skills/github/SKILL.md` guided setup/connect; do not end with a manual connect command as the next step. GitHub guided setup/connect owns repo create, remote add, first push, and connect consent.

     ```bash
     axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json
     ```

     Do not present the command above as the user's next manual command. It is the final command that the GitHub skill may run only after its guided ladder verifies repo visibility and mints consent. If the account is already installed and the desired repo appears in `axhub apps git status --app "$APP_ID" --json` (or dry-run `axhub apps git connect`), tell the user the repo is ready and route directly to `skills/github/SKILL.md` Step 4 consent-connect without another yes/no handoff.
   - exit 4 (CLI watch) / 65 (helper deploy-prep·preflight) → token expired template + AskUserQuestion to run auth login
   - exit 5 (CLI watch) / 67 (helper deploy-prep) → resource not found + did-you-mean suggestion from apps list
   - exit 6 (CLI watch) / 68 (helper) → rate limit + Retry-After backoff
   - exit 1 → transport error; retry at most once for read paths, never for create

7. **Dry-run NL trigger** — if the user said "한번 해보기만", "리허설", "테스트로", "진짜 안 올리고", add `--dry-run` to step 4 and skip step 5.

8. **Statusline live tick + cache last-deploy (Phase 17 US-1707, Phase 1 live update).** During Step 5 watch, run a 5-second polling loop that writes the statusline cache only when the deploy phase changes (vibe coder sees real-time progress instead of a stale "starting" line). After Step 5 terminal status, write the deploy summary so statusline readers can show it across sessions. The Bash block below is for POSIX/Git Bash/WSL tool execution; native Windows statusLine wiring must use the documented helper/PowerShell path only after the Windows packaging spike promotes it:

   ```bash
   # Phase 1: live tick beside the watch loop (5s polling, write-on-change).
   echo '[deploy:Step 8 statusline-live] entered' >&2
   STATUSLINE_LAST=""
   while kill -0 $WATCH_PID 2>/dev/null; do
     sleep 5
     CURRENT=$(axhub deploy status "$DEPLOY_ID" --json 2>/dev/null || true)
     if [[ -n "$CURRENT" && "$CURRENT" != "$STATUSLINE_LAST" ]]; then
       PHASE=$(echo "$CURRENT" | jq -r '.phase // .status // "?"')
       APP=$(echo "$CURRENT" | jq -r '.app_slug // "?"')
       mkdir -p ~/.cache/axhub-plugin
       echo "axhub: $APP · $PHASE" > ~/.cache/axhub-plugin/statusline.cache
       STATUSLINE_LAST="$CURRENT"
     fi
   done

   # Terminal cache write (existing behavior — preserved across sessions).
   echo '[deploy:Step 8 statusline-cache] entered' >&2
   mkdir -p ~/.cache/axhub-plugin
   cat > ~/.cache/axhub-plugin/last-deploy.json <<JSON
   {"deployment_id":"$DEPLOY_ID","status":"$TERMINAL_STATUS","commit_sha":"$COMMIT_SHA","app_slug":"$APP_SLUG","timestamp":"$(date -u +%Y-%m-%dT%H:%M:%SZ)"}
   JSON
   ```

   Skip on `--dry-run` (statusline 은 실제 deploy 만 추적).

## v0.2.0 command coverage polish

### deploy list

Read-only deployment browsing uses the current CLI command:

```bash
axhub deploy list --app "$APP_ID" --json
```

If pagination appears in JSON, show the first page and offer a follow-up instead of dumping a long list.

### deploy cancel

Cancel is a mutation. Preview the in-progress deployment first:

- app id / slug
- deployment id
- branch and commit if present
- current status
- expected effect

Mint consent with stdin JSON using `action=deploy_cancel`, top-level `app_id`, and `context={deployment_id}` and then run:

```bash
axhub deploy cancel "$DEPLOYMENT_ID" --app "$APP_ID" --execute --json
```

After cancellation, run a read-only status check and summarize the terminal state.

## NEVER

- NEVER treat `axhub-helpers bootstrap --auto-chain --json` as approval; it is only a plan/record FSM.
- NEVER retry `apps_create` or `deploy_create` unless bootstrap returns a confirmed idempotency key and retry policy that allows retry.
- NEVER skip `bootstrap --record` after a returned top-level destructive command finishes; pending action correlation is the audit trail.

- NEVER retry `axhub deploy create` on exit 64.
- NEVER drop `--json` (parsing relies on it).
- NEVER call `axhub deploy create` without going through `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint` first; the PreToolUse hook will deny.
- NEVER bypass the `preauth-check` consent hook by changing command semantics (e.g. omitting `--execute`, changing `--app`, or changing `--commit`), building a command wrapper, symlinking `axhub` to a shim, or reordering `PATH` to shadow the real CLI. A denied `deploy create` means the consent binding did not match — improvising around the gate defeats the safety primitive. Surface the typed reason in one jargon-free line and stop, or fall back to the Step 1.7 status-first watch.
- NEVER instruct the user to run `axhub deploy create`, `axhub deploy watch`, or any other deploy CLI command themselves — in their own terminal or via a `!`-prefixed prompt. Handing the raw command to the user is a consent-hook bypass equivalent to flag-stripping: it defeats the same safety primitive and skips the `--watch-timeout` polling contract. The agent runs deploy and watch itself inside this SKILL flow; if blocked, surface the typed reason in one jargon-free line and stop, or fall back to the Step 1.7 status-first watch — never delegate the raw command to the user.
- NEVER mint consent or run `deploy create` when Step 1.7 status-first already found an in-flight deploy for this app; route to watch instead. `deploy create` is the fallback only when no deploy is running.
- NEVER call `axhub deploy cancel` without a matching `deploy_cancel` consent token.
- NEVER infer `app_id` from `pwd` or git remote alone in the mutation path; always live resolve through the helper.
- NEVER bypass the AskUserQuestion preview card on slash invocation; slash is explicit consent for the SKILL invocation, not for the destructive operation.

## Additional Resources

For Korean trigger lexicon (informal, honorific, demo-context variants): `references/nl-lexicon.md`.
For exit-code → 4-part Korean error template (emotion + cause + action + button): `references/error-empathy-catalog.md`.
For multi-machine cold cache, headless/Codespaces, version skew, watch narration: `references/recovery-flows.md`.
For working transcripts, use captured `.omc/evidence/` pilot logs; no standalone example transcript files ship in this plugin.
