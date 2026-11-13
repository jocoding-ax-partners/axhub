---
name: plugins
description: '회사 플러그인 마켓플레이스에서 플러그인을 찾고, Claude Code·Codex 에 설치하고, 내 플러그인을 올려 게시 심사를 신청해요. "플러그인 목록 보여줘", "OO 플러그인 설치해줘", "OO 1.2.0 받아줘", "이 스킬 플러그인으로 올려줘", "플러그인 심사 어떻게 됐어?", "회사 마켓플레이스 연결해줘", "플러그인 내려줘" 같은 요청에 써요. 앱 배포는 deploy, 새 앱은 bootstrap, axhub 플러그인 자체 업데이트는 setup 이에요.'
examples:
  - utterance: "회사 플러그인 목록 보여줘"
    intent: "list company marketplace plugins"
  - utterance: "scaffold 1.23.0 을 Claude 에 설치해줘"
    intent: "install one exact plugin version into Claude Code"
  - utterance: "scaffold 1.23.0 이 폴더에 받아줘"
    intent: "download one exact plugin version"
  - utterance: "이 폴더 스킬들 하나의 플러그인으로 올려줘"
    intent: "publish a plugin and request its publication review"
  - utterance: "내 플러그인 심사 어떻게 됐어?"
    intent: "show plugin publication status"
  - utterance: "배포해줘"
    intent: "yield to deploy"
allows-dependency-execution: false
model: sonnet
---

회사 플러그인을 찾고, 설치하고, 올려 게시 심사까지 신청하고, 결과를 표로 끝내요.
명령은 지금 CLI 에 묻고, 제공 조건·자동 검수·내리기와 회수의 뜻은 문서 원문으로 읽어요.

## 순서

### 0. 확인
- `axhub plugin-support preflight --json` 으로 확인해요. 안 되면 setup 으로 보내고 멈춰요.
- 작업공간이 여럿이면 `--tenant` 를 붙여요. 올리기·심사 상태는 회사 UUID(`axhub auth status --json` 의 `tenants[].tenant_id`)를 써요.
- 올리기·마켓플레이스 연결·내리기는 `https://docs.axhub.ai/llms.txt` 의 「회사 플러그인 빠른 시작」·「회사 플러그인 마켓플레이스」를 `curl -fsSL` 로 원문 그대로 읽고 해요.
- 쓰기 전에 그 명령의 `--help` 를 읽고, 거기 있는 인자만 써요.

### 1. 찾기·받기
- 목록은 `axhub plugin list --json` 을 검색어·페이지로 좁혀 봐요.
- 받기는 정확한 버전만 새 파일로 받아요(`plugin download --app --version --output`).

### 2. 설치
- 플러그인·정확한 버전·도구(claude 또는 codex) 하나를 정해요. 도구를 모르면 물어요.
- 미리보기(`plugin install … --json`)와 목록의 위험 요약(hooks·MCP·실행 파일 수)을 보여줘요. 설치하려는 버전이 지금 제공 중인 버전과 다르면 함께 알려요.
- 한 번 물은 뒤 `--execute --yes` 로 설치하고, 그 도구를 다시 시작하라고 알려요.
- 마켓플레이스 연결은 문서 「회사 마켓플레이스 연결하기」대로 하고, 30일짜리 읽기 키를 저장하는 일이라 한 번 물어요.

### 3. 올리기
- 무엇을 올리는지 정해요: SKILL.md 하나(이름·버전), 플러그인 폴더(`.claude-plugin/plugin.json`), 이미 있는 플러그인의 새 버전(`--app` + 새 버전). 남이 만든 플러그인을 새로 재배포하면 `--third-party` 를 붙여요.
- `plugin publish <경로> --json` 미리보기로 파일·크기·위험 요약을 보여줘요. 비밀 파일(`.env`·`*.pem`·`*.key`·`id_rsa*`)이 있으면 빼 달라고 해요.
- "이 플러그인을 배포할 권리가 있음을 확인하고, 올린 뒤 게시 심사를 신청할까요?"라고 한 번 묻고, `--attest-distribution-rights --execute --submit` 로 올려요. 다시 시도할 때는 같은 `--idempotency-key` 를 써요.
- 올리기는 됐는데 신청만 안 됐으면 `plugin submit` 으로 신청만 해요.
- 끝나면 "결과 알림은 없어요. `plugin manage status` 로 확인해요"라고 알려요.

### 4. 상태·내리기
- `plugin manage status --app … --release-version … --json` 으로 먼저 한 번 보고, `headline` 과 `next_action` 을 쉬운 말로 알려줘요.
- 업로드 처리·자동 검수 중일 때만 `--watch` 로 한 번(5분) 기다려요. 관리자 승인처럼 사람의 결정을 기다리는 상태면 기다리지 않고 나중에 볼 명령만 알려줘요.
- 내리기는 `plugin delist` 미리보기를 보여주고 한 번 물은 뒤 실행해요.

### 5. 실패하면
- 출력의 `message_ko`·`next_action` 대로 해요. 게시용 키는 CLI 가 준비하고, 직접 발급은 `next_action` 이 그렇게 말할 때만 한 번 물은 뒤 해요.
- 게시용 키를 켰으면 끝나고 다른 axhub 작업으로 돌아가기 전에 `axhub auth pat unset` 으로 내려요.

## 끝
무엇을 했고 결과가 어떤지 한두 줄로 말해요. 목록은 표로 보여줘요. 다음 일이 있으면 한 줄로 알려줘요. 예: 심사 신청 → "승인되면 '설치해줘'라고 하면 돼요."

## 알아둘 것
- 질문에 답할 수 없는 실행(headless)이면 목록·받기·상태만 하고, 설치·올리기·내리기는 미리보기까지만 해요.
- 받아 달라는 요청을 설치 허락으로 보지 않아요. 받은 코드를 풀거나 실행하지 않아요.
- 위험 요약이 비어 있으면 "검사 안 됨", 조회가 실패하면 "확인 못 함"이에요. "없음"이라고 하지 않아요.
- 업로드가 끝났거나 심사를 신청한 것은 아직 설치할 수 없어요. `servable` 일 때만 "설치할 수 있어요"라고 해요.
- 플러그인 버전 심사는 "게시 심사", 앱 공개범위 심사는 "공개범위 변경 심사"예요. 섞어 부르지 않아요.
- 검수 결과 보고 사유 적기, 공개범위 변경 심사처럼 콘솔에서 사람이 할 일은 `next_action` 의 주소만 넘기고 CLI 로 대신하지 않아요.
- 반려된 버전을 같은 버전으로 다시 신청(`--reason`)하는 건 사용자가 고를 때만 해요. 운영팀 문의(`axhub feedback`)도 원할 때만 보내요.
- 보안 회수는 운영팀이 하는 일이라 문서 안내만 해요.
- 키(token) 값은 화면에 내지 않아요.
