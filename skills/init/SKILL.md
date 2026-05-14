---
name: init
description: '이 스킬은 사용자가 새 axhub 앱을 만들거나 템플릿으로 프로젝트를 시작하고 싶어할 때 사용해요. 다음 표현에서 활성화: "결제 앱 만들어", "결제 앱 만들어줘", "빈 디렉토리", "새 앱 만들어", "새 앱 만들어줘", "앱 만들어줘", "프로젝트 만들어", "프로젝트 초기화", "프로젝트 초기화해줘", "apphub.yaml 만들어", "apphub.yaml 만들어줘", "axhub.yaml 만들어", "axhub.yaml 만들어줘", "fastapi 앱", "FastAPI 앱 만들어줘", "next.js 앱", "Next.js 앱 만들어줘", "nextjs 앱", "init", "scaffold", 또는 빈 디렉토리에서 새 앱 시작 의도. ax-hub-cli 의 init template 목록을 보여주고 선택한 template 으로 scaffold 해요.'
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
multi-step: true
needs-preflight: false
allows-dependency-execution: true
model: sonnet
---

# Init

새 axhub 앱을 현재 CLI 템플릿 목록에서 시작해요. v0.2.0 에서는 패키지 설치와 원격 template 내려받기를 하지 않고 `axhub --json init --list-templates` 를 단일 정답 소스로 써요. Sprint 3 부터 프로젝트 파일을 만든 뒤에는 helper bootstrap 을 plan-only (계획만 보기) 모드로만 호출해서 다음 안전 단계를 보여줘요.

## Workflow

To start an axhub app:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "CLI와 template registry 확인", status: "in_progress", activeForm: "CLI 확인 중" },
     { content: "template 선택", status: "pending", activeForm: "template 고르는 중" },
     { content: "axhub init 실행", status: "pending", activeForm: "프로젝트 만드는 중" },
     { content: "다음 액션 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **CLI 존재를 확인해요.**

   ```bash
   axhub --version
   ```

   실패하면 install/update 안내를 짧게 보여주고 중단해요. auth 는 init template 목록 조회에 필요하지 않아요.

2. **CLI registry 에서 template 목록을 읽어요.**

   ```bash
   axhub --json init --list-templates
   ```

   `schema_version` 은 helper API 응답 검증용 **internal verification primitive** 예요 — `init/v1` 인지 확인만 하고 raw 값을 사용자 chat 에 echo 하면 안 돼요 (deploy SKILL Visibility Rules 와 같은 규칙). CLI가 반환한 template 만 선택 후보로 쓰고, `templates[].id`, `framework`, `description` 에 아래 로컬 가이드를 덧붙여 보여줘요.

## 템플릿 선택 가이드

이 가이드는 두 번째 registry 가 아니에요. 먼저 `axhub --json init --list-templates` 로 CLI가 반환한 template 목록을 읽고, 그 안에 있는 id 에만 설명을 덧붙여요. 선택 값은 반드시 CLI가 반환한 template id 여야 해요.

알 수 없는 새 template 이 CLI에서 오면 숨기지 않아요. 로컬 설명이 없는 항목은 CLI의 `framework` 와 `description` 을 그대로 보여주고, “CLI 설명을 보고 고르면 돼요. 잘 모르겠으면 먼저 Next.js 계열 추천 항목을 봐요.”처럼 중립 안내만 덧붙여요.

| template id | 이렇게 만들고 싶을 때 골라요 |
|---|---|
| `nextjs-axhub` | 쇼핑몰, 예약, 결제, 로그인, 관리자 화면처럼 화면과 기능이 함께 있는 웹서비스를 만들 때 추천해요. 자동 선택은 아니고 사용자가 고를 때만 실행해요. |
| `astro-axhub` | 회사 소개, 랜딩 페이지, 블로그, 문서처럼 글과 이미지 중심이고 자주 바뀌지 않는 사이트에 좋아요. |
| `vite-react-axhub` | 로그인한 뒤 쓰는 설정 화면, 입력 폼, 관리 화면처럼 버튼을 눌러 내용이 자주 바뀌는 화면에 좋아요. |
| `remix-axhub` | 입력한 내용을 바로 저장하고, 페이지 이동 중에도 자연스럽게 이어지는 서비스에 좋아요. 예약, 신청서, 설문, 주문처럼 작성하고 제출하는 흐름이 많다면 Next.js 대신 고려해요. |
| `express-axhub` | 화면은 거의 없고, 다른 앱이 요청하면 주문 처리나 데이터 저장 같은 일을 해주는 서버가 필요할 때 골라요. |
| `hono-axhub` | 아주 작고 빠른 연결용 서버를 만들 때 골라요. 예를 들면 외부 서비스가 부르면 바로 응답하는 작은 기능이에요. |

CLI가 반환한 template 전체 목록은 먼저 텍스트로 보여줘요. structured AskUserQuestion 은 UI 제한에 맞춰 **최대 3개 선택지**만 써요. 알려진 id 는 위 설명을 짧게 붙이고, 알 수 없는 id 는 CLI `description` 과 `framework` 를 붙여요. 항상 `취소` 선택지를 함께 보여줘요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — template 선택은 `abort` 예요.

3. **template 을 선택해요.**

   먼저 위 가이드와 CLI가 반환한 template id 전체 목록을 텍스트로 보여줘요. 사용자가 발화에 exact template id 를 이미 적었다면 AskUserQuestion 없이 그 id 로 진행해요. id 가 없으면 structured AskUserQuestion 은 3개 이하 선택지만 써요.

   ```json
   {
     "question": "어떤 템플릿으로 시작할까요?",
     "header": "템플릿",
     "options": [
       {"label": "Next.js 추천", "value": "nextjs-axhub", "description": "쇼핑몰·예약·결제·로그인·관리자 화면"},
       {"label": "직접 고르기", "value": "manual_template_id", "description": "위 목록에서 exact template id 를 말해요"},
       {"label": "취소", "value": "abort", "description": "프로젝트를 만들지 않아요"}
     ]
   }
   ```

   위 JSON 은 예시예요. `Next.js 추천` 은 CLI가 `nextjs-axhub` 를 반환할 때만 보여줘요. CLI가 `nextjs-axhub` 를 반환하지 않으면 첫 번째 알려진 template 하나를 추천 버튼으로 쓰거나, 추천 없이 `직접 고르기` + `취소` 만 보여줘요. `manual_template_id` 를 고르면 AskUserQuestion 을 다시 호출하지 말고, 이미 보여준 텍스트 목록에서 exact template id 를 한 번만 물어요. 사용자가 답한 id 가 CLI 목록에 없으면 파일을 만들지 말고 다시 목록을 보여줘요. subprocess 에서는 자동 선택하지 않아요.

4. **선택된 template 으로 프로젝트 파일을 만들어요.**

   ```bash
   axhub init --from-template "$TEMPLATE_ID" --json
   ```

   이 명령은 앱 설정 파일 (`apphub.yaml`, 또는 옛 방식인 `axhub.yaml`) 을 자동으로 만들어요. 이 파일에는 앱 이름과 배포 정보가 담겨 있고, 다음 단계 (앱 등록과 배포) 에서 사용해요. 둘 다 정상 결과로 다뤄요.


5. **Bootstrap plan-only 로 다음 안전 단계를 보여줘요.** 프로젝트 파일이 만들어진 뒤에는 원격 앱 생성이나 배포를 실행하지 말고, Rust bootstrap FSM 을 읽기/계획 모드로만 호출해요.

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers bootstrap --dry-run --json
   ```

   `consent_required_apps_create`, `git_init_required`, `first_commit_required`, `template_required`, `conflict_existing_files` 같은 상태는 helper 가 다음 단계를 알려주는 **internal verification primitive** 예요 — raw 식별자를 사용자 chat 에 그대로 echo 하면 안 돼요 (deploy SKILL Visibility Rules 와 같은 규칙). 대신 한국어 한 줄로 humanize 해서 알려드려요 (예: "앱 등록 동의가 필요해요" / "저장 지점을 먼저 만들어야 해요" / "템플릿을 골라야 해요" / "현재 폴더에 이미 같은 이름 파일이 있어요"). 앱 등록이나 배포는 deploy/apps 흐름으로 이어가요. init 은 파일 생성까지만 맡고, `bootstrap --auto-chain`, `apps create`, `deploy create` 는 실행하지 않아요.

6. **결과와 다음 액션을 안내해요.** 아래 5단계를 정해진 순서와 라벨로만 보여줘요. 임의로 "(선택)" 같은 라벨을 붙이거나 단계 순서를 바꾸지 않아요. backend 가 강제하는 단계 (앱 등록, GitHub 연결, 배포) 는 스킵 안내를 만들지 않아요.

   ```
   다음 안전 단계예요:
   1. 앱 등록 — `axhub 앱 만들어줘` 로 apphub.yaml 을 서버에 등록해요.
   2. 의존성 설치 — `의존성 설치해` 라고 말하면 쓰는 패키지 매니저로 깔아드려요.
   3. GitHub 연결 (배포에 꼭 필요해요) — `깃허브 연결` 로 repo 와 axhub 앱을 묶어요.
   4. 환경 변수 — `환경변수 추가` 로 API 키 / DB URL 등을 주입해요.
   5. 배포 — `배포해줘` 로 라이브로 띄워요.
   ```

### Dependency install (lockfile-aware)

이 subsection 의 단계는 `D1.` ~ `D5.` 로 별도 namespace 를 써요. workflow 의 top-level Step 0~6 과 번호 충돌을 막기 위해서예요.

D1. plan 을 조회해요:
   `!${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers bootstrap dependency-plan --json`
   결과의 `recommended_command` 필드 보존.

D2. AskUserQuestion `dependency_install_strategy` (default `inline_session`, options `inline_session` / `manual_terminal` / `skip`) fire.

D3. 사용자가 `inline_session` 선택 시:
   - lockfile 단일 (`requires_pm_choice == false`) → `recommended_command` 필드를 그대로 inline 실행 (`!<recommended_command>`)
   - lockfile 다중 (`requires_pm_choice == true`) → `package_manager_choice` AskUserQuestion fire 후 선택된 manager 의 `recommended_command` inline 실행

D4. 실행 후 verify 단계:
   helper 가 자동으로 byte-identical 검사 + log file 저장 (TOCTOU race 차단을 위한 atomic mode 또는 직전 re-verify).

D5. 실패 시 에러 분류 후 사용자에게 generic fallback 메시지 출력 (catalog 5 entry follow-up).

설치가 60초+ 걸리면 ctrl-C 로 멈추고 strategy 재선택할 수 있어요.

## NEVER

- NEVER init 흐름에서 `axhub-helpers bootstrap --auto-chain` 을 실행하지 않아요. init 은 `bootstrap --dry-run --json` 으로 다음 단계만 보여줘요.
- NEVER init 흐름에서 `axhub apps create` 또는 `axhub deploy create` 를 실행하지 않아요.

- NEVER Node, package manager, dependency install 을 자동 실행하지 않아요. 단 다음 모든 조건을 동시에 만족하는 경우에만 예외를 허용해요:
  1. SKILL frontmatter 에 `allows-dependency-execution: true` 선언
  2. inline `!` prefix 가 호출하는 명령이 `axhub-helpers bootstrap dependency-plan --json` 가 emit 한 `recommended_command` 필드와 byte-identical
  3. user 가 AskUserQuestion 의 "install dependencies" option 을 explicit 선택
- NEVER helper `fetch-template` 또는 remote `templates.json` 를 v0.2.0 source 로 쓰지 않아요.
- NEVER subprocess 에서 template 을 임의로 고르지 않아요.
- NEVER auth 실패를 init template 조회 실패로 오해하지 않아요.
