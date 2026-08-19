#!/usr/bin/env bash
# SessionStart entry 1: auto-update 훅이에요. hooks.json 인라인 command 를
# 동작 불변으로 옮긴 wrapper 라서 set 옵션 없이 plain bash -c 의미를 그대로 둬요.
# 순서: kill switch(env·marker) → dev 가드 → CLI 존재 → 24h throttle(훅이 직접
# touch) → suppressed JSON context 발행이에요.

[ -n "$AXHUB_NO_AUTO_UPDATE" ] && exit 0; [ -f "$HOME/.axhub/config/no-auto-update" ] && exit 0; { [ -e "${CLAUDE_PLUGIN_ROOT}/../../.git" ] || [ -e "${CLAUDE_PLUGIN_ROOT}/.git" ]; } && exit 0; command -v axhub >/dev/null 2>&1 || exit 0; CACHE="$HOME/.axhub/cache/.plugin-update-check-codex"; [ -f "$CACHE" ] && [ -n "$(find "$CACHE" -mmin -1440 2>/dev/null)" ] && exit 0; mkdir -p "$HOME/.axhub/cache" 2>/dev/null && : > "$CACHE" 2>/dev/null || true; printf '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"[axhub] axhub CLI/plugin 업데이트를 확인할 시점이에요(24h throttle). %s/hooks/auto-update-prompt.md 를 읽고 그 지침대로 버전을 확인·적용하세요. best-effort·비차단 — 실패하면 조용히 건너뛰고 사용자의 작업을 절대 막지 마세요."}}\n' "${CLAUDE_PLUGIN_ROOT//\\//}"
