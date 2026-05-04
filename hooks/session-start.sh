#!/usr/bin/env bash
# Phase 5 US-502 + Phase 7 US-701: SessionStart shim.
#
# Flow:
#   1. If ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers missing AND
#      AXHUB_SKIP_AUTODOWNLOAD != "1", run install.sh which auto-downloads
#      the arch-specific binary from the v0.1.x GitHub release.
#   2. Phase 7 US-701: If helper token file missing AND axhub CLI is
#      authenticated AND AXHUB_SKIP_AUTODOWNLOAD != "1", auto-trigger
#      `axhub-helpers token-init` so vibe coders never see a separate token
#      setup step (axhub auth login is the single user-visible login).
#   3. Exec axhub-helpers session-start with stdin pass-through.
#
# This shim is committed (NOT gitignored). install.sh handles all the OS/arch
# detection + download + symlink logic so this stays a thin POSIX wrapper.
set -eu

ROOT="${CLAUDE_PLUGIN_ROOT:?CLAUDE_PLUGIN_ROOT must be set by Claude Code}"
HELPER="${ROOT}/bin/axhub-helpers"
INSTALL_SH="${ROOT}/bin/install.sh"

if [ ! -x "$HELPER" ] && [ ! -L "$HELPER" ]; then
  if [ -x "$INSTALL_SH" ]; then
    "$INSTALL_SH" >&2 || {
      echo '{"systemMessage":"[axhub] helper 바이너리 설치 실패. 진단: /axhub:doctor"}'
      exit 0
    }
  else
    echo '{"systemMessage":"[axhub] install.sh 없음 — 플러그인 install 손상. 재설치: /plugin install axhub@axhub"}'
    exit 0
  fi
fi

# Phase 7 US-701: auto-trigger token-init when helper token file is missing
# but axhub CLI has a valid login. Silent skip on any failure — never block
# session-start. Honors AXHUB_SKIP_AUTODOWNLOAD as the single opt-out switch.
if [ "${AXHUB_SKIP_AUTODOWNLOAD:-0}" != "1" ]; then
  TOKEN_FILE="$("$HELPER" path token-file 2>/dev/null || true)"
  case "$TOKEN_FILE" in
    ""|\{*) TOKEN_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/axhub-plugin/token" ;;
  esac
  if [ ! -f "$TOKEN_FILE" ] && command -v axhub >/dev/null 2>&1; then
    if axhub auth status --json 2>/dev/null | grep -q '"user_email"'; then
      "$HELPER" token-init >&2 2>&1 || true
    fi
  fi
fi

exec "$HELPER" session-start
