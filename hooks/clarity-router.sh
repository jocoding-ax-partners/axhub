#!/usr/bin/env bash
set -euo pipefail

if [ -n "${AXHUB_NO_CLARITY_ROUTER:-}" ]; then
  exit 0
fi
[ -f "$HOME/.axhub/config/no-clarity-router" ] && exit 0

input="$(cat)"

# "prompt": 키 이후 구간만 매칭 — cwd·transcript_path 경로 오탐(F1) 차단.
# 값 내 따옴표는 \" 이스케이프라 원문 "prompt": 는 키뿐이에요. 키 부재 =
# fail-closed (스킬 frontmatter 라우팅은 훅 없이도 동작). 스키마 가정·극단
# 케이스 상세는 update-router.sh 헤더 주석 참고.
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
    exit 0
    ;;
esac

case "$prompt_part" in
  *GitHub*|*github*|*깃허브*) ;;
  *) exit 0 ;;
esac

case "$prompt_part" in
  *재연결*|*다시\ 연결*|*연결*|*계정*|*인증*|*device*|*Device*|*code*|*Code*|*코드*|*link*|*reconnect*)
    cat <<'JSON'
{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"[axhub clarity router] The current user prompt is an exact axhub GitHub reconnect/device-code operation. Apply the axhub clarity GitHub device-flow contract inline before generic Code-mode shell handling, Finding tools, App/MCP tools, update, import, bootstrap, deploy, or status overview. Do not invoke the /axhub:clarity slash command or show a failing skill badge. Do not run generic discovery such as axhub --help | grep, shell pipes, redirects, grep, head, sed, awk, bash -lc, sh -c, temp files, Read file steps, or Claude Desktop axhub App/MCP tools. Follow the clarity Device Flow fast path: the shell tool title AND description must both be exactly 계정 인증 시작 for the device-flow command; never use axhub GitHub device flow 인증 시작, GitHub 인증 시작, or any descriptive sentence. The visible command is one direct CLI call AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link (add --tenant <tenant> only if already known). The assistant body must show two standalone lines before any confirmation command: 인증 URL: `https://github.com/login/device` and 입력 코드: <USER_CODE>. Use the inline-code URL form to prevent Claude Desktop auto-linking; never render the URL as [https://github.com/login/device](github.com/login/device), [https://github.com/login/device](https://github.com/login/device), <https://github.com/login/device>, or a clickable Markdown link. Do not hide the code behind prose. Do not end the turn after showing the code or after any pending/waiting CLI text; the next Desktop-visible command in the same assistant turn must be one direct confirmation command whose shell tool title AND description are both exactly 인증 확인 and whose command is exactly axhub github accounts list --json (add --tenant <tenant> only if already known). Never add sleep, &&, pipes, redirects, grep, jq, bash -lc, sh -c, temp files, or watcher logic to the confirmation command. Do not ask the user to say 승인했어 or 배포 상태 확인해줘."}}
JSON
    ;;
esac
