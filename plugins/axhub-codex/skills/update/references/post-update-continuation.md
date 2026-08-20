# Update 후속 계약 (mixed-request continuation, codex)

update SKILL 이 로드하는 내부 reference 예요. 업데이트 결과 카드 뒤 같은 사용자 요청의 남은 axhub 작업(앱 상태 overview·GitHub 재연결/device code 등)을 이어갈 때 이 계약을 그대로 따라요.

**CRITICAL mixed-request continuation.** 사용자가 "업데이트 확인하고 앱 상태도 봐줘"처럼 다른 axhub 운영 요청을 함께 말하면, 이 스킬 실행 중에는 앱 목록·앱 상태·최근 배포·로그·환경변수 조회를 섞지 않아요. 먼저 업데이트 결과 카드까지 완료한 뒤, 같은 사용자 요청의 남은 일을 이어서 처리해요. 사용자가 `앱 상태 확인해줘`, `배포해줘`, `새 앱 만들어줘` 같은 말을 다시 하지 않아도 돼요. 원문이 영어로 `then`, `and then`, `after that`, `help me understand` 를 써도 업데이트 뒤 남은 요청을 버리지 않아요. 다만 남은 요청을 실제로 이어갈 때만 `업데이트 확인은 끝났어요. 이어서 요청하신 작업을 계속할게요.` 라고 짧게 말하고, 곧바로 다음 적절한 axhub 흐름을 시작해요.

**CRITICAL post-update app overview.** 업데이트 결과 뒤 남은 요청이 "내 앱들이 지금 어떤 상태인지", "내 앱 상태를 알아서 봐줘", "app status overview" 같은 읽기 전용 앱 현황이면 명령을 추측하지 않아요. `업데이트 확인은 끝났어요. 이어서 요청하신 앱 상태 확인을 계속할게요.` 를 말한 뒤에는 설치 확인·버전 확인·플러그인 확인을 다시 하지 않아요. 즉 `command -v axhub`, `axhub --version`, `codex plugin list --json` 을 다시 실행하지 않아요. 첫 overview 의 사용자에게 보이는 셸 command 는 아래 두 개만 허용해요. 먼저 정확히 `axhub apps --help` 를 1회 실행하고, 그 다음 정확히 `axhub apps list --json` 로 접근 가능한 앱 목록부터 읽어요. 명령 문자열 뒤에 공백 외 어떤 문자도 붙이지 않아요.

```bash
axhub apps --help
axhub apps list --json
```

목록에서 현재 폴더명·대화 맥락·가장 최근 수정 앱 중 하나로 관련 앱을 식별했으면 멈춰서 "어느 앱을 더 볼까요?"라고 묻지 않아요. 그 앱의 상세와 최근 배포 이력까지 같은 흐름에서 바로 확인해요. axhub MCP/App 도구를 찾지 않고, 계속 셸 command 로만 이어가요:

```bash
axhub apps get <app> --json
axhub deploy list --app <app> --json
```

여기서 `<app>` 은 사용자에게 보이는 앱 slug/name 을 우선 써요. CLI 가 app id 를 반환해도 다음 명령의 `--app` 값으로 raw id 를 드러내지 않고, slug/name 으로 조회할 수 없을 때만 내부적으로 좁혀요. 관련 앱을 하나로 좁혔는데 `이 중 어느 앱의 배포 상태나 로그를 더 자세히 확인하고 싶으신가요?`, `어느 앱을 볼까요?`, `더 자세히 확인하고 싶은 앱을 말해 주세요` 같은 질문으로 끝나면 실패예요. 존재하지 않는 단수 명령 `axhub app list` 또는 `axhub app get`, 존재하지 않는 `axhub deployment list`, 또는 MCP/App permission card 로 빠지면 실패예요. `command -v axhub && axhub --version`, `axhub apps list --json 2>/dev/null | head -100`, `axhub --help | head`, `grep`, `sed`, `awk`, `head`, `tail`, pipe, redirect, `&&`, `2>/dev/null`, `bash -lc`, `sh -c` 가 붙은 순간 실패예요. 그런 명령이 떠오르면 실행하지 말고 정확히 `axhub apps --help` → `axhub apps list --json` → `axhub apps get <app> --json` → `axhub deploy list --app <app> --json` 로 바꿔요. 출력이 길어도 shell 로 자르지 말고 tool 결과를 내부에서 필요한 만큼만 읽어요. 앱 overview 를 읽은 다음 같은 원문에 새 앱 생성·배포가 남아 있으면, 직접 low-level 명령을 추측하지 말고 bootstrap/deploy 흐름으로 이어가요.

**CRITICAL post-update GitHub reconnect/device-code.** 업데이트 결과 뒤 남은 요청이 GitHub 계정 재연결, GitHub account link, device code, 브라우저 자동 열기, 입력 코드 노출, 승인 뒤 자동 확인 같은 인증 흐름이면 앱 상태 overview 로 오해하지 않아요. `업데이트 확인은 끝났어요. 이어서 GitHub 계정 연결을 확인할게요.` 를 말한 뒤에는 설치 확인·버전 확인·플러그인 확인을 다시 하지 않고, axhub clarity GitHub device-flow 계약을 inline 으로 적용해요. 이 자연어 후속 흐름에서는 clarity 스킬을 새로 호출하지 않아요. 사전 탐색용 `axhub --help | grep`, `axhub github status`, `axhub git_connection_status`, `axhub github link --help | grep`, `command -v axhub && axhub --version`, shell pipe, redirect, `bash -lc`, `sh -c` 를 실행하지 않아요. device flow 시작 command 는 정확히 아래 계열만 써요. tenant 가 이미 명확할 때만 `--tenant <tenant>` 를 붙이고, 모르면 tenant 옵션 없이 시작해요.

```bash
AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link
AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --tenant <tenant>
AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --fresh
```

이 command 의 사용자에게 보이는 title/description 은 모두 정확히 `계정 인증 시작` 이에요. `axhub GitHub device flow 인증 시작`, `GitHub device flow 인증 시작 (브라우저 자동 열기)` 같은 긴 제목이나 description 을 쓰지 않아요. 실행 결과에서 `https://github.com/login/device` 와 코드를 읽으면, 승인 확인 명령을 실행하기 전에 먼저 assistant 본문에 아래처럼 두 줄로 URL 과 코드를 노출해요. URL 은 자동 링크로 바뀌지 않도록 inline code span 으로 써요. Markdown 링크 문법을 쓰지 않아요. `[https://github.com/login/device](github.com/login/device)`, `<https://github.com/login/device>`, bare `https://github.com/login/device` 처럼 링크/자동링크 형태로 꾸미면 실패예요.

```text
인증 URL: `https://github.com/login/device`
입력 코드: <USER_CODE>
```

사용자가 `승인했어` 라고 다시 말하기를 기다리지 말고, 코드 노출 뒤 응답을 끝내지도 말아요. CLI 가 `After approving in the browser, rerun axhub github link (or axhub github accounts list --json)` 같은 pending 문구를 출력해도 그 문구를 사용자에게 다음 요청처럼 떠넘기지 말고, 같은 assistant turn 에서 단일 확인 명령까지 이어가요. 확인이 pending 으로 끝났다면 승인 뒤 같은 확인 명령을 다시 실행하면 저장된 pending link 가 이어져요(새 코드 발급 없음). 단 사용자가 코드를 놓쳤거나 코드가 만료돼서 승인 자체를 못 했으면 그 저장된 pending link 가 죽은 코드를 그대로 돌려줘요 — 그때만 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --fresh` 로 새 코드를 발급해 두 줄을 다시 노출해요. 보고 있는 코드가 아직 유효하면 `--fresh` 를 붙이지 않아요(그 코드가 무효가 돼요). `--fresh` 가 exit 64 로 거부되는 구 CLI 면 플래그 없이 한 번만 다시 실행해요. tenant 가 이미 명확할 때만 `--tenant <tenant>` 를 붙이고, 모르면 tenant 옵션 없이 확인해요.

```bash
axhub github accounts list --json
axhub github accounts list --tenant <tenant> --json
```

이 확인 command 의 사용자에게 보이는 title/description 은 모두 정확히 `인증 확인` 이에요. `계정 인증 시작` command 뒤에 이 `인증 확인` command 가 보이지 않고 assistant 응답이 끝나면 실패예요. `while true`, `sleep`, background watcher, persistent monitor, shell loop 로 자동 watch 를 만들지 않아요. `sleep 3 && axhub github accounts list --json`, `axhub git_connection_status`, `axhub github status`, `axhub --help | grep`, `axhub github accounts list --json | jq`, `2>/dev/null`, `head`, `grep`, `sed`, `awk`, `&&`, pipe, redirect 가 붙은 command 가 떠오르면 실패예요. 이 경우 반드시 위의 단일 device-flow command 와 단일 accounts-list command 로 바꿔요.

**CRITICAL no background detour.** mixed request 의 남은 일을 백그라운드 작업·하위 에이전트로 우회하지 않아요. 업데이트 결과 뒤 같은 assistant 흐름에서 직접 이어가요. `axhubed 앱 상태 조회`, `앱 상태 백그라운드 조회` 같은 작업·카드·제목을 만들지 않아요.

**섞인 요청 처리.** 사용자가 "최신인지 확인하고 내 앱 상태도 봐줘"처럼 버전 확인과 다른 axhub 운영 요청을 함께 말하면, 이 스킬은 **버전 확인/업데이트 결과를 먼저** 처리해요. 앱 목록·앱 상태·배포 상태·로그·환경변수·데이터 조회는 업데이트 단계 안에서 직접 실행하지 않아요. axhub MCP/App 도구는 read 라도 호출하지 않아요. 업데이트 결과 카드 뒤에는 남은 요청을 이어서 처리하되, 이때도 앱 상태/배포 이력은 MCP/App 도구가 아니라 위의 CLI overview 흐름으로 실행해요. GitHub 계정 재연결/device code 남은 요청은 위의 GitHub device-flow fast path 를 inline 으로 이어가요. `앱 상태 조회`, `배포 상태 조회`, `최근 배포 조회`, `GitHub 연결 상태 확인` 같은 tool 제목이 떠올랐다면 업데이트 결과 뒤 다음 axhub CLI 흐름에서 실행해요. 백그라운드 작업으로 우회하지 않아요.
