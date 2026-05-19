# Phase 26 v0.6.0 — SessionStart autowire dispatcher (Windows PowerShell).
# Mirror of hooks/session-start-autowire.sh.
#
# Thin wrapper after sh/ps1-absorption Phase 2.2 (T6). The body
# (disclosure marker check, scope detection, mtime race guard, orphan-stub
# install, settings.json merge) lives in `crates/axhub-helpers/src/autowire.rs`
# and is invoked via `axhub-helpers autowire-statusline --scope auto`.
#
# Responsibilities retained in this wrapper:
#   1. Kill switch fast-path — avoids helper cold-start.
#   2. Helper-absent silent exit — install.ps1 owns binary downloads.
#   3. Background detach — Start-Process keeps SessionStart hook non-blocking.
#
# Fail-open contract per docs/HOOKS.md: every path exits 0.

$ErrorActionPreference = 'SilentlyContinue'

try {
  # ── kill switch ──────────────────────────────────────────────────────────────
  if ($env:AXHUB_DISABLE_HOOKS -eq '1' -or $env:DISABLE_AXHUB -eq '1') {
    exit 0
  }
  if ($env:AXHUB_DISABLE_HOOK) {
    $disabled = $env:AXHUB_DISABLE_HOOK -split ',' | ForEach-Object { $_.Trim() }
    if ($disabled -contains 'session-start-autowire') {
      exit 0
    }
  }
  if ($env:AXHUB_DISABLE_STATUSLINE_AUTOWIRE -eq '1') {
    exit 0
  }

  # ── helper presence check ────────────────────────────────────────────────────
  if (-not $env:CLAUDE_PLUGIN_ROOT) {
    exit 0
  }
  $Helper = Join-Path $env:CLAUDE_PLUGIN_ROOT 'bin\axhub-helpers.exe'
  if (-not (Test-Path -Path $Helper -PathType Leaf)) {
    exit 0
  }

  # ── background detach ────────────────────────────────────────────────────────
  # Start-Process detaches so Claude Code session is not blocked.
  # Helper handles scope detection (--scope auto), disclosure marker, mtime
  # race guard, orphan-stub install, and settings.json merge internally.
  Start-Process -FilePath $Helper `
    -ArgumentList @('autowire-statusline', '--scope', 'auto', '--silent') `
    -NoNewWindow -PassThru | Out-Null

} catch {
  # Fail-open: never let any error block the Claude Code session.
}

exit 0
