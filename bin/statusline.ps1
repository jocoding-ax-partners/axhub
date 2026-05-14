# Phase 17 US-1707 — axhub plugin statusline mirror for Windows — PowerShell native.
# Mirrors statusline.sh contract: same envs, same paths, same output, same 해요체.
#
# Flow:
#   1. If axhub-helpers.exe exists, delegate to `statusline` subcommand (fast path <20ms).
#   2. Otherwise, check AXHUB_TOKEN env or token file under XDG_CONFIG_HOME /
#      USERPROFILE\.config\axhub-plugin\token (auth presence check, no network).
#   3. On auth miss -> print `axhub: 로그인 안 됐어요` and exit 0.
#   4. Parse last-deploy.json cache (ConvertFrom-Json, no jq dep).
#      Extract commit_sha (head 8), status, app_slug.
#   5. Full hit  -> axhub: <app> . <profile> . 최근 배포 <SHA8> (<status>)
#      Empty     -> axhub: <profile> . 배포 기록 없어요
#
# Path resolution mirrors src/axhub-helpers contracts:
#   $TokenDir = XDG_CONFIG_HOME or %USERPROFILE%\.config\axhub-plugin (index.ts:441 cmdTokenInit)
#   $CacheDir = XDG_CACHE_HOME  or %USERPROFILE%\.cache\axhub-plugin  (telemetry.ts:40-44)
#
# Wiring (user adds to ~/.claude/settings.json or ~/.claude/settings.local.json):
#   {
#     "statusLine": {
#       "type": "command",
#       "command": "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1\""
#     }
#   }
#
# Latency budget: <50ms cold (no network, file reads only).

$ErrorActionPreference = 'Stop'

try {
  $Root = if ($env:CLAUDE_PLUGIN_ROOT) { $env:CLAUDE_PLUGIN_ROOT } else {
    Split-Path -Parent (Split-Path -Parent $PSCommandPath)
  }
  $Helper = Join-Path $Root 'bin\axhub-helpers.exe'

  # --- Fast path: delegate to Rust helper ---
  if (Test-Path -Path $Helper -PathType Leaf) {
    $helperOut = & $Helper statusline 2>$null
    if ($LASTEXITCODE -eq 0 -and $helperOut) {
      Write-Output $helperOut
      exit 0
    }
  }

  # --- Inline fallback ---

  # Token file path: XDG_CONFIG_HOME or USERPROFILE fallback
  $ConfigBase = if ($env:XDG_CONFIG_HOME) { $env:XDG_CONFIG_HOME } else {
    Join-Path $env:USERPROFILE '.config'
  }
  $TokenFile = Join-Path $ConfigBase 'axhub-plugin\token'

  # Auth presence check — env var or token file. No network.
  $AuthOk = ($env:AXHUB_TOKEN -and $env:AXHUB_TOKEN.Length -gt 0) -or (Test-Path -Path $TokenFile -PathType Leaf)

  if (-not $AuthOk) {
    Write-Output 'axhub: 로그인 안 됐어요'
    exit 0
  }

  # Cache path: XDG_CACHE_HOME or USERPROFILE fallback
  $CacheBase = if ($env:XDG_CACHE_HOME) { $env:XDG_CACHE_HOME } else {
    Join-Path $env:USERPROFILE '.cache'
  }
  $CacheFile = Join-Path $CacheBase 'axhub-plugin\last-deploy.json'

  $Profile_ = if ($env:AXHUB_PROFILE -and $env:AXHUB_PROFILE.Length -gt 0) { $env:AXHUB_PROFILE } else { 'default' }

  $Sha = $null
  $Status_ = $null
  $App = $null

  if (Test-Path -Path $CacheFile -PathType Leaf) {
    $raw = Get-Content -Raw $CacheFile
    $cache = $raw | ConvertFrom-Json
    if ($cache.commit_sha) {
      $Sha = $cache.commit_sha.Substring(0, [Math]::Min(8, $cache.commit_sha.Length))
    }
    if ($cache.status) { $Status_ = $cache.status }
    if ($cache.app_slug) { $App = $cache.app_slug }
  }

  if ($Sha -and $Status_) {
    if (-not $App) { $App = '?' }
    Write-Output ("axhub: {0} · {1} · 최근 배포 {2} ({3})" -f $App, $Profile_, $Sha, $Status_)
  } else {
    Write-Output ("axhub: {0} · 배포 기록 없어요" -f $Profile_)
  }
  exit 0
} catch {
  Write-Output 'axhub: '
  exit 0
}
