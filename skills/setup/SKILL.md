---
name: setup
description: 'axhub 를 쓸 준비가 됐는지 점검하고 빠진 것을 채워요. CLI 설치·버전·로그인·회사·git 설치를 차례로 확인해요. "셋업해줘", "처음인데", "온보딩", "시작하기", "axhub 설치", "로그인해줘", "axhub 최신이야?", "axhub 업데이트해줘", "버전 확인", "환경 점검", "setup", "update axhub" 같은 요청에 써요. 앱 만들기·배포는 하지 않아요.'
examples:
  - utterance: "axhub 셋업해줘"
    intent: "check and fix the axhub environment"
  - utterance: "axhub 처음인데 뭐부터 해?"
    intent: "first-time setup"
  - utterance: "axhub 최신인지 확인하고 업데이트해줘"
    intent: "check and update the axhub CLI"
  - utterance: "axhub 로그인해줘"
    intent: "log in to axhub"
  - utterance: "새 앱 만들어줘"
    intent: "yield to bootstrap"
allows-dependency-execution: false
model: sonnet
---

axhub 를 쓸 수 있는 상태인지 차례로 확인하고, 빠진 것만 채워요. 이미 된 단계는 건너뛰어요.
시킨 점검만 하고 끝내요. 다음에 할 일을 추천하거나 다른 작업을 이어서 시작하지 않아요.

## 순서

### 1. CLI
- `axhub --version` 으로 확인해요.
- 안 되면 `~/.axhub/bin/axhub` (Windows 는 `~/.axhub/bin/axhub.exe`) 가 있는지 봐요. 있으면 그 전체 경로로 `plugin-support repair-path --json` 을 실행하고, 결과의 `bin_path` 로 남은 명령을 이어가요.
- 정말 없으면 설치해요. 받은 스크립트는 실행 뒤 지워요.
  - macOS · Linux: `curl -fsSL https://cli.axhub.ai/install.sh -o ~/axhub-install.sh && sh ~/axhub-install.sh; rm ~/axhub-install.sh`
  - Windows PowerShell: `irm https://cli.axhub.ai/install.ps1 -OutFile "$env:USERPROFILE\axhub-install.ps1"` 다음 `powershell -NoProfile -ExecutionPolicy Bypass -File "$env:USERPROFILE\axhub-install.ps1"` 다음 `Remove-Item "$env:USERPROFILE\axhub-install.ps1"`
  - Windows Git Bash: `curl -fsSL https://cli.axhub.ai/install.ps1 -o ~/axhub-install.ps1` 다음 `powershell -NoProfile -ExecutionPolicy Bypass -File "$(cygpath -w ~/axhub-install.ps1)"` 다음 `rm ~/axhub-install.ps1`
- 설치 직후에는 PATH 에 아직 없을 수 있어요. 그때는 위 전체 경로로 실행해요.
- npm 의 `axhub`·`axhub-cli` 는 이름만 맡아 둔 빈 패키지예요. 설치는 위 공식 설치 스크립트로만 해요.

### 2. 버전
- `axhub update check --json` 으로 확인해요.
- 새 버전이 있으면 `axhub update apply --execute --yes --json` 으로 올려요. 바뀐 버전을 한 줄로 알려요.
- 보안 검증 실패(exit 14·66)면 강제로 진행하지 않고, 회사 IT·보안팀에 알려 달라고 해요.
- 플러그인이 낡았으면, 지금 쓰는 AI 앱의 플러그인 화면에서 axhub 를 업데이트하고 앱을 다시 켜 달라고 안내해요.

### 3. 로그인
- `axhub auth status --json` 으로 확인해요.
- 안 돼 있으면 `axhub auth login --json` 을 실행해요. 저장된 로그인이 만료됐다고 나오면 `--force` 를 붙여요.
  - 바로 끝나고(exit 10) 승인 주소 `verification_uri_complete` 를 줘요. 실패가 아니라 승인 대기예요.
  - 주소가 `https://axhub.ai` 아래인지 확인하고 브라우저로 열어요. macOS `open "<주소>"`, Windows PowerShell `Start-Process "<주소>"`, Git Bash `cmd //c start "" "<주소>"`.
  - 명령 출력은 사용자에게 안 보여요. 채팅에 이렇게 알려요: `브라우저에서 axhub 로그인 [승인]을 눌러 주세요. 안 열렸으면 여기를 눌러 주세요: <주소>`
  - `axhub auth login --resume-last --wait-approval 90s` 로 기다려요. 명령 시간 제한은 120초 이상으로 잡아요. 아직이면(`device_code_pending`) 안내 없이 다시 실행해요. 만료되면(`device_code_expired`) 처음부터 해요.

### 4. 회사
- `axhub tenants list --json` 으로 소속 회사를 보여줘요. 고르게 하지는 않아요.

### 5. git
- `git --version` 으로 확인해요. 앱을 만들고 배포하려면 git 이 있어야 해요.
- 없으면 설치해도 되는지 묻고, 좋다고 하면 설치해요.
  - macOS: `xcode-select --install` (설치 창에서 [설치]를 눌러 달라고 알려요)
  - Windows: `winget install --id Git.Git -e --source winget`
- 저장소 자격 증명(`git setup`)은 여기서 만들지 않아요. 앱을 만들거나 배포할 때 고른 회사로 만들어요.

## 끝
점검 결과만 표로 보여주고 끝내요.

| 항목 | 상태 |
|---|---|
| axhub CLI | 버전 |
| 로그인 | 계정 |
| 회사 | 소속 회사 |
| git | 버전 또는 없음 |

확인하지 못한 항목은 됐다고 표시하지 않아요.

## 알아둘 것
- 에이전트 샌드박스 안이면 로그인·`repair-path`·CLI 설치·업데이트는 홈 폴더에 써야 해서 처음부터 권한 상승으로 실행해요.
- CLI 가 오류를 내면 오류 내용을 쉬운 말로 설명하고 어떻게 할지 물어요.
- 사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요.
