#!/usr/bin/env bash
# Phase 3.5 token-freshness gate — thin shim that delegates to the Rust helper.
#
# Behavior preserved verbatim (mtime polling, AXHUB_GATE_* env contract,
# exit 65 on UNAUTHORIZED). The body has been absorbed into
# `crates/axhub-helpers/src/main.rs::cmd_token_gate` (sh/ps1-absorption Phase 1.1).
#
# This shim exists for two reasons:
#   1. `skills/deploy/SKILL.md` Step 3.5 still calls `bash hooks/token-freshness-gate.sh`
#      to preserve the user-facing fixture surface. Removed in Phase 4 (T8).
#   2. Windows parity — invoking `axhub-helpers token-gate` directly works on
#      every OS once the helper exists; this shim only runs on POSIX where the
#      SKILL still pipes through `bash`.
#
# Env contract is unchanged. See `axhub-helpers token-gate --help` or the
# original Rust handler doc comment for details.
set -u

HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"
if [ ! -x "$HELPER" ]; then
  # Helper absent — fail-open per docs/HOOKS.md contract.
  exit 0
fi
exec "$HELPER" token-gate "$@"
