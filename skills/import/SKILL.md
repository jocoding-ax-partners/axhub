---
name: import
description: '다른 곳에서 만든 앱(이미 코드가 있는 폴더)을 axhub 에 맞게 고쳐서 처음 올리고 첫 배포까지 해요. "이 폴더 axhub 에 올려줘", "기존 앱 올려줘", "Lovable·Bolt·Vercel 에서 만든 거 axhub 에 올려줘" 같은 요청에 써요. 빈 폴더 새 앱은 bootstrap, 이미 올린 앱 재배포는 deploy 예요.'
examples:
  - utterance: "이 폴더 axhub 에 올려줘"
    intent: "import an existing local app into axhub"
  - utterance: "Lovable 로 만든 앱 axhub 로 옮겨줘"
    intent: "migrate an app built elsewhere into axhub"
  - utterance: "기존 Express 서버 앱을 axhub 에 올려서 배포까지 해줘"
    intent: "import an existing backend app into axhub"
  - utterance: "새 앱 만들어줘"
    intent: "yield to bootstrap"
allows-dependency-execution: false
model: sonnet
---

다른 곳에서 만든 앱을 axhub 에 맞게 고쳐서 처음 올리고, 첫 배포를 확인한 뒤 결과 표로 끝내요.
axhub 에 올리면 axhub 가 주는 것을 써요: DB 는 axhub PostgreSQL, 파일은 보관함, 로그인은 axhub 로그인, 서버는 axhub 가 띄우는 앱이에요.

## 순서

### 0. 올릴 수 있나
- 사용자가 말한 앱 폴더에서 모든 명령을 실행해요. GitHub 주소면 `git clone` 으로 받고, 압축 파일이면 풀어서 써요. 폴더가 비어 있으면 bootstrap 으로 보내고 멈춰요.
- `axhub plugin-support preflight --json` 으로 확인해요. 안 되면 setup 스킬로 보내고 멈춰요. `capabilities.import.supported`·`capabilities.self_hosted_git.git_policy_decision` 이 true 가 아니면 CLI 를 업데이트해 달라고 하고 멈춰요.
- `axhub plugin-support tenant-resolve --json` 으로 작업공간을 정해요. `needs_pick` 이 true 면 "앱을 어느 작업공간에 올릴까요?"로 `candidates` 중에서 골라요.
- 앱 서비스 권한·저장 위치·GitHub 연결은 bootstrap 스킬 3·4단계와 똑같이 해요.

### 1. 숙지
- preflight 의 `capabilities.readiness.context_url` 문서를 `curl -fsSL <주소>` 로 한 번 원문 그대로 받아 읽어요. 요약해 주는 웹 읽기 도구는 쓰지 않아요. 규칙별 처리(바로 고침·물어봄·알림)의 기준이에요.
- DB·파일 저장·로그인·웹훅처럼 기능을 바꿔야 하면 `https://docs.axhub.ai/llms.txt` 목차에서 그 기능 문서를 찾아 같은 방법으로 읽어요.

### 2. 비교
- `axhub --json plugin-support import --mode preview --tenant <작업공간> --git-choice <key>` 로 CLI 판정(배포 방식·할 일·추정 이름)을 받아요. 내 GitHub 이면 `--github-owner <계정>`, deploy 에서 넘어온 앱이면 `--app <앱>` 을 붙여요.
- `axhub deploy check --json` 과 기준 문서로 코드를 봐요.
- 다른 서비스에 기대는 부분을 찾아요: Supabase·Firebase 같은 외부 DB·로그인, Vercel·Netlify 전용 함수, 고정 포트, 서버 폴더 저장. 외부 DB·로그인은 그대로 연결하지 않고 axhub 것으로 바꾸기(추천)·그대로 쓰기를 물어요.
- 빌드가 있으면 로컬에서 5분 안에 한 번 돌려 봐요.
- 찾은 것을 넷으로 나눠요: 바로 고칠 것, 물을 것, axhub 에서 안 되는 것, 사용자가 직접 할 일.

### 3. 안내와 확인
- 한 화면에 보여줘요: 앱 이름(추천 이름)·주소·저장 위치·배포 방식, 바로 고칠 것, 물을 것, 안 되는 것, 사용자가 직접 할 일.
- 안 되는 것(로컬 AI 모델 실행 등)은 "axhub 에서는 안 돼요"라고 먼저 분명히 말하고 기준 문서의 대안을 보여줘요. 대안이 없으면 그 기능을 빼고 올릴지 물어요.
- 바꾸면 달라지는 것(서버 코드가 필요해짐·기존 데이터 옮기기)도 알려요.
- `.env` 에 없는 필수 설정값은 값을 채팅으로 받지 않고 `.env` 에 넣어 달라고 해요.
- 물을 것을 모아 한 번에 묻고 "지금 이대로 고쳐서 axhub 에 올릴까요?"(올리기 / 그만두기)를 물어요. "바로 해줘"라는 말은 승인이 아니에요.
- 선택 카드에 빈 답변이 돌아오면 답하지 않은 거예요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.

### 4. 고치기
- 기준 문서의 고치는 법대로 고쳐요. 못 바꾸는 건 지어내지 않고 알려요.
- git 저장소가 아니면 `git init` 부터 해요. 한 덩어리를 고칠 때마다 `배포 전 맞추기` 로 커밋해서, 끊겨도 다시 부르면 이어가요. `.env*`·키 파일·에이전트 폴더는 커밋하지 않아요.
- 배포 설정 파일이 없으면 프로젝트 파일을 근거로 `axhub.yaml` 을 만들고(`name` 은 앱 주소 이름) `axhub deploy --explain --json` 으로 확인해요.
- 고치다가 새로 물을 일이 생기면 그것만 물어요.

### 5. 다시 비교
- `axhub deploy check --json` 과 로컬 빌드를 다시 돌려요. 통과해야 올려요. 안 되면 4번으로 돌아가고, 두 번 고쳐도 안 되면 고친 데까지 커밋하고 원인을 알린 뒤 멈춰요.

### 6. 올리기
- 미리보기와 같은 옵션에 `--mode execute --approved --commit-manifest --verify-wait none` 을 붙여 한 번만 실행해요. 등록할 설정값이 있으면 `--env-from-dotenv .env --env-only <이름,…>` 을 붙여요.
- 끝나면 `axhub deploy verify <deployment_id> --app <앱> --wait --wait-timeout 9m --json` 으로 확인해요(명령 시간 제한 10분).
- 정적 사이트가 `active_release_id` 로 끝나면 확인 없이 그 주소로 끝내요.
- 실행은 한 번만 해요. 기다리다 끊기면 다시 실행하지 말고 같은 확인 명령으로 이어가요.
- 시간 안에 안 끝난 건(exit 6·10) 실패가 아니에요. 같은 명령으로 다시 확인할 수 있다고 알리고 끝내요. 배포가 실패하면 diagnosis 스킬로 원인을 봐요.

## 끝
결과만 표로 보여주고 끝내요.

| 항목 | 상태 |
|---|---|
| 앱 | 이름 |
| 주소 | `apps get` 의 `access_url` (스테이징이면 운영 반영은 심사 뒤) |
| 저장 위치 | Axhub 저장소 · GitHub 저장소 |
| 맞춘 것 | 고친 것 · 등록한 설정값 이름 |
| 배포 | 성공 · 진행 중 · 실패 |
| 직접 할 일 | 외부 로그인 허용 주소에 새 주소 넣기 등, 없으면 없음 |

확인하지 못한 항목은 됐다고 표시하지 않아요.

## 알아둘 것
- 질문에 답할 수 없는 실행(headless)이면 `--headless` 미리보기와 요약까지만 하고 바꾸지 않고 멈춰요.
- 에이전트 샌드박스 안이면 Axhub 저장소로 올리는 실행은 홈 폴더에 git 자격증명을 써야 해서 처음부터 권한 상승으로 실행해요. 그래도 `local.write_denied` 가 나오면 `recovery_action` 대로 해요.
- CLI 가 오류를 내면 `message_ko`·`recovery_action` 을 쉬운 말로 설명하고 어떻게 할지 물어요.
- 사용자에게 보이는 문장에는 내부 명령 이름이나 상태 코드를 쓰지 않아요.
