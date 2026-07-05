---
name: bootstrap
description: 'Use this skill in an empty folder. Do not explain the skill match, do not mention axhub:bootstrap in chat, and start the visible response with a Korean progress sentence. Triggers: "새 앱 만들어줘", "앱 만들어줘", "초기화해줘", "Next.js 앱 만들어줘", "Please make my first app. I want a small gym class booking website and put it online", "Create a small bakery preorder web app and deploy it to the internet", "Build a cafe booking website and put it online", "Make a flower shop reservation app". Run the backend template picker, confirm the app name, check GitHub owner, preview, then create/deploy with axhub apps bootstrap. Use import instead for a non-empty existing local app.'
allows-dependency-execution: true
model: sonnet
---

# Bootstrap

## Fast Start

첫 visible 응답은 반드시 한국어 진행 문장으로 시작해요. 스킬 선택 이유, 빈 디렉토리 판단, route label, `axhub:bootstrap 스킬 호출한다` 같은 내부 라벨은 말하지 않아요. Claude Desktop 이 이미 `/axhub:bootstrap` native badge 를 보여줘도 chat 본문에서 반복하지 않아요. 금지문: `Using axhub:bootstrap skill`, `matches new app + deploy request`, `axhub의 새 앱 생성 스킬`, `스킬을 사용하겠습니다`.

사용자가 새 앱 생성과 배포 목표를 말하면 목표 승인은 받은 상태예요. execute 승인은 별도라서 아래 순서대로 CLI 확인과 템플릿 질문까지 바로 진행하고, `진행해줘라고 말해` 같은 일반 안내만 남기고 멈추지 않아요.

중단된 첫 턴 뒤에 다시 이어질 때도 CLI 명령이 하나도 실행되지 않았다면 fresh path 를 그대로 시작해요. 앱 생성이 아직 시작 안 됐다는 말만 하고 사용자에게 다시 시작 문구를 요구하지 않아요.

## Scope

이 스킬은 빈 디렉토리에서 새 axhub 템플릿 앱을 만들고 첫 배포까지 진행하는 전용 흐름이에요. 비어 있지 않은 기존 로컬 앱, 이미 만든 앱 가져오기, "이 폴더 올려줘" 요청은 `import` 스킬로 넘겨요.

creation path 는 backend `axhub apps bootstrap` saga 하나예요. `axhub init`, `axhub apps create`, `axhub deploy create` 로 우회하지 않아요.

같은 대화 맥락 이어받기는 이미 본 것만. infer-tables-env 분석은 scaffold 코드뿐 아니라 실제 조회 근거도 봐요. 리소스를 지어내지 않아요, carry-over 를 주장하지 않아요. install-link 를 보여줬으면 재안내는 생략, 0-install gate 는 맥락과 무관하게 그대로 실행해요.

## Reference Loading Policy

정상 fresh path 에서는 reference 파일을 읽지 않아요. 이 본문만으로 CLI guard, 작업공간 선택, template/app-name 질문, GitHub gate, dry-run preview, execute/status/verify/result 까지 진행해요. edge case 도 먼저 이 본문과 `axhub` CLI 상태 명령만 쓰고, plugin cache 파일 읽기를 복구 조건으로 삼지 않아요.

## Visibility

- 내부 라벨 노출 금지. `Folder near empty`, `Invoke axhub:bootstrap skill`, `Tenanting`, `Bootstraping`, `Idempotencying key`, `saga 실행`, `Saga 완료`, `GitHubed repo`, `DB 선언된 템플릿`, `development 단계` 는 chat/tool/progress/question 에 쓰지 않아요.
- Tool/Bash 제목은 사용자가 이해하는 한국어 명사구로만 쓰고 반드시 한글로 시작해요. 제품명·명령어·영어 단어에 `ing`/`ed` 를 붙인 제목, `raw`, `route`, `tenant`, `실행 중 명령`, `명령 실행` 은 쓰지 않아요.
- 가능한 제목: `작업공간 확인`, `CLI 준비 확인`, `앱 설정 확인`, `템플릿 목록 확인`, `저장소 계정 확인`, `앱 이름 확인`, `만들기 전 확인`, `앱 생성 진행`, `첫 배포 진행`, `배포 상태 확인`, `코드 가져오기`, `마무리 확인`, `인증 대기`.
- `rtk` 같은 Codex/개발자 전용 래퍼는 이 Claude Desktop skill 에서 절대 쓰지 않아요. `pwd`, `ls`, `find`, `cat`, `curl` 같은 generic shell probe 로 작업공간이나 실패 상태를 추측하지 말고 `axhub` CLI 표면만 써요.
- Desktop-visible command 는 한 tool call 에 하나의 직접 CLI 호출만 넣어요. 이미 고른 값은 shell 변수, `export`, command substitution, semicolon chain 으로 조립하지 말고 literal flag 로 넣어요. device flow 자동 브라우저 열기용 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1` prefix 만 execute/resume 명령에서 허용해요.
- 배포 상태 대기/확인도 예외가 아니에요. `for`, `while`, `sleep`, `grep`, `cut`, `awk`, `jq` 로 polling/파싱하지 않아요. 상태를 다시 볼 때마다 별도 tool call 로 `axhub deploy status <deployment-id> --tenant <tenant> --json` 한 명령만 실행해요. 성공/실패 판정은 shell text parsing 이 아니라 tool output JSON 을 읽어서 해요.
- Echo 금지: `schema_version`, template `id`, `folder_name`, `resource_tier`, `bootstrap_id`, `status_url`, `stage`, `app_id`, `deployment_id`, `error_code`, `error_message`, `request_id`, `idempotency_key`, `installation_id`, `device_code`.
- 예외: GitHub device-flow event 가 나오면 `verification_uri` 또는 `verification_uri_complete`, `user_code`, 대략적인 만료 시간은 즉시 humanize 해서 보여줘요.
- 공개 URL Markdown link 는 label 과 target 모두 확인된 `https://...` 절대 URL 을 그대로 써요. `[$PUBLIC_URL]($PUBLIC_URL)` 형태를 지켜요.
- 이 스킬은 CLI-only 흐름이에요. Claude Desktop 에 `App get (axhub)`, `App list`, deployment MCP, app connector 도구가 보여도 호출하지 않아요. 배포 상태·검증은 `axhub deploy status <deployment-id> --tenant <tenant> --json` 및 `axhub deploy verify <deployment-id> --json`, 앱 상세·URL 확인은 `axhub apps get <app-slug> --tenant <tenant> --json` 또는 `--field-expr` CLI 로만 해요. `Finding tools` 로 이동해서 MCP/App 도구를 찾지 않아요.

## Fresh Workflow

실제 순서:

1. CLI guard: `axhub plugin-support preflight --json`.
2. Resume/workspace: `axhub plugin-support init-resume route --json`, then `axhub plugin-support tenant-resolve --field-expr '.tenant // empty'`.
3. Template registry: `axhub apps templates list --tenant <tenant-slug> --json`.
4. Template picker: backend registry 에 있는 값만 고르고, native 질문 card 로 먼저 물어요.
5. App name: 앱 이름이 발화에서 유추되더라도 새 앱 생성에서는 한 번 확인해요.
6. GitHub App gate: `axhub github accounts list --json`.
7. Dry-run preview: `axhub apps bootstrap ... --dry-run --json`.
8. Preview confirmation: 사용자가 `진행`을 고른 뒤에만 execute 해요.
9. Execute saga: `AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap ... --execute --idempotency-key <literal>`.
10. Clone/current dir, manifest check, result/follow-up.

Slash command, skill name, route label 은 사용자에게 말하지 않아요.

### 1. CLI Guard

Tool 제목은 `CLI 준비 확인`을 써요.

```bash
axhub plugin-support preflight --json
```

command not found 이면 onboarding 안내 후 stop, `plugin-support` unknown/빈 출력이면 update 안내 후 stop, 정상 JSON 이면 계속해요. raw stderr 는 보여주지 않아요. shell 에서 CLI 버전 숫자를 직접 파싱·비교하지 않아요.

### 2. Resume And Workspace

Tool 제목은 `작업공간 확인` 또는 `앱 설정 확인`을 써요.

```bash
axhub plugin-support init-resume route --json
```

`watch_status` 또는 `resume_last` 이고 `clone_done=false` 면 이어서 할지 물어요. 새 폴더/새 앱 요청이거나 `새로 시작`을 고르면 이전 상태는 무시하고 template registry, template picker, app-name 질문으로 이어가요. route 가 fresh 이면 reference 를 읽지 않아요.

```bash
axhub plugin-support tenant-resolve --field-expr '.tenant // empty'
```

반환된 tenant slug 를 `--tenant <literal>` 로 넘겨요. 사용자에게는 tenant/테넌트라고 말하지 말고 `작업공간`이라고 말해요. 여러 작업공간이면 `새 앱을 어느 작업공간에 만들까요?`라고 물어요. 선택한 값은 `.axhub/state/tenant.json` 같은 로컬 파일로 저장하지 않아요.

### 3. Template Registry

Tool 제목은 `템플릿 목록 확인`을 써요.

```bash
axhub apps templates list --tenant test --json
```

실제 Desktop-visible command 에서는 확정된 tenant literal 로 바꿔요.

### 4. Template Picker

Claude Desktop 에서는 template 선택을 native Question/AskUserQuestion card 로 먼저 물어요. 제목은 `템플릿 선택`, 질문은 `어떤 템플릿으로 시작할까요?`; backend 실제 template 만 보여줘요. 3개 초과면 추천 3개만 올리고, card 가 렌더링되지 않거나 선택지가 보이지 않는 경우에만 일반 채팅 텍스트로 fallback 해요.

섞인 요청에서 update/clarity 가 먼저 처리됐어도 template 은 여기서만 확정해요. 이전 콘셉트·slug·이름 질문 답은 추천 힌트일 뿐이에요.

`웹앱`, `쇼핑몰`, `예약`, `preorder`, `booking`, `shop`, `dashboard`, `admin` 같은 일반 장르·기능 단어는 exact template 선택이 아니에요. 추천 순서를 정하는 근거일 뿐 선택 확정이 아니며, `--template ... --dry-run` 은 템플릿 질문 답변을 받은 뒤에만 실행해요.

사용자가 `추천해줘`, `알아서`, `best option`, `recommend the best option` 처럼 말해도 그 말은 추천을 원한다는 뜻이지 선택 확정이 아니에요. 1번 추천은 가능하지만 반드시 `어떤 템플릿으로 시작할까요?` 질문을 보여주고 답을 기다려요. 질문 뒤 `추천대로`, `1번`, template 이름 답변이면 확정해요.

### 5. App Name

앱 이름 질문 문구는 반드시 `앱 이름을 무엇으로 할까요?`; `앵 이름` 같은 오타나 줄임말을 쓰지 않아요. 표시 제목은 `앱 이름 확인`; 앱 이름 확인도 native Question/AskUserQuestion card 로 먼저 물어요. 답변 입력이 막힐 때만 일반 채팅 텍스트로 fallback. 사용자가 답한 뒤에만 `--name`/`--slug` 를 확정해요.

`추천 이름으로 해줘`, `알아서 이름 지어줘`, `use the recommended name` 은 앱 이름 질문이 먼저 보인 뒤의 답변일 때만 확정으로 봐요. 아직이면 추천 이름을 제안하고 `앱 이름을 무엇으로 할까요?` 로 확인해요.

다른 스킬/일반 채팅의 이름·slug 답은 추천 후보로만 쓰고, `앱 이름 확인` card 를 다시 보여줘요. 앱 이름 라벨은 오타·반말 없이 써요. 선택지 설명은 짧고 검수된 한국어만 써요: `기존 앱들과 겹치지 않는 새 콘셉트`, `예약 폼과 시간 선택에 적합`, `정적 페이지 중심이면 가까운 구조`.

repo name 과 subdomain 은 명시 입력이 없으면 app slug 로 맞춰요. dry-run 과 execute 모두 `--repo-name <app-slug>` 및 `--subdomain <app-slug>` 를 붙여요.

### 6. GitHub App Gate

Template 과 앱 이름이 사용자에게 확정되면 dry-run 직전에 저장소 계정 상태를 확인해요. 템플릿/앱 이름 질문보다 먼저 실행하지 않아요. Tool 제목은 `저장소 계정 확인`으로 써요.

```bash
axhub github accounts list --json
```

설치된 계정이 0개로 확인되면 설치가 확인될 때까지 dry-run/execute 로 진행하지 않아요. 설치된 계정이 1개면 자동 owner 로 쓰고, 2개 이상이면 owner 를 고르게 해요. 확인할 수 없는 상태는 막지 않고 진행해요. auth 에러는 `다시 로그인해줘` 로 안내해요.

### 7. Dry-Run Preview

```bash
axhub apps bootstrap --template nextjs-axhub --name bakery-preorder --slug bakery-preorder --repo-name bakery-preorder --subdomain bakery-preorder --github-owner realitsyourman --tenant test --dry-run --json
```

위 값들은 예시예요. 실제 실행 전에는 템플릿, 앱 이름, slug, GitHub owner, tenant 를 확정한 literal 값으로 바꿔요. Dry-run envelope 에서 template, slug, subdomain, repo name, private/public preview 만 한국어로 보여줘요. raw JSON/stderr 를 dump 하지 않아요.

미리보기 뒤 확인 필수. 사용자가 처음부터 "바로 올려줘", "배포까지 해줘"라고 말했어도 그 말은 목표이지 execute 승인 토큰이 아니에요. `--dry-run` preview 를 보여준 뒤 `진행`/`취소` 질문을 한 번 받고, 사용자가 `진행`을 고른 뒤에만 `--execute` 를 호출해요.

### 8. Execute Bootstrap Saga

idempotency key 는 OS별 UUID 생성 명령으로 만들지 말고 `axhub plugin-support init-resume put` 에 생성을 맡겨요.

```bash
axhub plugin-support init-resume put --template nextjs-axhub --app-name bakery-preorder --slug bakery-preorder --subdomain bakery-preorder --json
```

```bash
AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input apps bootstrap --template nextjs-axhub --name bakery-preorder --slug bakery-preorder --repo-name bakery-preorder --subdomain bakery-preorder --github-owner realitsyourman --tenant test --execute --idempotency-key 00000000-0000-4000-8000-000000000000
```

실행 때 예시 UUID 는 `init-resume put` 반환 literal UUID 로 바꿔요.

Execute/resume 명령에는 `--json` 금지.

`device_code_issued` event 는 `auto_poll:true` + `browser_opened:true` 여도 user code 를 즉시 보여줘요. Claude Desktop 이 output file 을 알려주면 그 파일을 즉시 읽어 `verification_uri` 와 `user_code` 를 보여줘요. 첫 execute/resume 에 `--watch`/`--watch-timeout` 을 붙이지 않아요. 사용자가 "승인했어"라고 다시 말하게 하지 않아요.

명령이 `device_flow_required_user_action` 으로 끝나도 거기서 멈추지 않아요. 링크/코드를 보여주고, 사용자에게 승인 완료를 채팅으로 알려 달라고 쓰지 않고 같은 turn 에서 resume 해요. 원래 execute background task 가 `auto_poll:true` 로 아직 돌고 있으면 중복 resume 을 실행하지 말고 같은 output/status 를 읽어 완료를 확인해요. `resume_command` 는 verbatim 실행 금지, `--json`/watch flag 제거.

### 8.1 Desktop Error Recovery

Claude Desktop 에서 `앱 생성 진행` 또는 `앱 생성 재시도` tool 이 `백그라운드 셸 실패` 로 끝나면 raw output 을 dump 하지 않아요. workspace 밖 plugin cache reference 읽기 권한 프롬프트가 뜨면 허용을 요구하지 말고 읽지 않아요.

복구 명령도 `rtk`, `curl`, `pwd`, `ls`, `find`, `cat` 같은 generic probe 로 빠지지 않아요. `axhub` CLI 상태 명령만 써요.

1. 출력에서 `bootstrap_id` 를 확인할 수 있으면 `axhub apps bootstrap-status <bootstrap-id> --tenant <tenant> --json`.
2. 출력에서 `deployment_id` 를 확인할 수 있으면 `axhub deploy status <deployment-id> --tenant <tenant> --json` 및 `axhub deploy verify <deployment-id> --json`.
3. 둘 다 없고 오류가 timeout/network 계열일 때만 같은 idempotency key 로 execute 를 한 번 재시도해요. 재시도는 최대 1회예요.
4. 나중에 상태 명령이 `succeeded` 를 반환하면 "앱 생성은 완료됐어요" 로 복구 보고하고, 새 앱을 다시 만들지 않아요.

### 9. Clone And Manifest

완료된 saga 의 `repo_full_name` 으로 사용자가 요청한 현재 폴더를 채워요. 값이 비면 임의 URL 을 만들지 않아요.

상단 폴더 표시가 사용자가 지정한 새 폴더와 다르면, 메시지의 absolute path 를 target 으로 써요. `pwd`, `ls`, `find`, `cat`, `rtk ls -la` probe 로 권한 프롬프트를 늘리지 않아요.

새 폴더에 `.omc/` 같은 Desktop 메타데이터가 있을 수 있으므로 `git clone ... .` 는 쓰지 않아요. `git init` 후 원격 `main` 을 받아 `.omc/` 를 보존해요.

clone/hydrate 명령 안에서는 raw `git`만 써요. `rtk git`, `grep`, `cut`, `awk`, `sed` 금지.

```bash
git -C <target> init -q -b main && (git -C <target> remote add origin https://github.com/<repo>.git 2>/dev/null || git -C <target> remote set-url origin https://github.com/<repo>.git) && git -C <target> fetch origin main --quiet --depth=1 && git -C <target> reset --hard FETCH_HEAD
```

실행 때 `<target>` 과 `<repo>` 은 확인된 literal 로 바꿔요. target 채운 뒤 추가 `rtk ls`, `ls`, `find`, `cat` 확인은 하지 않아요.

성공하면 추가 파일 읽기 없이 `axhub deploy --explain --json` 으로 check 해요. 정상 fresh path 에서 밖 reference 읽기 권한 프롬프트가 뜨면 허용을 요구하지 말고, 이미 확보한 repo/app/deployment 값과 CLI 명령으로만 마무리해요.

### 10. Result

공개 URL 은 절대 합성하지 않아요. 배포 성공 후 `axhub apps get <app-slug> --tenant <tenant> --json` 또는 `--field-expr` 로 `access_url`, `visibility`, `review_status` 를 확인해요. `App get (axhub)` 같은 Desktop/App/MCP 도구는 쓰지 않아요. `url_checked=false` 인 경우 URL 확인 증거를 보강해요. `visibility=private` 또는 `review_status=pending` 이면 친구에게 바로 공개됐다고 말하지 않아요.

사용자가 공개·누구나·친구에게 보여주기까지 원했으면 `axhub publish --app "$APP_SLUG" --visibility public --json` 으로 공개 신청을 넣고 `review_status=pending` 또는 review request id 를 알려줘요. 승인 전 공개 확대를 `axhub apps update --visibility public` 로 시도하지 않아요.

## Non-Interactive Defaults

Subprocess/CI/no TTY 에서는 사람 선택을 대신하지 않아요. Defaults: resume `새로 시작`; GitHub owner 는 `AXHUB_GITHUB_OWNER` 없으면 `취소`; template/app name 은 `abort`; execute 는 `취소`; local preview 는 `아니요`.

## NEVER

- NEVER GitHub App 미설치 상태에서 bootstrap dry-run/execute.
- NEVER `axhub init`, `axhub init --from-template`, `axhub apps create`, `axhub deploy create` 로 우회.
- NEVER remote `templates.json` / 폐기된 fetch-template 사용.
- NEVER subprocess/headless 에서 template/app name 임의 선택.
- NEVER `--execute` 를 `--dry-run` 미리보기와 사용자 확인 없이 호출.
- NEVER auth 만료를 template 조회 실패로 오해.
- NEVER GitHub device flow code 를 긴 watch tool 안에 숨긴 채 사용자를 빈 GitHub code 입력 화면에 남겨두지 않아요.
- NEVER `repo_full_name` 없이 임의 URL clone.
- NEVER shell 에서 CLI 버전 숫자를 직접 파싱·비교하지 않아요.
