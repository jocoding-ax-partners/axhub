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

# Phase 25 PR 25.2 — hook safety kill switch. Canonical envs per
# .plan/matrix-absorption/00-overview.md §10.6 (Env Var Taxonomy ADR).
# Legacy DISABLE_AXHUB=1 alias honored through v0.8.0 deprecation window.
if [ "${AXHUB_DISABLE_HOOKS:-0}" = "1" ] || [ "${DISABLE_AXHUB:-0}" = "1" ]; then
  exit 0
fi
case ",${AXHUB_DISABLE_HOOK:-}," in
  *,session-start,*) exit 0 ;;
esac

ROOT="${CLAUDE_PLUGIN_ROOT:?CLAUDE_PLUGIN_ROOT must be set by Claude Code}"
HELPER="${ROOT}/bin/axhub-helpers"
INSTALL_SH="${ROOT}/bin/install.sh"
WRAPPER="${ROOT}/hooks/axhub-helpers.sh"

if [ ! -x "$HELPER" ] && [ -x "$WRAPPER" ]; then
  if RESOLVED_HELPER="$("$WRAPPER" --resolve-helper 2>/dev/null)" && [ -x "$RESOLVED_HELPER" ]; then
    HELPER="$WRAPPER"
  fi
fi

if [ ! -x "$HELPER" ]; then
  if [ "${AXHUB_SKIP_AUTODOWNLOAD:-0}" = "1" ]; then
    echo '{"systemMessage":"[axhub] AXHUB_SKIP_AUTODOWNLOAD=1 이라 helper 자동 설치를 건너뛰었어요. 수동 설치 후 다시 시작해요: bash bin/install.sh"}'
    exit 0
  fi
  if [ -x "$INSTALL_SH" ]; then
    "$INSTALL_SH" >&2 || {
      echo '{"systemMessage":"[axhub] helper 바이너리 설치 실패. 설치 상태를 먼저 확인해 주세요."}'
      exit 0
    }
  else
    echo '{"systemMessage":"[axhub] install.sh 없음 — 플러그인 install 손상. 재설치: /plugin install axhub@axhub"}'
    exit 0
  fi
fi

# spec 006 — marker gate for session-start eager infra (token-init / Gatekeeper
# warmup / quality-context). The helper computes the axhub.yaml git-root walk-up
# marker + a cheap token-file auth stat. Non-axhub projects (no marker) get a
# zero-footprint session. The helper download above is NOT gated — it is the
# prerequisite for this very call (spec §범위).
#
# `session-eager-gate` exits 0 = run, 1 = skip. Any other exit (spawn error /
# timeout) falls open auth-conditionally: run iff a token-file exists (authed →
# preserve existing axhub.yaml users; unauthed → stay zero-footprint).
EAGER_INFRA="skip"
GATE_TOKEN_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/axhub-plugin/token"
if [ "$(uname -s)" = "Darwin" ] && ! command -v timeout >/dev/null 2>&1; then
  # macOS without `timeout`: never spawn the gate unbounded (wedged-Gatekeeper
  # hang risk — mirrors the warmup hard-skip below). Use the auth fallback.
  if [ -f "$GATE_TOKEN_FILE" ]; then EAGER_INFRA="run"; fi
else
  GATE_RC=0
  if [ "$(uname -s)" = "Darwin" ]; then
    timeout 3 "$HELPER" session-eager-gate >/dev/null 2>&1 || GATE_RC=$?
  else
    "$HELPER" session-eager-gate >/dev/null 2>&1 || GATE_RC=$?
  fi
  if [ "$GATE_RC" = "0" ]; then
    EAGER_INFRA="run"            # marker present (or unknown+authed) → run
  elif [ "$GATE_RC" = "1" ]; then
    EAGER_INFRA="skip"           # marker absent → zero-footprint
  elif [ -f "$GATE_TOKEN_FILE" ]; then
    EAGER_INFRA="run"            # spawn error/timeout + authed → run (fail-open)
  fi                             # else: spawn error + unauthed → skip (default)
fi

# Phase 7 US-701: auto-trigger token-init when helper token file is missing
# but axhub CLI has a valid login. Silent skip on any failure — never block
# session-start. Honors AXHUB_SKIP_AUTODOWNLOAD as the single opt-out switch.
# spec 006: gated on EAGER_INFRA so non-axhub (no-marker) projects skip token-init.
if [ "$EAGER_INFRA" = "run" ] && [ "${AXHUB_SKIP_AUTODOWNLOAD:-0}" != "1" ]; then
  TOKEN_FILE="$("$HELPER" path token-file 2>/dev/null || true)"
  case "$TOKEN_FILE" in
    ""|\{*) TOKEN_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/axhub-plugin/token" ;;
  esac
  if [ ! -f "$TOKEN_FILE" ] && command -v axhub >/dev/null 2>&1; then
    if axhub auth status --json 2>/dev/null | grep -q '"user_email"'; then
      "$HELPER" token-init >/dev/null 2>&1 || true
    fi
  fi
fi

# Phase 2 B-07: macOS Gatekeeper / notarization cache warmup. The first
# helper spawn after a fresh boot or signature recheck pays a 3-6s
# codesign/notarization cost; running `--version --quiet` here primes the
# OS cache so the deploy hot path skips it. Best-effort + 3s timeout +
# AXHUB_GATEKEEPER_WARMUP=0 opt-out so a wedged Gatekeeper never blocks
# session-start.
if [ "$EAGER_INFRA" = "run" ] && [ "${AXHUB_GATEKEEPER_WARMUP:-1}" != "0" ] && [ "$(uname -s)" = "Darwin" ]; then
  # Hard-skip when `timeout` is unavailable: an unbounded warmup against a
  # wedged Gatekeeper would block session-start indefinitely. Losing the
  # 3-6s warmup on stripped macOS hosts is the right tradeoff vs hang risk.
  if command -v timeout >/dev/null 2>&1; then
    timeout 3 "$HELPER" --version --quiet >/dev/null 2>&1 || true
  fi
fi

# Phase 3.5 B-08: detached auth refresh trigger. When `axhub` CLI reports
# UNAUTHORIZED in this session, fire-and-forget the helper's auth-refresh-bg
# subcommand so token refresh runs in parallel with the user's deploy
# preview prompt. Helper writes the result sentinel; SKILL Step 3.5 polls
# token mtime + reads sentinel before deploy_create.
# AXHUB_AUTH_BG_REFRESH=0 disables. axhub CLI absent → skip.
if [ "${AXHUB_AUTH_BG_REFRESH:-1}" != "0" ] && [ -z "${CI:-}" ] && [ -z "${CLAUDE_NON_INTERACTIVE:-}" ] && command -v axhub >/dev/null 2>&1; then
  if ! axhub auth status --json 2>/dev/null | grep -q '"user_email"'; then
    nohup "$HELPER" auth-refresh-bg >/dev/null 2>&1 &
    disown >/dev/null 2>&1 || true
  fi
fi

# Plugin version-drift fetch (proactive update nudge): warm the latest-release
# TTL cache in the background so the first prompt-route turn can compare versions
# without a network call on the hot path. Detached + best-effort; never blocks
# session-start. The helper is TTL-gated (24h) and honors AXHUB_DISABLE_HOOK=
# plugin-drift internally, so the spawn is cheap even when opted out.
if [ -z "${CI:-}" ] && [ -z "${CLAUDE_NON_INTERACTIVE:-}" ]; then
  nohup "$HELPER" plugin-latest-fetch-bg >/dev/null 2>&1 &
  disown >/dev/null 2>&1 || true
fi

# CLI binary version-drift fetch (separate channel). Mirrors the plugin fetch but
# shells `axhub update check --json`, so — unlike the ureq-based plugin fetch — it
# MUST be guarded by `command -v axhub` (same as auth-refresh-bg). Merging the two
# fetches into one unguarded fork would hit exit-127 for pre-onboarding users who
# have not installed the CLI yet. TTL-gated (24h) + honors AXHUB_DISABLE_HOOK=cli-drift.
if [ -z "${CI:-}" ] && [ -z "${CLAUDE_NON_INTERACTIVE:-}" ] && command -v axhub >/dev/null 2>&1; then
  nohup "$HELPER" cli-latest-fetch-bg >/dev/null 2>&1 &
  disown >/dev/null 2>&1 || true
fi

# Phase 26: warn once when superpowers using-superpowers may conflict with axhub megaskill.
if [ "${AXHUB_DISABLE_MEGASKILL:-0}" != "1" ] && [ -d "$HOME/.codex/superpowers/skills/using-superpowers" ]; then
  MARKER="${XDG_STATE_HOME:-$HOME/.local/state}/axhub-plugin/megaskill-superpowers-warning"
  if [ ! -f "$MARKER" ]; then
    mkdir -p "$(dirname "$MARKER")" 2>/dev/null || true
    printf '%s
' '[axhub] using-superpowers skill detected. axhub quality auto-mode stays best-effort; set AXHUB_DISABLE_MEGASKILL=1 to disable.' >&2
    date -u +%FT%TZ > "$MARKER" 2>/dev/null || true
  fi
fi

exec "$HELPER" session-start
