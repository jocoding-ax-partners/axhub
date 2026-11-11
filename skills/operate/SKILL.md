---
name: operate
description: '이미 axhub 에 있는 앱과 회사를 운영하는 명령을 찾아 실행해요. "로그 보여줘", "앱 이름·아이콘 바꿔줘", "앱 DB 에서 주문 몇 건인지 봐줘", "환경변수 바꿔줘", "이전 버전으로 되돌려줘", "앱 삭제해줘", "OO 에게 권한 줘" 같은 요청에 써요. 배포와 실패 원인은 deploy, 앱 코드는 development, 설치·업데이트·로그인은 setup 이에요.'
examples:
  - utterance: "이 앱 로그 보여줘"
    intent: "run an axhub operation command"
  - utterance: "앱 이름이랑 아이콘 바꿔줘"
    intent: "update app info"
  - utterance: "앱 DB 에서 이번 주 주문 몇 건인지 봐줘"
    intent: "query the app database"
  - utterance: "어제 버전으로 되돌려줘"
    intent: "roll back an app"
  - utterance: "내 앱들 지금 상태 어때?"
    intent: "show my apps status"
  - utterance: "내가 쓸 수 있는 커넥터 뭐 있어?"
    intent: "list my accessible connectors"
  - utterance: "GitHub 계정 다시 연결해줘"
    intent: "reconnect the GitHub account"
  - utterance: "배포해줘"
    intent: "yield to deploy"
model: sonnet
---

axhub 운영 요청을 CLI 명령으로 찾아 실행하고, 결과를 쉬운 말로 알려줘요.
명령 목록은 외우지 않아요. 무엇이 있는지와 쓰는 법은 항상 지금 CLI 에 물어요.

## 순서

### 0. 확인
- `axhub plugin-support preflight --json` 으로 확인해요. 안 되면 setup 스킬로 보내고 멈춰요.
- 앱을 가리키는 요청이면 사용자가 말한 앱 이름이나 현재 폴더의 앱을 써요. 작업공간이 여럿이면 그 앱의 작업공간을 `--tenant` 로 붙여요(`axhub apps get <앱> --json` 의 `tenant_id`).

### 1. 뜻 알기 (바꾸는 일일 때)
- 삭제·멈추기·되돌리기·공개 범위·권한처럼 결과가 큰 일은 `https://docs.axhub.ai/llms.txt` 목차의 운영 문서(`.md`)를 `curl -fsSL` 로 원문 그대로 읽고 뜻과 되돌릴 수 있는지 확인해요. 예: 삭제와 절전·일시정지의 차이, 롤백이 무엇을 되돌리는지.
- 비슷한 일이 여럿이면(예: "멈춰줘" → 절전·일시정지·삭제) 하나를 고르지 말고 차이를 한 줄씩 보여주고 어느 쪽인지 물어요.

### 2. 명령 찾기
- `axhub --json-schema --field-expr '.commands["<명사>"]["<동사>"]'` 처럼 필요한 부분만 잘라 찾아요. 없을 때만 한 단계씩 넓혀요. 전체를 통째로 읽지 않아요.
- 고른 명령의 `--help` 를 실행 전에 한 번 읽고, 거기 있는 인자·플래그만 써요.
- 내 범위를 묻는 질문은 내 범위 명령으로 답해요: 볼 수 있는 커넥터는 `axhub connectors mine`, 바꿀 수 있는지는 `axhub authz grants mine`. 회사 전체 목록으로 대신하지 않아요.
- 회사 권한 관리처럼 목록에 안 보이는 관리자 명령(예: 권한 주기 `axhub authz grants create`)은 그 명령의 `--help` 로 있는지 확인해요.
- 찾지 못하면 "axhub 에 그 기능은 없어요"라고 말하고 가장 가까운 명령을 알려줘요. 없는 명령을 지어내지 않아요.

### 3. 실행
- 조회는 바로 실행해요.
- 바꾸는 명령은 먼저 미리보기(실행 플래그 없이)로 무엇이 바뀌는지 받아요.
  - 사용자가 정확히 말한, 되돌릴 수 있는 바꾸기(이름·소개·아이콘·환경변수 추가 등)는 미리보기가 요청과 같으면 바로 `--execute` 로 실행해요.
  - 삭제·일시정지·롤백·앱 DB 쓰기·권한 주기처럼 되돌리기 어렵거나 영향이 큰 일, 또는 요청이 모호하면 미리보기를 보여주고 한 번 물은 뒤 `--execute`(필요하면 `--yes`)로 실행해요. 되돌릴 수 없으면 그 사실을 먼저 말해요.
- 앱 DB 에 쓰는 SQL(INSERT·UPDATE·DELETE·테이블 변경)은 묻는 쪽이에요. 조회 SQL 만 바로 실행해요.
- 브라우저 승인이 필요한 명령(예: GitHub 다시 연결 `axhub github link`)은 출력의 주소와 입력 코드를 채팅 본문에 바로 보여주고, 승인은 한 번만(`--wait-approval 90s`) 기다려요. 그래도 없으면 "승인하고 알려 주세요"라고 하고 멈춰요. 코드가 만료됐으면 `--fresh` 로 새로 받아요.
- 값(환경변수 등)은 채팅으로 받지 않고 `.env` 에서 읽게 해요(`--from-dotenv`).
- 실패하면 `message_ko`·`recovery_action` 을 쉬운 말로 설명해요. 인자 오류(exit 64)면 `--help` 를 다시 읽고 한 번만 고쳐 실행해요.

## 끝
무엇을 했고 결과가 어떤지 한두 줄로 말해요. 목록은 표로 보여줘요.
결과가 다음 일로 이어지면 한 줄로 알려줘요. 예: 커넥터 목록 → "이 데이터로 화면을 만들려면 '대시보드 만들어줘'라고 하면 돼요."

## 알아둘 것
- 배포·배포 실패 원인은 deploy, 앱 코드는 development, 새 앱은 bootstrap, 설치·업데이트·로그인은 setup 이 맡아요. 섞여 오면 그 부분은 넘기고 운영 부분만 해요.
- 질문에 답할 수 없는 실행(headless)이면 조회만 하고, 바꾸는 명령은 어떤 것이든 미리보기까지만 해요.
- `axhub plugin-support` 명령은 운영에 쓰지 않아요.
- 사용자에게 보이는 문장에는 내부 id·raw 출력·비밀값을 쓰지 않아요. 앱은 이름으로 말해요.
