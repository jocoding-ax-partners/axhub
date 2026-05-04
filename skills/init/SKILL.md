---
name: init
description: 이 스킬은 사용자가 새 axhub 앱을 만들거나 템플릿으로 프로젝트를 시작하고 싶어할 때 사용해요. 다음 표현에서 활성화: "새 앱 만들어줘", "결제 앱 만들어줘", "프로젝트 만들어", "Next.js 앱 만들어줘", "FastAPI 앱 만들어줘", "init", "scaffold", "axhub.yaml 만들어줘", "apphub.yaml 만들어줘", 또는 빈 디렉토리에서 새 앱 시작 의도. ax-hub-cli 의 init template 목록을 보여주고 선택한 template 으로 scaffold 해요.
multi-step: true
needs-preflight: false
---

# Init

새 axhub 앱을 current CLI template registry 에서 시작해요. v0.2.0 에서는 helper bootstrap, dependency install, remote template fetch 를 하지 않고 `axhub --json init --list-templates` 를 source of truth 로 써요.

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

1. **CLI 존재를 확인해요.**

   ```bash
   axhub --version
   ```

   실패하면 install/update 안내를 짧게 보여주고 중단해요. auth 는 init template 목록 조회에 필요하지 않아요.

2. **CLI registry 에서 template 목록을 읽어요.**

   ```bash
   axhub --json init --list-templates
   ```

   `schema_version` 이 `init/v1` 인지 확인하고, CLI가 반환한 template 만 선택 후보로 써요. `templates[].id`, `framework`, `description` 에 아래 로컬 가이드를 덧붙여 보여줘요.

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

AskUserQuestion 선택지는 CLI가 반환한 template 들로 만들어요. 알려진 id 는 위 설명을 짧게 붙이고, 알 수 없는 id 는 CLI `description` 과 `framework` 를 붙여요. 항상 `취소` 선택지를 함께 보여줘요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — template 선택은 `abort` 예요.

3. **template 을 선택해요.**

   먼저 위 가이드를 보여주고, CLI가 반환한 template id 로 AskUserQuestion 선택지를 만들어요. 가능한 경우 template 을 바로 고르게 하고, 선택지 값은 exact template id 를 써요.

   ```json
   {
     "question": "어떤 템플릿으로 시작할까요?",
     "header": "템플릿",
     "options": [
       {"label": "Next.js 추천", "value": "nextjs-axhub", "description": "쇼핑몰·예약·결제·로그인·관리자 화면"},
       {"label": "Astro 추천", "value": "astro-axhub", "description": "회사 소개·랜딩·블로그·문서"},
       {"label": "Vite 화면", "value": "vite-react-axhub", "description": "설정 화면·입력 폼·관리 화면"},
       {"label": "Remix 흐름", "value": "remix-axhub", "description": "신청서·설문·주문처럼 작성 후 제출"},
       {"label": "Express 서버", "value": "express-axhub", "description": "주문 처리·데이터 저장 중심"},
       {"label": "Hono 연결", "value": "hono-axhub", "description": "외부 서비스가 부르는 작은 기능"},
       {"label": "취소", "value": "abort", "description": "scaffold 를 실행하지 않아요"}
     ]
   }
   ```

   위 JSON 은 예시예요. 실제 선택지는 runtime CLI 목록에 맞춰 만들고, CLI가 반환한 template 만 보여줘요. template 선택지의 `value` 는 exact template id 여야 하고, 항상 `취소` 선택지를 함께 보여줘요. UI 선택지 수 제한이 있으면 가이드를 먼저 보여준 뒤 사용자가 원하는 exact template id 를 고르게 해요. subprocess 에서는 자동 선택하지 않아요.

4. **선택된 template 으로 scaffold 해요.**

   ```bash
   axhub init --from-template "$TEMPLATE_ID" --json
   ```

   visible examples 는 `apphub.yaml` 을 만들 수 있고, legacy builtin flow 는 `axhub.yaml` 을 만들 수 있어요. 둘 다 정상 결과로 다뤄요.

5. **결과와 다음 액션을 안내해요.** 앱 등록, GitHub 연결, env 설정, deploy, open 흐름을 자연어로 이어갈 수 있다고 짧게 말해요.

## NEVER

- NEVER Node, package manager, dependency install 을 자동 실행하지 않아요.
- NEVER helper `fetch-template` 또는 remote `templates.json` 를 v0.2.0 source 로 쓰지 않아요.
- NEVER subprocess 에서 template 을 임의로 고르지 않아요.
- NEVER auth 실패를 init template 조회 실패로 오해하지 않아요.
