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

if (-not $env:CLAUDE_PLUGIN_ROOT) {
  Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] CLAUDE_PLUGIN_ROOT 미설정 — Claude Code 가 hook 호출 시 자동 설정해야 하지만 누락됐어요. /plugin install axhub@axhub 로 재설치 시도." } -Compress)
  exit 0
}

$Root = $env:CLAUDE_PLUGIN_ROOT
$Helper = Join-Path $Root 'bin\axhub-helpers.exe'
$InstallPs1 = Join-Path $Root 'bin\install.ps1'

# Step 1: ensure helper binary exists
if (-not (Test-Path -Path $Helper -PathType Leaf)) {
  if (-not (Test-Path -Path $InstallPs1 -PathType Leaf)) {
    Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] install.ps1 없음 — 플러그인 install 손상. 재설치: /plugin install axhub@axhub" } -Compress)
    exit 0
  }

  try {
    & powershell -NoProfile -ExecutionPolicy Bypass -File $InstallPs1
    $installExit = $LASTEXITCODE
    if ($installExit -ne 0) {
      Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] helper 바이너리 설치 실패 (exit $installExit). 진단: /axhub:doctor" } -Compress)
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
      Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] install.ps1 실행 중 알 수 없는 에러: $errMsg. 진단: /axhub:doctor" } -Compress)
    }
    exit 0
  }
}

# Step 2: auto-trigger token-init when helper token file is missing
# but axhub CLI has a valid login. Silent skip on any failure.
if ($env:AXHUB_SKIP_AUTODOWNLOAD -ne '1') {
  # Token dir mirrors index.ts:441 cmdTokenInit (XDG_CONFIG_HOME)
  $TokenDir = if ($env:XDG_CONFIG_HOME) {
    Join-Path $env:XDG_CONFIG_HOME 'axhub-plugin'
  } else {
    Join-Path $env:USERPROFILE '.config\axhub-plugin'
  }
  $TokenFile = Join-Path $TokenDir 'token'

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
  Write-Output (ConvertTo-Json @{ systemMessage = "[axhub] axhub-helpers session-start 실행 실패: $($_.Exception.Message). 진단: /axhub:doctor" } -Compress)
  exit 0
}
