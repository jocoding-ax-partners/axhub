---
name: import
description: '기존 앱을 axhub에 가져올 준비를 확인할게요. Use this before bootstrap for any existing/local-folder app import or first deploy: "기존 앱", "기존 Express 서버 앱", "이미 만든 앱", "작업 폴더는 /path", "이 폴더 axhub에 올려", "이 앱을 axhub에 올려", "가져오기가 잘 끝났는지 확인", "중단된 가져오기 이어서 마무리", "import existing app". Start directly with that Korean sentence; no preamble; do not explain why this path was chosen or expose a route/skill label. 스킬 실행 전 사용자 문장 0개. 기존 앱의 첫 연결·첫 배포와 resume 완료 확인을 맡겨요. Broad stack: Express/Fastify/Nest/FastAPI/Flask/Django/Rails/Go/Rust/Java/PHP/.NET. 빈 폴더는 bootstrap, 재배포는 deploy로 양보해요.'
examples:
  - utterance: "기존 앱 올려"
    intent: "import existing local app into axhub"
  - utterance: "이미 만든 앱 axhub로 연결해"
    intent: "import existing local app into axhub"
  - utterance: "기존 Express 서버 앱을 axhub에 올려서 실제 배포까지 해줘"
    intent: "import existing backend app into axhub"
  - utterance: "작업 폴더는 /path/to/app 이야. axhub에 올려줘"
    intent: "import existing local app from provided directory into axhub"
  - utterance: "import existing app"
    intent: "import existing local app into axhub"
allows-dependency-execution: false
model: sonnet
---

# Import Existing App

> **Windows 실행 계약 (AP-13):** axhub 명령은 Git Bash 전용으로 실행해요. PowerShell 금지, PATH 는 `axhub plugin-support repair-path`, `auth status` 는 `auth login` 한 그 셸에서 검증해요.

> **CLI 경로 계약 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 가 있으면 그 절대경로로 `plugin-support repair-path --json` 을 실행하고 반환된 `bin_path` 절대경로로 이 세션을 이어가요. 셋 다 없을 때만 onboarding 을 안내해요.

비어 있지 않은 로컬 앱을 axhub 앱으로 가져와 앱 설정, GitHub 연결, 첫 배포 증거까지 한 번에 정리해요. 이 스킬은 판단·실행 로직을 거의 직접 갖지 않아요. `axhub plugin-support import` 가 내보내는 `import/v1` envelope 를 검증하고, 사람이 이해할 수 있는 미리보기와 복구 문구를 렌더링해요. 딱 하나 예외로, 새 앱 설정 파일이 필요한 경우 프로젝트 파일 근거로 axhub.yaml 을 풍부하게 작성하고 `axhub deploy --explain --json` 으로 같은 파서가 읽는지 검증하는 보강 단계만 직접 맡아요. 그 외 모든 변경 판정·실행은 CLI 가 해요.

## 라우팅 경계

- `import`: 현재 폴더가 비어 있지 않고 기존 앱 소스가 있는 상태에서 axhub 앱으로 처음 가져오는 요청.
- `bootstrap`: 빈 디렉토리에서 axhub 템플릿 앱을 새로 만드는 요청.
- `deploy`: 이미 axhub 앱과 manifest 가 연결된 앱을 다시 배포하는 요청.
- `development`: 기존 앱 안에 화면, 대시보드, CRUD 같은 기능 코드를 새로 쓰는 요청.
- `clarity`: 로그·환경변수·롤백·테이블/컬럼/데이터·connector grant·GitHub 재연결 같은 명시된 axhub 운영 명령.

`import` 는 `deploy` 를 감싸지 않아요. 첫 연결·첫 배포 준비는 import, 이후 반복 배포는 deploy 가 맡아요.

발화에 axhub 언급이 없고 대화에도 axhub 맥락(직전 axhub 작업)이 없으면 — 예를 들어 "이 폴더 올려" 를 GitHub push 로 뜻할 수 있으면 — manifest 를 만들기 전에 "axhub에 연결하려는 거예요?" 를 한 번만 묻고, 아니라는 답이면 종료해요. headless 에서는 묻지 않고 멈춰요.

## 작업 폴더 실행 계약

사용자가 `작업 폴더`, `폴더`, `경로`, `디렉토리` 로 앱 경로를 지정하면 그 절대 경로를 `APP_DIR` 로 고정해요. 현재 Claude Code workspace 가 상위 폴더이거나 다른 폴더여도, 모든 import 관련 `axhub`, `git`, `npm`, build, manifest 검증 명령은 반드시 `APP_DIR` 안에서 실행해요. tool 이 cwd 를 지정할 수 없거나 권한 카드에 cwd 가 따로 보이지 않으면 명령 앞에 실제 절대 경로를 넣은 `cd "<absolute APP_DIR>" &&` 를 붙여요. 권한 카드에는 `$APP_DIR` 변수를 그대로 쓰지 말고 사용자가 준 실제 경로를 따옴표로 넣어요.

Claude Desktop 에서 현재 선택된 프로젝트가 이미 정확히 `APP_DIR` 이면 `cd ... &&` 를 붙이지 않고 **한 카드에 한 개의 bare 명령만** 실행해요. 먼저 tool 의 cwd 지정 기능을 쓰고, 현재 프로젝트도 다르고 cwd 지정도 불가능할 때만 `cd "<absolute APP_DIR>" && <single command>` 를 마지막 수단으로 써요. 같은 폴더를 매번 `cd` 하는 compound shell 은 명령마다 새 권한 카드를 만들 수 있으므로 Desktop 매끄러움 실패예요.

`APP_DIR` 이 정해진 뒤에는 workspace root 에서 `axhub --json plugin-support import`, `axhub deploy --explain --json`, `git check-ignore`, `git status`, `npm run build` 를 실행하지 않아요. preview 가 통과했더라도 권한 카드 명령이 bare `axhub ...`, bare `git ...`, bare `npm ...` 로 시작하거나 실제 절대 경로가 들어간 `cd "<absolute APP_DIR>" &&` 또는 동등한 cwd 지정이 보이지 않으면 실행하지 말고 같은 명령을 `APP_DIR` 기준으로 다시 호출해요.

사용자가 앱 경로를 말하지 않았고 현재 workspace 가 실제 앱 폴더인지 확실하지 않으면, preview 전에 앱 폴더를 먼저 확인해요. 잘못된 폴더에서 import preview 를 돌려서 bootstrap/import 라우팅을 다시 설명하는 흐름으로 가지 않아요.

## Desktop 도구 제목 hard rule

모든 Bash/tool call 제목·progress title 은 한국어 명사구로 직접 정해요. Claude Desktop 자동 제목에 맡기지 않아요. 특히 `Expressing ...`, `Expressed ...`, `FastAPIing ...`, `axhubed ...` 처럼 제품명·스택명을 영어 동사처럼 만든 제목은 절대 쓰지 않아요. 파일 확인은 `서버 설정 확인`, `작업 폴더 파일 확인`, `package.json 확인`, 설정 파일 검증은 `앱 설정 파일 검증`, 가져오기 실행은 `가져오기 실행` 처럼 써요. 스택 이름은 본문 설명에만 써요.

## 개발자 스택 지원 범위

이 스킬은 Next.js 전용이 아니에요. CLI 의 `import/v1` 감지와 execute 증거를 기준으로 기존 개발자 앱을 가져오며, 대표적인 프론트·백엔드 스택을 지원해요.

- 정적 프론트: Vite, Astro, React Scripts, Angular, Vue CLI, Parcel, Gatsby, Svelte 등 build-only 앱은 static 후보로 다뤄요.
- Compose 기반 앱: `compose.yml`, `compose.yaml`, `docker-compose.yml`, `docker-compose.yaml` 이 있으면 compose 배포 후보로 다뤄요.
- Dockerfile 보유 앱: 기존 Dockerfile 을 존중하고 새 Dockerfile 을 만들지 않아요.
- Dockerfile 없는 백엔드 앱: CLI 가 근거 파일을 보고 Node, Python(FastAPI/Flask/Django), Go, Rust, Java(Maven/Gradle), PHP, Ruby(Rails/Rack), Deno, .NET 용 기본 Dockerfile 을 생성할 수 있어요.

스킬은 이 스택 판정을 직접 재구현하지 않아요. preview 의 `deploy_method`, `manifest_hints`, `required_mutations`, `safety_notes` 를 검증해서 보여주고, execute 는 CLI 가 한 번만 수행해요.

## 첫 문장

대화형에서 이 스킬이 시작되면 첫 visible chat sentence 는 정확히 이렇게 시작해요.

```text
기존 앱을 axhub에 가져올 준비를 확인할게요.
```

첫 visible chat sentence 는 반드시 정확히 `기존 앱을 axhub에 가져올 준비를 확인할게요.` 로 시작하고, 그 앞에는 공백·설명·스킬 선택 이유를 포함해 어떤 문장도 쓰지 않아요.

import preview 정상이면 axhub 가져오기 대상 확정이에요. Interactive 는 별도 진입 질문 없이 아래 6의 preview 승인 AskUserQuestion **하나가 axhub 진입 확인을 겸해요** — 질문 문구에 axhub 대상임을 명시해요 (AP-12 통합 게이트). (headless 는 이 AUQ 생략)

## AskUserQuestion JSON 안전 규칙

AskUserQuestion 입력은 평문 UTF-8 만 쓰고 raw JSON/수동 escape 를 만들지 않아요. preview 승인과 commit 선택을 합친 exact copy 는 [references/visibility-rules.md](references/visibility-rules.md) 의 AskUserQuestion 절을 그대로 써요.

## Vibe Coder Visibility Rules

이 섹션은 workflow 보다 우선해요. 사용자-facing 문구·tool 제목을 쓰기 전(진행 안내·preview 카드·성공/실패 요약 직전)에 [references/visibility-rules.md](references/visibility-rules.md) 를 읽고 그 금지어·치환·제목 규칙을 그대로 따라요. 핵심만 요약하면: 내부 field name·영어 진행어·스킬/route 라벨 금지, tool 제목은 한국어 명사구만, URL 은 평문 절대 URL 만, 비공개 앱의 로그인 화면 200 응답을 본문 검증으로 치지 않기예요.

## import/v1 envelope 계약

CLI preview/execute 결과는 정확히 하나의 envelope shape 로 와야 해요.

필수 top-level field:

- `schema_version`: `import/v1` 만 허용해요.
- `mode`: `preview` 또는 `execute`.
- `headless`: true 면 mutation 금지, preview semantics 만 허용해요.
- `correlation_id`: 내부 추적용. 사용자 chat 에 노출하지 않아요.
- `detected_state.starting_state`: `local_github_no_axhub_app`, `local_only`, `existing_axhub_app_repair` 중 하나.
- `deploy_method`: `docker`, `compose`, `static` 중 하나.
- `required_mutations[]`: 닫힌 enum 만 허용해요.
- `preview`: 사용자가 이해할 title, summary, mutations, safety_notes.
- `approval`: `required`, `approved`, `interactive_only` 를 포함해요.
- `result.evidence`: preview 에서는 null, execute 성공 후에만 채워요.
- `error`: null 또는 정해진 error object.

닫힌 enum:

- `required_mutations`: `manifest_create`, `manifest_migrate`, `manifest_repair`, `app_create`, `app_select`, `github_repo_create`, `github_connect`, `first_deploy`, `static_release`
- `typed_failure`: `auth`, `version`, `manifest`, `git`, `repo`, `app`, `static`, `deploy`, `rate_limit`, `transport`
- `owner`: `plugin`, `cli`, `backend`
- `phase`: `preflight`, `detect`, `preview`, `approval`, `manifest`, `app`, `repo`, `git`, `deploy`, `verify`, `static`, `finalize`

성공 증거:

- docker/compose: `kind: "deployment"`, non-empty `deployment_id`, `verification_status: "success"`, non-empty `public_url`
- static: `kind: "static_release"`, non-empty `active_release_id`, `verified: true`, non-empty `public_url`, optional non-empty `access_note`

Static 성공은 `active_release_id`, `verified === true`, `public_url`, `error === null` 이 모두 있어야 해요. 하나라도 없으면 `typed_failure: "static"` 으로 다뤄요. Static lane 에서는 deployment record `deploy verify` 를 대신 호출하지 않아요.

## Fail-closed 검증

스킬은 envelope 를 받은 뒤 아래 조건이면 즉시 멈춰요.

- `axhub plugin-support preflight --json` 의 `capabilities.import.supported !== true`
- `capabilities.import.schemas` 에 `import/v1` 이 없음
- envelope 의 `schema_version` 이 `import/v1` 이 아님
- 필수 field 가 없거나 타입이 맞지 않음
- 닫힌 enum 밖 값이 있음
- `error` object 가 누락 field 또는 알 수 없는 owner/phase/failure 를 가짐
- preview 가 headless 에서 mutation 가능하다고 말함
- execute 가 대화형 승인 없이 진행되려 함
- success evidence 가 deploy method 와 맞지 않음

멈출 때도 low-level 명령을 조합해서 우회하지 않아요. `apps create`, `apps git connect`, `deploy create`, static release 명령을 plugin 이 직접 이어붙이지 않아요.

## Headless rule

`claude -p`·`codex exec`, CI, `$CLAUDE_NON_INTERACTIVE`, TTY 없음, 네이티브 선택 UI 와 명시 텍스트 승인이 둘 다 불가한 상태(사용자 확인 채널 자체가 없음)는 headless 예요.

- AskUserQuestion 0회.
- `axhub --json plugin-support import --mode preview --headless` 만 호출해요.
- `--mode execute` 를 호출하지 않아요.
- preview 결과를 한국어 요약으로 보여주고, 실제 가져오기는 대화형에서 다시 실행하라고 안내해요.

## Manifest 보강

`required_mutations` 에 `manifest_create` 가 있고 대화형일 때만, execute 전에 axhub.yaml 을 프로젝트 파일 근거로 풍부하게 작성해요 — 이 스킬이 직접 authoring 하는 유일한 단계예요. 실행 시점이 오면 [references/manifest-authoring.md](references/manifest-authoring.md) 를 읽고 그 규칙대로 진행해요: 무시 파일 선행/사후 정리(`git check-ignore`), manifest_hints·실파일 근거 grounding, 정규 스키마 필드만 작성, env 값 절대 금지(키 이름만), `axhub deploy --explain --json` 검증 게이트(최대 2회, 실패 시 최소 manifest 로 degrade), commit+push 는 `capabilities.import.commit_manifest` + preview 승인에서 고른 명시적 동의가 있을 때만이에요. headless 에서는 실행하지 않아요.

Desktop ignore 확인은 bare `git check-ignore -q axhub.yaml` 1회예요. exit 0/1을 tool 결과로 읽고 `$?`, `echo`, 변수, shell 연산자를 붙이지 않아요. ignore 규칙을 바꾼 때만 사후 1회 재검증해요.

## Workflow

1. CLI 가드와 capability 확인

아래 Workflow 명령 예시는 모두 tool cwd 가 `APP_DIR` 로 지정된 bare 명령이에요. Desktop 카드에는 shell 연산·출력 가공을 붙이지 않아요.

완료/재개 시 폴더명·manifest `name`·preflight `current_app`을 원격 slug로 추론하거나 `apps get/list`, `deploy list/status`로 우회하지 않아요. 아래 resume 식별자를 써요.

```bash
axhub plugin-support preflight --json
```

이 명령이 command not found 로 끝나면 미설치로 판정하지 않아요 — AP-17 경로 계약대로 `~/.axhub/bin-path` 나 `~/.axhub/bin/axhub`(.exe) 로 `plugin-support repair-path --json` 을 실행한 뒤 같은 절대경로로 이 preflight 를 다시 실행해요.

`capabilities.import.supported` 가 true 이고 `capabilities.import.schemas` 에 `import/v1` 이 있어야 해요. 아니면 업데이트 안내 후 멈춰요.

1.5. 완료 확인 short-circuit

완료 확인 요청이면 작업공간/preview 전에 `.axhub/import-resume.json`을 1회 읽어요. 유효한 `app_id`와 `deployment_id`가 있으면 다음 bare 명령을 1회 실행해요.

```bash
axhub deploy verify <deployment-id> --app <app-id> --json
```

성공이면 완료를 안내하고 preview/승인/execute/추가 조회 없이 끝내요. `not_found` 또는 식별자 없음만 일반 흐름으로 가고, 다른 실패는 성공이라 하지 않고 멈춰요. 새 가져오기에는 적용하지 않아요.

2. 작업공간 확정

preview 와 첫 승인보다 먼저 아래 읽기 전용 resolver 를 정확히 한 번 실행해요.

```bash
axhub plugin-support tenant-resolve --json
```

반환된 `tenant` 가 비어 있지 않으면 그 값을 이후 preview 와 execute 모두에 literal `--tenant <slug>` 로 넘겨요. `needs_pick: true` 면 후보를 사용자에게 `작업공간`이라고만 부르고, `앱을 어느 작업공간에 만들까요?`를 승인 전에 한 번 물은 뒤 선택한 slug 를 같은 literal 로 유지해요. 사용자-facing 문구에 `tenant` 또는 `테넌트`를 쓰지 않아요.

여러 작업공간이 있는 상태에서 `--tenant` 없는 execute 를 먼저 호출해 오류를 낸 뒤 `axhub tenants list`, `axhub tenant list`, `--help | grep`로 복구하지 않아요. 작업공간 선택 전에는 preview 승인이나 execute mutation을 시작하지 않아요. 이 순서가 깨지면 동일 가져오기 승인을 다시 받게 되어 Desktop deadlock UX예요.

3. Preview envelope 요청

아직 안 읽었다면 resume을 1회 읽어요. 유효한 `app_id`는 preview의 literal `--app <app_id>`로 넘기고, 없을 때만 옵션 없이 호출해요. resume ID는 폴더·manifest·preflight 이름보다 우선해요.

사용자나 현재 컨텍스트에서 app slug, GitHub owner/repo, tenant 가 이미 정해졌으면 preview 부터 그대로 넘겨요. `--slug` 는 axhub 앱 slug, `--name` 은 표시 이름, `--repo` 는 GitHub 저장소예요. repo owner 를 별도 flag 로 만들지 말고 `--repo "$OWNER/$REPO"` 형태로 넘겨요.

import helper 에 넘길 수 있는 옵션은 `--mode`, `--headless`, `--approved`, `--commit-manifest`, `--verify-wait`, `--app`, `--name`, `--slug`, `--repo`, `--repo-private`, `--repo-public`, `--branch`, `--from-dir`, `--installation-id`, `--tenant`, `--deploy-method`, `--json` 뿐이에요. 이 목록에 없는 옵션은 추론하거나 `--help`로 복구하지 않아요. 특히 사용자가 비공개 저장소를 원하면 정확히 `--repo-private` 를 쓰고, 존재하지 않는 `--private` 를 절대 붙이지 않아요. 실수로 unknown option 이 나오면 임의 옵션을 뺀 재시도나 `plugin-support import --help` 호출로 이어가지 말고 멈춰요. preview 와 execute 는 검증된 동일 옵션 집합을 유지해요.

GitHub owner 만 명시되고 repo 이름이 따로 없으면 `$REPO_NAME` 은 반드시 `$APP_SLUG` 와 정확히 같게 둬요. 날짜·숫자·QA suffix 를 추론으로 자르거나 정리하지 않아요. 예를 들어 app slug 가 `uqa-exp-public-11021-0708` 이고 owner 가 `realitsyourman` 이면 `--repo "realitsyourman/uqa-exp-public-11021-0708"` 이어야 하며 `--repo "realitsyourman/uqa-exp-public-11021"` 처럼 줄인 명령은 실행하지 말고 고쳐서 다시 호출해요.

단, static 앱은 CLI 가 GitHub repo 없이 `app_create/app_select → local build → static_release` 로 배포할 수 있어요. static lane 에서는 사용자가 명시적으로 "GitHub 저장소도 만들고 연결해줘" 라고 말하지 않는 한 `--repo` 를 붙이지 않아요. Docker/compose 같은 GitHub 기반 첫 배포에서만 `local_only` 새 repo 생성 전에 `--repo owner/name` 이 확정돼 있어야 해요.

Docker/compose `local_only` 에서 새 GitHub repo 를 만들 때 owner 는 먼저 로컬 GitHub 계정과 맞춰요. `gh api user --jq .login` 으로 현재 `gh` 로그인 이름을 확인할 수 있으면 CLI 가 그 login 과 app slug 로 기본 repo 를 정해요. org owner 는 이미 그 org repo 를 직접 만들고 push 가능한 `origin` 이 있거나 사용자가 명시적으로 그 owner 를 지정했을 때만 써요. CLI 는 새 repo 생성 전에 owner mismatch 를 `typed_failure: git` 으로 막을 수 있어요. 이때는 같은 로컬 login owner 로 다시 preview/execute 하거나, org repo 를 직접 만들고 push 가능한 origin 에서 다시 실행해요.

```bash
axhub --json plugin-support import --mode preview
```

예시:

```bash
axhub --json plugin-support import --mode preview --slug "$APP_SLUG" --tenant "$TENANT"

# Docker/compose local_only 처럼 GitHub repo 가 필요한 경우에만:
axhub --json plugin-support import --mode preview --slug "$APP_SLUG" --repo "$GITHUB_OWNER/$REPO_NAME" --repo-private --tenant "$TENANT"
```

headless 에서는 이렇게 호출해요.

```bash
axhub --json plugin-support import --mode preview --headless
```

4. Envelope 검증

`import/v1` schema, closed enum, `error`, `approval`, success evidence shape 를 fail-closed 로 확인해요. raw JSON 은 chat 에 붙이지 않아요.

5. Preview 카드 렌더링

사용자에게는 아래 항목만 보여줘요.

- 앱 이름 또는 추정 이름
- 감지된 상태 요약
- 진행할 변경 요약
- 배포 방식
- 안전 메모

새 앱 설정 파일이 필요하면, axhub.yaml 을 프로젝트 파일 근거로 자세히 작성할 예정이라고 한 줄로 같이 알려요.
기존 axhub.yaml 복구가 필요하면, 문법이 깨져 있어서 CLI 가 가져오기 중 백업 파일을 남기고 안전한 최소 설정으로 복구한다고 한 줄로 같이 알려요. 이 경우 plugin 이 직접 덮어쓰지 않아요.

6. 대화형 통합 승인 1회

AskUserQuestion 은 preview 직후 딱 한 번 써요. 질문은 `이 앱을 axhub에 가져와서 미리보기대로 진행할까요?` 처럼 axhub 대상임을 명시해요 — 이 질문 하나가 axhub 진입 확인과 manifest commit 동의를 모두 겸해요. 네이티브 선택 UI 가 있으면 그걸로 묻고, 없으면 같은 확인을 명시 텍스트 승인 1회로 받고, 둘 다 불가한 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.

- `capabilities.import.commit_manifest` 가 true 이고 GitHub 기반 첫 배포면 `설정도 반영하고 시작`, `커밋 없이 시작`, `먼저 수정할게요`, `취소` 네 옵션을 써요. 첫 옵션은 `--commit-manifest`, 둘째 옵션은 옵션 없는 execute 로 고정해요.
- 그 외에는 `가져오기 시작`, `먼저 수정할게요`, `취소` 세 옵션만 써요.

승인 선택 뒤에는 manifest 작성·검증 결과가 예상과 달라도 커밋 질문이나 같은 가져오기 질문을 다시 띄우지 않아요. 실행 가능한 첫째/둘째 옵션 외에는 execute 를 호출하지 않아요.

7. axhub.yaml 보강 (manifest_create 일 때만)

통합 승인 직후, execute 전에 진행해요. `required_mutations` 에 `manifest_create` 가 있을 때만 위 `## Manifest 보강` 규칙대로 프로젝트 파일 근거로 axhub.yaml 을 작성하고 `axhub deploy --explain --json` 로 검증해요. `manifest_create` 가 없거나 headless 면 이 단계를 건너뛰어요. `설정도 반영하고 시작` 을 골랐다면 검증 통과 뒤 질문 없이 `--commit-manifest` 를 쓰고, `커밋 없이 시작` 이면 질문 없이 그 옵션을 빼요. local_only 라서 아직 git remote 가 없어도 통합 승인에서 선택한 값을 그대로 유지해요.

8. Execute 호출

대화형 승인 직후 한 번만 호출해요. preflight 의 `capabilities.import.early_return` 과 `capabilities.import.verify_wait` 가 모두 true 일 때만 execute 에 `--verify-wait none` 을 붙여요 — execute 는 첫 배포 생성과 deployment id 확보까지만 반환하고, 이어서 **권한 카드 한 번으로 끝나는** `axhub deploy verify <deployment-id> --app <app> --wait --wait-interval 20s --wait-timeout 10m --json` 를 정확히 한 번 호출해요. 이 한 호출의 내부 폴링 예산은 최대 30회 또는 10분(AP-16)이고, `--wait` 가 성공·실패·예산 제한까지 책임지므로 같은 verify 를 반복 호출하거나 `axhub apps get`, `deploy list`, `deploy status` 같은 사후 확인을 덧붙이지 않아요. 둘 중 capability 하나라도 없으면 `--verify-wait none` 을 빼서 execute 가 동기 검증을 끝내게 하고, 반환된 정확한 deployment id 로 일반 `axhub deploy verify <deployment-id> --app <app> --json` 를 최종 증거용으로 정확히 한 번만 호출해요.

Claude Desktop 에서는 Monitor, background task, ScheduleWakeup, output 파일 읽기, `for`/`while`/`until`, `sleep`, command substitution, pipe, `grep`, `head`, `status=` 같은 shell 폴링을 쓰지 않아요. 첫 verify 가 exit 6 또는 timeout 이면 성공이라고 말하지 않고, 같은 명령을 자동 재실행하거나 새 승인 카드를 띄우지 않은 채 현재 진행 중이며 나중에 같은 ID로 이어볼 수 있다고 요약해요. UI 에 "실행 중"이 남아 있는데 실제 `axhub` 프로세스가 없거나 output 을 회수하지 못해도 execute 를 다시 실행하지 않아요.

`커밋 없이 시작` 이거나 commit+push capability 가 없는 경우:

```bash
axhub --json plugin-support import --mode execute --approved
```

preview 에 `--slug`, `--repo`, `--tenant`, `--name`, `--deploy-method`, `--from-dir`, `--branch` 같은 import 옵션을 넘겼다면 execute 에도 같은 값을 그대로 반복해요. static lane preview 에 `--repo` 를 넣지 않았다면 execute 에도 넣지 않아요. Docker/compose `local_only` 에서 `--repo owner/name` 이 없으면 CLI 가 현재 `gh` 로그인과 app slug 로 repo 를 정하고, owner 를 확인할 수 없으면 repo failure 로 멈춰요.

```bash
axhub --json plugin-support import --mode execute --approved --slug "$APP_SLUG" --tenant "$TENANT"

# Docker/compose local_only 처럼 GitHub repo 가 필요한 경우에만:
axhub --json plugin-support import --mode execute --approved --slug "$APP_SLUG" --repo "$GITHUB_OWNER/$REPO_NAME" --repo-private --tenant "$TENANT"
```

`설정도 반영하고 시작` 을 골랐으면 `--commit-manifest` 를 더해요. CLI 가 backend mutation 전에 axhub.yaml 을 커밋·push 해서 첫 배포에 반영해요.

```bash
axhub --json plugin-support import --mode execute --approved --commit-manifest
```

동일 승인으로 두 번 호출하지 않아요. execute 결과도 `import/v1` 로 다시 검증해요. push 실패는 `typed_failure: git` 으로 와요(아래 10의 git 행으로 안내).

권한 복구 뒤 같은 import 를 재개할 때는 preview 가 `existing_axhub_app_repair` 를 줄 수 있어요. 이 경우에도 execute 는 CLI envelope 한 번으로 처리하고, 위 capability 규칙에 맞는 verify 한 번만 실행해요.

9. 성공 안내

- docker/compose 성공: 공개 URL 을 평문 절대 URL 로 보여주고, 배포 확인이 끝났다고 말해요. 앱이 비공개라 로그인 없는 URL 요청이 로그인 화면으로 돌아오면, 배포 검증과 앱 본문 확인을 분리해 설명해요.
- static 성공: 공개 URL 을 평문 절대 URL 로 보여주고, 정적 사이트 활성 릴리스 확인이 끝났다고 말해요. `access_note` 가 있으면 같은 성공 블록에서 "참고: ..." 형태로 함께 말해요.

내부 id 는 필요할 때만 상태 이어보기에 쓰고 chat 에 raw 값으로 노출하지 않아요.

10. 실패 안내

`error.message_ko` 를 우선 쓰되, raw field 를 그대로 복사하지 않아요. `recovery_action` 은 한국어 행동 문장으로 바꿔요.

| typed_failure | 사용자 문구 |
|---|---|
| `auth` | 로그인이 필요해요. 다시 로그인한 뒤 이어갈게요. |
| `version` | axhub CLI가 import 기능을 아직 지원하지 않아요. 업데이트한 뒤 다시 시도해요. |
| `manifest` | 앱 설정 파일을 정리해야 해요. CLI가 제안한 안전한 수정만 진행해요. |
| `git` | Git/GitHub 연결이 준비되지 않았어요. recovery_action 을 따라요 — GitHub 인증 승인이 필요하다는 안내면 `axhub github link` 연동(승인 후 재실행하면 이어져요)이 먼저고, 그 외에는 커밋·원격 연결·repo owner 를 확인해요. |
| `repo` | GitHub 저장소 연결을 확인해야 해요. 권한이나 원격 저장소를 다시 볼게요. |
| `app` | axhub 앱 생성 또는 선택에서 막혔어요. 앱 이름과 소유 권한을 확인해요. |
| `static` | 정적 앱 처리에서 막혔어요. retryable=false 면 빌드 스크립트나 산출물 경로를 고친 뒤에만 다시 시도해요 — 재시도만으로는 결과가 안 바뀌어요. 그 외에는 공개 URL과 활성 릴리스를 확인해요. |
| `deploy` | 첫 배포 확인에서 막혔어요. 실패로 끝났다면 `axhub deploy diagnose` 로 원인을 보고, 코드 원인이면 수정 커밋을 만든 뒤 다시 import 해요 — 같은 커밋 재실행은 같은 배포를 재사용해요. 진행 중이면 배포 상태만 다시 확인해요. |
| `rate_limit` | 요청이 잠시 많아요. 조금 뒤 다시 시도해요. |
| `transport` | 네트워크 연결이 불안정해요. 연결 상태를 확인한 뒤 다시 시도해요. |

## Regression guard

- bootstrap 은 빈 디렉토리 template bootstrap 만 맡아요.
- deploy 는 ordinary redeploy 만 맡아요.
- import 는 non-empty existing app first-connect flow 만 맡아요.
- plugin 은 low-level CLI primitive 를 조합하지 않아요.
- manifest 보강은 plugin 이 직접 authoring 하는 유일한 단계예요 — `manifest_create` 일 때만, 증거 있는 필드만, env 값 없이 작성하고 `axhub deploy --explain --json` 통과를 강제해요. 실패하면 최소 manifest 로 fallback 해요.
- `manifest_repair` 는 CLI execute 가 처리해요. plugin 은 preview 에서 사용자에게 알리고, 직접 파일을 덮어쓰지 않아요.
- commit+push 는 opt-in 이에요 — `capabilities.import.commit_manifest` true + GitHub 기반 첫 배포 + preview 통합 승인에서 `설정도 반영하고 시작` 선택이 모두 있을 때만 `--commit-manifest` 로 호출하고, 그 git 커밋·push 는 CLI 가(force 없이) 맡아요. local_only 는 remote 없음만으로 제외하지 않아요. capability 가 없으면 옵션을 제공하지 않아요. headless 에서는 없어요.
- malformed envelope, unknown schema, unknown enum, missing static URL, `verified !== true`, headless execute, approval bypass 는 모두 중단해요.
- 성공을 말하기 전 항상 execute envelope 의 method-specific evidence 를 확인해요.
