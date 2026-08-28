---
name: bootstrap
description: 'Use this skill only in an empty folder to create a brand-new axhub template app. Strongly exclude existing/local-folder apps: never use for "기존 앱", "기존 Express 서버 앱", "이미 만든 앱", "작업 폴더는 /path", "이 폴더 axhub에 올려", "이 앱을 axhub에 올려", or any non-empty app source; use import instead. Also yield to scaffold whenever the user wants the repository under THEIR OWN account/org — "내 계정에 레포", "내 깃허브에", "우리/회사 조직(org)에 저장소" 처럼 저장소 소유를 지목하면 bootstrap 이 아니라 scaffold 예요 (bootstrap 은 axhub 이 저장소를 만들어 줘요). Positive triggers only when the user asks to create/start/init a new app: "새 앱 만들어줘", "앱 만들어줘", "초기화해줘", "Next.js 앱 만들어줘", "처음부터 앱 만들어줘", "진행 중이던 axhub 앱 만들기/배포 상태 이어서 확인". Do not explain the skill match, do not mention axhub:bootstrap in chat, and start the visible response with a Korean progress sentence.'
allows-dependency-execution: true
model: sonnet
---
> 이 본문이 중간에 끊겨 보이면 설치 경로의 이 SKILL.md 원문을 열어 전체 절차를 확인한 뒤 진행해요.


# Bootstrap

## Fast Start

첫 visible 응답은 반드시 한국어 진행 문장. 스킬 선택 이유, route label, `axhub:bootstrap 스킬 호출한다` 금지. Codex 이 이미 `/axhub:bootstrap` native badge 를 보여줘도 chat 본문에서 반복하지 않아요. 금지문: `Using axhub:bootstrap skill`, `matches new app + deploy request`, `axhub의 새 앱 생성 스킬`, `스킬을 사용하겠습니다`.

새 앱 생성과 배포 목표는 받은 상태예요. execute 승인은 별도라서 아래 순서대로 CLI 확인과 템플릿 질문까지 바로 진행하고, `진행해줘라고 말해` 같은 일반 안내만 남기고 멈추지 않아요.
저장소 인증보다 backend 판정이 먼저예요. resume/existing app은 `axhub apps get <app> --json`, fresh app은 `axhub apps git-backend --tenant <tenant> --json`의 top-level `git_backend`만 써요. `git_backend.backend=selfhosted`면 device flow·GitHub App 설치·owner 질문을 모두 건너뛰고 owner flag 없이 bootstrap해요. GitHub·`legacy_github`만 기존 gate를 유지해요.


중단 뒤 이어질 때도 CLI 명령이 하나도 실행되지 않았다면 fresh path 를 그대로 시작해요. 다시 시작 문구를 요구하지 않아요.

`If you need an app name or template, choose a reasonable one yourself`, `choose whatever is reasonable`, `pick the best template/name`, `deploy it for real`, `put it online`, `do it for real` 는 목표·추천 허용일 뿐이에요. 템플릿 선택 카드, 앱 이름 확인 카드, dry-run 뒤 `진행` 확인은 절대 생략하지 않아요.

## 승인 게이트 계약 (요약)

codex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 아래 네 게이트는 뒤쪽 절차 설명이 잘려도 그대로 지켜요.

1. 템플릿 선택 — `어떤 템플릿으로 시작할까요?` 로 묻고, backend 가 실제로 가진 template 만 보여줘요.
2. 앱 이름 확인 — `앱 이름을 무엇으로 할까요?` 로 묻고, 답을 받은 뒤에만 `--name`/`--slug` 를 확정해요.
3. 생성·배포 승인 — dry-run preview 를 보여준 뒤 `지금 만들고 배포까지 진행할까요?` 로 묻고, 승인을 받은 뒤에만 `--execute` 를 실행해요.
4. 승인 방식 — 같은 확인을 명시 텍스트 승인 1회로 받아요. preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.
5. 빈 답변 = 미승인 — 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.

**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.

## Scope

빈 디렉토리 axhub 템플릿 앱 + 첫 배포 전용이에요. axhub 맥락 없는 일반 앱 작업은 맡지 않아요. 비어 있지 않은 기존 로컬 앱·가져오기·"이 폴더 올려줘" 요청은 `import` 스킬로 넘겨요.

사용자 발화에 `기존`, `이미 만든`, `작업 폴더`, `이 폴더`, `Express`, `Fastify`, `Nest`, `FastAPI`, `Flask`, `Django`, `Rails`, `Go 서버`, `Dockerfile` 처럼 기존 소스가 있음을 뜻하는 단서와 axhub 배포 의도가 함께 있으면 bootstrap 을 시작하지 않아요. CLI guard, preflight, 템플릿 목록 확인을 실행하기 전에 즉시 import 경계로 양보하고, chat 본문에서 `/axhub:bootstrap` 또는 bootstrap 선택 이유를 설명하지 않아요.

creation path는 `axhub apps bootstrap` saga 하나뿐이에요. GitHub 차단 시 로컬 소스 배포만 예외예요. 사용자 계정/조직 소유 저장소 요청은 scaffold로 양보해요. 같은 대화 맥락 이어받기: 이미 본 것만. infer-tables-env 분석은 scaffold 코드뿐 아니라 실제 조회 근거도 봐요. 리소스를 지어내지 않아요; carry-over 를 주장하지 않아요. GitHub branch에서 install-link 를 보여줬으면 재안내는 생략하지만 0-install gate 는 항상 실행해요.

## Reference Loading Policy

정상 fresh path 에서는 reference 파일을 읽지 않아요. 이 본문만으로 CLI guard, 작업공간 선택, template/app-name 질문, backend gate, backend별 preview와 execute/status/verify/result 까지 진행해요. plugin cache 파일 읽기를 복구 조건으로 삼지 않아요.

## Visibility

- 내부 라벨 노출 금지. `Folder near empty`, `Invoke axhub:bootstrap skill`, `Tenanting`, `Bootstraping`, `Idempotencying key`, `saga 실행`, `Saga 완료`, `GitHubed repo`, `DB 선언된 템플릿`, `axhub:bootstrap 스킬 호출한다`, `development 단계` 는 chat/tool/progress/question 금지.
- Tool/Bash 제목은 한국어 명사구로 쓰고 반드시 한글로 시작해요. 제품명·명령어·영어 단어에 `ing`/`ed` 를 붙인 제목, `실행 중 명령`, `명령 실행` 금지.
- 제목: `CLI 준비 확인`, `작업공간 확인`, `앱 설정 확인`, `템플릿 목록 확인`, `저장소 계정 확인`, `앱 이름 확인`, `앱 주소 확인`, `앱 생성 미리보기`, `계정 인증 시작`, `인증 확인`, `앱 생성 상태 확인`, `배포 상태 확인`, `검증 확인`.
- `rtk` 같은 개발자 전용 CLI 래퍼는 이 skill 에서 절대 쓰지 않아요. `pwd`, `ls`, `find`, `cat`, `curl` 같은 generic shell probe 대신 `axhub` CLI 표면만 써요.
- Desktop-visible command 는 한 tool call 에 하나의 직접 CLI 호출만 넣어요. 이미 고른 값은 shell 변수, `export`, command substitution, semicolon chain 없이 literal flag 로 넣어요. execute/resume 명령에는 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1` 을 붙이지 않아요 — 이 prefix 가 붙으면 CLI 가 코드를 돌려주는 대신 블로킹 폴링으로 들어가서 tool call 이 끝나지 않고, 사용자는 빈 GitHub 코드 화면만 봐요. 이 prefix 는 즉시 끝나는 `github link` fast path 에서만 써요.
- 배포 상태 대기/확인도 예외가 아니에요: 백그라운드 감시와 `for`, `while`, `until`, `sleep`, `grep`, `head`, `tail`, `cut`, `awk`, `sed`, `jq` polling/파싱 금지. CLI 내부 대기인 `deploy verify --wait` 는 이 금지에 걸리지 않고 오히려 우선이에요. fallback 으로 상태를 다시 볼 때는 별도 tool call 로 `axhub deploy status <deployment-id> --tenant <tenant> --json` 한 명령만 실행. `until axhub ... | grep ...`, `axhub ... | head ...` 같은 권한 요청창이 뜨는 긴 shell watch 는 UX 실패예요. 성공/실패 판정은 shell text parsing 이 아니라 tool output JSON 을 읽어서 해요. deployment id 를 알면 terminal/verify 완료 전 응답을 끝내지 않아요. 단, 상태 확인 tool call 의 폴링 예산은 최대 30회 또는 10분(AP-16)이에요 — 예산에 먼저 닿으면 실패 선언 없이 `아직 진행 중이에요` 와 재개 명령(`axhub deploy status <deployment-id> --tenant <tenant> --json`)을 남기는 재개 요약으로 응답을 끝내고 deployment id 를 보존해요. 이 예산 종료가 앞 규칙의 유일한 예외예요.
- Echo 금지: `bootstrap_id`, `deployment_id`, `idempotency_key`, `device_code`.
- 사용자에게 보이는 모든 URL 은 평문 `https://...` 절대 URL; Markdown URL 링크 문법은 전부 금지; 도메인-only target 금지: `[https://x](https://x)`, `[열기](https://x)`, `<https://x>`.
- GitHub device flow 는 평문 URL+코드를 본문에 다시 써요: `https://github.com/login/device`, `ABCD-1234`. Markdown 링크(`[https://...](github.com/...)` 포함), 백그라운드 감시, `읽는 중 <output>`, 임시 출력 파일 읽기 카드로 코드 노출 금지. 승인 확인은 별도 `axhub` leaf 명령으로 이어가요.
- 이 스킬은 CLI-only 흐름이에요. `App get (axhub)`, `App list`, deployment MCP 호출 금지. 상태·검증은 CLI 명령만, 앱 상세·URL 확인은 `axhub apps get <app-slug> --tenant <tenant> --json` 또는 `--field-expr`. `Finding tools` 로 이동해서 MCP/App 도구를 찾지 않아요.
- 다른 플러그인/워크플로 상태를 정리하지 않아요. 외부 자동화·취소·state 정리 도구를 부르지 않고, chat/tool/progress 에 다른 플러그인 이름, 자동화 정리 문구, `Finding tools` 를 쓰지 않아요.

## Fresh Workflow

실제 순서:

1. CLI guard: `axhub plugin-support preflight --json`.
2. Resume/workspace: `axhub plugin-support init-resume route --json`, then `axhub plugin-support tenant-resolve --field-expr '.tenant // empty'`.
3. Template registry: `axhub apps templates list --tenant <tenant-slug> --json`.
4. Template picker: backend registry 에 있는 값만 고르고, native 질문 card 로 먼저 물어요.
5. App name: 앱 이름이 발화에서 유추되더라도 새 앱 생성에서는 한 번 확인해요.
6. Git backend 판정 뒤 GitHub App gate: 기존/resume 앱은 `axhub apps get <app> --json`, fresh 앱은 `axhub apps git-backend --tenant <tenant> --json`으로 먼저 판정해요. selfhosted는 GitHub App gate를 건너뛰고, GitHub만 `axhub github accounts list --json`로 기존 경로를 이어가요.
7. Availability check: `axhub apps check-availability --tenant <tenant> --slug <app-slug> --subdomain <app-slug> --json`.
8. Dry-run preview: backend에 맞는 `axhub apps bootstrap ... --dry-run --json`.
9. Preview confirmation: 사용자가 `진행`을 고른 뒤에만 execute 해요.
10. Execute saga: backend에 맞는 `axhub --no-input apps bootstrap ... --execute --idempotency-key <literal>`.
11. Clone/current dir, result.

Slash command, skill name, route label 은 사용자에게 말하지 않아요.

### 1. CLI Guard

Tool 제목은 `CLI 준비 확인`을 써요.

```bash
axhub plugin-support preflight --json
```

preflight의 `capabilities.self_hosted_git.apps_git_backend`와 `capabilities.self_hosted_git.app_git_backend`가 모두 `true`여야 backend 판정을 시작해요. 누락·malformed/false면 GitHub 질문과 mutation 없이 `axhub CLI를 최신 버전으로 업데이트해 주세요.`라고 안내하고 멈춰요.

> **CLI 경로 계약 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 반환된 `bin_path` 절대경로로 이 세션을 이어가요. 셋 다 없을 때만 onboarding 을 안내해요.

command not found 여도 곧바로 onboarding 으로 돌리지 않아요 — 낡은 PATH 때문에 설치된 CLI 를 못 찾는 상태가 macOS 에서도 흔해요. 같은 제목으로 `"$HOME/.axhub/bin/axhub" plugin-support repair-path --json` 을 한 번 더 실행하고, 나온 `bin_path`(없으면 그 canonical 경로)로 preflight 부터 다시 실행해요. 그 명령까지 파일 없음이면 onboarding 안내 후 stop. `plugin-support` unknown/빈 출력이면 update 안내 후 stop, 정상 JSON 이면 계속해요. raw stderr 는 보여주지 않아요. shell 에서 CLI 버전 숫자를 직접 파싱·비교하지 않아요.

### 2. Resume And Workspace

Tool 제목은 `작업공간 확인` 또는 `앱 설정 확인`을 써요.

```bash
axhub plugin-support init-resume route --json
```
`watch_status`/`resume_last`이면 emitted command 전에 `axhub apps get <app> --json`을 실행해요. missing/malformed면 멈추고, selfhosted면 provider-auth resume을 버려 ownerless status/resume으로 이어가요. GitHub/`legacy_github`만 emitted provider-auth command를 써요.


`watch_status` 또는 `resume_last` 이고 `clone_done=false` 면 이어서 할지 물어요. 새 폴더/새 앱 요청이거나 `새로 시작`이면 이전 상태를 무시하고 template 질문으로 이어가요. fresh 이면 reference 를 읽지 않아요.

```bash
axhub plugin-support tenant-resolve --field-expr '.tenant // empty'
```

반환된 tenant slug 는 `--tenant <literal>` 로 넘겨요. 사용자에게는 tenant/테넌트라고 말하지 말고 `작업공간`이라고 말해요. 여러 작업공간이면 `새 앱을 어느 작업공간에 만들까요?`라고 물어요. 선택한 값은 `.axhub/state/tenant.json` 같은 로컬 파일로 저장하지 않아요.

### 3. Template Registry

Tool 제목은 `템플릿 목록 확인`을 써요.

```bash
axhub apps templates list --tenant test --json
```

실제 Desktop-visible command 에서는 확정된 tenant literal 로 바꿔요.

### 4. Template Picker

Codex 에서는 template 선택을 native Question/명시 텍스트 승인 card 로 먼저 물어요. 제목 `템플릿 선택`, 질문 `어떤 템플릿으로 시작할까요?`; backend 실제 template 만 보여줘요. 3개 초과면 추천 3개, card 가 렌더링되지 않거나 선택지가 보이지 않는 경우에만 chat fallback.

섞인 요청에서 update/clarity 가 먼저 처리됐어도 template 은 여기서만 확정해요. 이전 콘셉트·slug·이름 질문 답은 추천 힌트일 뿐이에요.

`웹앱`, `쇼핑몰`, `예약`, `preorder`, `booking`, `shop`, `dashboard`, `admin` 같은 일반 장르·기능 단어는 exact template 선택이 아니에요. 추천 순서를 정하는 근거일 뿐 선택 확정이 아니며, `--template ... --dry-run` 은 템플릿 질문 답변을 받은 뒤에만 실행해요.

`추천해줘`, `알아서`, `best option`, `recommend the best option` 처럼 말해도 그 말은 추천을 원한다는 뜻이지 선택 확정이 아니에요. 1번 추천은 가능하지만 반드시 `어떤 템플릿으로 시작할까요?` 질문을 보여주고 답을 기다려요. 질문 뒤 `추천대로`, `1번`, template 이름 답변이면 확정.

### 5. App Name

앱 이름 질문 문구는 반드시 `앱 이름을 무엇으로 할까요?`; `앵 이름` 같은 오타나 줄임말을 쓰지 않아요. 표시 제목은 `앱 이름 확인`; 앱 이름 확인도 native Question/명시 텍스트 승인 card 로 먼저 물어요. 답변 입력이 막힐 때만 일반 채팅 텍스트로 fallback. 사용자가 답한 뒤에만 `--name`/`--slug` 를 확정해요.

`추천 이름으로 해줘`, `알아서 이름 지어줘`, `use the recommended name` 은 앱 이름 질문이 먼저 보인 뒤의 답변일 때만 확정으로 봐요. 아직이면 추천 이름을 제안하고 `앱 이름을 무엇으로 할까요?` 로 확인해요.

다른 스킬/일반 채팅의 이름·slug 답은 추천 후보로만 쓰고 `앱 이름 확인` card 를 다시 보여줘요. 선택지 설명은 짧고 검수된 한국어만 써요: `기존 앱들과 겹치지 않는 새 콘셉트`, `예약 폼과 시간 선택에 적합`, `정적 페이지 중심이면 가까운 구조`, `가볍고 빠른 시작`, `진행하지 않고 멈춰요`. 오타·비문 금지.

repo name 과 subdomain 은 명시 입력이 없으면 app slug 로 맞춰요. dry-run 과 execute 모두 `--repo-name <app-slug>` 및 `--subdomain <app-slug>` 를 붙여요.

### 6. Git Backend Gate

provider 대사 전에 backend를 확정해요. resume/기존 app은 첫 명령, app row 없는 fresh path는 둘째 read-only 명령을 써요. app row를 먼저 만들지 않아요.

```bash
axhub apps get <app> --json
axhub apps git-backend --tenant <tenant> --json
```

top-level `git_backend.backend`와 `git_backend.source`만 읽어요. app source는 `tenant_default|app_override|legacy_github`, tenant 응답 source는 `tenant|platform_default`예요. Gitea/C1/remote는 보지 않고 read 실패·malformed면 provider 질문·mutation 전에 멈춰요.

`git_backend.backend=selfhosted`이면 두 source 모두 인증 branch를 건너뛰어요. `references/templates-and-github.md` 전체를 읽지 않아요. selfhosted 사용자-facing 대사는 `저장소는 axhub에서 준비할게요.`만 허용하며 계정 인증·저장소 App 설치 질문을 0회로 유지해요.

`git_backend.backend=github` 또는 `git_backend.source=legacy_github`이면 기존 GitHub App gate를 실행해요. `references/templates-and-github.md`의 owner picker와 `GitHub App 설치를 끝냈을까요?`를 유지해요.

```bash
axhub github accounts list --json
```

이 조회가 정상 응답하면 계정이 이미 연동된 상태라 **인증 단계가 없어요**. 계정·설치는 CLI가 정하니 device flow 를 미리 시작하지 않아요. 설치 계정 0개면 설치 확인 전 dry-run/execute 금지. 1개면 자동 owner, 2개 이상이면 고르게 해요. `github_relogin_required` 는 연동이 없거나 만료된 상태라 재연동으로 풀고, 9단계 device flow 안무는 이 fallback 전용이에요. 재연동까지 막히면 **12단계**로 가요.

### 7. Availability Check

Tool 제목은 `앱 주소 확인`을 써요.

```bash
axhub apps check-availability --tenant <tenant> --slug <app-slug> --subdomain <app-slug> --json
```

이 단계는 dry-run preview 전 pre-preview guard 예요. slug 또는 subdomain 중 하나라도 unavailable 이면 bootstrap dry-run/execute 금지. 다른 앱 이름을 고르게 하고, 새 slug/subdomain 으로 availability check 부터 다시 실행해요. execute 실패 뒤 복구로 처리하지 않아요.

### 8. Dry-Run Preview

selfhosted는 owner flag 없이 실행하고, GitHub만 확인된 owner를 붙여요.

```bash
axhub apps bootstrap --template nextjs-axhub --name bakery-preorder --slug bakery-preorder --repo-name bakery-preorder --subdomain bakery-preorder --tenant test --dry-run --json
axhub apps bootstrap --template nextjs-axhub --name bakery-preorder --slug bakery-preorder --repo-name bakery-preorder --subdomain bakery-preorder --github-owner realitsyourman --tenant test --dry-run --json
```

첫 명령은 selfhosted, 둘째는 GitHub예요. 확정 literal로 바꾸고 preview의 template·slug·subdomain·repo·visibility만 한국어로 보여주며 raw JSON/stderr는 숨겨요.

미리보기 뒤 확인 필수. 처음부터 "바로 올려줘", "배포까지 해줘", `deploy it for real`이라고 말했어도 그 말은 목표이지 execute 승인 토큰이 아니에요. 추천 허용일 뿐이에요. `--dry-run` preview 뒤 axhub 진입 확인: 정확히 `지금 만들고 배포까지 진행할까요?` 질문과 `진행`/`취소` 선택지를 보여줘요. 질문·선택지·설명은 의역하거나 새로 만들지 않아요. 사용자가 `진행`을 고른 뒤에만 `--execute` 를 호출해요. 명시 텍스트 승인 1회만 받아요. 승인 채널 없는 headless 에서는 execute하지 않아요 — 승인을 조용히 건너뛰지 않아요.

### 9. Execute Bootstrap Saga

idempotency key 는 OS별 UUID 생성 명령으로 만들지 말고 `axhub plugin-support init-resume put` 에 생성을 맡겨요.

```bash
axhub plugin-support init-resume put --template nextjs-axhub --app-name bakery-preorder --slug bakery-preorder --subdomain bakery-preorder --json
```

```bash
axhub --no-input apps bootstrap --template nextjs-axhub --name bakery-preorder --slug bakery-preorder --repo-name bakery-preorder --subdomain bakery-preorder --tenant test --execute --idempotency-key 00000000-0000-4000-8000-000000000000
axhub --no-input apps bootstrap --template nextjs-axhub --name bakery-preorder --slug bakery-preorder --repo-name bakery-preorder --subdomain bakery-preorder --github-owner realitsyourman --tenant test --execute --idempotency-key 00000000-0000-4000-8000-000000000000
```

첫 명령은 selfhosted, 둘째는 GitHub 예시예요. 실행 때 예시 UUID는 `init-resume put` 반환 literal UUID로 바꾸고 Execute/resume 명령에는 `--json`을 붙이지 않아요.

아래 계정 연동/device 처리는 GitHub branch 전용이에요. selfhosted에서는 시작하지 않아요. 연동된 계정이면 execute 는 device flow 없이 끝나요. 첫 execute/resume 에 `--watch`/`--watch-timeout` 금지예요. CLI 가 pending 으로 끝나면 URL·코드를 본문에 쓰고, `device_flow_required_user_action` 에서 멈추거나 사용자에게 승인 완료를 채팅으로 알려 달라고 쓰지 않아요. `인증 확인` 제목의 단일 `axhub github accounts list --tenant <tenant> --json` 또는 watch flag를 제거한 resume 명령으로 승인 반영을 확인해요.

본문 두 줄 형식(URL 부분만 inline code span):

인증 URL: `https://github.com/login/device`
입력 코드: <USER_CODE>

**코드가 안 보인 채 execute 가 끝났을 때 (AP-18).** 출력 없이 실패·거부·중단됐는데 브라우저 device 화면이 열렸으면 같은 `--execute` 를 절대 다시 실행하지 않아요 — 새 device code 를 발급해 사용자가 보고 있는 코드를 무효로 만들어요. 즉시 끝나는 `AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link`(제목 `계정 인증 시작`)로 코드를 받아 위 두 줄을 본문에 먼저 쓰고, 같은 turn 에 `인증 확인` 단일 `axhub github accounts list --json` 뒤 `--resume-last` 로 이어가요. 코드를 보여주기 전에 `인증 확인` 을 먼저 실행하거나, 코드만 보여주고 응답을 끝내면 실패예요. 단 그 코드를 놓쳤거나 만료됐으면 fast path 를 반복하지 않고 `github link --fresh` 로 새 코드를 받아 두 줄을 다시 써요 — 저장된 pending link 는 죽은 코드를 그대로 돌려줘요. 유효한 코드가 화면에 있으면 붙이지 않아요(무효가 돼요).

execute/status 가 `deployment_id` 와 빌드/실행중 상태를 주면, preflight 의 `capabilities.import.verify_wait` 가 true 일 때 **권한 카드 한 번으로 끝나는** `axhub deploy verify <deployment-id> --app <app> --wait --wait-interval 20s --wait-timeout 10m --json` 를 정확히 한 번 호출해 terminal 까지 봐요. `--wait` 가 성공·실패·예산 제한(최대 30회 또는 10분, AP-16)까지 책임지므로 같은 verify 나 `배포 상태 확인` 을 연달아 호출하지 않아요 — 대기 수단 없이 같은 exit 6 을 화면에 쌓는 연타 폴링은 UX 실패예요. capability 가 없는 구 CLI 에서만 `배포 상태 확인` tool call 을 폴링 예산 안에서 반복하고 성공 뒤 `axhub deploy verify <deployment-id> --app <app> --json` 를 실행해요. 예산에 닿으면 재개 요약으로 끝내요. verify 성공 전 최종 성공 문구 금지, `잠시 후 확인해보세요` 로 끝내기 금지.

### 9.1 Desktop Error Recovery

Codex 에서 `앱 생성 진행`/`앱 생성 재시도` tool 이 `백그라운드 셸 실패` 로 끝나면 raw output dump 금지. workspace 밖 plugin cache reference 읽기 권한 프롬프트는 허용 요구 없이 생략. 복구 명령도 `rtk`, `curl`, `pwd`, `ls`, `find`, `cat` 같은 generic probe 로 빠지지 않아요. `axhub` CLI 상태 명령만 써요.

1. 출력에서 `bootstrap_id` 를 확인할 수 있으면 `axhub apps bootstrap-status <bootstrap-id> --tenant <tenant> --json`.
2. 출력에서 `deployment_id` 를 확인할 수 있으면 `axhub deploy status <deployment-id> --tenant <tenant> --json` 및 `axhub deploy verify <deployment-id> --app <app> --json`.
3. 둘 다 없고 오류가 timeout/network 계열일 때만 같은 idempotency key 로 execute 를 한 번 재시도해요. 재시도는 최대 1회예요. 단 device flow 가 시작된 흔적(브라우저 device 화면, `device_code_issued`, 사용자가 코드 입력 화면을 봤다는 말)이 있으면 이 재시도를 쓰지 않아요 — AP-18 대로 `github link` fast path 로 코드를 먼저 보여주고 `--resume-last` 로만 이어가요.
4. 나중에 상태 명령이 `succeeded` 를 반환하면 "앱 생성은 완료됐어요" 로 복구 보고하고, 새 앱을 다시 만들지 않아요.
5. 실패 원인이 GitHub(권한·정책·미연동·device flow) 이면 여기서 멈추지 말고 **12단계**로 가요. org owner 를 바꾸거나 개인 계정으로 만들고 나중에 transfer 하라는 제안은 하지 않아요.

### 10. Clone And Manifest

selfhosted는 Gitea API·remote URL을 추측하지 않고 선택한 literal tenant를 두 명령에 그대로 써요.

```bash
axhub --tenant <tenant> git setup --json
axhub --tenant <tenant> repo clone <app-slug> --json
```

`data.destination`을 모든 repository-local 후속 tool call의 `cwd`로 고정해요. 원래 CWD를 쓰거나 값 누락을 추측하면 실패예요.

GitHub는 `repo_full_name` hydrate를 유지해요. 새 폴더의 `.omc/` 같은 Desktop 메타데이터 때문에 `git clone ... .` 는 쓰지 않아요. clone/hydrate 명령 안에서는 raw `git`만 써요. `rtk git`, `grep`, `cut`, `awk`, `sed` 금지.

```bash
git -C <target> init -q -b main && (git -C <target> remote add origin https://github.com/<repo>.git 2>/dev/null || git -C <target> remote set-url origin https://github.com/<repo>.git) && git -C <target> fetch origin main --quiet --depth=1 && git -C <target> reset --hard FETCH_HEAD
```

`<target>`/`<repo>`은 literal이에요. target 채운 뒤 추가 `rtk ls`, `ls`, `find`, `cat` probe 금지. GitHub fetch 권한 실패는 **12단계**로 가요.

성공하면 확정한 repository `cwd`에서 `axhub deploy --explain --json`을 실행해요. 외부 reference 권한은 요구하지 말고 확보한 값·CLI로 마무리해요.

### 11. Result

공개 URL 은 합성하지 않아요. 배포 성공 후 `axhub apps get <app-slug> --tenant <tenant> --json` 또는 `--field-expr` 로 `access_url`, `visibility`, `review_status` 를 확인해요. `App get (axhub)` 같은 Desktop/App/MCP 도구는 쓰지 않아요. `url_checked=false` 면 URL 확인 증거를 보강해요. `visibility=private` 또는 `review_status=pending` 이면 친구에게 바로 공개됐다고 말하지 않아요.

공개·누구나·친구에게 보여주기까지 원했으면 `axhub publish --app "$APP_SLUG" --visibility public --execute --json` 으로 공개 신청하고 `review_status=pending` 또는 review request id 를 알려줘요. publish dry-run 을 먼저 호출하지 않고, chat 에 `Dry-run 기본값` 같은 내부 CLI dry-run semantics 를 설명하지 않아요. 승인 전 공개 확대를 `axhub apps update --visibility public` 로 시도하지 않아요.

### 12. GitHub 이 막혔을 때 — 로컬 소스로 배포

어느 지점이든 GitHub 때문에 못 나가면 빈손으로 돌려보내지 않아요. 일시적 실패
(네트워크·타임아웃·5xx)는 그 단계를 한 번만 재시도하고, 그래도 막히면 **묻지 않고**
아래를 그대로 해요. 이 절차는 본문만으로 완결돼요 — reference 는 plugin cache 라
읽기 권한 프롬프트가 뜨므로, 읽히면 좋고 못 읽어도 여기서 끝냅니다.

1. 한 줄 알리고 바로 진행: `GitHub 연결에 실패해서 우선 로컬 소스로 배포할게요. GitHub 는 이후 다시 연결할 수 있어요` (저장소는 있는데 권한만 없으면 `권한이 풀리면 평소대로 push 배포를 쓸 수 있어요`)
2. 앱이 이미 있는지 확인해요 — saga 가 조금이라도 돌았으면 앱만 만들어졌을 수 있어요. `axhub apps bootstrap-status <bootstrap-id> --tenant <tenant> --json` 또는 `axhub apps get <app-slug> --tenant <tenant> --json`. 있으면 3 을 건너뛰고 slug 는 확인된 값을 써요.
3. 앱이 없으면 주소 확인(7단계) 후 만들어요 — `axhub apps create --tenant <tenant> --name <이름> --slug <slug> --subdomain <slug> --deploy-method docker --resource-tier XS`
4. 코드가 폴더에 없으면 공개 템플릿에서 받아요(인증 불필요) — `git clone --depth 1 --branch main https://github.com/jocoding-ax-partners/axhub-template.git <target>/.axhub-template` → `cp -R <target>/.axhub-template/<template-id>/. <target>/` → `rm -rf <target>/.axhub-template`. 지우는 경로는 정확히 그 임시 폴더 하나뿐이에요.
5. **placeholder 치환 (템플릿을 받았을 때 필수).** 정상 bootstrap 은 서버가 push 전에 치환하지만 이 갈래는 그 단계가 없어요. 건너뛰면 앱이 `'{{API_BASE}}'` 라는 글자 그대로 API 를 불러 **로그인·데이터 연동만 조용히 죽어요**(화면은 떠요). `grep -rl '{{' <target>` 로 찾은 파일들의 6개 토큰을 전부 바꿔요 — `{{APP_SLUG}}`→slug · `{{APP_SUBDOMAIN}}`→subdomain · `{{APP_NAME}}`→앱 이름 · `{{TENANT}}`→테넌트 slug · `{{API_BASE}}`→CLI 가 쓰는 API 주소(prod `https://axhub.ai`) · `{{APP_ORIGIN}}`→`apps get` 의 `access_url`. `axhub.yaml` 의 `name:` 도 앱 slug 로 바꿔요.
6. 소스 폴더 안에서 배포 — `axhub up --app <slug> --execute` (CLI 0.29.0+, `--execute` 없으면 미리보기만)
7. 결과 확인은 11단계 그대로. clone 단계는 건너뛴 상태예요.

상세·주의점은 [`references/github-blocked-local-deploy.md`](references/github-blocked-local-deploy.md)
에 있어요(compose 는 루트 `Dockerfile` 이 같이 있으면 docker 로 해석되는 함정 포함).

## NEVER

- NEVER GitHub App 미설치 상태에서 bootstrap dry-run/execute.
- NEVER `axhub init`, `axhub init --from-template`, `axhub apps create`, `axhub deploy create` 로 우회. **단 12단계(GitHub 차단)의 세 조건을 전부 충족한 경우만 예외**예요 — 그때도 `apps create` + `axhub up` 두 명령만 쓰고 `deploy create` 는 쓰지 않아요.
- NEVER remote `templates.json` / 폐기된 fetch-template 사용.
- NEVER subprocess/headless 에서 template/app name 임의 선택.
- NEVER `--execute` 를 `--dry-run` 미리보기와 사용자 확인 없이 호출.
- NEVER auth 만료를 template 조회 실패로 오해.
- NEVER GitHub device flow code 를 긴 watch tool 안에 숨긴 채 사용자를 빈 GitHub code 입력 화면에 남겨두지 않아요.
- NEVER 코드가 안 보인 채 끝난 execute 를 같은 `--execute` 로 재실행하지 않아요 — 새 코드가 발급돼 사용자가 보고 있는 코드가 무효가 돼요.
- NEVER `repo_full_name` 없이 임의 URL clone.
- NEVER shell 에서 CLI 버전 숫자를 직접 파싱·비교하지 않아요.
