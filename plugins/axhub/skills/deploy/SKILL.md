---
name: deploy
description: '이미 axhub 에 있는 앱의 바뀐 코드를 배포하고, 실패하면 원인을 봐요. "배포해줘", "이 폴더 그대로 올려줘", "서버 앱으로 바꿔서 배포해줘", "왜 배포 실패했어" 같은 요청에 써요. 처음 올리는 폴더는 import, 빈 폴더 새 앱은 bootstrap 이에요.'
examples:
  - utterance: "배포해줘"
    intent: "deploy the current app"
  - utterance: "커밋 말고 지금 폴더 그대로 올려줘"
    intent: "deploy the working tree as is"
  - utterance: "방금 배포 왜 실패했어?"
    intent: "diagnose a failed deployment"
  - utterance: "이 폴더 axhub 에 처음 올려줘"
    intent: "yield to import"
allows-dependency-execution: false
model: sonnet
---

이미 axhub 에 있는 앱의 코드를 axhub 에 맞게 맞춘 뒤 배포하고, 결과를 확인해 표로 끝내요.
올리는 길(Axhub 저장소·GitHub·정적 사이트·폴더 업로드)은 CLI 가 골라요. 스킬은 맞추기와 실패 진단을 맡아요.

## 순서

### 0. 대상
- 다른 배포 서비스(Vercel 등)를 말했으면 하지 않고 멈춰요. axhub 와 연결 안 된 폴더에서 배포할 곳을 말하지 않았으면 "어디에 배포할까요?"(axhub 추천)로 먼저 물어요.
- `axhub plugin-support preflight --json` 으로 확인해요. 안 되면 setup 스킬로 보내고 멈춰요. `capabilities.deploy.push` 가 true 가 아니면 CLI 를 업데이트해 달라고 하고 멈춰요.
- `axhub plugin-support deploy-prep --intent deploy --user-utterance "<사용자 문장>" --json` 으로 앱을 정해요.
  - `bootstrap_plan` 이 있으면 처음 올리는 폴더예요. 코드가 있으면 import, 비어 있으면 bootstrap 으로 보내고 멈춰요.
  - 앱 후보가 여럿이면 "어느 앱을 배포할까요?"로 이름 중에서 골라요.
  - 앱 이름만 말했고 폴더가 비어 있으면 `axhub repo clone <앱> . --json` 으로 받아요.
- "왜 실패했어"처럼 원인만 물으면 5번만 하고 아무것도 바꾸지 않아요.

### 1. 숙지
- preflight 의 `capabilities.readiness.context_url` 문서를 `curl -fsSL <주소>` 로 한 번 원문 그대로 받아 읽어요. 요약해 주는 웹 읽기 도구는 쓰지 않아요.
- 바뀐 코드가 DB·파일·로그인 같은 기능을 쓰면 `https://docs.axhub.ai/llms.txt` 목차에서 그 기능 문서(`.md`)를 찾아 같은 방법으로 읽어요.

### 2. 점검
- `axhub deploy check --json` 과 기준 문서로 지난 배포 뒤 바뀐 코드를 봐요.
- 빌드가 있으면 의존성을 설치하고 로컬에서 5분 안에 한 번 돌려 봐요.
- 찾은 것을 넷으로 나눠요: 바로 고칠 것, 물을 것, axhub 에서 안 되는 것, 사용자가 직접 할 일.

### 3. 맞추기
- 이미 있는 앱의 배포라 승인 카드는 띄우지 않아요. 물을 것이 있을 때만 모아서 한 번에 물어요.
- 안 되는 것은 "axhub 에서는 안 돼요"라고 먼저 말하고 기준 문서의 대안을 보여줘요.
- 정적 앱인데 서버 코드가 필요하면 "서버 앱으로 바꾸면 되돌릴 수 없어요"라고 알리고 물어요. 바꾸기로 하면 `axhub apps deploy-method --app <앱> --to docker --execute --yes`(서비스가 여럿이면 `compose`) 를 실행해요.
- 고칠 것을 한 항목씩 끝낼 때마다 `배포 전 맞추기: <항목>` 으로 커밋해요. `.env*`·키 파일·에이전트 폴더는 커밋하지 않아요.
- 새 설정값은 `axhub env set --app <앱> --from-dotenv .env --only <이름,…> --execute` 로 등록해요. 값은 채팅으로 받지 않고, 예시 값(`your-`·`xxx` 등)은 등록하지 않고 직접 할 일에 적어요. `.env` 의 기존 값은 지우지 않아요.
- 고친 뒤 2번 점검을 다시 해요. 두 번 고쳐도 안 되면 고친 데까지 커밋하고 원인을 알린 뒤 멈춰요.

### 4. 올리기
- `axhub deploy push --app <앱> --json` 으로 미리보기를 받아요. `blocked` 가 있으면 `next` 대로 해요.
  - 커밋 안 된 변경은 3번에서 커밋해요. 사용자가 "지금 폴더 그대로"라고 했을 때만 `--working-tree` 를 붙여요.
  - 이 폴더가 앱 저장소에 안 붙어 있다면 import 로 보내고 멈춰요.
- 막힌 게 없으면 같은 명령에 `--execute --wait` 를 붙여 한 번만 실행해요(명령 시간 제한 10분).
- 끝나지 않았거나(`status` 가 `started`·`running`, exit 6·10) 기다리다 끊기면 실패가 아니에요. 다시 올리지 말고 `axhub deploy verify <deployment_id> --app <앱> --wait --wait-timeout 9m --json` 으로 이어가요.
- `environment` 가 `staging` 이면 스테이징에만 반영됐고 운영은 심사 뒤라고 알려요.

### 5. 실패하면
- 배포 번호를 모르면 `axhub deploy list --app <앱> --json` 의 최근 배포에서 찾아요.
- `axhub deploy status <deployment_id> --app <앱> --json` 으로 실패 단계와 시각을 보고, 그 시간대 로그를 `axhub deploy logs --app <앱> --since <시작> --until <끝> --json --limit 100` 으로 봐요(스테이징 배포면 `--environment staging`). 앱이 지금 떠 있는지는 `axhub --json deploy diagnose <앱>` 으로 봐요.
- 원인을 쉬운 말로 한두 줄로 설명해요. 로그의 비밀값·토큰은 가려요.
- 코드로 고칠 수 있으면 고칠지 묻고, 그러자고 하면 2번부터 한 번 더 해요. 롤백·이전 배포 다시 올리기는 사용자가 말할 때만 해요.

## 끝
결과만 표로 보여주고 끝내요.

| 항목 | 상태 |
|---|---|
| 앱 | 이름 |
| 주소 | `url` (스테이징이면 운영 반영은 심사 뒤) |
| 맞춘 것 | 고친 것 · 등록한 설정값 이름, 없으면 없음 |
| 배포 | 성공 · 진행 중 · 실패와 원인 |
| 직접 할 일 | 없으면 없음 |

확인하지 못한 항목은 됐다고 표시하지 않아요.

## 알아둘 것
- 질문에 답할 수 없는 실행(headless)이면 미리보기와 요약까지만 하고 바꾸지 않고 멈춰요.
- 에이전트 샌드박스 안이면 Axhub 저장소로 올리는 실행은 처음부터 권한 상승으로 해요. `local.write_denied` 면 `recovery_action` 대로 해요.
- CLI 가 오류를 내면 `message_ko`·`recovery_action`·`next` 를 쉬운 말로 설명하고 어떻게 할지 물어요. 같은 배포를 새로 만들어 다시 시도하지 않아요.
- 사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요.
