#!/usr/bin/env bash
set -euo pipefail

# ┌─ update-router 게이트 파이프라인 (통과 순서 자체가 계약이에요) ─────────┐
# │ stdin(JSON) ─▶ [1] kill switch (env AXHUB_NO_UPDATE_ROUTER            │
# │                    또는 marker ~/.axhub/config/no-update-router)→ exit 0│
# │             ─▶ [2] "prompt": 키 추출 — 부재 시 fail-closed   → exit 0  │
# │             ─▶ [3] axhub 토큰 (axhub|Axhub|AxHub|AXHUB)      → exit 0  │
# │             ─▶ [4] freshness 키워드 → additionalContext 발행           │
# └─────────────────────────────────────────────────────────────────────────┘
# [2] 근거: UserPromptSubmit stdin 은 session_id → transcript_path → cwd →
# permission_mode → hook_event_name → prompt 를 담아요 (공식 문서 예시 순서,
# code.claude.com/docs/en/hooks). cwd·transcript_path 가 prompt 앞에 오므로
# "prompt": 이후 구간만 매칭해 경로 유래 오탐(F1)을 차단해요. JSON 문자열
# 값 내부의 따옴표는 반드시 \" 로 이스케이프되므로 원문 "prompt": 바이트열은
# 키 위치에서만 나타나요 ("prompt_id": 는 콜론 위치가 달라 비충돌). 직렬화
# 순서는 계약이 아니라서 prompt 뒤에 미래 필드가 붙거나 prompt 앞 필드가
# 중첩 "prompt": 키를 갖는 극단 케이스는 advisory fail-open 수용 범위예요.
# 키 부재 = fail-closed — 이 훅은 보강재라 꺼져도 스킬 frontmatter 라우팅은
# 계속 동작하고, 가드의 최악은 "가드 없음" 상태로 수렴해야 해요.

if [ -n "${AXHUB_NO_UPDATE_ROUTER:-}" ]; then
  exit 0
fi
[ -f "$HOME/.axhub/config/no-update-router" ] && exit 0

input="$(cat)"

case "$input" in
  *\"prompt\":*) ;;
  *) exit 0 ;;
esac
prompt_part=${input#*\"prompt\":}

case "$prompt_part" in
  *axhub*|*Axhub*|*AxHub*|*AXHUB*) ;;
  *) exit 0 ;;
esac

case "$prompt_part" in
  *최신*|*버전*|*업데이트*|*latest*|*"up to date"*|*"version check"*|*update*|*upgrade*)
    cat <<'JSON'
{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"[axhub update router] The current user prompt asks about axhub freshness/update. Before Finding tools, any axhub App/MCP tool, app list/status tool, shell version probe, clarity, bootstrap, deploy, /oh-my-claudecode:autopilot, unrelated plugin mode, or generic Code-mode handling, invoke the axhub update skill. The first visible assistant body must be exactly: 현재 버전을 확인할게요. Finish update first, then continue any remaining app status/create/deploy/GitHub reconnect-device-code request in the same conversation. Once you say 업데이트 확인은 끝났어요, do not run command-v, axhub --version, claude plugin list, plugin-location checks, or version probes again for the remaining app-status flow. For GitHub reconnect/device-code after update, apply the axhub clarity device-flow contract inline; do not invoke /axhub:clarity or show a failing skill badge; never run axhub git_connection_status, axhub github status, axhub --help | grep, shell pipes, or generic status probes; use exactly AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link then axhub github accounts list --json; when the earlier device code was lost or expired, use AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --fresh instead, never while a valid code is still on screen, with tenant option only when already known, shell tool titles and descriptions exactly 계정 인증 시작 and 인증 확인. The assistant body must print exactly two normal chat lines with the URL in inline code: 인증 URL: `https://github.com/login/device` and 입력 코드: <USER_CODE>; never print the URL as a bare auto-link or Markdown link such as [https://github.com/login/device](github.com/login/device). Do not finish after showing the code, after pending text, or after telling the user to approve; continue to 인증 확인 in the same assistant turn, and never wait for the user to say 승인했어 or 배포 상태 확인해줘. For plugin version lookup, the visible command is exactly claude plugin list; never claude plugin list 2>&1 or any redirected variant. For app status overview after update, allowed visible commands are exactly axhub apps --help then axhub apps list --json. If folder/current conversation/latest list clearly identifies a related app, do not ask which app; continue with CLI-only axhub apps get <app> --json and axhub deploy list --app <app> --json before any summary. Never call Claude Desktop axhub App/MCP tools such as Deployment list (axhub), App get (axhub), or Tenant recent deployments (axhub). Never run nonexistent singular axhub app list or axhub deployment list. Never run command -v axhub && axhub --version, command -v claude && claude plugin list, or claude plugin list 2>&1 | grep. Never add sleep, &&, pipes, redirects, head, tail, grep, sed, awk, bash -lc, sh -c, or 2>/dev/null."}}
JSON
    ;;
esac
