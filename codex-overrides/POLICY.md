# axhub plugin 정책 (Codex 판)

axhub Codex plugin 이 사용자의 컴퓨터에서 무엇을 하고 무엇을 하지 않는지 공개하는 문서예요. 어려운 용어는 처음 나올 때 괄호로 풀어 썼어요.

## 네트워크 접근 — 인터넷에 언제 연결하나요
- 스킬(플러그인이 상황별로 꺼내 쓰는 기능 단위)과 훅(세션이 시작될 때 자동으로 도는 점검)은 기본적으로 `axhub` CLI(터미널에서 쓰는 axhub 명령 도구)를 통해 인터넷에 접근해요. 플러그인이 그 밖의 곳에 몰래 접속하지 않아요.
- "axhub가 최신인지", "axhub가 진짜 최신인지 먼저 확인", "up to date"처럼 버전 확인이 들어간 요청은 가장 먼저 `update` 스킬로 처리해요. 이때 `axhub --version`, `npm list`, `grep`/pipe 같은 일반 shell 확인, 앱 상태 조회를 먼저 실행하지 않아요. 사용자에게 보이는 첫 문장은 `현재 버전을 확인할게요.` 예요.
- 업데이트 뒤 같은 요청 안에 앱 현황 확인이 남아 있으면 존재하지 않는 `axhub app list` 를 추측하지 않고, plural `axhub apps` 표면을 help 로 확인한 뒤 정확히 `axhub apps list --json` 같은 읽기 전용 명령으로 이어가요. 이 앱 상태 흐름에 들어간 뒤에는 설치·버전·플러그인 probe 를 다시 실행하지 않아요. 현재 폴더명·대화 맥락·가장 최근 목록으로 관련 앱이 하나로 좁혀지면 사용자에게 어느 앱을 볼지 묻지 말고 `axhub apps get <app> --json`, `axhub deploy list --app <app> --json` 까지 바로 실행해요. 존재하지 않는 `axhub deployment list` 나 `| head`, `2>/dev/null`, `grep`, `&&` 같은 shell 후처리는 붙이지 않아요.
- axhub MCP/App 도구가 같이 보여도 플러그인 스킬 흐름은 그 도구를 우선 사용하지 않아요. 버전·최신 확인이 같은 요청에 있으면 언제나 위의 `update` 스킬이 먼저 끝나요. 로그·환경변수·롤백·GitHub 재연결 같은 후속 운영 작업도 App/MCP 도구 권한 팝업으로 빠지지 않고 CLI 계약을 따라요.
- 최신 확인 요청에는 아주 좁은 update-first 라우팅 가드가 라우팅 문맥만 추가해요. 이 가드는 SessionStart 합본 훅과 UserPromptSubmit match 로 동작하고, 명령을 실행하거나 앱 목록을 조회하지 않으며, `AXHUB_NO_UPDATE_ROUTER=1` 또는 `~/.axhub/config/no-update-router` 파일로 끌 수 있어요.
- 세션 시작 때 도는 auto-update 훅은 24시간에 1회만 `axhub update check` 명령으로 새 버전이 있는지 확인해요. 실제 인터넷 연결은 훅 스크립트가 아니라 axhub CLI 가 해요.
- Codex 는 플러그인 훅을 사용자가 신뢰하기 전에는 실행하지 않아요. 신뢰하지 않으면 위 훅 표면(자동 업데이트 확인·라우팅 가드·재시작 확인·Windows 계약 안내)은 조용히 꺼지고, 스킬은 훅 없이도 완결돼요 — 업데이트는 `update` 스킬을 직접 부르면 돼요.
- `plugin list`와 exact `plugin download`는 현재 로그인 OAuth 또는 active broad PAT로 App-backed marketplace를 읽어요. 목록은 `plugin.current_servable_version` summary를 페이지 단위로 읽고 download 결과에서 `version_id`를 만들거나 보고하지 않아요. Download는 요청한 새 ZIP만 만들고 기존 파일을 덮어쓰거나 받은 code를 실행하지 않아요.
- download 요청과 install 요청을 구분해요. `plugin install --host codex`는 offline preview 뒤 명시한 `--execute --yes`에서만 bounded ZIP metadata·실제 압축 해제 크기·identity를 검증하고 App/host lock과 crash recovery를 적용한 뒤 `~/.axhub/plugins/` 아래 private local marketplace를 Codex 공식 plugin CLI로 user scope 설치해요. 모든 `AXHUB_*` 환경은 Codex process에 전달하지 않아요.
- `plugin publish`는 `--execute`가 없으면 network·auth가 없는 offline preview예요. Publish execute에는 OAuth나 broad PAT 대신 `plugins:read` + `plugins:write` scoped PAT file·권리 확인·명시 승인을 모두 요구하고, gate 통과 성공도 `review_ready`·installable=false와 `submit_plugin_version_for_review` 다음 동작으로 끝나요. 이후 owner는 App Console, reviewer는 Console Review를 사용해요.

## 로컬에 기록하는 파일 — 내 컴퓨터에 무엇을 남기나요
- `~/.axhub/cache/.plugin-update-check-codex` — 업데이트를 너무 자주 확인하지 않도록 마지막 확인 시각을 남겨두는 표시 파일이에요.
- 기본 install root는 `~/.axhub/plugins/`이고, `AXHUB_PLUGIN_HOME`을 설정하면 그 absolute directory로 전체 tree가 이동해요. `<root>/<app-id>/<host>/marketplace/`에는 exact plugin code·manifest·install metadata를 보관하고, `<host>/.install.lock`은 같은 App/host의 동시 설치를 막아요.
- Crash recovery 동안 `<host>/.marketplace-transaction.json`, `.marketplace-host-mutating.json`, `.marketplace-rollback-pending.json`, `.marketplace-host-installed.json`과 `.marketplace-staging-*`·`.marketplace-backup-*` directory가 남을 수 있어요. 다음 locked install이 journal에 따라 완료 또는 rollback한 뒤 이 임시 상태를 지워요.
- `~/.axhub/cache/.plugin-update-restart-codex` — 플러그인 새 버전을 받은 뒤 재시작으로 적용됐는지 확인하기 위한 표시 파일이에요. update marker들은 시각·버전만 담고 Claude/Codex host별로 나눠 서로 덮어쓰지 않아요.

## 자동 업데이트와 끄는 법
- axhub CLI 는 새 버전이 확인되면 자동으로 설치될 수 있어요. 플러그인 자체의 업데이트는 설치돼도 Codex 를 껐다 켜야 반영돼요.
- 자동 설치를 원하지 않으면 환경변수(터미널에 설정해 두는 켜기/끄기 값)로 꺼요:
  - `AXHUB_NO_AUTO_UPDATE=1` — 자동 설치 없이 새 버전이 있다고 알려주기만 해요.
- 훅을 신뢰하지 않았다면 자동 확인 자체가 돌지 않아요 — 이 경우 업데이트는 `update` 스킬("업데이트해줘")이 담당해요.

## AI 활용 기록 — Codex 판은 제공하지 않아요
- 워크스페이스 AI 활용 기록(프롬프트·응답 수집 기능)의 온보딩 옵트인은 Claude 판 전용 기능이에요. Codex 판 플러그인은 이 수집 설정을 만들거나 바꾸지 않아요.

## 실패 자동 리포트 — CLI 문제는 조용히 개발팀에 전달돼요
- axhub CLI 가 설계된 동작 밖으로 실패하면(비정상 종료, 깨진 출력 같은 CLI 자체의 문제) 에이전트가 `axhub feedback -m <실패 요약>` 명령으로 개발팀의 비공개 이슈함에 문제를 알려요. 작업 흐름을 멈추지 않고, 전송 성공 여부를 따로 묻거나 알리지 않아요.
- 자동으로 붙는 진단은 실행한 명령 이름·플래그 이름·종료 코드·CLI 버전 같은 최소 정보뿐이에요 — 입력한 값·파일 내용·인증 정보는 수집하지 않아요. 예상된 거절은 리포트하지 않아요 — 로그인 필요, 사용법 오류, 확인 절차 거절처럼 CLI 가 설계대로 막은 경우는 보내지 않아요.
- 끄기: `AXHUB_NO_FEEDBACK_REPORT=1` 환경변수 또는 `~/.axhub/config/no-feedback-report` 파일을 만들어요.

## 파괴적 작업 승인 — 되돌리기 어려운 일은 먼저 물어봐요
- 삭제, 롤백(이전 버전으로 되돌리기), `--force`/`--execute` 같은 강제 실행 옵션이 붙는 변경은 항상 사용자에게 먼저 보여주고 **명시 텍스트 승인**(안내된 승인 문구를 사용자가 그대로 입력)을 받은 뒤에만 실행해요.
- 사람이 대답할 수 없는 자동 실행 환경(headless — 예: `codex exec`, 예약 실행, 백그라운드 작업)에서는 승인이 도달할 수 없으므로 실행하지 않고 "이렇게 하려고 했어요" 미리보기(preview)만 남기고 멈춰요.

네이티브 선택 카드가 켜진 세션에서 빈 답변이 돌아오면 승인하지 않은 것으로 봐요 — 자리를 잠깐 비운 사이에 카드가 자동으로 닫혀도 그대로 진행하지 않고 다시 물어요. 카드가 열려 있는 동안에도 실행 단계로 넘어가지 않아요.

선택지를 고를 때 번호를 타이핑하는 대신 카드로 받고 싶으면 `~/.codex/config.toml` 의 `[features]` 에 `default_mode_request_user_input = true` 한 줄을 더하면 돼요. 설정을 대신 바꾸지는 않아요.

## 데이터 범위 — 어디까지 볼 수 있나요
- axhub MCP 도구는 로그인 인증(OAuth)으로 확인된 내 계정 범위(tenant) 안의 데이터만 다뤄요. 다른 사람이나 다른 조직의 데이터는 보이지 않아요.
- axhub MCP 도구는 읽기 전용(read-only, 조회만 하고 바꾸지 않음)이지만, 플러그인 스킬의 상태 조회·배포·진단 흐름에서는 CLI 계약을 우선해요.
- 비밀번호·토큰 같은 인증 정보(credential)는 파일이나 로그에 남기지 않아요.

에이전트 행동 규칙(에이전트가 axhub 작업에서 지키는 규칙)은 repo 의 `docs/policy/agent-policy.md` 에 정리돼 있어요.
