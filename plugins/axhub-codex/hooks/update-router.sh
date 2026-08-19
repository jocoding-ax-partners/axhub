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
{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"[axhub update router] 현재 사용자 프롬프트가 axhub 의 최신성/업데이트를 물어요.\nupdate 스킬을 다른 무엇보다 먼저 실행해요.\n첫 visible 응답 텍스트는 정확히 \"현재 버전을 확인할게요.\" 예요.\n도구 탐색, 앱 목록/상태 도구, 셸 버전 probe, clarity, bootstrap, deploy, 일반 세션 처리보다 먼저 axhub update 스킬을 실행해요. 플러그인 버전 확인 명령은 정확히 codex plugin list 예요 — 파이프·리다이렉트 변형을 붙이지 않아요. update 를 먼저 끝낸 뒤 같은 대화에서 남은 앱 상태/생성/배포/GitHub 재연결 요청을 이어가요. \"업데이트 확인은 끝났어요\" 라고 말한 뒤에는 command -v, axhub --version, codex plugin list, 버전 probe 를 다시 실행하지 않아요. 앱 상태 개요는 axhub apps --help 로 표면을 확인한 뒤 읽기 전용 axhub apps list --json 으로 시작하고, 폴더명·대화 맥락·최근 수정 앱으로 관련 앱이 특정되면 어느 앱인지 되묻지 않고 axhub apps get <app> --json 과 axhub deploy list --app <app> --json 까지 이어가요. 존재하지 않는 단수 axhub app list 나 axhub deployment list 를 추측하지 않아요. GitHub 재연결/device-code 는 AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link 실행 후 axhub github accounts list --json 으로 확인하고, 인증 URL 과 입력 코드를 일반 텍스트 두 줄로 보여준 뒤 같은 턴에서 인증 확인까지 이어가요 — 사용자가 승인했다고 말할 때까지 기다리지 않아요. 셸 명령에 sleep, &&, 파이프, 리다이렉트, head, tail, grep, sed, awk 후처리를 붙이지 않아요."}}
JSON
    ;;
esac
