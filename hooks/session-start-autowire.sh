#!/usr/bin/env bash
# Phase 26 v0.6.0 — SessionStart autowire dispatcher (POSIX).
#
# Thin wrapper after sh/ps1-absorption Phase 2.2 (T6). The body
# (disclosure marker check, scope detection, mtime race guard, orphan-stub
# install, settings.json merge) lives in `crates/axhub-helpers/src/autowire.rs`
# and is invoked via `axhub-helpers autowire-statusline --scope auto`.
#
# This wrapper retains three responsibilities the Rust helper cannot own:
#   1. Kill switch fast-path — avoids helper cold-start (Gatekeeper / EDR scan)
#      when the user has opted out.
#   2. Helper-absent silent exit — install.sh path handles binary downloads;
#      autowire MUST NOT block SessionStart while the binary is missing.
#   3. Background detach — `nohup ... & disown` ensures the SessionStart hook
#      returns inside its 10s timeout regardless of the merge duration.
#
# Fail-open contract per docs/HOOKS.md: every path exits 0.

# ── kill switch ────────────────────────────────────────────────────────────────
if [ "${AXHUB_DISABLE_HOOKS:-0}" = "1" ] || [ "${DISABLE_AXHUB:-0}" = "1" ]; then
  exit 0
fi
case ",${AXHUB_DISABLE_HOOK:-}," in
  *,session-start-autowire,*) exit 0 ;;
esac
if [ "${AXHUB_DISABLE_STATUSLINE_AUTOWIRE:-0}" = "1" ]; then
  exit 0
fi

# ── helper presence check ─────────────────────────────────────────────────────
ROOT="${CLAUDE_PLUGIN_ROOT:-}"
if [ -z "$ROOT" ]; then
  exit 0
fi
HELPER="${ROOT}/bin/axhub-helpers"
if [ ! -x "$HELPER" ]; then
  exit 0
fi

# ── background detach ─────────────────────────────────────────────────────────
# nohup + disown so Claude Code session is not blocked by merge I/O.
# Helper handles scope detection (--scope auto), disclosure marker, mtime
# race guard, orphan-stub install, and settings.json merge internally.
nohup "$HELPER" autowire-statusline --scope auto --silent >/dev/null 2>&1 &
disown >/dev/null 2>&1 || true

exit 0
