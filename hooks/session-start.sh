#!/usr/bin/env bash
# Phase 5 US-502: SessionStart shim — bootstraps axhub-helpers binary on first
# CC session if marketplace install left bin/ empty (binaries are gitignored).
#
# Flow:
#   1. If ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers missing AND
#      AXHUB_SKIP_AUTODOWNLOAD != "1", run install.sh which auto-downloads
#      the arch-specific binary from the v0.1.0 GitHub release.
#   2. Exec axhub-helpers session-start with stdin pass-through.
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

exec "$HELPER" session-start
