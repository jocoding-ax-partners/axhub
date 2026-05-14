#!/usr/bin/env bash
# Phase 26 v0.6.0 — SessionStart autowire dispatcher (POSIX).
#
# Implements Option B-revised-v2 dual-channel disclosure + silent merge.
# Flow:
#   0. Disclosure marker check — first fire shows trust disclosure, exits 0
#      without merging (next session performs the actual merge).
#   1. hook_safety kill switches (AXHUB_DISABLE_HOOKS / AXHUB_DISABLE_HOOK)
#   2. Per-feature opt-out (AXHUB_DISABLE_STATUSLINE_AUTOWIRE)
#   3. Scope detection via CLAUDE_PLUGIN_ROOT prefix
#   4. Mtime race guard — 60s window prevents S5 subprocess duplicate writes
#   4.5. Orphan-stub install + verify via axhub-helpers orphan-stub --install
#   5. Background detach: nohup axhub-helpers autowire-statusline &
#   6. exit 0  (fail-open contract — always)
#
# NEVER use set -e without a trap; every path must exit 0.
# Kill switches honored: AXHUB_DISABLE_HOOKS / AXHUB_DISABLE_HOOK=session-start-autowire
#                        / AXHUB_DISABLE_STATUSLINE_AUTOWIRE

# ── state dir helper (XDG_STATE_HOME-aware) ───────────────────────────────────
_axhub_state_dir() {
  if [ -n "${XDG_STATE_HOME:-}" ]; then
    printf '%s/axhub-plugin' "$XDG_STATE_HOME"
  else
    printf '%s/.local/state/axhub-plugin' "$HOME"
  fi
}

STATE_DIR="$(_axhub_state_dir)"
DISCLOSURE_MARKER="${STATE_DIR}/install-disclosure-shown.txt"

# ── step 0: disclosure marker (dual-channel) ──────────────────────────────────
# Marketplace install path skips install.sh — this first-SessionStart branch
# guarantees disclosure regardless of install method. Marker is written once
# (idempotent). Next session proceeds to merge silently.
if [ ! -f "$DISCLOSURE_MARKER" ]; then
  mkdir -p "$STATE_DIR" 2>/dev/null || true
  printf 'shown-by=session-start-autowire\n' > "$DISCLOSURE_MARKER" 2>/dev/null || true
  printf '%s\n' '{"systemMessage":"[axhub] 자동 설정 알림\n\naxhub 이 다음을 수행해요:\n  (1) 인증 토큰을 keychain / file 에 저장해요.\n  (2) opt-in telemetry 가 활성화되어 있어요 (AXHUB_TELEMETRY=0 로 disable).\n  (3) macOS Gatekeeper quarantine attribute 를 제거해요.\n  (4) auth-refresh 백그라운드 task 가 token 갱신해요.\n  (5) helper binary 를 GitHub release 에서 HTTPS 로 다운로드해요.\n  (6) ~/.claude/settings.json 의 statusLine 필드를 추가·관리해요 (other plugins preserved).\n\n거부하려면: AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1 후 재시작. 상세: README.md#trust-uninstall"}'
  exit 0
fi

# ── step 1: hook_safety kill switches ────────────────────────────────────────
# Canonical envs per Env Var Taxonomy ADR §10.6.
# Legacy DISABLE_AXHUB=1 honored through v0.8.0 deprecation window.
if [ "${AXHUB_DISABLE_HOOKS:-0}" = "1" ] || [ "${DISABLE_AXHUB:-0}" = "1" ]; then
  exit 0
fi
case ",${AXHUB_DISABLE_HOOK:-}," in
  *,session-start-autowire,*) exit 0 ;;
esac

# ── step 2: per-feature opt-out ──────────────────────────────────────────────
if [ "${AXHUB_DISABLE_STATUSLINE_AUTOWIRE:-0}" = "1" ]; then
  exit 0
fi

# ── step 3: scope detection ───────────────────────────────────────────────────
# Concrete algorithm per plan §B step 3 (resolved Open Q #4).
# user scope:    $CLAUDE_PLUGIN_ROOT starts with $HOME/.claude/plugins/
# project scope: $CLAUDE_PLUGIN_ROOT starts with $(git rev-parse --show-toplevel)/.claude/plugins/
# else:          ambiguous — fail-closed (skip merge entirely)
ROOT="${CLAUDE_PLUGIN_ROOT:-}"
if [ -z "$ROOT" ]; then
  exit 0
fi

SCOPE=""
case "$ROOT" in
  "${HOME}/.claude/plugins/"*)
    SCOPE="user"
    ;;
  *)
    REPO="$(git -C "${PWD:-/}" rev-parse --show-toplevel 2>/dev/null || true)"
    if [ -n "$REPO" ]; then
      case "$ROOT" in
        "${REPO}/.claude/plugins/"*)
          SCOPE="project"
          ;;
      esac
    fi
    ;;
esac

if [ -z "$SCOPE" ]; then
  exit 0
fi

# ── step 4: mtime race guard (60s window — S5 subprocess protection) ──────────
# Only the dispatcher (initial SessionStart) writes the done-marker.
# A child process (claude -p) that finds a marker written within 60s skips
# entirely — it must NOT write its own marker to prevent stale-mtime cascade.
DONE_MARKER="${STATE_DIR}/auto-wire-done-${SCOPE}.json"
if [ -f "$DONE_MARKER" ]; then
  if find "$DONE_MARKER" -mmin -1 2>/dev/null | grep -q .; then
    exit 0
  fi
fi

# ── step 4.5: ensure helper binary available ──────────────────────────────────
HELPER="${ROOT}/bin/axhub-helpers"
if [ ! -x "$HELPER" ]; then
  exit 0
fi

# ── step 4.5: orphan-stub install + verify ────────────────────────────────────
# axhub-helpers orphan-stub --install prints the verified stub path to stdout.
# Verify path exists and is executable; skip merge on any failure so we never
# write a broken path into settings.json.
STUB_PATH="$("$HELPER" orphan-stub --install 2>/dev/null || true)"
if [ -z "$STUB_PATH" ] || [ ! -x "$STUB_PATH" ]; then
  exit 0
fi

# ── step 5: background detach ─────────────────────────────────────────────────
# nohup + disown so Claude Code session is not blocked by merge I/O.
# Dispatcher (this initial SessionStart shell) does NOT pass --child — the
# binary writes the scope done-marker after merge completes.
# Child processes (claude -p) that reach this point will have already been
# short-circuited at step 4 (mtime race guard) before reaching here.
nohup "$HELPER" autowire-statusline \
  --silent \
  "--scope=${SCOPE}" \
  "--command-path=${STUB_PATH}" \
  >/dev/null 2>&1 &
disown >/dev/null 2>&1 || true

# ── step 6: fail-open ─────────────────────────────────────────────────────────
exit 0
