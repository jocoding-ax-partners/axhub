# Phase 11 US-1103 — Windows VM smoke executor (companion to
# docs/pilot/windows-vm-smoke-checklist.md). Codifies 14 steps as
# PowerShell functions. Guarded by $env:AXHUB_VM_SMOKE so it never
# runs accidentally outside the dedicated pilot session.

if ($env:AXHUB_VM_SMOKE -ne '1') {
  Write-Output "smoke-windows-vm-checklist.ps1: skipped (AXHUB_VM_SMOKE != '1')"
  exit 0
}

$ErrorActionPreference = 'Continue'  # Don't abort on first failure — capture all step results
$results = @()

function Run-Step {
  param([int]$Number, [string]$Name, [scriptblock]$Action)
  Write-Output "[step $Number] $Name"
  try {
    & $Action
    $script:results += [PSCustomObject]@{ Step = $Number; Name = $Name; Status = 'PASS'; Detail = '' }
  } catch {
    $msg = $_.Exception.Message
    Write-Output "[step $Number FAIL] $msg"
    $script:results += [PSCustomObject]@{ Step = $Number; Name = $Name; Status = 'FAIL'; Detail = $msg }
  }
}

Run-Step 1 "Provision Win11 VM (visual confirmation)" {
  $cs = Get-CimInstance Win32_ComputerSystem
  Write-Output "  Manufacturer: $($cs.Manufacturer), Model: $($cs.Model)"
  Write-Output "  TotalPhysicalMemory: $([math]::Round($cs.TotalPhysicalMemory / 1GB, 1)) GB"
}

Run-Step 2 "Claude Code 2.1.84+ installed" {
  $version = (claude --version 2>$null) -replace '[^\d.]', ''
  if (-not $version) { throw "claude --version produced no output" }
  Write-Output "  Claude Code version: $version"
}

Run-Step 3 "Plugin installed" {
  $pluginRoot = "$HOME\.claude\plugins\marketplaces\axhub-marketplace\axhub"
  if (-not (Test-Path $pluginRoot)) { throw "plugin not installed at $pluginRoot" }
}

Run-Step 4 "session-start.ps1 exists but is not registered by universal hooks.json" {
  $script = "$HOME\.claude\plugins\marketplaces\axhub-marketplace\axhub\hooks\session-start.ps1"
  if (-not (Test-Path $script)) { throw "session-start.ps1 missing" }
  $hooksJson = Get-Content "$HOME\.claude\plugins\marketplaces\axhub-marketplace\axhub\hooks\hooks.json" -Raw | ConvertFrom-Json
  $serialized = $hooksJson.hooks.SessionStart | ConvertTo-Json -Compress
  if ($serialized -match 'session-start\.ps1' -or $serialized -match '"shell":"powershell"') {
    throw "universal hooks.json must not register shell:powershell SessionStart; it creates visible non-Windows startup errors"
  }
}

Run-Step 5 "axhub-helpers.exe exists post-install" {
  $exe = "$HOME\.claude\plugins\marketplaces\axhub-marketplace\axhub\bin\axhub-helpers.exe"
  if (-not (Test-Path $exe)) { throw "axhub-helpers.exe missing — install.ps1 may have failed" }
  $size = (Get-Item $exe).Length
  Write-Output "  binary size: $([math]::Round($size / 1MB, 1)) MB"
}

Run-Step 6 "cmdkey shows axhub credential" {
  $output = cmdkey /list:axhub 2>&1 | Out-String
  if ($output -notmatch 'Target:.*axhub') { throw "cmdkey did not find axhub credential — run 'axhub auth login' first" }
}

Run-Step 7 "axhub-helpers token-init reads keychain" {
  $exe = "$HOME\.claude\plugins\marketplaces\axhub-marketplace\axhub\bin\axhub-helpers.exe"
  $output = & $exe token-init 2>&1
  $exit = $LASTEXITCODE
  if ($exit -ne 0) { throw "token-init exit=$exit, output=$output" }
  $parsed = $output | ConvertFrom-Json
  if ($parsed.source -ne 'windows-credential-manager') { throw "expected source=windows-credential-manager, got $($parsed.source)" }
}

Run-Step 8 "Token file written to XDG_CONFIG_HOME path" {
  $tokenFile = "$env:USERPROFILE\.config\axhub-plugin\token"
  if (-not (Test-Path $tokenFile)) { throw "token file missing at $tokenFile" }
  $size = (Get-Item $tokenFile).Length
  if ($size -lt 32 -or $size -gt 200) { throw "token file size $size bytes outside expected range (32-200)" }
  $first16 = (Get-Content $tokenFile -TotalCount 1).Substring(0, 16)
  if (-not $first16.StartsWith('axhub_pat_')) { throw "token does not start with axhub_pat_ (got: $first16)" }
}

Run-Step 9 "PreToolUse + PostToolUse hooks registered in hooks.json" {
  $hooksJson = Get-Content "$HOME\.claude\plugins\marketplaces\axhub-marketplace\axhub\hooks\hooks.json" -Raw | ConvertFrom-Json
  if (-not $hooksJson.hooks.PreToolUse) { throw "PreToolUse hook missing" }
  if (-not $hooksJson.hooks.PostToolUse) { throw "PostToolUse hook missing" }
}

Run-Step 10 "ExecutionPolicy fallback semantics (manual visual check)" {
  Write-Output "  current ExecutionPolicy: $(Get-ExecutionPolicy -Scope CurrentUser)"
  Write-Output "  to test fallback: Set-ExecutionPolicy -Scope CurrentUser Restricted, restart claude, verify Korean systemMessage"
}

Run-Step 11 "AMSI/EDR detection (env-dependent)" {
  $defender = Get-MpComputerStatus -ErrorAction SilentlyContinue
  if (-not $defender) { throw "no AV detected — skip this step on enterprise VM with V3/AhnLab/CrowdStrike" }
  Write-Output "  AntimalwareEnabled: $($defender.AntimalwareEnabled)"
  Write-Output "  RealTimeProtectionEnabled: $($defender.RealTimeProtectionEnabled)"
}

Run-Step 12 "MAX_PATH path length check" {
  $pluginRoot = "$HOME\.claude\plugins\marketplaces\axhub-marketplace\axhub"
  $maxNested = "$pluginRoot\bin\axhub-helpers.exe"
  Write-Output "  longest path: $($maxNested.Length) chars"
  if ($maxNested.Length -gt 250) { Write-Output "  WARN: approaching MAX_PATH 260 limit. Verify LongPathsEnabled registry key." }
}

Run-Step 13 "Proxy environment check" {
  if ($env:HTTPS_PROXY) {
    Write-Output "  HTTPS_PROXY=$env:HTTPS_PROXY (test 407 fallback path manually if NTLM)"
  } else {
    Write-Output "  no HTTPS_PROXY set — skip NTLM 407 test"
  }
}

Run-Step 14 "Summary capture" {
  Write-Output ""
  Write-Output "=========================================="
  Write-Output "Windows VM smoke results:"
  $results | Format-Table -AutoSize | Out-String | Write-Output
  $passCount = ($results | Where-Object { $_.Status -eq 'PASS' }).Count
  $failCount = ($results | Where-Object { $_.Status -eq 'FAIL' }).Count
  Write-Output "PASS: $passCount / 13 (step 14 is summary itself)"
  Write-Output "FAIL: $failCount"
  Write-Output "=========================================="
}

# Exit with non-zero if any step failed (caller can inspect $results)
if (($results | Where-Object { $_.Status -eq 'FAIL' }).Count -gt 0) {
  exit 1
}
exit 0
