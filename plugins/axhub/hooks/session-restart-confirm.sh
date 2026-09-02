#!/usr/bin/env bash
# SessionStart entry 4: 플러그인 업데이트 재시작 확인 훅이에요 (AP-26). auto-update
# worker 가 남긴 marker(~/.axhub/cache/.plugin-update-restart, 7일 TTL)를 보고, 이
# 세션이 실제로 로드한 플러그인 버전(${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json)
# 이 marker 버전 이상이면 적용 확인 한 줄을 emit 하고 marker 를 지워요. 낮으면
# 재시작 안내 한 줄만 emit 하고 marker 는 둬요. 에이전트 명령은 0개예요 — 플러그인
# 목록 명령을 실행하지 않아요. dev 가드는 두지 않아요 — marker 는 머신 전역이라
# 어느 세션의 확인도 유효해요. 사용자에게 묻지 않아요. plain bash 의미를 유지해요 (set -u 없음).

[ -n "$AXHUB_NO_AUTO_UPDATE" ] && exit 0
[ -f "$HOME/.axhub/config/no-auto-update" ] && exit 0
M="$HOME/.axhub/cache/.plugin-update-restart"
[ -f "$M" ] || exit 0
[ -n "$(find "$M" -mmin -10080 2>/dev/null)" ] || exit 0
EXPECTED=$(head -n 1 "$M" 2>/dev/null | cut -d'|' -f1 | tr -d '[:space:]')
[ -n "$EXPECTED" ] || exit 0
LOADED=$(sed -n 's/.*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "${CLAUDE_PLUGIN_ROOT//\\//}/.claude-plugin/plugin.json" 2>/dev/null | head -n 1)
[ -n "$LOADED" ] || exit 0

# semver 3자리 숫자 비교 — macOS sort 에 -V 가 없어요. 앞의 v 와 pre-release 꼬리는 떼요.
ver_ge() {
  local a="${1#v}" b="${2#v}" a1 a2 a3 b1 b2 b3
  IFS=. read -r a1 a2 a3 <<EOT
$a
EOT
  IFS=. read -r b1 b2 b3 <<EOT
$b
EOT
  a1=${a1%%[!0-9]*}; a2=${a2%%[!0-9]*}; a3=${a3%%[!0-9]*}
  b1=${b1%%[!0-9]*}; b2=${b2%%[!0-9]*}; b3=${b3%%[!0-9]*}
  a1=${a1:-0}; a2=${a2:-0}; a3=${a3:-0}; b1=${b1:-0}; b2=${b2:-0}; b3=${b3:-0}
  [ "$a1" -ne "$b1" ] && { [ "$a1" -gt "$b1" ]; return; }
  [ "$a2" -ne "$b2" ] && { [ "$a2" -gt "$b2" ]; return; }
  [ "$a3" -ge "$b3" ]
}

if ver_ge "$LOADED" "$EXPECTED"; then
  rm -f "$M" 2>/dev/null
  printf '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"[axhub] 플러그인 v%s 적용을 확인했어요. 다음 응답 앞머리에 이 한 줄만 알리고 확인 명령은 실행하지 마세요."}}\n' "$LOADED"
else
  printf '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"[axhub] 플러그인 새 버전(v%s)을 받았어요. 다음 응답 앞머리에 Claude Code 를 재시작하면 적용된다고 한 줄만 알리고 확인 명령은 실행하지 마세요."}}\n' "$EXPECTED"
fi
exit 0
