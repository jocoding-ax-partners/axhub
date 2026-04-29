# axhub-helpers Windows installer — picks windows-amd64 binary, downloads from
# GitHub releases, places at bin/axhub-helpers.exe.
#
# Mirrors bin/install.sh logic for Windows hosts without Git Bash/WSL.
# Run automatically by hooks/session-start.ps1, or manually after release.
#
# NOTE: bin/install.sh:80 has a known operator precedence bug
#   ([ -e "$LINK_PATH" ] || [ -L "$LINK_PATH" ] && rm -f "$LINK_PATH")
# This PS mirror uses explicit Test-Path / Remove-Item to avoid replicating it.
# Tracked for sh-side fix: future v0.1.x.
#
# Maintainer: when bumping plugin version, update $RELEASE_VERSION below to
# match install.sh:48. Override via env $env:AXHUB_PLUGIN_RELEASE.

$ErrorActionPreference = 'Stop'

$BinDir = Split-Path -Parent $PSCommandPath
$ReleaseVersion = if ($env:AXHUB_PLUGIN_RELEASE) { $env:AXHUB_PLUGIN_RELEASE } else { 'v0.1.25' }
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

# Remove existing link before relinking. Explicit pattern (NOT install.sh:80 ||).
if (Test-Path -Path $LinkPath -PathType Any) {
  Remove-Item -Path $LinkPath -Force
}

# Windows: copy (symlinks need admin / developer mode)
Copy-Item -Path $TargetPath -Destination $LinkPath -Force

Write-Information "axhub-helpers -> $TargetName (OS=windows, arch=amd64)" -InformationAction Continue
exit 0
