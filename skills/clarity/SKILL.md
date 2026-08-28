---
name: clarity
description: '요청하신 운영 명령을 확인할게요. Use for "axhub 로그 보여줘", "axhub 환경변수 확인해줘", "axhub 앱 삭제해줘", "axhub clarity" and other axhub 로그·환경변수·롤백·데이터·connector·GitHub 계정 재연결/device code·앱 삭제 operations. Start directly with that Korean sentence; no preamble; do not mention the skill or routing. First tool title/description exactly "명령 찾기". 스킬 실행 전 사용자 문장·App/MCP·파일 조회는 0개예요. app status overview·app creation·mixed freshness+status+create prompts·최신/버전/update/latest/freshness checks·bootstrap·import·deploy는 담당 스킬로 양보하고 axhub 맥락 없는 요청에는 쓰지 않아요.'
---

# axhub clarity 브리지

## When to invoke

- **Explicit clarity request.** 사용자가 `/axhub:clarity`, "axhub clarity", "Use the axhub clarity skill" 처럼 이 스킬을 직접 지목해요.
- **Exact CLI operations.** axhub 맥락에서 로그, 환경변수, 롤백, 테이블/컬럼/데이터, connector grant/mine, GitHub 재연결/device code, 앱 삭제 같은 공개 CLI 운영 명령을 요청해요. 이 운영 요청은 frontmatter description 라우팅으로 자연어에서도 이 스킬이 직접 받아요 — 별도 훅 라우터 없이요.

## Do not invoke / route guard

최신성·버전·업데이트가 보이면 이 스킬이 아니에요. `최신`, `버전`, `업데이트`, `latest`, `up to date`, `version check`, `freshness` 가 있으면 앱 상태·새 앱 생성·배포가 함께 있어도 먼저 `update` 가 끝나야 해요. 이미 `/axhub:clarity` 배지가 뜬 뒤 이 문서를 읽었다면 `Using /axhub:clarity...` 같은 문장을 쓰거나 명령을 실행하지 말고, 첫 visible assistant text 를 정확히 `현재 버전을 확인할게요.` 로 둔 뒤 update 계약으로 양보해요.

앱 상태 overview, "내 앱들이 지금 어떤 상태인지도 알아서 봐줘", 새 앱 생성, bootstrap, import, deploy 가 보이면 clarity 가 아니에요. 특히 `command -v axhub && axhub --version`, `Checking axhub CLI installation and version`, `MCP tools to check axhub status and your apps` 로 시작하면 실패예요. 그런 경우 앱 상태 조회나 CLI probe 를 시작하지 말고 담당 스킬(update/bootstrap/import/deploy)로 양보해요.

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

> **CLI 경로 계약 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 반환된 `bin_path` 절대경로로 이 세션을 이어가요. 셋 다 없을 때만 onboarding 을 안내해요.

이 스킬은 axhub CLI 운영 명령 브리지예요. 로그, 환경변수, 롤백, 테이블/컬럼/데이터, connector grant, GitHub 재연결처럼 명시된 운영 명령만 처리해요. 작업→명령 카탈로그는 없어요 — **매번 라이브 CLI 의 `--help` 트리를 탐색**해 맞는 명령을 찾고, 조회 명령은 바로 실행하되 파괴적 변경은 승인 뒤 실행해요.

현재 폴더에 axhub 연결(manifest)이 없고 발화에 axhub 언급도, 대화에 axhub 맥락도 없으면 — 예를 들어 일반 프로젝트에서 "로그 보여줘" — axhub CLI 탐색을 시작하지 않고 일반 작업으로 양보하며 조용히 종료해요.

스킬이 호출되면 `스킬 가이드가 반환됐네요` 같은 메타 설명을 사용자에게 말하지 않아요. 첫 visible 문장을 정확히 `요청하신 운영 명령을 확인할게요.`로 쓰며, 첫 visible 문장은 사용자가 요청한 일을 바로 하는 말이에요. 이어 제목 `명령 찾기`의 bare `axhub --json-schema --field-expr '<가장 좁은 후보>'` 한 번으로 시작하고, 그 전 설치 확인·App/MCP·파일 조회·다른 문장은 0개예요. 명사·동사 후보가 하나면 leaf 부터, 불명확하면 root 부터 검증해요.

**CLI-only.** 이 스킬의 조회·운영 브리지는 Claude Desktop 에 보이는 `axhub` App/MCP 도구가 아니라 Bash/명령 도구로 실행하는 `axhub` CLI 만 사용해요. read-only 조회라도 MCP/App tool 로 빠지면 CLI help gate·제목 계약·권한 UX 를 검증할 수 없어서 이 스킬의 실패예요.

사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL 로만 써요. Markdown URL 링크 문법은 전부 금지예요. `[https://...](https://...)`, `[열기](https://...)`, `<https://...>` 처럼 URL 을 괄호나 label 로 감싸지 말고 `https://...` 그대로 보여줘요.

**Desktop-visible command allowlist.** Claude Desktop 에 보이는 모든 `clarity` 명령은 `axhub ...` 단일 명령이어야 해요. 탐색(`--json-schema`), 사용법 확인(`--help`), 실행(`--json`) 모두 공통으로 `bash -lc`, `sh -c`, `| head`, `| grep`, `| sed`, `| awk`, shell pipe, `>`, `<`, `2>`, `&>`, `;`, `&&`, `||`, `echo`, `cat`, `wc`, `tee`, `xargs`, `jq`, `python`, `node`, `perl`, `mktemp`, command substitution, 임시 파일, Read/Write/file tool 을 쓰지 않아요. `--field-expr` 문자열 내부의 `|` 는 허용되지만 shell pipe 로 출력 후처리하면 실패예요. 출력이 크면 `head -c` 로 자르지 말고 더 좁은 `--field-expr` 경로를 다시 고른 단일 `axhub --json-schema --field-expr '...'` 명령을 실행해요.

## 자연어 라우팅 계약

이 스킬은 명시적 clarity 요청이나 axhub CLI 운영 브리지 요청에만 써요. 로그, 환경변수, 롤백, 테이블/컬럼/데이터, connector grant, GitHub 재연결처럼 전용 스킬이 없는 공개 CLI 작업만 맡아요.

Claude Desktop 에서는 slash 명령이 채팅에서 인식되지 않을 수 있으므로, 사용자가 아래처럼 영어로 직접 clarity 를 지목해도 이 스킬을 실행해요:

- `Use the axhub clarity skill. Show logs for <app>.`
- `Use axhub clarity to set an environment variable.`
- `reconnect my GitHub account with axhub`
- `GitHub device code`

이런 요청을 받으면 직전 답변을 재사용해서 끝내지 말고, 필요한 공개 CLI 조회를 새로 실행해 현재 결과를 확인해요. slash 명령이 실패한 직후라도 자연어 요청은 독립된 새 요청으로 취급해요.

## Device Flow 코드 표시

GitHub 연결처럼 OAuth device flow 가 열리는 명령은 코드 표시가 사용자 행동의 핵심이에요. `axhub github link`, 로그인·연결 명령, 또는 실행 출력에 `github.com/login/device`, `verification_uri`, `verification_uri_complete`, `user_code`, `Enter code`, `XXXX-XXXX` 형태의 입력 코드가 보이면 예외적으로 URL과 입력 코드는 사용자 가치 정보로 취급해요.

- device flow fast path 에서는 shell loop, background watcher, persistent monitor 를 쓰지 않아요. `while true`, `sleep`, `grep`, command substitution, 임시 로그 파일 저장/재읽기, `Monitor 사용` 권한 카드가 뜨는 명령은 실패예요. device flow 시작은 `계정 인증 시작` 단일 CLI 호출 하나로 처리하고, CLI stdout/stderr 에서 URL과 코드를 바로 읽어요.
- device flow fast path 에서는 다른 사전 점검을 건너뛰어요. 사용자가 지금 필요한 건 코드와 브라우저 승인이라서, 권한 팝업이나 사전 안내로 앞단을 늦추지 않아요.
- Claude Desktop 에서 `axhub github link` 를 실행할 때는 자동 브라우저 열기와 agent-safe 흐름을 위해 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --tenant <tenant>` 형태로 실행해요.
- **코드 유실·만료는 `--fresh` 로 재발급해요.** `github link` 는 저장된 pending device link 를 그대로 이어 줘서, 코드를 놓쳤거나 만료된 뒤 같은 명령을 다시 실행하면 이미 죽은 코드가 그대로 돌아와요 — 사용자가 아무리 승인해도 영영 안 풀려요. 코드를 못 봤다·사라졌다·만료됐다는 신호가 있거나 같은 코드로 `인증 확인` 이 계속 pending 이면 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --fresh` 로 새 코드를 발급받아 두 줄을 다시 보여줘요. 사용자가 지금 보고 있는 코드가 아직 유효하면 `--fresh` 를 붙이지 않아요 — 그 코드가 무효가 돼요. `--fresh` 가 unknown flag(exit 64)로 거부되면 구 CLI 라서 그런 거예요. 플래그 없이 한 번만 다시 실행하고 update 스킬로 CLI 를 올리도록 안내해요.
- device flow 를 시작하는 Bash/tool call 제목과 description 은 모두 정확히 `계정 인증 시작` 으로 직접 채워요. 자동 제목이나 description 에 제품명, `axhub GitHub device flow 인증 시작`, `GitHub 인증 시작`, 영어 단어, 설명 문장이 붙어 보이면 같은 명령이라도 이 제목과 description 으로 고쳐 다시 호출해요.
- 로그를 짧게 폴링해서 URL과 입력 코드를 찾고, 발견 즉시 일반 채팅 본문에 URL과 입력 코드를 다시 써요. device-flow URL 은 Claude Desktop 이 자동 링크로 바꾸지 못하도록 URL 부분만 inline code span 으로 써요. `[https://github.com/login/device](https://github.com/login/device)`, `[https://github.com/login/device](github.com/login/device)`, `[GitHub 열기](https://github.com/login/device)`, `<https://github.com/login/device>`, bare `https://github.com/login/device` 같은 링크/자동링크 형태는 모두 금지예요. 반드시 `인증 URL: \`https://github.com/login/device\`` 와 `입력 코드: <USER_CODE>` 두 줄을 일반 채팅 본문에 그대로 보여줘요.
- 입력 코드를 찾았으면 승인 확인이나 계정 목록 조회를 시작하기 전에 먼저 assistant 본문 문장으로 URL과 코드를 노출해요. `실행됨 명령 N개`, `TaskOutput 사용함`, tool 카드, 접힌 로그만 남기고 응답을 끝내면 실패예요.
- 코드를 명령 출력이나 로그 읽기 결과 안에만 남기지 않아요. Claude Desktop 에서는 tool 출력이 접혀 보일 수 있으므로, 최종 요약이나 다음 안내 문장에도 사용자가 입력할 코드를 한 번 더 써요.
- 자동 브라우저 열기는 입력 코드가 포함된 직접 URL을 우선 열 수 있어요. 사용자가 "승인했어"라고 다시 말하게 하지 않아요. 코드 표시 뒤 assistant 응답을 끝내지 말고, `계정 인증 시작` command 뒤 같은 assistant turn 에서 title 과 description 이 모두 정확히 `인증 확인` 인 단일 `axhub github accounts list --json` 조회까지 이어가요. tenant 가 이미 명확할 때만 `axhub github accounts list --tenant <tenant> --json` 를 써요. CLI 가 pending 으로 끝나도 `sleep`, `&&`, shell loop, watcher 로 감시하지 말고 같은 단일 확인 조회로 연결 반영 여부를 확인해요.
- 브라우저가 성공 화면인데 계정 연결이 아직 반영되지 않았으면 같은 leaf 명령을 한 번만 더 실행할 수 있어요. 그래도 pending 이면 현재 결과와 다음에 확인할 명령을 짧게 안내하고 멈춰요. 승인 확인용 `while true ... accounts list ... sleep ...` 루프나 persistent monitor 는 쓰지 않아요. 승인했는데도 `not authenticated` 가 계속 반복되면 보고 있는 코드가 이미 죽은 경우가 흔해요 — 위 `--fresh` 재발급을 한 번 먼저 쓰고, 그래도 같으면 CLI 가 pending 연동 재개를 지원하지 않는 구버전일 수 있으니 update 스킬로 CLI 를 먼저 올린 뒤 다시 시도하도록 안내해요.
- device_code 같은 내부 교환용 값은 절대 쓰지 않아요. 사용자에게 필요한 건 verification URL 과 `user_code` 뿐이에요.

## 원칙

- **카탈로그 금지.** 이 문서에 작업→명령 매핑을 적지 않아요. CLI 가 릴리즈될 때마다 표면이 변하니 진실은 항상 라이브 `axhub --json-schema` (없으면 `axhub ... --help`) 예요. (본문의 `axhub env` 류는 절차 예시일 뿐 매핑이 아니에요.)
- **사용법 선숙지 강제 (--help gate).** 명령을 찾았다고 바로 실행하지 않아요. 실행 전 그 정확한 leaf 명령(서브커맨드 포함)의 `--help` 를 반드시 1회 읽어 사용법(positional 인자 순서·필수/선택 플래그·파괴적 실행 플래그)을 확정하고, 거기 나온 인자·플래그만 써요. 사용법을 안 읽은 명령은 실행 금지예요.
- **조회는 바로 실행, 파괴적 변경은 승인.** 목록·로그처럼 읽기 전용 명령은 확인 없이 실행해요. 삭제·롤백·force/yes/execute 같은 파괴적 플래그가 있으면 대화형 승인 1회를 받고, headless 에서는 preview/summary 로 멈춰요.
- **공개 표면만.** `axhub plugin-support ...` (hidden 그룹) 는 plugin 내부 프로토콜이라 이 스킬의 탐색·실행 대상이 아니에요.
- **내 접근 가능 범위는 grant 기준.** 사용자가 "내가 조회 가능한", "내가 접근 가능한", "connected 된", "연결된 connector"처럼 **본인 범위**를 물으면 반드시 공개 명령 `axhub connectors mine` 으로 확인해요. tenant-admin 전체 카탈로그인 `axhub connectors list` / `--enabled-only` 결과를 "내가 조회 가능한 connector" 로 요약하지 말아요.
- **읽기와 쓰기는 다른 질문이에요.** "내가 올릴 수 있어?", "쓰기 권한 있어?", "여기 파일 넣을 수 있어?" 처럼 **바꾸는 쪽**을 물으면 `axhub authz grants mine` 으로 확인해요. 응답의 `scope_resource_paths` 가 닿는 대상이고 `scope_levels` 가 대상별로 어디까지인지(`read` | `write`) 예요. 커넥터 목록만 보고 "접근 가능하니 올릴 수도 있다" 고 답하지 말아요 — 목록에 있어도 그 대상이 `read` 면 못 올려요. 반대로 `scope_levels` 가 비어 있으면 대상을 안 나눈 권한이라 프리셋이 정해요.
- **지어내지 않기.** 탐색으로 못 찾은 기능은 "axhub 에 그 기능은 없어요" 라고 정직하게 말하고, 가장 가까운 명령을 제안해요.

**대표 정직성 계약.** `clarity` 는 hidden `plugin-support` 를 탐색하지 않아요. 공개 `--json-schema` / `--help` 트리에서 맞는 leaf 를 찾지 못하면 존재하지 않는 명령을 만들지 말고, "axhub 에 그 기능은 없어요" + 가장 가까운 공개 명령만 말해요. 로그·환경변수·롤백·테이블·connector grant 처럼 대표 여정 뒤 작업은 이 경로로 이어가요.

## Anti-Patterns · 진행 알림 · TodoWrite

탐색·실행 단계에 들어가기 전에 [references/execution-guardrails.md](references/execution-guardrails.md) 를 읽고 anti-pattern 목록·진행 알림 형식·TodoWrite 규칙을 그대로 따라요. 핵심: Desktop-visible 명령은 단일 `axhub` 명령만(후처리·임시 파일 금지), 전체 schema 통째 읽기 금지, 담당 스킬 의도 가로채기 금지예요.

## Workflow

1. **첫 명령.** 제목·description 이 정확히 `명령 찾기`인 Step 3의 bare schema 탐색으로 시작해요. 그 결과가 command-not-found면 바로 넘기지 않고 AP-17 경로 계약대로 `"$HOME/.axhub/bin/axhub"` 로 `plugin-support repair-path --json` 을 한 번 실행해 그 절대경로로 이어가요. 디스크에도 없을 때만 "axhub CLI 가 아직 없네요. 온보딩부터 진행할게요"라고 말하고 onboarding으로 넘겨요.

2. **명시된 운영 작업 확인.** 사용자가 요청한 로그·환경변수·롤백·테이블/컬럼/데이터·connector grant·GitHub 재연결 작업의 핵심 동사·명사를 잡아요. 후보 leaf 가 여럿이면 한 번만 짧게 되물어요 — 단, 되묻기는 마지막 수단이고 대개는 다음 탐색으로 스스로 판별해요.

3. **탐색 (discover).** axhub 는 **에이전트용 기계가독 표면** `--json-schema` 를 제공해요 — `--help` prose 를 긁는 것보다 안정적이니 이걸 우선 써요. 단 전체 schema 는 ~270KB 라 **반드시 `--field-expr` 로 필요한 부분만 슬라이스**하고 통째로 읽지 않아요.

   ```bash
   # 하나로 좁혀진 후보는 leaf 부터
   axhub --json-schema --field-expr '.commands["<명사 후보>"]["<동사 후보>"]'
   # 실패하거나 불명확할 때만 parent → root
   axhub --json-schema --field-expr '.commands["<명사 후보>"]'
   axhub --json-schema --field-expr '.commands | keys[]'
   ```

   - **leaf-first 권한 UX.** leaf 결과에 `description`/`request`/`agent` 등 명령 정보가 있으면 parent/root 를 더 조회하지 않아요. null/empty/error 일 때만 한 단계씩 넓히며 같은 leaf 를 재조회하지 않아요.
   - 후보는 라이브 검증 전까지 추측일 뿐이에요. 작업→명령 매핑을 문서에 새기지 않아요.
   - `--json-schema` 가 없거나 비면(구 CLI) `--help` 트리로 폴백해요: `axhub --help` → `axhub <후보> --help` → 필요하면 더 깊이.
   - 후보가 여럿이면 description 으로 판별하고, 탐색 출력(schema/help 본문)은 chat 에 붙이지 않아요 — 판단 재료로만 써요.

3b. **사용법 선숙지 (--help gate) — 실행 전 필수, 건너뛰기 금지.** leaf 명령(서브커맨드까지)을 정했으면 조립·실행 전에 **그 정확한 명령의 `--help` 를 반드시 1회 읽어** 사용법을 숙지해요: positional 인자 순서, 필수/선택 플래그, 파괴적 실행 플래그(`--execute`/`--yes`/`--force`), 그리고 예시. 추측으로 인자를 조립해 바로 실행하지 않아요.

   ```bash
   # 고른 정확한 leaf 명령의 사용법 (서브커맨드 포함) — 실행 전 필수
   axhub <명령> <서브커맨드> --help
   ```

   - 여기서 확인한 인자·플래그만 Step 4 에서 써요. help 에 없는 플래그·인자는 지어내지 않아요.
   - `--help` 가 비거나 없으면(구 CLI) `axhub --json-schema --field-expr '.commands["<명령>"]'` 의 해당 서브커맨드 노드로 같은 정보(인자·플래그)를 확정해요.
   - help 본문은 chat 에 echo 하지 않아요 — 읽고 사용법만 내재화해요.

4. **실행 (execute).** Step 3b 사용법 확정을 통과한 명령만 조립해 실행해요 (사용법 미확인 명령 실행 금지).

   - 기계 파싱이 필요하면 `--json` (global flag) 을 붙여요.
- help 가 `--execute` / `--yes` / `--force` 같은 명시 실행 플래그를 요구하는 파괴적 명령이면 대화형에서 한 번 승인받은 뒤 붙여요. headless 에서는 붙이지 않고 preview/summary 로 멈춰요.
- 인자가 부족하면(앱 이름 등) 먼저 조회 명령으로 채울 수 있는지 시도하고, 정말 사용자만 아는 값일 때만 물어요.
- 앱을 가리키는 인자는 사용자가 아는 slug/name 을 우선 써요. 앞 단계 조회 결과에서 얻은 raw app id 를 다음 mutation 명령의 `--app` 값으로 넘기지 않아요. CLI 가 내부 id 를 반환해도 chat 에 raw 로 옮겨 적지 말고, 사용자에게는 "앱 이름 기준으로 실행했어요"처럼 요약해요.
- help 의 어떤 플래그가 **플러그인 자신의 설치 정보**를 요구하면, clarity 에서는 플러그인 캐시 파일을 읽어 채우지 않아요. 캐시 경로가 작업 디렉토리 밖이면 Claude Desktop 권한 팝업이 떠서 운영 브리지 흐름이 거칠어져요.

   ```bash
   axhub <명령> <인자...> --json
   ```

   실행도 allowlist 그대로 단일 `axhub` 명령이에요 — `mktemp`·redirect·임시 파일 없이 tool 결과(stdout/stderr)를 assistant 내부에서 읽고 아래 규칙대로 요약해요.

5. **결과 제시.** exit 0 이면 무엇이 어떻게 됐는지 한국어 한두 문장으로 요약해요 (URL·이름 같은 사용자 가치 정보만, 내부 id·raw JSON 생략). 비-0 이면:
   - 인증 계열(exit 4 등) → "axhub 로그인이 만료됐어요. 다시 로그인할까요?"
   - 사용법 오류(exit 64) → Step 3b 로 돌아가 그 명령의 `--help` 를 다시 읽고 인자를 고쳐 1회 재시도, 그래도 실패하면 정직하게 설명.
   - 그 외 → 원인을 한국어로 풀어 설명하고 다음 행동을 제안. raw stderr 는 노출하지 않아요.

6. **기능 부재 처리.** 두 단계 깊이까지 탐색해도 맞는 명령이 없으면: "axhub 에 그 기능은 없어요. 가장 가까운 건 `axhub <근접 명령>` 인데 이걸로 할까요?" — 존재하지 않는 명령을 만들어 실행하지 않아요.

## 경계

| 요청 | 담당 |
|---|---|
| 처음 셋업·CLI 설치·로그인부터 | onboarding |
| 빈 디렉토리 새 앱 만들기·템플릿·스캐폴드 | bootstrap |
| 기존 로컬 앱 첫 axhub 연결·첫 배포 가져오기 | import |
| 배포 실행·배포 검증 | deploy |
| 기존 앱 화면·페이지·대시보드·엔드포인트 코드 생성 | development |
| 배포 실패 원인 진단·해결 후보 요약 | diagnosis |
| CLI·플러그인 관리 | update |
| 로그·환경변수·롤백·테이블/컬럼/데이터·connector grant·GitHub 재연결 | **clarity (이 스킬)** |

경계가 섞여 오면(예: "배포하고 로그 봐줘") 배포는 deploy 에 양보하고 로그 부분만 이 스킬이 이어받아요.

여러 전용 경로가 섞인 요청에서는 clarity 가 앱 콘셉트, 이름, slug, 템플릿을 대신 묻거나 확정하지 않아요. 남은 새 앱 생성은 `새 앱 생성은 이어서 진행할게요.` 라고만 말한 뒤 bootstrap 으로 handoff 해요. bootstrap 이 `어떤 템플릿으로 시작할까요?` 와 `앱 이름을 무엇으로 할까요?` 카드를 묻게 해요.

특히 Claude Desktop 에서 다음처럼 보이면 clarity 실패예요:

- question 제목 `예약 사이트 컨셉`
- 질문 `새로 만들 예약 웹사이트, 어떤 컨셉으로 할까요?`
- 선택지 `네일샵 예약`, `사진관 예약`, `애견미용 예약`, `피아노 레슨 예약`

이 실패 형태가 떠오르면 해당 질문을 띄우지 말고 즉시 `새 앱 생성은 이어서 진행할게요.` 한 줄로 마무리한 뒤 bootstrap 을 호출해요. 추천 후보는 bootstrap 이 묻고 clarity 는 만들지 않아요.

## 다음 단계 이어주기

조회 결과가 앱 기능으로 이어질 만한 리소스(connector·table·데이터 카탈로그 등)면, 결과 요약 끝에 다음 단계를 한 줄로 권해요 — 예: "이 데이터로 화면을 만들래요? '이걸로 대시보드 만들어줘' 라고 하면 돼요." 순수 안내 문장이에요. 이때도 `axhub plugin-support` 같은 hidden 표면을 호출하거나 state 를 쓰지 않아요 — clarity 는 그대로 공개 표면만 탐색·실행하고, 실제 기능 코드는 development 가 같은 대화 맥락을 이어받아 처리해요.

## Visibility

- 탐색의 `--help` 호출·명령 본문·raw stdout/stderr 는 chat 에 echo 하지 않아요.
- 사용자에게 보이는 건 무엇을 했는지 한 줄 + 결과 요약 + (있으면) 다음 행동 제안 — 전부 해요체.
