# Phase 26 v0.6.0 — SessionStart autowire dispatcher (Windows PowerShell).
# Mirror of hooks/session-start-autowire.sh.
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
#   5. Background dispatch: Start-Process axhub-helpers autowire-statusline
#   6. exit 0  (fail-open contract — always)
#
# Kill switches: AXHUB_DISABLE_HOOKS / AXHUB_DISABLE_HOOK=session-start-autowire
#                / AXHUB_DISABLE_STATUSLINE_AUTOWIRE
# Path contracts mirror telemetry.ts:40-44 + index.ts XDG resolution.

$ErrorActionPreference = 'Stop'

# Helper: state dir (XDG_STATE_HOME-aware, mirrors telemetry.ts:40-44)
function Get-AxhubStateDir {
  if ($env:XDG_STATE_HOME) {
    return Join-Path $env:XDG_STATE_HOME 'axhub-plugin'
  }
  return Join-Path $env:USERPROFILE '.local\state\axhub-plugin'
}

try {
  $StateDir        = Get-AxhubStateDir
  $DisclosureMarker = Join-Path $StateDir 'install-disclosure-shown.txt'

  # ── step 0: disclosure marker (dual-channel) ────────────────────────────────
  # Marketplace install path skips install.ps1 — first-SessionStart branch
  # guarantees disclosure regardless of install method.
  if (-not (Test-Path -Path $DisclosureMarker -PathType Leaf)) {
    if (-not (Test-Path -Path $StateDir -PathType Container)) {
      New-Item -Path $StateDir -ItemType Directory -Force | Out-Null
    }
    Set-Content -Path $DisclosureMarker -Value 'shown-by=session-start-autowire' -Encoding UTF8 -ErrorAction SilentlyContinue
    $disclosureText = @"
[axhub] 자동 설정 알림

axhub 이 다음을 수행해요:
  (1) 인증 토큰을 keychain / file 에 저장해요.
  (2) opt-in telemetry 가 활성화되어 있어요 (AXHUB_TELEMETRY=0 로 disable).
  (3) macOS Gatekeeper quarantine attribute 를 제거해요.
  (4) auth-refresh 백그라운드 task 가 token 갱신해요.
  (5) helper binary 를 GitHub release 에서 HTTPS 로 다운로드해요.
  (6) ~/.claude/settings.json 의 statusLine 필드를 추가·관리해요 (other plugins preserved).

거부하려면: AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1 후 재시작. 상세: README.md#trust-uninstall
"@
    Write-Output (ConvertTo-Json @{ systemMessage = $disclosureText } -Compress)
    exit 0
  }

  # ── step 1: hook_safety kill switches ──────────────────────────────────────
  # Canonical envs per Env Var Taxonomy ADR §10.6.
  # Legacy DISABLE_AXHUB=1 honored through v0.8.0 deprecation window.
  if ($env:AXHUB_DISABLE_HOOKS -eq '1' -or $env:DISABLE_AXHUB -eq '1') {
    exit 0
  }
  if ($env:AXHUB_DISABLE_HOOK) {
    $disabled = $env:AXHUB_DISABLE_HOOK -split ',' | ForEach-Object { $_.Trim() }
    if ($disabled -contains 'session-start-autowire') {
      exit 0
    }
  }

  # ── step 2: per-feature opt-out ────────────────────────────────────────────
  if ($env:AXHUB_DISABLE_STATUSLINE_AUTOWIRE -eq '1') {
    exit 0
  }

  # ── step 3: scope detection ────────────────────────────────────────────────
  # Concrete algorithm per plan §B step 3 (resolved Open Q #4).
  if (-not $env:CLAUDE_PLUGIN_ROOT) {
    exit 0
  }
  $Root            = $env:CLAUDE_PLUGIN_ROOT
  $UserPluginsPrefix = Join-Path $env:USERPROFILE '.claude\plugins\'
  $Scope           = $null

  if ($Root.StartsWith($UserPluginsPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    $Scope = 'user'
  } else {
    try {
      $Repo = (& git -C $PWD rev-parse --show-toplevel 2>$null | Select-Object -First 1)
      if ($Repo) {
        $ProjectPrefix = Join-Path $Repo '.claude\plugins\'
        if ($Root.StartsWith($ProjectPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
          $Scope = 'project'
        }
      }
    } catch {
      # git absent or not a repo — ambiguous scope, $Scope remains $null
    }
  }

  if (-not $Scope) {
    exit 0  # Ambiguous scope — fail-closed per plan §B step 3
  }

  # ── step 4: mtime race guard (60s window — S5 subprocess protection) ────────
  # Only the dispatcher (initial SessionStart) writes the done-marker.
  # Child process (claude -p) finding a marker within 60s skips and does NOT
  # write its own marker (prevents stale-mtime cascade).
  $DoneMarker = Join-Path $StateDir "auto-wire-done-$Scope.json"
  if (Test-Path -Path $DoneMarker -PathType Leaf) {
    $mtime = (Get-Item $DoneMarker).LastWriteTimeUtc
    $ageSec = ([datetime]::UtcNow - $mtime).TotalSeconds
    if ($ageSec -lt 60) {
      exit 0
    }
  }

  # ── step 4.5: ensure helper binary available ──────────────────────────────
  $Helper = Join-Path $Root 'bin\axhub-helpers.exe'
  if (-not (Test-Path -Path $Helper -PathType Leaf)) {
    exit 0
  }

  # ── step 4.5: orphan-stub install + verify ─────────────────────────────────
  # axhub-helpers orphan-stub --install prints the verified stub path to stdout.
  # Skip merge if stub absent or unexecutable — never write a broken path.
  $StubPath = (& $Helper orphan-stub --install 2>$null | Select-Object -First 1)
  if (-not $StubPath) {
    exit 0
  }
  if (-not (Test-Path -Path $StubPath)) {
    exit 0
  }

  # ── step 5: background dispatch ────────────────────────────────────────────
  # Start-Process detaches so Claude Code session is not blocked.
  # Helper subprocess writes the done-marker after completing the merge.
  # Dispatcher (initial SessionStart) does NOT pass --child — binary writes
  # the scope done-marker internally after merge completes.
  $procArgs = @(
    'autowire-statusline'
    '--silent'
    '--scope'
    $Scope
    '--command-path'
    $StubPath
  )
  Start-Process -FilePath $Helper -ArgumentList $procArgs -NoNewWindow -PassThru | Out-Null

} catch {
  # Fail-open: never let any error block the Claude Code session.
  # All paths below unconditionally exit 0.
}

# ── step 6: fail-open ─────────────────────────────────────────────────────────
exit 0
