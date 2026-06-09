# SessionStart shim for Windows — PowerShell mirror of hooks/session-start.sh.
#
# Flow (mirrors sh version):
#   1. If ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers.exe missing AND
#      $env:AXHUB_SKIP_AUTODOWNLOAD != "1", run install.ps1 which auto-downloads
#      windows-amd64 binary from GitHub release.
#   2. If helper token file missing AND axhub CLI is authenticated AND
#      $env:AXHUB_SKIP_AUTODOWNLOAD != "1", auto-trigger
#      `axhub-helpers token-init` so vibe coders never see a separate token
#      setup step.
#   3. Exec axhub-helpers session-start with stdin pass-through.
#
# Path resolution mirrors src/axhub-helpers contracts:
#   $StateDir = XDG_STATE_HOME or %USERPROFILE%\.local\state\axhub-plugin (telemetry.ts:40-44)
#   $TokenDir = XDG_CONFIG_HOME or %USERPROFILE%\.config\axhub-plugin (index.ts:441 cmdTokenInit)

$ErrorActionPreference = 'Stop'

# Phase 25 PR 25.2 — hook safety kill switch. Canonical envs per
# .plan/matrix-absorption/00-overview.md §10.6 (Env Var Taxonomy ADR).
# Legacy DISABLE_AXHUB=1 alias honored through v0.8.0 deprecation window.
if ($env:AXHUB_DISABLE_HOOKS -eq '1' -or $env:DISABLE_AXHUB -eq '1') {
  exit 0
}
if ($env:AXHUB_DISABLE_HOOK) {
  $disabled = $env:AXHUB_DISABLE_HOOK -split ',' | ForEach-Object { $_.Trim() }
  if ($disabled -contains 'session-start') {
    exit 0
  }
}

if (-not $env:CLAUDE_PLUGIN_ROOT) {
  Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] CLAUDE_PLUGIN_ROOT 미설정 — Claude Code 가 hook 호출 시 자동 설정해야 하지만 누락됐어요. /plugin install axhub@axhub 로 재설치 시도." } -Compress)
  exit 0
}

$Root = $env:CLAUDE_PLUGIN_ROOT
$Helper = Join-Path $Root 'bin\axhub-helpers.exe'
$InstallPs1 = Join-Path $Root 'bin\install.ps1'

function Write-InstallFailureOrFallback {
  param(
    [object[]]$InstallOutput,
    [string]$FallbackMessage
  )
  $installJson = $InstallOutput | ForEach-Object { "$_" } | Where-Object { $_.TrimStart().StartsWith('{') } | Select-Object -Last 1
  if ($installJson) {
    Write-Output $installJson
  } else {
    Write-Output (ConvertTo-Json @{ systemMessage = $FallbackMessage } -Compress)
  }
}

# Step 1: ensure helper binary exists
if (-not (Test-Path -Path $Helper -PathType Leaf)) {
  if ($env:AXHUB_SKIP_AUTODOWNLOAD -eq '1') {
    Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] AXHUB_SKIP_AUTODOWNLOAD=1 이라 helper 자동 설치를 건너뛰었어요. 수동 설치 후 다시 시작해요: powershell -NoProfile -ExecutionPolicy Bypass -File bin\install.ps1" } -Compress)
    exit 0
  }
  if (-not (Test-Path -Path $InstallPs1 -PathType Leaf)) {
    Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] install.ps1 없음 — 플러그인 install 손상. 재설치: /plugin install axhub@axhub" } -Compress)
    exit 0
  }

  try {
    $installOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File $InstallPs1 2>&1
    $installExit = $LASTEXITCODE
    if ($installExit -ne 0) {
      $failureMessage = "[axhub] helper 바이너리 설치 실패 (exit $installExit). 설치 상태를 먼저 확인해 주세요."
      Write-InstallFailureOrFallback -InstallOutput $installOutput -FallbackMessage $failureMessage
      exit 0
    }
    if (-not (Test-Path -Path $Helper -PathType Leaf)) {
      $failureMessage = "[axhub] install.ps1 실행 후에도 axhub-helpers.exe 를 찾지 못했어요. 설치 상태를 먼저 확인해 주세요."
      Write-InstallFailureOrFallback -InstallOutput $installOutput -FallbackMessage $failureMessage
      exit 0
    }
  } catch [System.IO.PathTooLongException] {
    # Pre-mortem #5 — MAX_PATH on Korean profile
    Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] Windows MAX_PATH (260자) 초과로 install.ps1 실행 실패. 해결: 관리자 PowerShell 에서 LongPathsEnabled 활성화 또는 'subst N: `$env:CLAUDE_PLUGIN_ROOT' 사용." } -Compress)
    exit 0
  } catch {
    # Catch-all — includes AMSI/EDR pre-mortem #2
    $errMsg = $_.Exception.Message
    if ($errMsg -match '(AntiMalwareProvider|AMSI|quarantine|virus|threat)') {
      Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] 보안 솔루션 (V3, AhnLab, CrowdStrike 등) 이 install.ps1 호출을 차단했어요. AXHUB_TOKEN 환경변수로 우회 가능 → `$env:AXHUB_TOKEN='axhub_pat_...' 후 token-init 재시도. v0.1.8 Authenticode 서명 후 EDR allowlist 가능." } -Compress)
    } else {
      Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] install.ps1 실행 중 알 수 없는 에러: $errMsg. 설치 상태를 먼저 확인해 주세요." } -Compress)
    }
    exit 0
  }
}

# spec 006 — marker gate for eager infra (mirror of session-start.sh). The
# helper computes the axhub.yaml git-root walk-up marker + a cheap token-file
# auth stat. Non-axhub projects (no marker) skip eager infra (token-init here;
# quality-context is gated helper-side). Helper download above is NOT gated.
#
# `session-eager-gate` exits 0 = run, 1 = skip. Any other exit (spawn error)
# falls open auth-conditionally: run iff a token-file exists (authed → preserve
# existing axhub.yaml users; unauthed → stay zero-footprint).
$EagerInfra = 'skip'
$GateTokenDir = if ($env:XDG_CONFIG_HOME) {
  Join-Path $env:XDG_CONFIG_HOME 'axhub-plugin'
} else {
  Join-Path $env:USERPROFILE '.config\axhub-plugin'
}
$GateTokenFile = Join-Path $GateTokenDir 'token'
$GateRc = 99
$GateOldEap = $ErrorActionPreference
try {
  # exit 1 = "skip" is a DECISION value, not an error. PS 7.4+ defaults
  # $PSNativeCommandUseErrorActionPreference to on, so under the file's top-level
  # 'Stop' a native non-zero exit would throw — misrouting the main (non-marker)
  # skip path into the catch fallback. Force 'Continue' so $LASTEXITCODE is read.
  $ErrorActionPreference = 'Continue'
  & $Helper session-eager-gate 2>$null | Out-Null
  $GateRc = $LASTEXITCODE
} catch {
  $GateRc = 99
} finally {
  $ErrorActionPreference = $GateOldEap
}
if ($GateRc -eq 0) {
  $EagerInfra = 'run'            # marker present (or unknown+authed) → run
} elseif ($GateRc -eq 1) {
  $EagerInfra = 'skip'           # marker absent → zero-footprint
} elseif (Test-Path -Path $GateTokenFile -PathType Leaf) {
  $EagerInfra = 'run'           # spawn error + authed → run (fail-open)
}                                # else: spawn error + unauthed → skip (default)

# Step 2: auto-trigger token-init when helper token file is missing
# but axhub CLI has a valid login. Silent skip on any failure.
# spec 006: gated on $EagerInfra so non-axhub (no-marker) projects skip token-init.
if ($EagerInfra -eq 'run' -and $env:AXHUB_SKIP_AUTODOWNLOAD -ne '1') {
  try {
    $TokenFile = (& $Helper path token-file 2>$null | Select-Object -First 1)
  } catch {
    $TokenFile = $null
  }
  if ((-not $TokenFile) -or $TokenFile.StartsWith('{')) {
    # Fallback mirrors the Rust path contract when an older helper lacks `path`.
    $TokenDir = if ($env:XDG_CONFIG_HOME) {
      Join-Path $env:XDG_CONFIG_HOME 'axhub-plugin'
    } else {
      Join-Path $env:USERPROFILE '.config\axhub-plugin'
    }
    $TokenFile = Join-Path $TokenDir 'token'
  }

  if (-not (Test-Path -Path $TokenFile -PathType Leaf)) {
    $axhubCmd = Get-Command axhub -ErrorAction SilentlyContinue
    if ($axhubCmd) {
      try {
        $authStatus = & axhub auth status --json 2>$null
        if ($authStatus -match '"user_email"') {
          # Run token-init silently — never block session-start on this
          & $Helper token-init 2>&1 | Out-Null
        }
      } catch {
        # Silent skip per session-start.sh:42 `|| true` semantics
      }
    }
  }
}

# Step 2.5: detached auth-refresh-bg trigger (mirror of session-start.sh:84-89).
# When `axhub` CLI reports UNAUTHORIZED, spawn `axhub-helpers auth-refresh-bg`
# in the background so token refresh runs in parallel with the user's deploy
# preview prompt. Helper writes the result sentinel; SKILL Step 3.5 polls
# token mtime + reads sentinel before deploy_create.
# AXHUB_AUTH_BG_REFRESH=0 disables. axhub CLI absent -> skip.
# Phase 3.2: this closes the Windows parity gap that previously only existed in
# the POSIX sh wrapper.
#
# Detach flag: `-NoNewWindow` keeps the dispatcher consistent with
# `session-start-autowire.ps1` Step 5 — both wrappers fire-and-forget a
# console-silent helper subcommand. `-WindowStyle Hidden` would create a
# spurious new (hidden) window per fire, which is louder for users with
# strict EDR / window-tracking policies. Reviewer Issue 5 (PR #114).
if ($env:AXHUB_AUTH_BG_REFRESH -ne '0') {
  $axhubCmd = Get-Command axhub -ErrorAction SilentlyContinue
  if ($axhubCmd) {
    try {
      $authStatus = & axhub auth status --json 2>$null
      if ($authStatus -notmatch '"user_email"') {
        Start-Process -FilePath $Helper -ArgumentList 'auth-refresh-bg' `
          -NoNewWindow -ErrorAction SilentlyContinue | Out-Null
      }
    } catch {
      # Silent — fire-and-forget, never block session-start on this.
    }
  }
}

# Step 2.6/2.7: plugin + CLI version-drift cache warming moved INTO the Rust
# helper's `session-start` subcommand (warm_drift_caches). It spawns the same
# detached `…-fetch-bg` children from one cross-platform place, so session-start
# adds zero latency and a short cache TTL makes a restart re-check almost
# immediately (turn-1 nudge still fires). The old detached Start-Process fetch
# spawns were removed here.

# Step 3: optional telemetry breadcrumb (only when AXHUB_TELEMETRY=1)
# State dir mirrors telemetry.ts:40-44 (XDG_STATE_HOME envvar with HOME-relative fallback)
if ($env:AXHUB_TELEMETRY -eq '1') {
  try {
    $StateDir = if ($env:XDG_STATE_HOME) {
      Join-Path $env:XDG_STATE_HOME 'axhub-plugin'
    } else {
      Join-Path $env:USERPROFILE '.local\state\axhub-plugin'
    }
    if (-not (Test-Path -Path $StateDir -PathType Container)) {
      New-Item -Path $StateDir -ItemType Directory -Force | Out-Null
    }
    $UsageFile = Join-Path $StateDir 'usage.jsonl'
    $entry = @{
      ts = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ssZ')
      event = 'windows.hook.session_start_ps1_invoked'
      ps_version = $PSVersionTable.PSVersion.ToString()
    } | ConvertTo-Json -Compress
    Add-Content -Path $UsageFile -Value $entry -Encoding UTF8
  } catch {
    # Silent — telemetry is best-effort
  }
}

# Step 4: exec helper session-start (final stage)
try {
  & $Helper session-start
  exit $LASTEXITCODE
} catch {
  Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] axhub-helpers session-start 실행 실패: $($_.Exception.Message). 설치 상태를 먼저 확인해 주세요." } -Compress)
  exit 0
}
