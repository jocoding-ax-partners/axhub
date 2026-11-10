---
name: setup
description: 'axhub 를 쓸 준비가 됐는지 점검하고 빠진 것을 채워요. CLI 설치·버전·로그인·회사·git 설치를 차례로 확인해요. "셋업해줘", "처음인데", "온보딩", "시작하기", "axhub 설치", "로그인해줘", "axhub 최신이야?", "axhub 업데이트해줘", "버전 확인", "환경 점검", "setup", "update axhub" 같은 요청에 써요. 앱 만들기·배포는 하지 않아요.'
allows-dependency-execution: false
model: sonnet
---
> 이 본문이 중간에 끊겨 보이면 설치 경로의 이 SKILL.md 원문을 열어 전체 절차를 확인한 뒤 진행해요.


axhub 를 쓸 수 있는 상태인지 차례로 확인하고, 빠진 것만 채워요. 이미 된 단계는 건너뛰어요.
시킨 점검만 하고 끝내요. 다른 작업을 이어서 시작하지 않아요.

**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.

## 승인 게이트 계약 (요약)

codex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 승인 방식은 명시 텍스트 승인 1회예요 — preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요. 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.

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
