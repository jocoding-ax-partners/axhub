---
name: bootstrap
description: '새 axhub 앱을 템플릿으로 만들고 첫 배포까지 해요. 회사 정책에 맞춰 코드를 Axhub 저장소나 GitHub(내 계정·회사 조직)에 둬요. "새 앱 만들어줘", "앱 만들어줘", "처음부터 앱 만들어줘", "내 깃허브에 새 앱 만들어줘", "회사 org 에 저장소 만들고 시작해줘", "앱 만들기 이어서 해줘" 같은 요청에 써요. 이미 코드가 있는 폴더는 import 로 가요.'
allows-dependency-execution: true
model: sonnet
---
> 이 본문이 중간에 끊겨 보이면 설치 경로의 이 SKILL.md 원문을 열어 전체 절차를 확인한 뒤 진행해요.


템플릿으로 새 앱을 만들어 첫 배포까지 하고, 코드를 이 컴퓨터에 받아 둔 뒤 결과 표로 끝내요. 이미 된 단계는 건너뛰어요.
회사 정책을 먼저 보고, 정책이 허락하지 않는 건 묻지도 하지도 않아요.

**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.

## 승인 게이트 계약 (요약)

codex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 승인 방식은 명시 텍스트 승인 1회예요 — preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요. 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.

## 순서

### 1. 준비
- `axhub plugin-support preflight --json` 으로 확인해요. 안 되면 setup 스킬로 보내고 멈춰요.
- `capabilities.self_hosted_git.git_policy_decision` 이 true 가 아니면 CLI 를 업데이트해 달라고 하고 멈춰요.

### 2. 작업공간
- `axhub plugin-support tenant-resolve --json` 으로 정해요. `needs_pick` 이 true 면 "새 앱을 어느 작업공간에 만들까요?"로 `candidates` 중에서 골라요.

### 3. 회사 정책
- `axhub tenants features --tenant <회사> --json` 의 `app_platform.enabled` 가 false 면, 이 회사에서 앱 서비스 권한이 없으니 회사 관리자에게 요청해 달라고 하고 멈춰요.
- `axhub apps git-backend --tenant <회사> --json` 의 `git_backend.decision` 으로 저장 위치를 정해요.
  - `use`: 묻지 않고 그 선택지를 써요.
  - `ask`: "코드를 어디에 저장할까요?"로 물어요. 선택지는 `options` 순서 그대로, 추천에 `(추천)`을 붙여요.
  - `link_github`: 아래 GitHub 계정 연결부터 하고 다시 조회해요.
- 사용자가 먼저 말한 저장 위치가 `options` 에 있으면 묻지 않고 써요. 없으면 이 회사는 그 위치를 쓸 수 없다고 알리고 멈춰요.

### 4. GitHub 일 때만
Axhub 저장소면 건너뛰어요. GitHub 연결·앱 설치 이야기는 꺼내지 않아요.
- 계정 연결(처음 한 번): `axhub github link --json` 이 주는 코드를 채팅에 이렇게 알려요: `https://github.com/login/device 에서 코드 <코드> 를 입력해 주세요.` 그다음 `axhub github link --wait-approval 90s --json` 으로 기다려요(명령 시간 제한 120초 이상). 코드가 만료됐으면 `--fresh` 로 새로 받아요.
- 계정 고르기: 내 GitHub 이고 `owners` 가 둘 이상이면 "어느 GitHub 계정에 둘까요?"로 물어요.
- 앱 설치: 고른 계정이 `installed: false` 면 `axhub github accounts list --json` 의 그 계정 `install_url` 을 주고, 설치하고 알려 달라고 해요. 알려 주면 다시 조회해요.

### 5. 이름
- "앱 이름을 무엇으로 할까요?"로 물어요. 요청에 맞는 추천 이름을 함께 보여줘요.
- slug 는 이름의 kebab-case, 주소와 저장소 이름은 slug 그대로예요.
- `axhub apps check-availability --tenant <회사> --slug <slug> --subdomain <slug> --json` 으로 확인하고, 못 쓰면 다른 이름을 받아요.
- 같은 이름의 앱이 이미 내 앱으로 있으면(이어 하기) 6·7번을 건너뛰어요.

### 6. 확인
- `axhub apps bootstrap --git-choice <key> --template nextjs-axhub --name <이름> --slug <slug> --repo-name <slug> --subdomain <slug> --tenant <회사> --dry-run --json`
  - 내 GitHub 이면 `--github-owner <계정>` 을 붙여요.
- 이름·주소·저장 위치를 보여주고 "지금 만들고 배포까지 진행할까요?"(진행 / 취소)를 물어요. "바로 해줘"라는 말은 승인이 아니에요.
- 선택 카드에 빈 답변이 돌아오면 답하지 않은 거예요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.

### 7. 만들기
- 진행을 고르면 같은 값으로 `--dry-run` 대신 `--execute` 를 붙여 `axhub --no-input apps bootstrap ... --json` 을 실행해요. 바로 끝나고 `bootstrap_id` 를 줘요.
- `axhub apps bootstrap-status <bootstrap_id> --tenant <회사> --watch --watch-timeout 9m --json` 으로 기다려요(명령 시간 제한 10분). 끝나면 `deployment_id`·`repo_full_name` 이 나와요.
- `--execute` 는 한 번만 해요. 기다리다 끊기면 다시 실행하지 말고 같은 상태 명령으로 이어가요. 앱이 두 번 만들어질 수 있어요.
- 실행 중 GitHub 연결이 끊겼다고 나오면(`github_relogin_required`) 4번 계정 연결을 다시 하고 같은 `--execute` 를 한 번 더 해요.

### 8. 코드 받기
- `warning_code` 가 `repo_access_invited` 면 GitHub 에서 저장소 초대를 먼저 수락해 달라고 해요.
- Axhub 저장소: `axhub --tenant <회사> git setup --json` 다음 `axhub --tenant <회사> repo clone <slug> --json`
- GitHub: `git clone https://github.com/<repo_full_name>.git <폴더>`
- 받은 폴더를 이후 작업 폴더로 써요.

### 9. 배포 확인
- `axhub deploy verify <deployment_id> --app <slug> --wait --json` 으로 확인해요(명령 시간 제한 10분).
- 시간 안에 안 끝난 건(exit 6·10) 실패가 아니에요. 같은 명령으로 다시 확인할 수 있다고 알리고 끝내요.
- 배포가 실패하면 diagnosis 스킬로 원인을 봐요.
- 사용자가 처음에 공개까지 원했으면 `axhub publish --app <slug> --visibility public --execute --json` 으로 공개 신청해요.

## 끝
결과만 표로 보여주고 끝내요.

| 항목 | 상태 |
|---|---|
| 앱 | 이름 |
| 주소 | `apps get` 의 `access_url` (스테이징이면 운영 반영은 심사 뒤) |
| 저장 위치 | Axhub 저장소 · GitHub 저장소 |
| 코드 폴더 | 경로 |
| 배포 | 성공 · 진행 중 · 실패 |

확인하지 못한 항목은 됐다고 표시하지 않아요.

## 알아둘 것
- 질문에 답할 수 없는 실행(headless)이면, 물어야 할 게 생긴 순간 아무것도 만들지 않고 선택지만 보여주고 멈춰요.
- 에이전트 샌드박스 안이면 `git setup` 은 홈 폴더에 써야 해서 처음부터 권한 상승으로 실행해요.
- CLI 가 오류를 내면 오류 내용을 쉬운 말로 설명하고 어떻게 할지 물어요.
- 사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요.
