---
name: init
description: '이 스킬은 사용자가 새 axhub 앱을 만들거나 템플릿으로 프로젝트를 시작하고 싶어할 때 사용해요. 다음 표현에서 활성화: "결제 앱 만들어", "결제 앱 만들어줘", "빈 디렉토리", "새 앱 만들어", "새 앱 만들어줘", "앱 만들어줘", "프로젝트 만들어", "프로젝트 초기화", "프로젝트 초기화 해줘", "프로젝트 초기화해줘", "초기화 해줘", "초기화해줘", "apphub.yaml 만들어", "apphub.yaml 만들어줘", "axhub.yaml 만들어", "axhub.yaml 만들어줘", "fastapi 앱", "FastAPI 앱 만들어줘", "next.js 앱", "Next.js 앱 만들어줘", "nextjs 앱", "init", "scaffold", 또는 빈 디렉토리에서 새 앱 시작 의도. `axhub apps bootstrap` saga 로 backend app + GitHub repo + 첫 deploy 를 한 번에 진행하고 `repo_full_name` 으로 현재 dir 에 git clone 해요.'
examples:
  - utterance: "결제 앱 만들어"
    intent: "scaffold new axhub app"
  - utterance: "결제 앱 만들어줘"
    intent: "scaffold new axhub app"
  - utterance: "프로젝트 초기화해줘"
    intent: "scaffold new axhub app"
  - utterance: "init"
    intent: "scaffold new axhub app"
  - utterance: "scaffold"
    intent: "scaffold new axhub app"
  - utterance: "빈 디렉토리"
    intent: "scaffold new axhub app"
  - utterance: "그걸로 앱 만들고 싶어"
    intent: "scaffold new axhub app"
  - utterance: "이제 앱 만들어줘"
    intent: "scaffold new axhub app"
  - utterance: "그거로 앱 만들어줘"
    intent: "scaffold new axhub app"
multi-step: true
needs-preflight: false
allows-dependency-execution: false
model: sonnet
---

# Init

새 axhub 앱을 `axhub apps bootstrap` saga 로 한 번에 만들어요. backend app 생성 + GitHub repo 생성 + 첫 deploy 를 server-side 에서 처리하고, saga 응답의 `repo_full_name` 으로 현재 dir 에 git clone 해서 local + remote 둘 다 채워줘요. 기존 `axhub init` 호출은 Rust v1.0.0-rc.1 에서 `--from-template` 미구현 stub 라 SKILL 에서 호출하지 않아요.

## Vibe Coder Visibility Rules

이 SKILL 을 쓰는 사람은 대부분 개발 지식이 없어요. CLI 와 helper 가 돌려주는 다음 field 는 **internal verification primitives** 예요. SKILL 안에서는 이 field 들을 변수에 담아 helper 와 주고받되, **raw 값을 사용자 chat 에 echo 하면 안 돼요** (deploy SKILL 의 동일 룰을 따라요):

- `schema_version` (예: `bootstrap/v1`) — API 응답 검증용. raw 값 echo 금지.
- `items[].id`, `items[].folder_name`, `items[].name`, `items[].resource_tier` — `axhub apps templates list` 가 반환한 backend template registry. id 는 사용자 발화 매칭에만 쓰고, raw 목록 dump 금지.
- `bootstrap_id`, `status_url`, `stage`, `app_id`, `deployment_id`, `repo_full_name`, `error_code`, `error_message` — bootstrap saga 의 internal verification primitives. raw 값 echo 금지 (단 `repo_full_name` 은 마지막 단계에서 `git clone` URL 로 사용자에게 보여줘요).
- `request_id`, `idempotency_key`, `installation_id`, `device_code` — internal correlation/auth primitives. raw 값 echo 금지. **예외**: CLI GitHub device-flow 준비 단계에서 `event: device_code_issued` 를 emit 하면 `verification_uri` (또는 `verification_uri_complete`) + `user_code` 쌍은 humanize 해서 사용자에게 즉시 보여줘요. 대화형 TTY 에서는 승인까지 polling 하고, 에이전트/비-TTY 에서는 event 직후 fast-exit 하므로 사용자는 안내를 못 보면 멈춘 줄 알아요. internal `device_code` 값 자체는 여전히 echo 금지.

대신 사용자에게는 한국어 한 줄로 진행 상황만 알려드려요. 예시:

| 시점 | 사용자 chat 한 줄 |
|------|-------------------|
| Step 1 CLI 존재 확인 | "axhub 도구가 있는지 보고 있어요." |
| Step 2 template 목록 조회 | "사용할 수 있는 템플릿을 확인하고 있어요." |
| Step 3 template 선택 | "어떤 종류 프로젝트를 만들지 골라요." |
| Step 4 앱 이름 입력 | "앱 이름을 정해요." |
| Step 5 bootstrap dry-run | "어떤 작업을 할지 미리 보여줘요." |
| Step 6 사용자 동의 | "지금 만들고 배포까지 한 번에 진행해요." |
| Step 7 bootstrap execute + watch | "앱 만들고, GitHub repo 만들고, 첫 배포까지 진행 중이에요. 보통 2~5분 정도 걸려요." |
| Step 7a GitHub 연결 필요 (`device_code_issued` 발생 시) | "GitHub 연결이 필요해요. 브라우저에서 `{verification_uri}` 열고 코드 `{user_code}` 를 입력해 주세요. 대화형 TTY 에서는 승인하면 자동으로 진행되고, 에이전트 컨텍스트에서는 안내 후 멈춰요." |
| Step 8 git clone | "코드를 현재 폴더로 가져와요." |
| Step 9 결과 안내 | "끝났어요. 이렇게 시작하면 돼요." |

raw helper JSON 이 디버깅에 필요한 환경 (개발 검증) 은 `AXHUB_INIT_VERBOSE=1` 환경변수가 켜진 경우에만 echo 해요. 기본 흐름은 항상 한 줄 자연어로 진행해요.

## Workflow

To start an axhub app:

**이 SKILL 이 앱 생성 전체를 담당해요.** "앱 만들어줘" / "그걸로 앱 만들고 싶어" 류 발화에 generic 한 스택/데이터-fetch 질문 (예: "어떤 스택으로 만들까요?", "데이터를 어떻게 가져올까요?") 을 즉석에서 만들지 말고, 반드시 아래 Step 1-8 template flow 를 따라요. 스택 선택은 Step 3 (backend template registry), 데이터 접근은 Step 8 의 `@ax-hub/sdk` 안내로 처리해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "CLI와 template registry 확인", status: "in_progress", activeForm: "CLI 확인 중" },
     { content: "template 선택", status: "pending", activeForm: "template 고르는 중" },
     { content: "앱 이름 입력", status: "pending", activeForm: "앱 이름 정하는 중" },
     { content: "bootstrap saga 실행 (app + repo + deploy)", status: "pending", activeForm: "bootstrap 진행 중" },
     { content: "git clone + 결과 안내", status: "pending", activeForm: "코드 가져오는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **CLI 존재를 확인해요.**

   ```bash
   axhub --version
   ```

   실패하면 `/axhub:install-cli` 안내를 짧게 보여주고 중단해요.

2. **Backend template registry 를 읽어요.**

   ```bash
   axhub apps templates list --json
   ```

   응답 envelope shape:

   ```json
   {
     "schema_version": "...",
     "data": {
       "items": [
         {"id": "<uuid>", "folder_name": "react-axhub", "name": "React (Axhub)", "resource_tier": "small"},
         {"id": "<uuid>", "folder_name": "nextjs-axhub", "name": "Next.js (Axhub)", "resource_tier": "small"},
         {"id": "<uuid>", "folder_name": "astro-axhub", "name": "Astro (Axhub)", "resource_tier": "micro"}
       ]
     }
   }
   ```

   `schema_version` 은 응답 검증용 internal primitive 예요 — raw 값을 사용자 chat 에 echo 하지 않아요. `items[]` 의 `id` 또는 built-in alias (`react` / `nextjs` / `astro`) 를 `--template` 인자로 써요.

   on exit 4 (auth 만료, CLI `axhub apps templates list` — 옛 sysexits 65 아님) → `/axhub:auth` 로 라우팅. on exit 8 (tenant 미해석) → `axhub profile current --json` 안내. 그 외 비정상 종료는 `/axhub:doctor` 권장.

## 템플릿 선택 가이드

이 가이드는 두 번째 registry 가 아니에요. 먼저 `axhub apps templates list --json` 로 backend 가 반환한 template 목록을 읽고, 그 안에 있는 alias / folder_name 에만 설명을 덧붙여요. 선택 값은 반드시 backend 가 반환한 `id` 또는 built-in alias (`react` / `nextjs` / `astro`) 여야 해요.

알 수 없는 새 template 이 backend 에서 오면 숨기지 않아요. 로컬 설명이 없는 항목은 backend `name` 과 `folder_name` 을 그대로 보여주고, "이름을 보고 고르면 돼요. 잘 모르겠으면 먼저 Next.js 추천을 봐요." 처럼 중립 안내만 덧붙여요.

| alias / folder | 이렇게 만들고 싶을 때 골라요 |
|---|---|
| `nextjs` (또는 `nextjs-axhub`) | 쇼핑몰, 예약, 결제, 로그인, 관리자 화면처럼 화면과 기능이 함께 있는 웹서비스를 만들 때 추천해요. |
| `astro` (또는 `astro-axhub`) | 회사 소개, 랜딩 페이지, 블로그, 문서처럼 글과 이미지 중심이고 자주 바뀌지 않는 사이트에 좋아요. |
| `react` (또는 `react-axhub`) | 로그인한 뒤 쓰는 설정 화면, 입력 폼, 관리 화면처럼 버튼을 눌러 내용이 자주 바뀌는 화면에 좋아요. |

backend 가 반환한 template 전체 목록은 먼저 텍스트로 보여줘요. structured AskUserQuestion 은 UI 제한에 맞춰 **최대 3개 선택지** 만 써요. 알려진 alias 는 위 설명을 짧게 붙이고, 알 수 없는 항목은 backend `name` 과 `folder_name` 을 붙여요. 항상 `취소` 선택지를 함께 보여줘요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — template 선택은 `abort`, 앱 이름은 `abort`, bootstrap consent 는 `취소` 예요.

3. **template 을 선택해요.**

   먼저 위 가이드와 backend 가 반환한 template 전체 목록을 텍스트로 보여줘요. 사용자가 발화에 exact alias 또는 backend folder_name 을 이미 적었다면 (예: "nextjs 앱 만들어줘") AskUserQuestion 없이 그 alias 로 진행해요. id 가 없으면 structured AskUserQuestion 은 3개 이하 선택지만 써요.

   ```json
   {
     "question": "어떤 템플릿으로 시작할까요?",
     "header": "템플릿",
     "options": [
       {"label": "Next.js 추천", "value": "nextjs", "description": "쇼핑몰·예약·결제·로그인·관리자 화면"},
       {"label": "직접 고르기", "value": "manual_template_id", "description": "위 목록에서 alias 또는 folder_name 을 말해요"},
       {"label": "취소", "value": "abort", "description": "프로젝트를 만들지 않아요"}
     ]
   }
   ```

   위 JSON 은 예시예요. `Next.js 추천` 은 backend 가 `nextjs` alias 또는 `nextjs-axhub` folder 를 반환할 때만 보여줘요. backend 가 두 표현 모두 반환하지 않으면 첫 번째 알려진 항목 하나를 추천 버튼으로 쓰거나, 추천 없이 `직접 고르기` + `취소` 만 보여줘요. `manual_template_id` 를 고르면 AskUserQuestion 을 다시 호출하지 말고, 이미 보여준 텍스트 목록에서 exact alias 또는 folder_name 을 한 번만 물어요. 사용자가 답한 값이 backend 목록에 없으면 saga 를 시작하지 말고 다시 목록을 보여줘요. subprocess 에서는 자동 선택하지 않아요.

4. **앱 이름을 정해요.**

   `--name` 은 bootstrap saga 의 required 인자예요. 사용자 발화에서 앱 이름을 유추할 수 있으면 (예: "결제 앱 만들어줘" → "결제 앱") AskUserQuestion 없이 그대로 써요. 없으면 한 번 물어요.

   ```json
   {
     "question": "앱 이름 뭘로 할래요?",
     "header": "앱 이름",
     "options": [
       {"label": "지금 발화 기준 자동", "value": "auto_from_utterance", "description": "발화에서 유추한 이름을 그대로 써요"},
       {"label": "직접 입력", "value": "manual_name", "description": "원하는 이름을 한 번만 말해요"},
       {"label": "취소", "value": "abort", "description": "프로젝트를 만들지 않아요"}
     ]
   }
   ```

   `--slug` 는 자동 유도해요 (이름을 소문자화 + 공백 → 하이픈, 특수문자 제거). slug 가 backend 정책과 충돌하면 saga 가 `error_code` 로 알려주고 SKILL 이 다시 한 번 물어요.

5. **Bootstrap dry-run 으로 미리보기를 만들어요.**

   ```bash
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --dry-run --json
   ```

   응답 envelope 의 미리보기 카드 (template / slug / subdomain / repo_name / private/public / installation_id 후보) 를 사용자에게 한국어 한 줄씩 보여줘요. raw JSON dump 금지. `--dry-run` default 가 true 라 명시적으로 안 적어도 같지만, 가독성을 위해 명시해요.

6. **사용자 동의 + execute.**

   ```json
   {
     "question": "지금 만들고 배포까지 진행할까요?",
     "header": "앱 만들기",
     "options": [
       {"label": "진행", "value": "execute", "description": "backend app + GitHub repo + 첫 deploy 를 자동으로 진행해요"},
       {"label": "취소", "value": "취소", "description": "지금은 만들지 않아요"}
     ]
   }
   ```

   동의 받으면 saga 를 실행해요:

   ```bash
   axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --execute --yes --watch --watch-timeout 9m --json
   ```

   **에이전트도 terminal 까지 폴링해요 (axhub-cli 0.15.3+).** bare `--watch` 는 비-TTY/에이전트 컨텍스트에서 단일 스냅샷으로 degrade 하지만, `--watch-timeout` (또는 `--watch-interval`) 을 붙이면 explicit streaming override 라 CLI 가 degrade 하지 않고 terminal(saga 완료 / 실패) 까지 직접 폴링해요. 그래서 saga 가 끝까지 진행돼 `repo_full_name` 을 받을 수 있어요 (snapshot 으로 끊기지 않아요). `--no-input` 같은 플래그는 따로 안 붙여도 돼요 (비-TTY 면 CLI 가 자동 감지). 이 bash 는 Bash tool `timeout: 570000` (9.5분) 으로 호출하고, 9분 초과 시 CLI Timeout + resume hint 를 받으면 "아직 만드는 중이에요, 계속 확인할게요" 후 아래 bootstrap-status 를 한 번 더 호출해요:

   ```bash
   BOOTSTRAP_ID=$(echo "$ACCEPTED_JSON" | jq -r '.data.bootstrap_id')
   axhub apps bootstrap-status "$BOOTSTRAP_ID" --watch --watch-timeout 9m --json
   ```

   진행 중 매 ~30s 마다 한국어 한 줄로 narrate 해요 — "앱 만들고 있어요" / "GitHub repo 만들고 있어요" / "첫 배포 중이에요. 거의 다 왔어요". 60s 이상 같은 stage 머무르면 "조용하네요, 계속 기다리고 있어요" 한 줄을 추가해요.

   **GitHub 연결 필요 (CLI stdout 의 `device_code_issued` event 처리).** CLI 가 GitHub App 미설치 / installation 만료 / scope 부족 상태를 해결하려고 device flow 를 시작하면, backend saga 를 시작하기 전에 다음 JSON line 을 emit 해요 (대화형 TTY 면 그 뒤 승인까지 polling 하고, 에이전트/비-TTY 면 event 직후 fast-exit 해요):

   ```json
   {"event":"device_code_issued","data":{"verification_uri":"https://github.com/login/device","verification_uri_complete":null,"user_code":"XXXX-XXXX","expires_in":899}}
   ```

   이 event 가 stdout 에서 나오면 narrate 보다 우선해서 즉시 사용자에게 한국어로 안내해요. raw JSON dump 금지 — `verification_uri` (또는 `verification_uri_complete` 가 non-null 이면 그걸 우선), `user_code`, `expires_in` 만 humanize 해요:

   > GitHub 연결이 필요해요. 다음 단계로 진행해 주세요:
   >
   > 1. 브라우저에서 열기: `<verification_uri_complete 우선, 없으면 verification_uri>`
   > 2. 코드 입력: `<user_code>`
   > 3. axhub GitHub App 설치 승인
   >
   > 대화형 TTY 에서는 브라우저 승인 뒤 CLI 가 폴링을 계속해 다음 단계를 이어가요. 에이전트 컨텍스트에서는 이 안내를 보여준 뒤 멈춰요. (유효시간 약 `<expires_in/60>` 분)

   `verification_uri_complete` 가 있으면 코드가 자동 입력되니 2번을 생략해도 돼요. 그 다음은 컨텍스트에 따라 갈라져요 (axhub-cli 0.15.3+): **대화형 TTY** 면 saga 가 polling 으로 GitHub App install 완료를 기다리니까 SKILL 은 stdout 의 다음 stage event (예: `app_created`, `repo_created`) 가 도착할 때까지 narrate 만 계속해요. **에이전트 / 비-TTY 컨텍스트** 면 CLI 가 `device_code_issued` emit 직후 fast-exit 하므로 다음 stage event 가 안 와요. challenge 를 보여준 뒤 멈추고, "이 호출은 승인 완료를 polling 하지 않아요. 계속하려면 대화형 터미널에서 `axhub init` 을 다시 실행해 새 device flow 를 완료해 주세요" 라고 안내해요. 이전 `user_code` 를 승인한 뒤 같은 에이전트 명령을 재호출해도 이어지지 않아요. CLI 가 internal `device_code` 를 노출하지 않기 때문이에요. 완전 autonomous 완료는 CLI device_code persist resume 기능을 기다려요 (`docs/superpowers/specs/2026-05-25-github-device-flow-surface-design.md`). 코드가 expire 되면 `/axhub:github` 안내로 재시도해요.

   상세한 device flow 안내 패턴은 `../github/SKILL.md` 의 OAuth device flow 섹션을 따라요.

7. **응답에서 `repo_full_name` 을 꺼내 CWD 로 받아요.**

   ```bash
   REPO=$(echo "$FINAL_JSON" | jq -r '.data.status.repo_full_name // empty')
   if [ -z "$REPO" ]; then
     echo '{"systemMessage":"GitHub repo 정보가 응답에 없어요. /axhub:doctor 로 진단해주세요."}'
     exit 65
   fi
   if [ -d .git ]; then
     echo "{\"systemMessage\":\"현재 dir 에 이미 .git 이 있어요. 자동 clone 건너뛸게요. 수동으로 origin 을 붙이려면: git remote add origin https://github.com/${REPO}.git && git fetch origin && git checkout -b main origin/main\"}"
   else
     git init -q -b main
     git remote add origin "https://github.com/${REPO}.git"
     git fetch origin --quiet --depth=1
     DEFAULT_BRANCH=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo main)
     git reset --hard "origin/$DEFAULT_BRANCH"
     git branch --set-upstream-to="origin/$DEFAULT_BRANCH" "$DEFAULT_BRANCH" 2>/dev/null || true
   fi
   ```

   항상 현재 dir (CWD) 에 코드를 받아요. 서브 dir (`$APP_SLUG/`) 만들지 않아서 사용자가 "cd $APP_SLUG" 추가 단계 안 해도 돼요. `.claude` / `.omc` / `.codegraph` 같은 IDE/도구 메타 dir 은 untracked 로 자연스럽게 보존돼요 (`git reset --hard` 는 tracked 파일만 건드려요). 이미 `.git` 이 있는 dir 은 안전을 위해 자동 clone 건너뛰고 수동 명령 안내를 한 줄로 보여줘요. clone 실패 시 (권한 / network) `repo_full_name` 만 사용자에게 알려주고 수동 clone 안내를 보여줘요.

8. **결과와 다음 액션을 안내해요.**

   saga 응답의 `app_id` / `deployment_id` / `repo_full_name` 을 humanize 해서 한국어 한 줄씩 보여줘요. 예시:

   ```
   끝났어요. 이렇게 시작하면 돼요:
   1. 의존성 설치 — package manager 자유 (`npm i` / `pnpm i` / `bun install`)
   2. 로컬 실행 — README 의 dev 스크립트 (`npm run dev` 등)
   3. 배포 상태 보기 — `/axhub:status` (방금 만든 첫 배포 진행 상황)
   4. 다음 배포 — 코드 수정 후 `/axhub:deploy`
   5. 데이터 읽기 — axhub API 를 raw fetch 로 직접 부르지 말고 template 에 설치된 @ax-hub/sdk 를 써요.
      import { AxHubClient } from '@ax-hub/sdk' 로 client 만든 뒤
      sdk.tenant(...).app(...).data.discover('<table>') 로 읽어요.
      전체 사용법: https://www.npmjs.com/package/@ax-hub/sdk
   ```

   `error_code` 로 saga 가 실패했으면 다음 routing 을 써요:
   - `github.installation_missing` / `github.repo_create_failed` → `/axhub:github` 가이드
   - `validation.template_not_found` → Step 2 로 돌아가 다시 목록을 보여줘요
   - `validation.slug_collision` → Step 4 로 돌아가 새 이름을 받아요
   - `auth` (CLI exit 4, auth 만료 — 옛 sysexits 65 아님) → `/axhub:auth`
   - `forbidden` / `tenant_scope` (CLI exit 12 / 8, 권한·scope 부족) → 사용자에게 권한 부족 안내 + workspace admin 문의
   - 그 외 → `/axhub:doctor`

## NEVER

- NEVER `axhub init` 또는 `axhub init --from-template` 을 호출하지 않아요. Rust v1.0.0-rc.1 에서 `--from-template` flag 가 미구현 stub (`initcmd.rs` run() 미사용) 이라 호출해도 generic docker manifest 만 만들어져요. SKILL 은 `axhub apps bootstrap` saga 만 써요.
- NEVER `axhub apps create` 또는 `axhub deploy create` 를 직접 호출하지 않아요. bootstrap saga 가 server-side 에서 둘 다 처리해요.
- NEVER `axhub-helpers fetch-template` 또는 remote `templates.json` 을 source 로 쓰지 않아요. backend `axhub apps templates list` 만 source-of-truth 예요.
- NEVER subprocess (`$CI` / `$CLAUDE_NON_INTERACTIVE` / no TTY) 에서 template 또는 앱 이름을 임의로 고르지 않아요. registry safe_default 가 `abort` 또는 `취소` 예요.
- NEVER `--execute` 를 `--dry-run` 미리보기 + 사용자 동의 없이 호출하지 않아요. backend app + GitHub repo + deploy 가 한 번에 mutate 돼요.
- NEVER auth 만료를 template 조회 실패로 오해하지 않아요. CLI auth 실패 (exit 4 / error_code `auth`, 옛 sysexits 65 아님) 는 `/axhub:auth` 로 라우팅 해요.
- NEVER `bootstrap --execute` 호출 직후 별도 `axhub deploy create` 를 다시 부르지 않아요. saga 가 첫 deploy 까지 포함해요.
- NEVER saga stdout 에서 `event: device_code_issued` 가 나왔는데 `verification_uri` + `user_code` 를 사용자에게 즉시 보여주지 않고 silent 하게 narrate 만 반복하지 않아요. saga 가 GitHub App install 승인을 기다리며 block 돼서 사용자는 SKILL 이 멈춘 줄 알아요. internal `device_code` raw 값은 여전히 echo 금지 — humanize 대상은 `verification_uri` + `user_code` + `expires_in` 만이에요.
- NEVER `repo_full_name` 응답이 비어 있는데 임의 URL 을 만들어 clone 시도하지 않아요. 응답이 비면 `/axhub:doctor` 로 라우팅 해요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template (Step 8 에서 사용).
- `../github/SKILL.md` — OAuth device flow surface 패턴 (Step 6 의 `device_code_issued` event 처리 기준).
