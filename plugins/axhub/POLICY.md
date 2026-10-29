# axhub plugin 정책

axhub Claude Code plugin 이 사용자의 컴퓨터에서 무엇을 하고 무엇을 하지 않는지 공개하는 문서예요. 어려운 용어는 처음 나올 때 괄호로 풀어 썼어요. 이 `POLICY.md`는 plugin 행동 설명이며 고객 조직 정책 데이터 파일이 아니에요. 고객 정책 정본은 서버의 현재 게시본이에요.

## 네트워크 접근 — 인터넷에 언제 연결하나요
- 스킬(플러그인이 상황별로 꺼내 쓰는 기능 단위)과 훅(세션이 시작될 때 자동으로 도는 점검)은 기본적으로 `axhub` CLI(터미널에서 쓰는 axhub 명령 도구)를 통해 인터넷에 접근해요. 플러그인이 그 밖의 곳에 몰래 접속하지 않아요.
- "axhub가 최신인지", "axhub가 진짜 최신인지 먼저 확인", "up to date"처럼 버전 확인이 들어간 요청은 가장 먼저 `update` 스킬로 처리해요. 이때 `axhub --version`, `npm list`, `grep`/pipe 같은 일반 shell 확인, 앱 상태 조회, `/axhub:clarity`, `/oh-my-claudecode:autopilot` 를 먼저 실행하지 않아요. 사용자에게 보이는 첫 문장은 `현재 버전을 확인할게요.` 예요.
- 업데이트 뒤 같은 요청 안에 앱 현황 확인이 남아 있으면 존재하지 않는 `axhub app list` 를 추측하지 않고, plural `axhub apps` 표면을 help 로 확인한 뒤 정확히 `axhub apps list --json` 같은 읽기 전용 명령으로 이어가요. 이 앱 상태 흐름에 들어간 뒤에는 `command -v axhub && axhub --version`, `command -v claude && claude plugin list`, `claude plugin list 2>&1 | grep` 같은 설치·버전·플러그인 probe 를 다시 실행하지 않아요. 현재 폴더명·대화 맥락·가장 최근 목록으로 관련 앱이 하나로 좁혀지면 사용자에게 어느 앱을 볼지 묻지 말고 `axhub apps get <app> --json`, `axhub deploy list --app <app> --json` 까지 바로 실행해요. 존재하지 않는 `axhub deployment list` 나 `| head`, `2>/dev/null`, `grep`, `&&` 같은 shell 후처리는 붙이지 않아요.
- Claude Desktop 에 axhub App/MCP 도구가 같이 보여도 플러그인 스킬 흐름은 그 도구를 우선 사용하지 않아요. 버전·최신 확인이 같은 요청에 있으면 언제나 위의 `update` 스킬이 먼저 끝나요. 로그·환경변수·롤백·GitHub 재연결 같은 후속 운영 작업도 `Tenant recent deployments`, `Deployment list`, `App list`, `App get` 같은 App/MCP 도구 권한 팝업으로 빠지지 않고 CLI 계약을 따라요.
- 앱 생성·배포·온보딩의 저장소 단계는 public CLI의 top-level `git_backend`를 먼저 확인해요. 기존 app은 `axhub apps get <app> --json`의 persisted 값을 쓰고, fresh onboarding/bootstrap은 `axhub apps git-backend --tenant <tenant> --json`의 회사 판정(`decision`)을 따라요 — 회사가 정한 곳이면 묻지 않고, 둘 다 쓰는 회사에서 GitHub를 연결한 사람에게만 저장 위치를 물어요. 판정이 없는 구 서버에서는 추천값으로만 읽은 뒤 GitHub와 Axhub self-hosted 중 사용자가 직접 고르게 해요. 같은 대화의 선택은 bootstrap에 전달하고, 선택 없는 headless에서는 저장소 mutation 전에 멈춰요. selfhosted는 정적 사이트를 포함해 외부 저장소 계정 인증·device flow·GitHub App 설치 없이 `git push` lane을 쓰고(앱 저장소에 안 붙은 로컬 소스는 `import`가 올려요), GitHub는 정적 사이트 업로드를 포함해 각 기존 lane을 유지해요.
- 그래서 최신 확인 요청에는 아주 좁은 Code-mode update router guard 가 라우팅 문맥만 추가해요. 이 guard 는 SessionStart fallback 과 UserPromptSubmit match 로 동작하고, 명령을 실행하거나 앱 목록을 조회하지 않으며, `AXHUB_NO_UPDATE_ROUTER=1` 또는 `~/.axhub/config/no-update-router` 파일로 끌 수 있어요.
- 세션 시작 때 도는 auto-update 훅은 24시간에 1회만 뒤에서(백그라운드) `axhub update check` 명령으로 새 버전이 있는지 확인하고, 있으면 묻지 않고 `axhub update apply` 와 `claude plugin update` 로 조용히 설치해요. 실제 인터넷 연결은 axhub CLI 와 Claude Code 의 플러그인 명령이 해요. 세션 시작을 기다리게 하지 않아요.
- `plugin list`와 exact `plugin download`는 현재 로그인 OAuth 또는 active broad PAT로 App-backed marketplace를 읽어요. 목록은 `plugin.current_servable_version` summary를 페이지 단위로 읽고, download 결과에서 `version_id`를 만들거나 보고하지 않아요. Download는 요청한 새 ZIP만 만들고 기존 파일을 덮어쓰거나 받은 code를 실행하지 않아요.
- download 요청과 install 요청을 구분해요. `plugin install`은 offline preview 뒤 사용자가 host·exact version을 보고 승인한 `--execute --yes`에서만 동작해요. CLI가 bounded ZIP metadata·실제 압축 해제 크기·경로·symlink·manifest identity를 검증하고 App/host lock과 crash recovery를 적용한 뒤 `~/.axhub/plugins/` 아래 private local marketplace를 Claude Code 또는 Codex의 공식 plugin CLI로 user scope 설치해요. 모든 `AXHUB_*` 환경은 host process에 전달하지 않아요.
- `up` 스킬은 사용자가 `저장소 없이`·`레포 없이`·`지금 폴더 그대로 업로드`를 명시했을 때만 현재 폴더를 올려요. `GitHub 없이`만 말한 selfhosted app은 정상 clone→push 배포를 써요. `up`은 커밋을 만들거나 push 하지 않고 연결된 저장소도 건드리지 않으며, `.gitignore`를 존중하고 `.git/`·`node_modules/`·`.env` 계열을 빼요. 이미 AXHub 에 있는 앱이라 파일 수·크기·소스 버전을 보여 주고 묻지 않고 올려요.
- `plugin publish`는 `--execute`가 없으면 network·auth가 없는 offline preview예요. Publish execute에는 OAuth나 broad PAT 대신 `plugins:read` + `plugins:write` scoped PAT·권리 확인·명시 승인을 모두 요구해요. PAT를 미리 발급하지 않아요 — 로그인돼 있으면 CLI가 이 PAT를 알아서 준비하고, `plugin_pat_required`면 CLI `next_action`대로 `axhub auth login`을 안내하거나 안내된 `axhub auth pat issue … --use`만 실행하며 token을 화면에 내지 않아요. 성공은 `upload_complete`·installable=false(`--submit`을 붙이면 `review_submitted`·installable=false)이고, 게시 심사 신청은 `submit_plugin_version_for_review` 다음 동작인 `axhub plugin submit`이에요. 결과는 `axhub plugin manage status --app <app>`로 확인하고, 이후 owner는 App Console, reviewer는 Console Review를 사용해요.

## 로컬에 기록하는 파일 — 내 컴퓨터에 무엇을 남기나요
- `~/.axhub/cache/.plugin-update-check` — 업데이트를 너무 자주 확인하지 않도록 마지막 확인 시각을 남겨두는 표시 파일이에요.
- `~/.axhub/cache/auto-update.log` — 자동 업데이트가 무엇을 했는지 한 줄씩 남기는 기록이에요 (최대 200줄). `.auto-update.lock/` 은 두 세션이 동시에 업데이트하지 않게 잠깐 잡는 잠금이고, `.plugin-update-restart` 는 플러그인 새 버전을 받았으니 재시작하면 적용된다는 표시, `.auto-update-halt` 는 보안 검증에 실패한 버전을 다시 시도하지 않게 적어 두는 표시예요.
- 기본 install root는 `~/.axhub/plugins/`이고, `AXHUB_PLUGIN_HOME`을 설정하면 그 absolute directory로 전체 tree가 이동해요. `<root>/<app-id>/<host>/marketplace/`에는 exact plugin code·manifest·install metadata를 보관하고, `<host>/.install.lock`은 같은 App/host의 동시 설치를 막아요.
- Crash recovery 동안 `<host>/.marketplace-transaction.json`, `.marketplace-host-mutating.json`, `.marketplace-rollback-pending.json`, `.marketplace-host-installed.json`과 `.marketplace-staging-*`·`.marketplace-backup-*` directory가 남을 수 있어요. 다음 locked install이 journal에 따라 완료 또는 rollback한 뒤 이 임시 상태를 지워요.
- `~/.claude/settings.json` — AI 활용 기록을 켜기로 선택한 경우에만 axhub CLI 가 이 파일에 수집 설정을 추가해요. 자세한 내용은 아래 "AI 활용 기록" 항목을 봐요.

## 조직 정책 자동 동기화
- 자동 hook은 Claude Code 2.1.295 이상이 필요하고 실제 host 검증 버전은 2.1.312예요. hook 활성·명시 프로젝트 연결·자동 sync 활성 조건에서 동작해요. Codex는 manual-only로 정책 hook을 제외하며 Desktop Code mode는 미검증이에요.
- SessionStart(`startup|resume|clear|compact`)는 초기 문맥을 제공해요. UserPromptSubmit은 매 prompt마다 인증된 현재 게시본을 동기 조회한 뒤 development 본문만 조직/revision·이전 안내 대체·철회 안내와 함께 다시 제공해요. deployment 본문은 prompt 문맥에 넣지 않아요.
- `.axhub/state/developer-policy.json`에는 연결 origin/tenant/schema만 저장해요. credential은 없고 Git/worktree 경계를 넘지 않는 가장 가까운 명시 binding만 읽어요. `bind --tenant <uuid> --project <path>`는 인증된 조회 후 연결하며 교체에는 `--force`가 필요해요. app manifest의 이름·8시간 tenant cache로 연결을 추측하지 않아요.
- HTTP 목적지는 신뢰된 CLI profile/endpoint에서 정하고 binding origin은 일치 확인만 해요. 불일치·redirect로 다른 주소에 credential을 보내지 않아요. 기존 OAuth/PAT/profile 우선순위를 따르며 hook은 로그인 창·keychain 승인창을 자동으로 열지 않아요.
- private cache는 origin+인증 주체+tenant+schema로 나눠 ETag/revision/body를 한 record로 보관하고 atomic write해요. 매번 서버에서 인증·권한·최신 여부를 확인하며 TTL로 조회를 생략하지 않아요. 304는 요청 당시 고정한 cache snapshot만 쓰고 유효 cache가 없으면 남은 시간 안에 무조건 GET을 최대 1회 해요. 오래된 cache로 실패를 덮지 않아요. 최신 200 뒤 disk write만 실패하면 메모리 본문을 제공할 수 있어요.
- 정확한 `AXHUB_NO_POLICY_SYNC=1` 또는 `~/.axhub/config/no-policy-sync` 파일 존재가 자동 sync를 꺼요(OR). 다른 env 값은 끄지 않아요. 다시 켜려면 두 원인을 모두 해제해요. CLI가 없고 wrapper의 실행 디렉터리가 확실히 미연결이면 skip하지만 손상·읽기 실패·불확실한 binding은 차단해요. 구 CLI의 최종 exit 64도 실행 디렉터리가 확실히 미연결인 경우만 조용히 skip하고, 연결된 경우 `UNSUPPORTED`로 차단해요. 구 CLI는 다른 host payload cwd를 검증할 수 없다는 호환성 한계가 있어요. 지원 helper는 실행 디렉터리가 미연결이어도 payload cwd를 검증하며 다른 오류를 숨기지 않아요.
- 전체 wrapper 시작부터 종료까지 2초 제한이며 UserPromptSubmit은 `onFailure: block`으로 최신 확인 실패 시 해당 prompt를 차단해요. 일반 network/429 자동 retry는 없고 OAuth refresh 최대 1회·cache 복구 GET 최대 1회도 같은 deadline 안에서만 해요. helper는 동기 OTLP export를 하지 않고 정책 본문·prompt·token·credential URL을 로그에 남기지 않아요.
- 배포 준비는 opt-out과 무관하게 최신 deployment 정책을 명시 조회해요. 실패하면 client 준비를 중단하고 unset/withdrawn이면 진행해요. 기존 플랫폼 context_url·서버 권한·승인·사용자 확인은 유지하며 자연어 self-check를 서버 강제 정책으로 표시하지 않아요.
- 관리자는 게시마다 사용자가 선택한 AI 서비스에 정책이 전달될 수 있음을 확인해요. 이 확인은 provider 승인·allowlist나 AI 전달 ACK가 아니에요. 정책 전달과 모델 준수는 다르고 실행 중 응답 중단·기존 코드 자동 수정·이전 대화 원문 회수를 보장하지 않아요.

### 정책 동기화 복구
- 채팅 밖 terminal에서 `axhub tenants developer-policy status --json`으로 profile·binding source 표시(절대 경로 아님)·모든 disabled 원인·capability·last checked/emitted를 보고, `axhub tenants developer-policy get --tenant <uuid> --json`으로 명시 조회해요. status는 서버에 접속하지 않는 로컬 진단이며 get 성공과 emitted 표시도 host 전달 ACK가 아니에요.
- `POLICY_AUTH_REQUIRED`·`ACCESS_DENIED`는 인증·조직 권한을 복구해요. OAuth는 필요할 때 `axhub auth login --force`로 다시 로그인하되 hook과 같은 credential backend를 확인해요. keychain-only 성공만으로 file-backed 비대화형 인증 성공을 판단하지 않아요. 저장 방식 변경은 먼저 알려요.
- `BINDING_CONFLICT`는 profile/origin과 명시 연결을 바로잡고, `NETWORK`·`DEADLINE`·`RATE_LIMITED`는 연결 또는 안내된 대기 시간을 확인해요. `CACHE_UNUSABLE`·`PROTOCOL`·`INTERNAL`은 정제된 진단과 CLI 상태를 확인하며 `UNSUPPORTED`는 CLI/host 기능을 업데이트해요. 본문이나 credential을 오류 보고에 붙이지 않아요.
- 차단 오류의 선택적 `error.context`에는 확인된 안전한 `profile`·`tenant_id`·고정 `binding_source`(`project:.axhub/state/developer-policy.json`)와 서버 `X-Request-ID`에서 실제 받은 `request_id`만 담아요. 모르는 값은 생략하며 adapter는 고정 복구 문구와 검증된 진단만 표시해요. 문의할 때 이 정제된 정보만 복사하고 raw 응답·절대 경로·비밀값은 붙이지 마세요.
- 복구 뒤 사용자가 수동 재전송해요. 차단은 대기열이 아니고 자동 재개·입력 복원을 약속하지 않아요. 자동 sync를 끄거나 `axhub tenants developer-policy unbind --project <path>`로 연결을 해제할 수도 있지만 명시 get·배포 준비의 조회는 계속해요.

## 자동 업데이트와 끄는 법
- axhub CLI 와 플러그인은 새 버전이 확인되면 묻지 않고 뒤에서 조용히 설치돼요. 설치가 끝나면 다음 답변 앞에 "axhub CLI 가 v0.38.0 → v0.39.0 로 자동 업데이트됐어요" 처럼 한 줄로만 알려요. 플러그인 자체의 업데이트는 받아 두었다가 Claude Code 를 껐다 켜야 반영되고, 재시작 뒤 첫 답변에서 적용됐다고 한 줄로 알려요.
- 내려받은 파일의 서명 검증(누가 만든 파일인지 확인하는 절차)에 실패하면 설치하지 않고 한 번만 알려요. 그 버전은 다시 시도하지 않아요.
- 자동 설치를 원하지 않으면 환경변수(터미널에 설정해 두는 켜기/끄기 값) 또는 파일로 꺼요. 둘 중 하나만 있으면 확인·설치·기록 어느 것도 하지 않아요:
  - `AXHUB_NO_AUTO_UPDATE=1` 환경변수
  - `~/.axhub/config/no-auto-update` 빈 파일 (환경변수가 앱에 전달되지 않는 GUI 실행에서도 통해요)
- 끈 상태에서 업데이트하려면 "axhub 업데이트해줘" 라고 말하면 `update` 스킬이 처리해요.

## AI 활용 기록 (선택 수집) — 프롬프트 수집은 물어보고 켜요
- 팀 워크스페이스가 AI 활용 기록(내 Claude Code 프롬프트·응답·툴콜 내용을 워크스페이스로 보내는 수집 기능)을 지원하면, 첫 설정(온보딩) 중에 켤지 한 번 물어봐요. 사용자가 켜기를 고르고 워크스페이스 콘솔에서 1회 동의한 경우에만 켜져요 — 동의 없이 켜지지 않아요.
- 켜면 axhub CLI 가 `~/.claude/settings.json` 에 수집 설정(Claude Code 의 기본 텔레메트리 기능을 켜는 환경변수)을 기록하고, 전송은 plugin 이 아니라 Claude Code 가 워크스페이스 수집 주소로 직접 해요.
- 끄기: `axhub axrouter monitor --off` (이 컴퓨터에서만 끔) / `axhub axrouter revoke` (등록까지 해제) / `axhub axrouter revoke-consent --execute` (본문 수집 동의 철회). 켠 적이 없으면 아무것도 기록되지 않아요.

## 실패 자동 리포트 — CLI 문제는 조용히 개발팀에 전달돼요
- axhub CLI 가 설계된 동작 밖으로 실패하면(비정상 종료, 깨진 출력 같은 CLI 자체의 문제) 에이전트가 `axhub feedback -m <실패 요약>` 명령으로 개발팀의 비공개 이슈함에 문제를 알려요. 요약은 첫 줄에 무엇이 어떻게 실패했는지 한 문장, 아래에 실행한 명령·기대·실제 결과·맥락·재현 여부를 적은 것이고, CLI 는 실패한 명령의 경로·플래그 이름·종료 코드·오류 분류·오류 메시지 한 줄을 함께 붙여요 — 인자 값·토큰·환경 내용은 붙이지 않아요. 작업 흐름을 멈추지 않고, 전송 성공 여부를 따로 묻거나 알리지 않아요.
- 자동으로 붙는 진단은 실행한 명령 이름·플래그 이름·종료 코드·CLI 버전 같은 최소 정보뿐이에요 — 입력한 값·파일 내용·인증 정보는 수집하지 않아요. 예상된 거절은 리포트하지 않아요 — 로그인 필요, 사용법 오류, 확인 절차 거절처럼 CLI 가 설계대로 막은 경우는 보내지 않아요.
- 끄기: `AXHUB_NO_FEEDBACK_REPORT=1` 환경변수 또는 `~/.axhub/config/no-feedback-report` 파일을 만들어요.

## 파괴적 작업 승인 — 되돌리기 어려운 일은 먼저 물어봐요
- 삭제, 롤백(이전 버전으로 되돌리기), `--force`/`--execute` 같은 강제 실행 옵션이 붙는 변경은 항상 사용자에게 먼저 보여주고 확인을 받은 뒤에만 실행해요.
- 사람이 대답할 수 없는 자동 실행 환경(headless — 예: 예약 실행, 백그라운드 작업)에서는 실행하지 않고 "이렇게 하려고 했어요" 미리보기(preview)만 남기고 멈춰요.

- 예외 — 이미 AXHub 에 있는 앱을 다시 올리는 배포(재배포)는 묻지 않고 진행해요. 배포 이력으로 언제든 되돌릴 수 있어서예요. 대신 올리기 전에 앱이 AXHub 에서 뜰 수 있는지 확인하고, 막히는 것은 고쳐서 올린 뒤 `이렇게 맞춰서 올렸어요` 로 무엇을 고쳤는지 알려요. 값을 찾을 수 없는 설정값이나 AXHub 에서 안 되는 기능처럼 고르셔야 하는 것만 물어요. 자동 실행 환경에서도 물을 것이 없으면 재배포를 실행하고, 물을 것이 있으면 아무것도 바꾸지 않고 멈춰요.
- 배포 전에 앱 폴더의 `.env` 값을 앱 설정값으로 등록할 때는 값을 화면·로그·대화에 내보내지 않아요. 내 컴퓨터 주소(`localhost`·사설 IP)를 가리키는 값과 AXHub 가 넣어 주는 값은 등록하지 않아요. "그대로 올리기"로 답한 항목은 앱 폴더의 `.axhub/deploy-answers.json` 에만 적어 두고 저장소에 커밋하지 않아요.

## 데이터 범위 — 어디까지 볼 수 있나요
- axhub MCP 도구는 로그인 인증(OAuth)으로 확인된 내 계정 범위(tenant) 안의 데이터만 다뤄요. 다른 사람이나 다른 조직의 데이터는 보이지 않아요.
- axhub MCP 도구는 읽기 전용(read-only, 조회만 하고 바꾸지 않음)이지만, 플러그인 스킬의 상태 조회·배포·진단 흐름에서는 CLI 계약을 우선해요.
- 비밀번호·토큰 같은 인증 정보(credential)는 파일이나 로그에 남기지 않아요.

## Codex 지원 — 같은 정책이 적용돼요
- axhub 는 OpenAI Codex CLI(≥ 0.147.0, 최종 검증 0.148.0)용 파생 번들(`axhub-codex`)도 제공해요. 네트워크 접근·파괴적 작업 승인·실패 자동 리포트·kill switch 정책은 Codex 판에도 적용돼요. 다만 조직 정책 자동 hook은 제외하고 manual-only로 제공하며, Codex 판 번들에는 이 차이를 설명한 정책 문서가 함께 실려요.
- Codex 는 플러그인 훅을 사용자가 신뢰하기 전에는 실행하지 않아요 — 신뢰 전에는 자동 업데이트 확인이 돌지 않고, 업데이트는 `update` 스킬을 직접 부르면 돼요. Codex 가 신뢰하는 대상은 훅 command(스크립트 경로)이고, wrapper 스크립트 내용은 플러그인 업데이트로 재신뢰 확인 없이 갱신돼요 — wrapper 동작 변경은 CHANGELOG 에 명시해요.
- Codex 판의 로컬 표시 파일은 `-codex` 붙은 별도 파일(`~/.axhub/cache/.plugin-update-check-codex` 등)을 써서 Claude 판과 확인 주기를 나눠요. 모든 kill switch(`AXHUB_NO_AUTO_UPDATE` 등)는 동일하게 동작해요.
- AI 활용 기록(선택 수집) 옵트인은 Claude 판 전용이에요 — Codex 판 플러그인은 이 수집 설정을 만들거나 바꾸지 않아요.

에이전트 행동 규칙(Claude 가 axhub 작업에서 지키는 규칙)은 repo 의 `docs/policy/agent-policy.md` 에 정리돼 있어요.
