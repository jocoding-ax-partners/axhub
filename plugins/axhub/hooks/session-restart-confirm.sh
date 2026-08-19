#!/usr/bin/env bash
# SessionStart entry 4: 플러그인 업데이트 재시작 확인 훅이에요. hooks.json 인라인
# command 를 동작 불변으로 옮긴 wrapper 예요 — kill switch(env·marker) 통과 후
# marker(7일 TTL)가 있으면 confirm prompt 지시를 suppressed JSON 으로 발행해요.
# 읽기 전용이라 marker 정리는 prompt 몫이에요.

[ -n "$AXHUB_NO_AUTO_UPDATE" ] && exit 0; [ -f "$HOME/.axhub/config/no-auto-update" ] && exit 0; M="$HOME/.axhub/cache/.plugin-update-restart"; [ -f "$M" ] || exit 0; [ -n "$(find "$M" -mmin -10080 2>/dev/null)" ] || exit 0; printf '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"[axhub] 플러그인 업데이트 재시작 확인이 남았어요(7일 내 marker). %s/hooks/plugin-restart-confirm-prompt.md 를 읽고 그 지침대로 적용 버전을 확인하세요. best-effort·비차단 — 실패하면 조용히 건너뛰고 사용자의 작업을 절대 막지 마세요."}}\n' "${CLAUDE_PLUGIN_ROOT//\\//}"
