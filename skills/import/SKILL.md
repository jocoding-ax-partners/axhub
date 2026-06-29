---
name: import
description: '이 스킬은 사용자가 비어 있지 않은 기존 로컬 앱을 axhub로 가져와 앱 생성/선택, manifest 정리, GitHub 연결, 첫 배포까지 진행하고 싶을 때 사용해요. 활성화 예: "기존 앱 올려", "이 폴더 axhub에 올려", "이미 만든 앱 배포 준비해", "내 로컬 프로젝트를 axhub 앱으로 가져와", "import existing app", "upload existing app", "이 앱 axhub로 연결해", 또는 non-empty repo 에서 "앱 만들어줘"처럼 템플릿 bootstrap 이 아니라 기존 소스를 axhub에 등록하려는 의도. 빈 디렉토리에서 새 템플릿 앱을 시작하는 요청은 init 으로 보내고, 이미 axhub에 연결된 앱의 ordinary redeploy 는 deploy 로 보내요.'
examples:
  - utterance: "기존 앱 올려"
    intent: "import existing local app into axhub"
  - utterance: "이 폴더 axhub에 올려"
    intent: "import existing local app into axhub"
  - utterance: "이미 만든 앱 axhub로 연결해"
    intent: "import existing local app into axhub"
  - utterance: "import existing app"
    intent: "import existing local app into axhub"
allows-dependency-execution: false
model: sonnet
---

# Import Existing App

비어 있지 않은 로컬 앱을 axhub 앱으로 가져와 manifest, GitHub 연결, 첫 배포 증거까지 한 번에 정리해요. 이 스킬은 판단·실행 로직을 거의 직접 갖지 않아요. `axhub plugin-support import` 가 내보내는 `import/v1` envelope 를 검증하고, 사람이 이해할 수 있는 preview 와 복구 문구를 렌더링해요. 딱 하나 예외로, `manifest_create` 일 때 프로젝트 파일 근거로 axhub.yaml 을 풍부하게 작성하고 `axhub manifest validate` 로 검증하는 보강 단계만 직접 맡아요 — 그 외 모든 mutation 판정·실행은 CLI 가 해요.

## 라우팅 경계

- `import`: 현재 폴더가 비어 있지 않고 기존 앱 소스가 있는 상태에서 axhub 앱으로 처음 가져오는 요청.
- `init`: 빈 디렉토리에서 axhub 템플릿 앱을 새로 만드는 요청.
- `deploy`: 이미 axhub 앱과 manifest 가 연결된 앱을 다시 배포하는 요청.
- `development`: 기존 앱 안에 화면, 대시보드, CRUD 같은 기능 코드를 새로 쓰는 요청.
- `clarity`: 위 경계 밖 운영 명령이나 의도가 모호한 axhub 요청.

`import` 는 `deploy` 를 감싸지 않아요. 첫 연결·첫 배포 준비는 import, 이후 반복 배포는 deploy 가 맡아요.

## 첫 문장

대화형에서 이 스킬이 시작되면 첫 visible chat sentence 는 정확히 이렇게 시작해요.

```text
기존 앱을 axhub에 가져올 준비를 확인할게요.
```

## Vibe Coder Visibility Rules

다음 값은 internal verification primitives 예요. 스킬 안에서는 검증에만 쓰고 사용자 chat 에 raw 값으로 보여주지 않아요.

- `schema_version`, `mode`, `headless`, `correlation_id`
- `detected_state`, `starting_state`, `required_mutations`, `approval`
- `deployment_id`, `active_release_id`, `verification_status`, `public_url` evidence field
- `typed_failure`, `owner`, `phase`, `mutation_performed`, `retryable`
- `request_id`, `stdout`, `stderr`, `command_argv`, raw JSON body

대신 사용자가 이해할 문장으로 바꿔요. 예: "정적 사이트 공개 URL 확인이 아직 안 됐어요. CLI를 업데이트하거나 다시 시도해요."
검증된 `public_url` 값은 사용자에게 열어볼 주소로 보여줘도 돼요. 단 field name, envelope 구조, raw evidence object 는 숨겨요.

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
- static: `kind: "static_release"`, non-empty `active_release_id`, `verified: true`, non-empty `public_url`

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

`claude -p`, CI, `$CLAUDE_NON_INTERACTIVE`, TTY 없음, AskUserQuestion 사용 불가 상태는 headless 예요.

- AskUserQuestion 0회.
- `axhub plugin-support import --mode preview --headless --json` 만 호출해요.
- `--mode execute` 를 호출하지 않아요.
- preview 결과를 한국어 요약으로 보여주고, 실제 가져오기는 대화형에서 다시 실행하라고 안내해요.

## Manifest 보강

`required_mutations` 에 `manifest_create` 가 있고 대화형일 때만, execute 전에 axhub.yaml 을 프로젝트 파일 근거로 풍부하게 작성해요. 이게 이 스킬이 직접 authoring 하는 유일한 단계예요 — minimal manifest 대신 포트·빌드·시작 명령 같은 실제 값을 채워 manifest 의 정확도와 정보량을 함께 높여요.

- **언제:** `가져오기 시작` 승인 직후, execute 호출 직전에만 해요. `manifest_create` 가 없으면(=axhub.yaml 이 이미 있으면) 절대 건드리지 않아요. headless 에서는 실행하지 않아요.
- **근거(grounding):** preview envelope 의 `detected_state.manifest_hints`(포트·health·build·start 와 그 출처)와 실제 프로젝트 파일만 봐요 — Dockerfile, docker-compose·compose, package.json(name·scripts·deps), requirements.txt·pyproject.toml·go.mod, 프레임워크 설정(next.config.\*, vite.config.\* 등), .env.example. 직접 근거가 있는 값만 적고, 불확실하면 비워요. 작고 정확한 manifest 가 크고 틀린 manifest 보다 나아요 — 비운 필드는 backend 가 자동 감지해요.
- **채우는 필드(axhub.yaml 정규 스키마):**
  - `version: axhub/v1` (필수)
  - `name`: 표시 이름(package.json name 또는 폴더 이름)
  - `runtime`: `port`, `health_path` (EXPOSE·HEALTHCHECK·compose·hints 근거)
  - `build`: `framework`, `install`, `build`, `start`, `dockerfile`, `compose_file`, `static_output_dir`, `deploy_method` (deploy_method 는 detect 가 정한 값을 그대로 써요)
  - `env`: `required`/`optional` 아래 `- name:`(+ 필요하면 `scope:`) — **키 이름만**
  - `database`: `engine` (분명히 감지될 때만)
- **보안(엄수):** env 값은 절대 적지 않아요. .env.example 키나 compose `environment:` 키처럼 값 없는 출처에서 key 이름만 가져오고, 비밀처럼 보이는 값은 건너뛰어요. axhub.yaml 에 secret·토큰·비밀번호를 쓰지 않아요.
- **검증 게이트:** 작성한 뒤 반드시 deploy 와 같은 파서로 검증해요.

  ```bash
  axhub manifest validate
  ```

  exit 0 이면 그대로 진행해요. 실패하면 typed error 가 가리키는 필드만 고쳐 최대 2회까지 다시 검증하고, 그래도 실패하면 작성한 axhub.yaml 을 지워 CLI 가 execute 때 최소 manifest 를 쓰게 두고, 최소 설정으로 진행한다고 한 줄로 알려요. validate 의 raw JSON 은 chat 에 붙이지 않아요.
- **이후:** execute 는 axhub.yaml 이 있으면 최소 manifest 를 새로 쓰지 않고 이 보강본을 그대로 둬요. 첫 배포는 현재 git HEAD 를 빌드하므로 이 보강본은 커밋해서 HEAD 에 들어간 뒤(또는 이후 `deploy`)부터 빌드에 반영돼요 — 그래서 정확하고 풍부한 manifest 를 프로젝트에 남기는 게 이 단계의 목적이에요. 첫 배포 자체는 기존 HEAD 와 앱의 deploy_method 로 진행돼요.

## Workflow

1. CLI 가드와 capability 확인

```bash
axhub plugin-support preflight --json
```

`capabilities.import.supported` 가 true 이고 `capabilities.import.schemas` 에 `import/v1` 이 있어야 해요. 아니면 업데이트 안내 후 멈춰요.

2. Preview envelope 요청

```bash
axhub plugin-support import --mode preview --json
```

headless 에서는 이렇게 호출해요.

```bash
axhub plugin-support import --mode preview --headless --json
```

3. Envelope 검증

`import/v1` schema, closed enum, `error`, `approval`, success evidence shape 를 fail-closed 로 확인해요. raw JSON 은 chat 에 붙이지 않아요.

4. Preview 카드 렌더링

사용자에게는 아래 항목만 보여줘요.

- 앱 이름 또는 추정 이름
- 감지된 상태 요약
- 진행할 변경 요약
- 배포 방식
- 안전 메모

`manifest_create` 가 있으면, axhub.yaml 을 프로젝트 파일 근거로 자세히 작성할 예정이라고 한 줄로 같이 알려요.

5. 대화형 승인 1회

AskUserQuestion 은 preview 직후 한 번만 써요. 옵션은 다음 네 가지예요.

- 가져오기 시작
- 먼저 수정할게요
- 취소
- 자세한 요약 보기

`가져오기 시작` 외에는 execute 를 호출하지 않아요.

6. axhub.yaml 보강 (manifest_create 일 때만)

`가져오기 시작` 승인 직후, execute 전에 진행해요. `required_mutations` 에 `manifest_create` 가 있을 때만 위 `## Manifest 보강` 규칙대로 프로젝트 파일 근거로 axhub.yaml 을 작성하고 `axhub manifest validate` 로 검증해요. `manifest_create` 가 없거나 headless 면 이 단계를 건너뛰어요.

7. Execute 호출

대화형 승인 직후 한 번만 호출해요.

```bash
axhub plugin-support import --mode execute --approved --json
```

동일 승인으로 두 번 호출하지 않아요. execute 결과도 `import/v1` 로 다시 검증해요.

8. 성공 안내

- docker/compose 성공: public URL 을 보여주고, 배포 확인이 끝났다고 말해요.
- static 성공: public URL 을 보여주고, 정적 사이트 활성 릴리스 확인이 끝났다고 말해요.

내부 id 는 필요할 때만 상태 이어보기에 쓰고 chat 에 raw 값으로 노출하지 않아요.

9. 실패 안내

`error.message_ko` 를 우선 쓰되, raw field 를 그대로 복사하지 않아요. `recovery_action` 은 한국어 행동 문장으로 바꿔요.

| typed_failure | 사용자 문구 |
|---|---|
| `auth` | 로그인이 필요해요. 다시 로그인한 뒤 이어갈게요. |
| `version` | axhub CLI가 import 기능을 아직 지원하지 않아요. 업데이트한 뒤 다시 시도해요. |
| `manifest` | 앱 설정 파일을 정리해야 해요. CLI가 제안한 안전한 수정만 진행해요. |
| `git` | Git 저장 지점이 준비되지 않았어요. 커밋이나 원격 연결을 먼저 확인해요. |
| `repo` | GitHub 저장소 연결을 확인해야 해요. 권한이나 원격 저장소를 다시 볼게요. |
| `app` | axhub 앱 생성 또는 선택에서 막혔어요. 앱 이름과 소유 권한을 확인해요. |
| `static` | 정적 사이트 확인 증거가 부족해요. 공개 URL과 활성 릴리스 확인 뒤 다시 시도해요. |
| `deploy` | 첫 배포 확인이 끝나지 않았어요. 배포 상태를 다시 확인해요. |
| `rate_limit` | 요청이 잠시 많아요. 조금 뒤 다시 시도해요. |
| `transport` | 네트워크 연결이 불안정해요. 연결 상태를 확인한 뒤 다시 시도해요. |

## Regression guard

- init 은 빈 디렉토리 template bootstrap 만 맡아요.
- deploy 는 ordinary redeploy 만 맡아요.
- import 는 non-empty existing app first-connect flow 만 맡아요.
- plugin 은 low-level CLI primitive 를 조합하지 않아요.
- manifest 보강은 plugin 이 직접 authoring 하는 유일한 단계예요 — `manifest_create` 일 때만, 증거 있는 필드만, env 값 없이 작성하고 `axhub manifest validate` 통과를 강제해요. 실패하면 최소 manifest 로 fallback 해요.
- malformed envelope, unknown schema, unknown enum, missing static URL, `verified !== true`, headless execute, approval bypass 는 모두 중단해요.
- 성공을 말하기 전 항상 execute envelope 의 method-specific evidence 를 확인해요.
