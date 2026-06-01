# axhub-helpers Windows installer — picks windows-amd64 binary, downloads from
# GitHub releases, places at bin/axhub-helpers.exe.
#
# Mirrors bin/install.sh logic for Windows hosts without Git Bash/WSL.
# Run automatically by hooks/session-start.ps1, or manually after release.
#
# NOTE: bin/install.sh uses an explicit if block for link cleanup; this PS mirror
# keeps the same Test-Path / Remove-Item shape for parity.
#
# Maintainer: when bumping plugin version, update $RELEASE_VERSION below to
# match install.sh:48. Override via env $env:AXHUB_PLUGIN_RELEASE.

$ErrorActionPreference = 'Stop'

# --- install-time disclosure (idempotent, marker-gated) ---
# Maintainer: keep $AxhubDisclosureVer in sync with $ReleaseVersion below.
$AxhubDisclosureVer = 'v0.9.23'
$AxhubStateDir = if ($env:XDG_STATE_HOME) {
  Join-Path $env:XDG_STATE_HOME 'axhub-plugin'
} else {
  Join-Path $env:USERPROFILE '.local\state\axhub-plugin'
}
$AxhubDisclosureMarker = Join-Path $AxhubStateDir 'install-disclosure-shown.txt'
$_axhubShowDisclosure = $true
if (Test-Path -Path $AxhubDisclosureMarker -PathType Leaf) {
  $markerContent = Get-Content -Path $AxhubDisclosureMarker -ErrorAction SilentlyContinue
  if ($markerContent -contains $AxhubDisclosureVer) { $_axhubShowDisclosure = $false }
}
# CI / scripted contexts suppress disclosure (AXHUB_SKIP_AUTODOWNLOAD=1 indicates
# automated test path; AXHUB_NO_DISCLOSURE=1 explicit override for scripts piping
# install.ps1 stdout to JSON parser).
if ($env:AXHUB_SKIP_AUTODOWNLOAD -eq '1' -or $env:AXHUB_NO_DISCLOSURE -eq '1') {
  $_axhubShowDisclosure = $false
}
if ($_axhubShowDisclosure) {
  Write-Host '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
  Write-Host 'axhub 이 다음을 수행해요:'
  Write-Host '  (1) 인증 토큰을 keychain (macOS/Windows) / file (Linux) 에 저장해요.'
  Write-Host '  (2) opt-in telemetry 가 활성화되어 있어요 (AXHUB_TELEMETRY=0 로 disable).'
  Write-Host '  (3) macOS Gatekeeper 의 helper binary quarantine attribute 를 제거해요.'
  Write-Host '  (4) auth-refresh 백그라운드 task 가 token 갱신해요.'
  Write-Host '  (5) helper binary 를 GitHub release 에서 HTTPS 로 다운로드 + 실행해요.'
  Write-Host '  (6) ~/.claude/settings.json 의 statusLine field 를 추가/관리해요 (other plugins preserved).'
  Write-Host ''
  Write-Host '거부하려면: $env:AXHUB_DISABLE_STATUSLINE_AUTOWIRE=''1'' 설정 후 install.'
  Write-Host 'uninstall 시 orphan stub 이 graceful fallback 을 보장해요.'
  Write-Host ''
  Write-Host '자세한 내용: https://github.com/jocoding-ax-partners/axhub#trust--uninstall'
  Write-Host '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
  if (-not (Test-Path -Path $AxhubStateDir)) {
    New-Item -ItemType Directory -Path $AxhubStateDir -Force | Out-Null
  }
  Set-Content -Path $AxhubDisclosureMarker -Value $AxhubDisclosureVer -Encoding UTF8
}
# --- end install-time disclosure ---

$BinDir = Split-Path -Parent $PSCommandPath
$ReleaseVersion = if ($env:AXHUB_PLUGIN_RELEASE) { $env:AXHUB_PLUGIN_RELEASE } else { 'v0.9.23' }
$ReleaseBase = "https://github.com/jocoding-ax-partners/axhub/releases/download/$ReleaseVersion"

# Windows only ships amd64 (per package.json build:all)
$ArchKey = if ($env:PROCESSOR_ARCHITECTURE -eq 'AMD64') { 'amd64' } else { $env:PROCESSOR_ARCHITECTURE.ToLower() }
if ($ArchKey -ne 'amd64') {
  $msg = "지원하지 않는 Windows 아키텍처에요 (요청: $ArchKey).`n" +
         "원인: 현재 release 는 windows-amd64 만 빌드해요.`n" +
         "해결: AXHUB_TOKEN 환경변수로 우회 → `$env:AXHUB_TOKEN='axhub_pat_...'`n" +
         "다음: arm64 Windows 지원은 다음 릴리즈에 추가될 예정이에요."
  Write-Output (ConvertTo-Json @{ systemMessage = $msg } -Compress)
  exit 0
}

$TargetName = "axhub-helpers-windows-amd64.exe"
$TargetPath = Join-Path $BinDir $TargetName
$LinkPath = Join-Path $BinDir "axhub-helpers.exe"

if (-not (Test-Path -Path $TargetPath -PathType Leaf)) {
  if ($env:AXHUB_SKIP_AUTODOWNLOAD -eq '1') {
    $msg = "axhub-helpers 바이너리가 없어요.`n" +
           "원인: AXHUB_SKIP_AUTODOWNLOAD=1 로 자동 다운로드 비활성화됨.`n" +
           "해결: 수동 빌드 → bun run build:all`n" +
           "다음: 또는 환경변수 해제 후 재시도 → Remove-Item Env:\AXHUB_SKIP_AUTODOWNLOAD"
    Write-Output (ConvertTo-Json @{ systemMessage = $msg } -Compress)
    exit 0
  }

  $Url = "$ReleaseBase/$TargetName"
  $TmpPath = "$TargetPath.tmp"
  Write-Information "axhub-helpers 바이너리 다운로드 중: $ReleaseVersion (windows-amd64)..." -InformationAction Continue

  try {
    # Use ProgressPreference=SilentlyContinue to speed up Invoke-WebRequest 10x
    $prevProgress = $ProgressPreference
    $ProgressPreference = 'SilentlyContinue'
    try {
      Invoke-WebRequest -Uri $Url -OutFile $TmpPath -TimeoutSec 600 -UseBasicParsing
    } finally {
      $ProgressPreference = $prevProgress
    }
  } catch [System.Net.WebException] {
    if (Test-Path $TmpPath) { Remove-Item -Path $TmpPath -Force -ErrorAction SilentlyContinue }
    $statusCode = $null
    if ($_.Exception.Response) { $statusCode = [int]$_.Exception.Response.StatusCode }
    if ($statusCode -eq 407) {
      # Pre-mortem #7 — corp NTLM proxy 407
      $msg = "회사 프록시 인증 (HTTP 407) 으로 다운로드 차단됐어요.`n" +
             "원인: PowerShell Invoke-WebRequest 가 HTTPS_PROXY 환경변수를 자동 인식하지 않아요.`n" +
             "해결: AXHUB_TOKEN 환경변수로 우회 가능 → `$env:AXHUB_TOKEN='axhub_pat_...'`n" +
             "다음: 또는 IT 팀에 github.com 다운로드 허용 요청."
    } else {
      $msg = "바이너리 다운로드 실패 ($Url).`n" +
             "원인: 네트워크 timeout (600s) 또는 GitHub release 접근 불가.`n" +
             "해결: AXHUB_TOKEN 환경변수로 우회 가능 → `$env:AXHUB_TOKEN='axhub_pat_...'`n" +
             "다음: 또는 수동 다운로드 → gh release download $ReleaseVersion --pattern '$TargetName' -D '$BinDir'"
    }
    Write-Output (ConvertTo-Json @{ systemMessage = $msg } -Compress)
    exit 0
  } catch [System.IO.PathTooLongException] {
    # Pre-mortem #5 — MAX_PATH on Korean profile + nested CLAUDE_PLUGIN_ROOT > 260 chars
    if (Test-Path $TmpPath) { Remove-Item -Path $TmpPath -Force -ErrorAction SilentlyContinue }
    $msg = "Windows 경로 길이 (MAX_PATH 260자) 초과로 설치 실패.`n" +
           "원인: CLAUDE_PLUGIN_ROOT 가 너무 깊거나 Korean profile 이름이 긴 경우 발생.`n" +
           "해결: 관리자 PowerShell 에서 LongPathsEnabled 활성화 → 'New-ItemProperty -Path HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem -Name LongPathsEnabled -Value 1 -PropertyType DWORD -Force'`n" +
           "다음: 또는 단축 경로 사용 → 'subst N: `$env:CLAUDE_PLUGIN_ROOT' 후 N: 에서 작업."
    Write-Output (ConvertTo-Json @{ systemMessage = $msg } -Compress)
    exit 0
  } catch {
    if (Test-Path $TmpPath) { Remove-Item -Path $TmpPath -Force -ErrorAction SilentlyContinue }
    $msg = "axhub-helpers 다운로드 중 알 수 없는 에러 발생.`n" +
           "원인: $($_.Exception.Message)`n" +
           "해결: AXHUB_TOKEN 환경변수로 우회 가능해요.`n" +
           "다음: `$env:AXHUB_TOKEN='axhub_pat_...' 후 token-init 재시도."
    Write-Output (ConvertTo-Json @{ systemMessage = $msg } -Compress)
    exit 0
  }

  # Move + post-Move re-check (Pre-mortem #6: Defender quarantine ~200ms after install)
  Move-Item -Path $TmpPath -Destination $TargetPath -Force
  Start-Sleep -Seconds 2
  if (-not (Test-Path -Path $TargetPath -PathType Leaf)) {
    $msg = "다운로드 후 바이너리가 사라졌어요 (Windows Defender / V3 / AhnLab 등 격리 가능성).`n" +
           "원인: 보안 솔루션이 Bun-compiled .exe 를 PUP 로 분류했을 수 있어요. v0.1.5 코드사이닝 전이라 우리 책임이에요.`n" +
           "해결: AXHUB_TOKEN 환경변수가 정식 회피 경로 → `$env:AXHUB_TOKEN='axhub_pat_...'`n" +
           "다음: 또는 IT 팀에 $TargetPath 예외 요청 (v0.1.8 Authenticode 서명 후 EDR allowlist 합법화 예정)."
    Write-Output (ConvertTo-Json @{ systemMessage = $msg } -Compress)
    exit 0
  }
}

# Remove existing link before relinking. Explicit pattern mirrors install.sh.
if (Test-Path -Path $LinkPath -PathType Any) {
  Remove-Item -Path $LinkPath -Force
}

# Windows: copy (symlinks need admin / developer mode)
Copy-Item -Path $TargetPath -Destination $LinkPath -Force

# sh/ps1-absorption Phase 3.1 (T7): .gitignore + post-commit hook + disclosure
# marker write delegated to `axhub-helpers post-install`. Single Rust source of
# truth + respects AXHUB_NO_DISCLOSURE / AXHUB_SKIP_AUTODOWNLOAD env semantics.
$RepoRoot = $null
try {
  $inside = git rev-parse --is-inside-work-tree 2>$null
  if ($LASTEXITCODE -eq 0 -and $inside -eq 'true') {
    $RepoRoot = git rev-parse --show-toplevel
  }
} catch {
  $RepoRoot = $null
}

if (Test-Path -Path $LinkPath -PathType Leaf) {
  $postInstallArgs = @(
    'post-install',
    '--target-name', $TargetName,
    '--bin-dir', $BinDir,
    '--link-path', $LinkPath
  )
  if ($RepoRoot) {
    $postInstallArgs += @('--repo-root', $RepoRoot)
  }
  try {
    & $LinkPath @postInstallArgs | Out-Null
  } catch {
    # Best-effort: post-install is non-fatal — broken binary or AMSI/EDR can
    # block execution, install.sh has the same fail-open contract.
  }
}

Write-Information "axhub-helpers -> $TargetName (OS=windows, arch=amd64)" -InformationAction Continue
exit 0
