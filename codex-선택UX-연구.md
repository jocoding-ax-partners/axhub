# Codex 선택 UX 연구 — AskUserQuestion 등가물을 어디까지 재현할 수 있나

> 조사일 2026-08-20 · 대상 codex-cli **0.148.0**(로컬 설치본) + openai/codex `rust-v0.148.0` 태그 소스 + main 브랜치 교차 확인
> 방법: dynamic workflow 6-lane 병렬 실측 → lane 별 핵심 주장 3개를 별도 에이전트가 **반박 전제**로 재검증 (24 에이전트, 검증 18건 중 REFUTED 0 · PARTIALLY_TRUE 4)
> 선행 문서: `codex-플러그인-호환-연구.md` (이 문서가 그 §2.2 "AskUserQuestion 등가물" 한 줄을 대체·확장해요)

---

## 0. 한 문단 결론

**Codex 에도 Claude AUQ 와 사실상 동형인 네이티브 선택 위젯이 이미 구현돼 있어요 — 막힌 건 위젯이 아니라 모드 게이트예요.** `request_user_input` 도구는 0.148.0 에 실존하고 (질문·헤더·선택지 label/description 까지 AUQ 와 거의 1:1), TUI 는 이걸 방향키+Enter 로 고르는 bottom-pane 오버레이로 렌더해요 — 텍스트 목록이 아니에요 `[confirmed]`. 다만 호출 가능 모드가 **Plan 모드 하나**이고, Default 모드는 `default_mode_request_user_input` feature flag(UnderDevelopment·기본 false)로만 열려요. 이 flag 는 사용자 `~/.codex/config.toml` 한 줄로 켤 수 있는 **완전히 열린 표면**이지 컴파일 타임 게이팅이 아니에요 `[confirmed]`. 두 번째 경로인 **MCP elicitation 은 오늘 기본 설정에서 동작하는 유일한 plugin-invocable 네이티브 선택 UI** 이고 `[confirmed]`, codex plugin manifest 가 `.mcp.json` 동봉을 지원해서 번들이 직접 실어 보낼 수 있어요 — 하지만 elicitation 답변은 codex Guardian 신뢰 모델에서 **untrusted** 로 취급되고 `approval_policy=never` 세션에선 소리 없이 자동 Decline 되므로, AP-12 같은 파괴적 실행 승인 게이트를 여기에 옮기면 안 돼요. 그래서 권고는 **3단 tier** 예요 — (T0) 모두에게 기본으로 동작하는 텍스트 프로토콜을 codex 자체 규격에 맞게 다시 쓰고, (T1) 한 줄 opt-in 으로 네이티브 위젯을 켜는 경로를 온보딩에 넣고, (T2) MCP elicitation 은 템플릿 선택처럼 **비파괴 선택지에만** 쓰는 거예요. 그리고 지금 관측되는 "1) 2) 몇 번?" 퇴화는 codex 자체 지침 위반이에요 — Default 모드 시스템 템플릿이 *"Never write a multiple choice question as a textual assistant message"* 를 명시하고 있어요 `[confirmed]`.

---

## 1. codex 가 사용자에게 묻는 경로 — 전수 5종

app-server 프로토콜의 서버→클라이언트 요청(`ServerRequest.json`, `codex app-server generate-json-schema` 로 직접 덤프)이 정본이에요. **이 밖의 경로는 없어요** `[confirmed]`.

| # | 경로 | 사용자에게 보이는 것 | 플러그인이 부를 수 있나 | 기본 활성 | 판정 |
|---|---|---|---|---|---|
| 1 | `item/tool/requestUserInput` (**request_user_input**) | 방향키+Enter 선택 오버레이, 질문별 notes(Tab), `None of the above` 자동 추가, `Question i/N` 진행 표시 | 모델이 직접 호출 (skill 이 지시 가능) | ❌ Plan 모드만 (Default 는 flag) | **T1 — opt-in 한 줄로 열림** |
| 2 | `mcpServer/elicitation/request` (**MCP elicitation**) | single-select enum·boolean 은 Select 위젯, string 은 입력창, Esc=cancel / Enter=submit | MCP 서버가 tool call 처리 중 발신 (번들 `.mcp.json` 가능) | ✅ 기본 on | **T2 — 비파괴 선택 전용** |
| 3 | `item/commandExecution/requestApproval` | 명령 승인 팝업 (`Yes, proceed` / `Yes, and don't ask again…` / `No, and tell Codex…`) | ❌ 선택지 문구가 codex 하드코딩 | 정책 의존 | **배제** |
| 4 | `item/fileChange/requestApproval` | 파일 변경 diff 승인 | ❌ 동일 | 정책 의존 | **배제** |
| 5 | `item/permissions/requestApproval` | 권한 상승 승인 (`request_permissions` 도구) | ❌ 승인 의미론 고정 | flag off | **배제** |

> 배제 3종의 이유: 선택지 label 이 codex 가 생성하는 승인 의미론으로 고정돼 있어 임의 N-지선다 질문으로 전용할 수 없고, `approval_policy=never` 에선 아예 뜨지 않아요 `[confirmed — tui/src/bottom_pane/approval_overlay.rs:820-890]`.

---

## 2. `request_user_input` 정밀 해부 — AUQ 와 얼마나 같은가

### 2.1 스키마 대조

| 항목 | Claude `AskUserQuestion` | Codex `request_user_input` | 차이의 실질 |
|---|---|---|---|
| 질문 수 | 1~4 | *Prefer 1, do not exceed 3* | 우리 게이트는 전부 1문항이라 무영향 |
| 헤더 | `header` ≤12자 chip | `header` ≤12자 | 동일 |
| 질문문 | 자유 문장 | 한 문장 권장 | 동일 |
| 선택지 | 2~4개 `{label, description, preview?}` | **2~3개** `{label 1-5단어, description 한 문장}` | 템플릿 선택 4개 이상이면 3개로 압축 필요 |
| 추천 표기 | 첫 옵션 + `(Recommended)` 관례 | 스펙 본문이 *"Put the recommended option first and suffix (Recommended)"* 로 **명문화** | codex 가 더 엄격 |
| 자유 입력 | `Other` 자동 제공 | `is_other=true` 를 클라이언트가 **강제 주입** | 동일 |
| 다중 선택 | `multiSelect: true` | **없음** | 다중 선택은 질문 분할로만 |
| 비밀 입력 | 없음 | `isSecret` 있음 | codex 우위 |
| 응답 형태 | 선택 label | `{질문id: {answers: [문자열]}}` (label + notes 합침) | id 를 snake_case 로 고정하면 분기 매핑 안정 |

`[confirmed — codex-rs/core/src/tools/handlers/request_user_input_spec.rs:16-90, protocol/src/request_user_input.rs; 동일 문자열이 로컬 0.148.0 바이너리에 실재]`

**결론: 표현력 손실이 거의 없어요.** AP-12 진입 확인(`진행`/`취소`)·bootstrap 앱 이름 확인·템플릿 선택 모두 1:1 로 옮겨져요.

### 2.2 게이트는 3층이고, 막힌 건 2층 하나뿐

| 층 | 무엇 | 0.148.0 기본값 | 우리가 통제 가능? |
|---|---|---|---|
| ① 등록 | 모델 tool list 에 실릴 것인가 | ✅ **등록됨** (`tools.experimental_request_user_input` 기본 enabled) | 건드릴 필요 없음 |
| ② 호출 모드 | 어느 collaboration mode 에서 호출 허용 | ❌ **Plan 만** (`allows_request_user_input()` = Plan) | **사용자 config 한 줄** |
| ③ blocking | 답을 기다리나 | Plan=blocking / 그 외=**non-blocking** | skill 문안으로 fail-closed 처리 |

`[confirmed — protocol/src/config_types.rs:667-683, tools/src/tool_config.rs:38-47, core/src/config/mod.rs:2572-2578, core/src/tools/handlers/request_user_input.rs:78-82]`

선행 연구의 "기본 모드에서 비활성" 은 **등록이 아니라 호출 게이트** 문제였어요. 모델은 Default 모드에서도 도구를 보지만 description 이 "Plan mode 전용" 이라 부르지 않고, 억지로 부르면 `request_user_input is unavailable in Default mode` 에러를 받아요.

### 2.3 ② 를 여는 방법 — 사용자에게 완전히 열려 있어요

```toml
# ~/.codex/config.toml
[features]
default_mode_request_user_input = true
```

같은 효과의 다른 두 경로도 있어요 `[confirmed]`:
- `codex features enable default_mode_request_user_input`
- `codex -c features.default_mode_request_user_input=true` (세션 1회용)

UnderDevelopment 단계라 경고 한 줄이 뜨는데 그것도 `suppress_unstable_features_warning=true` 로 끌 수 있고, **차단 로직은 없어요** (`features/src/lib.rs:494` 의 `apply_map` 이 stage 필터 없이 TOML 을 그대로 반영) `[confirmed]`.

### 2.4 ③ 이 만드는 함정 — 2분 방치가 승인으로 새요

Default 모드(flag on)에서는 `is_blocking=false` 라, 사용자가 **~60초(숨김 유예) + 60초(카운트다운)** 동안 아무것도 안 하면 TUI 가 **빈 답변으로 auto-resolve** 해서 모델에게 돌려줘요 `[confirmed — tui/src/bottom_pane/request_user_input/mod.rs:70-71, PR #36410 2026-08-01]`.

> **이게 이 연구에서 가장 위험한 발견이에요.** AP-12 게이트를 이 도구로 옮기면서 "빈 답변 = 미승인" 을 skill 본문에 못 박지 않으면, 사용자가 잠깐 자리를 비운 2분에 배포 승인 게이트가 뚫려요. Plan 모드에서만 blocking 이고, Plan 모드는 실행 금지 모드라 `deploy create --execute` 직전 게이트로는 못 써요.

무기한 대기·timeout 설정을 요구하는 이슈가 다섯 건 열려 있고(#37472, #34455, #28969, #29702, #29104), 커뮤니티가 `default_mode_is_blocking` config 프로토타입까지 냈지만 미머지예요 `[confirmed]`.

### 2.5 나머지 제약

- **root thread 전용** — 서브에이전트에서 호출하면 `request_user_input can only be used by the root thread` `[confirmed]`
- **`codex exec`(headless) 미지원** — `not supported in exec mode` `[confirmed]`. 우리 headless 계약(AUQ 0회)과 정확히 같은 방향이에요.

### 2.6 업스트림 전망 — 기다려서 해결되지 않아요

- flag 는 main 기준으로도 여전히 `Stage::UnderDevelopment` · `default_enabled: false` 이고 `/experimental` 메뉴에도 없어요 `[confirmed, 2026-08-20 fetch]`
- OpenAI contributor(shijie-oai)가 2026-08-13 에 *"We are still iterating on request user input tool outside of plan mode"* 라고 코멘트했고 **ETA 없음** `[confirmed — issue #37472]`
- 표준 쪽에도 신호가 없어요 — agentskills.io spec 과 OpenAI skills 문서 모두 interactive question primitive 를 다루지 않아요 `[confirmed]`

→ **설계는 "flag opt-in 전제" 로 잡는 게 안전해요.**

---

## 3. MCP elicitation — 오늘 기본으로 동작하는 유일한 plugin-invocable 위젯

### 3.1 되는 것

- codex 0.148.0 은 MCP 클라이언트로서 `elicitation/create`(2025-06-18 revision)를 지원하고, initialize 때 `capabilities.elicitation` 을 **항상** 선언해요 `[confirmed — codex-mcp/src/rmcp_client.rs:993-1011]`
- custom MCP 서버(사용자가 등록한 서버)의 elicitation 은 PR #17043(2026-04-08 머지)으로 열렸고 0.148.0 에 포함돼요 `[confirmed]`
- TUI 가 **titled oneOf(const+title) / legacy enum+enumNames / untitled enum / boolean** 을 방향키 Select 위젯으로 렌더해요 `[confirmed — tui/src/bottom_pane/mcp_server_elicitation.rs]`
- codex **plugin manifest 가 `mcpServers`(`./.mcp.json` 경로 또는 inline)를 지원**해요 — 실제로 설치된 context7·computer-use 플러그인이 이 구조를 써요 `[confirmed — core-plugins/src/manifest.rs:61,446-467,980 + 로컬 캐시 실물]`
- elicitation 대기 중에는 **MCP tool-call 타임아웃 시계가 멈춰요** — 사용자가 오래 고민해도 도구가 죽지 않아요 `[confirmed — rmcp-client/src/rmcp_client.rs:170-235]`

### 3.2 안 되는 것 / 위험한 것

| 제약 | 내용 | 우리에게 주는 영향 |
|---|---|---|
| **신뢰 등급** | Guardian 모델에서 request_user_input 응답은 **trusted**, MCP tool output(=elicitation 답변)은 **untrusted** `[confirmed — 바이너리 문자열]` | 파괴적 실행 승인을 elicitation 으로 받으면 codex 자체 신뢰 모델과 어긋나요 |
| **정책 의존** | `approval_policy=never` 면 비어있지 않은 스키마 폼은 **자동 Decline** `[confirmed — codex-mcp/src/elicitation.rs:449-456]` | full-access 사용자에겐 질문이 소리 없이 사라져요 |
| **빈 스키마 함정** | properties 가 빈 confirm 은 full-access 에서 **자동 Accept** `[confirmed — elicitation.rs:461-470]` | 진입 확인을 빈 스키마로 만들면 승인이 새요 — 반드시 실제 필드를 넣어야 해요 |
| **스키마 폭** | number·multi-select enum 이 하나라도 있으면 폼 전체 파싱 실패 → 일반 Accept/Decline 모달로 강등 `[confirmed]` | string / boolean / single-select 만 쓰기 |
| **옵션 설명 없음** | enum 옵션은 label 만 렌더 (per-option description 하드코딩 None) `[confirmed]` | 설명을 title 문자열 안에 넣어야 해요 |
| **headless** | `codex exec` 는 elicitation 을 무조건 자동 Cancel `[confirmed — exec/src/lib.rs:1716-1723]` | 우리 headless 계약과 동형 |
| **서버 주도** | skill 텍스트가 클라이언트에서 직접 못 띄워요 — 서버가 tools/call 처리 중에만 발신 가능 | 확인 로직을 **ax-hub-cli 쪽 MCP tool 안으로 옮기는 서버측 작업**이 필요해요 |
| **sampling/roots 미지원** | codex 는 `sampling/createMessage`·`roots/list` 를 선언하지 않아요 `[confirmed]` | 서버가 되묻는 다른 설계는 불가 |

> **정정 (adversarial 검증 결과)**: 초기 조사는 `tool_call_mcp_elicitation` feature 가 elicitation 을 켠다고 봤지만, 실제로 이 feature 는 codex 자신의 MCP tool approval prompt 의 persist 옵션만 좌우해요. 서버발 `elicitation/create` 는 **feature flag 게이트 없이 무조건 처리**되고, 클라이언트 capability 광고는 `auth_elicitation`(Stable·기본 on)이 담당해요 `[PARTIALLY_TRUE → 교정 confirmed]`.

---

## 4. 배제된 경로 — 못 박아 두는 근거

| 아이디어 | 판정 | 근거 |
|---|---|---|
| bash 로 `fzf`/`gum`/`whiptail` 실행해 터미널 위젯 그리기 | **구조적으로 불가능** | exec 자식은 `portable_pty` 위에서 돌고 출력은 델타로 캡처돼 transcript 텍스트로만 렌더돼요. 그 PTY 의 stdin 에 쓸 수 있는 주체는 **모델의 `write_stdin` 도구뿐**이고, 사용자 키보드는 ratatui TUI 가 소유해요. `/ps` 는 나열만 하고 attach 경로가 없어요 `[confirmed]` |
| `~/.codex/prompts` 에 `/yes` `/no` 를 깔아 클릭 가능한 응답 채널로 | **불가능** | 커스텀 프롬프트 지원이 2026-03-28 PR #16115 로 완전 제거됐고, 0.148.0 슬래시 popup 은 builtin+service tier 전용이에요 `[confirmed]` (learn.chatgpt.com 문서에 아직 남아 있는 건 stale doc 으로 보여요 `[likely]`) |
| 승인 팝업을 질문으로 전용 | **불가능** | 선택지 문구 통제 불가 + `never` 정책에서 미표시 `[confirmed]` |
| plugin manifest 로 feature flag 켜기 | **불가능** | flag 는 사용자 config 표면이고 플러그인이 선언할 수단이 없어요 `[confirmed]` |

---

## 5. 왜 지금 "1) 2) 몇 번?" 으로 퇴화하나 — 원인 3중

1. **도구가 안 보임** — Default 모드에서 `request_user_input` 이 호출 불가라 모델에게 위젯 선택지가 없어요 `[confirmed]`
2. **codex 지침이 텍스트 객관식을 금지** — Default 모드 시스템 템플릿이 *"Never write a multiple choice question as a textual assistant message"* + *"strongly prefer making reasonable assumptions and executing"* 를 명시해요 `[confirmed — collaboration-mode-templates/templates/default.md:7-11]`. 즉 지금의 번호 목록은 **codex 자체 지침과 정면 충돌하는 상태**이고, 그래서 모델이 질문을 뱉고도 기다리지 않고 진행하는 편향이 겹쳐요
3. **persistence 편향** — OpenAI 공식 프롬프팅 가이드가 *"Never stop or hand back to the user when you encounter uncertainty"* 를 권장해요 `[confirmed — GPT-5 prompting guide]`. LLM 이 모호성을 인지하고도 질문 대신 추측하는 실패 모드는 공개 벤치마크로도 반복 실증됐어요 (CLAMBER arxiv:2405.12063, ClarQ-LLM arxiv:2409.06099 등) `[confirmed]`

**추가 교란 변수 (중요)**: 이 머신의 `~/.codex/config.toml` 은 `approval_policy = "never"` + `sandbox_mode = "danger-full-access"` 예요 `[confirmed]`. 이 상태에선 승인 팝업도 elicitation 폼도 **전부 안 떠요**. 지금까지의 "카드가 안 뜬다" QA 결과에는 이 설정이 섞여 있을 수 있어요 — 번들 QA 는 반드시 기본 정책(`on-request`) 프로파일에서 다시 해야 해요.

---

## 6. 설계안 비교

| 안 | UX 품질 | 안전 보장 | 구현비용 | 사용자 부담 | 유지보수 | 합계 |
|---|---|---|---|---|---|---|
| **A. 텍스트 프로토콜만 재설계** | 2 | 4 | 4 | 5 | 5 | **20** |
| **B. request_user_input opt-in** | 5 | 3 | 4 | 3 | 2 | **17** |
| **C. MCP elicitation 번들** | 4 | 2 | 1 | 2 | 2 | **11** |
| **D. A+B+C 계층 (권고)** | 4.5 | 4.5 | 3 | 4 | 3 | **19** |

(5=좋음. B 의 안전 점수가 낮은 건 §2.4 auto-resolve 때문이고, C 가 낮은 건 §3.2 untrusted + 정책 의존 때문이에요. D 는 각 tier 를 **역할별로** 배치해서 약점을 상쇄해요.)

---

## 7. 권고 — 3단 tier, 역할 분리

### T0 · 기본 lane (모두에게 항상 동작) — 텍스트 프로토콜을 codex 규격으로 다시 쓰기

지금의 blanket 치환(`AskUserQuestion` → `명시 텍스트 승인`)은 안전 계약은 지키지만 문장이 어색하고 codex 지침(객관식 금지)과의 정합도 명시가 없어요. 다음 5요소로 재작성해요:

1. **게이트 예외 선언 + 턴 종료 강제** — "이 질문 뒤에는 도구를 호출하지 않고 턴을 끝내요. 답을 받기 전에는 진행하지 않아요."
2. **한 문장 확인형 질문 + 추천 기본값 표기** — 번호 메뉴를 쓰지 않고, 추천안을 먼저 두고 `(추천)` 접미를 붙여요 (codex `request_user_input` 스펙의 `(Recommended)` 규격과 동형)
3. **답→행동 매핑 병기** — 질문 메시지 자체에 "`진행` 이라고 답하면 배포를 시작하고, `취소` 면 여기서 멈춰요" 를 넣어요 (사용자의 짧은 답은 구조 없는 bare message 로 도착하므로 직전 assistant 메시지에 매핑이 있어야 해요)
4. **2-tier 답변 매칭** — 비파괴 선택은 느슨하게(숫자·서수·라벨·prefix 허용), **파괴 게이트는 canonical 문구 byte-exact 만** (기존 AP-12 codex 조항 유지)
5. **CLI 2-phase 백스톱** — dry-run → `--execute` 의 기계적 2단 확인은 그대로 유지해요 (모델 순응도와 무관한 마지막 방어선)

### T1 · opt-in lane — 네이티브 위젯을 켜는 한 줄

onboarding·README 에 선택 안내를 넣어요. 켠 사용자는 Claude 와 동급의 선택 카드를 받아요.

```toml
# ~/.codex/config.toml — axhub 질문을 선택 카드로 받고 싶을 때
[features]
default_mode_request_user_input = true
```

**필수 동반 규칙** (skill 본문·always-on 훅 양쪽에):
> 선택 카드가 빈 답변으로 돌아오면 **미승인**이에요. 자동 해제된 것이므로 실행하지 않고 다시 물어요.

### T2 · 향상 lane — MCP elicitation 은 비파괴 선택만

템플릿 선택·앱 이름 확인처럼 **틀려도 되돌릴 수 있는** 질문에만 써요. AP-12 진입 확인·`--execute` 승인에는 쓰지 않아요 (untrusted 등급 + `never` 정책 자동 Decline). 전제 조건이 ax-hub-cli 쪽 `ask_user` MCP tool 신설이라 이 repo 범위 밖 follow-up 이에요.

---

## 8. 즉시 적용 가능한 변경 목록

| # | 파일 | 변경 | 이유 |
|---|---|---|---|
| 1 | `scripts/build-plugin-bundle.ts` | `CODEX_SUBSTITUTIONS` 에 `["AUQ", "명시 텍스트 승인"]` 추가 (longest-first 정렬 유지) | 산출물에 `AUQ` 약어가 그대로 남아 있어요 — `plugins/axhub-codex/skills/deploy/SKILL.md:34`, `import/SKILL.md:65`, `development/references/write-gate.md:19,21` `[confirmed]` |
| 2 | `tests/codex-bundle.test.ts` | `FORBIDDEN_STRINGS` 에 `"AUQ"`, `"CLAUDE_NON_INTERACTIVE"` 추가 | 후자는 `plugins/axhub-codex/skills/onboarding/SKILL.md:63` 에 잔존 `[confirmed]` |
| 3 | `tests/codex-bundle.test.ts` | **게이트 문자열 byte-offset assert** 신설 (승인 문구가 파일 첫 8,000B 안) | 현재 bootstrap 게이트는 **전부 절단선 밖**(첫 `--execute` @9,0xx), import canonical 질문 @19,851 CUT, scaffold 는 straddle `[confirmed]` — 절단 대비책이 recovery-line 한 줄뿐이에요 |
| 4 | `codex-overrides/skills/{bootstrap,import,scaffold}/SKILL.md` | 게이트 문단을 첫 8,000B 안으로 끌어올린 codex 판 저작 (세부는 references 로) | 위와 동일. codex 트리 per-skill 여유는 bootstrap 7,166B / import 6,0xxB 로 충분해요 `[confirmed]` |
| 5 | `codex-overrides/hooks/context/*.md` (always-on 훅) | T0 의 5요소 질문 프로토콜을 **훅 문맥에 편입** | 훅은 skill 8KB 절단과 무관하게 항상 주입돼요 — AP-13 을 훅에 둔 것과 같은 논리 |
| 6 | `codex-overrides/skills/onboarding/` + `codex-overrides/README.md` | T1 한 줄 opt-in 안내 + `approval_policy` 주의 | §5 교란 변수 해소 |
| 7 | `docs/policy/agent-policy.md` AP-12 | codex 판 프리미티브 절에 "네이티브 카드가 켜져 있으면 그것으로 묻되, **빈 답변은 미승인**" 을 추가 | §2.4 auto-resolve 함정 |

---

## 9. 실행 순서

**Phase 0 — 실기 확인 (반나절)**
- 기본 정책 프로파일(`-p` 또는 임시 `CODEX_HOME`)에서 codex TUI 를 띄우고 ① flag off 상태 ② `codex -c features.default_mode_request_user_input=true` 상태를 각각 QA
- 검증 기준: ②에서 axhub deploy 게이트가 **선택 오버레이**로 뜨는가, 2분 방치 시 무엇이 모델에게 돌아가는가

**Phase 1 — T0 + 잔여물 정리 (§8 의 1·2·3·4)**
- 검증 기준: `bun run plugin:bundle:all` 후 codex 트리에서 `AUQ`·`CLAUDE_NON_INTERACTIVE` 0건, 새 byte-offset assert green, `lint:tone --strict` 0 err

**Phase 2 — T1 안내 + 훅 편입 (§8 의 5·6·7)**
- 검증 기준: flag on 세션에서 승인 게이트가 카드로 뜨고, 빈 답변 auto-resolve 가 **실행 없이 재질문**으로 처리되는가

**Phase 3 (별건 follow-up)** — ax-hub-cli 에 `ask_user` MCP tool + 번들 `.mcp.json`. 이 repo 범위 밖이에요.

---

## 10. 미해결 · 업스트림

- **라이브 검증 미실시** — 조사 규칙상 대화형 codex 세션을 띄우지 않았어요. 위젯 렌더·키 처리·auto-resolve 직렬화는 소스와 바이너리 문자열 기반이에요 `[unknown → Phase 0 에서 해소]`
- **Codex 데스크톱 앱** (`codex app` 으로 설치되는 `Codex.app`)이 `openai/standard-form-input` extension 을 선언하는지 미확인 — 클로즈드소스예요. 선언한다면 데스크톱 사용자는 full-access 에서도 폼을 볼 수 있어요 `[likely]`
- **업스트림 요청거리 2개**: ① `default_mode_request_user_input` 의 Stable 승격 ② Default 모드에서도 blocking 을 고를 수 있는 config (커뮤니티 프로토타입 `default_mode_is_blocking` 존재, 미머지)
- **최소 codex 버전 게이트** — custom-server elicitation 은 2026-04-08 이후 릴리스에만 있어요. 정확한 릴리스 번호 매핑이 남아 있어요 `[unknown]`

---

## 부록 A — 재현 명령

```bash
# 도구 실재 확인
BIN=~/.nvm/versions/node/v24.14.0/lib/node_modules/@openai/codex/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/bin/codex
rg -a "request_user_input" "$BIN" | head
strings "$BIN" | rg -i "unavailable in Default mode|None of the above|can only be used by the root thread"

# feature 상태
codex features list | rg "request_user_input|elicitation"

# 서버→클라이언트 요청 전수 (선택 UI 경로의 정본)
codex app-server generate-json-schema --out /tmp/codex-schema && rg '"method"' /tmp/codex-schema/ServerRequest.json

# 현재 승인 정책 (QA 교란 변수 확인)
rg "approval_policy|sandbox_mode" ~/.codex/config.toml

# 소스 원문
# https://github.com/openai/codex/blob/rust-v0.148.0/codex-rs/core/src/tools/handlers/request_user_input_spec.rs
# https://github.com/openai/codex/blob/rust-v0.148.0/codex-rs/tui/src/bottom_pane/mcp_server_elicitation.rs
# https://github.com/openai/codex/blob/rust-v0.148.0/codex-rs/collaboration-mode-templates/templates/default.md
```

## 부록 B — 이 문서가 갱신한 선행 실측

| 선행 문서 기술 | 갱신 내용 |
|---|---|
| "`request_user_input` 도구는 존재하나 기본 모드에서 비활성 (config_types.rs:669-671)" | 라인 **667-683** 으로 갱신. "비활성" 은 등록이 아니라 **호출 모드 게이트**이고, 사용자 config 한 줄로 열려요 |
| (선행 문서에 없음) | **MCP elicitation 경로가 기본 활성**으로 존재해요 — 2026-04-08 이후 추가된 표면이라 선행 조사 시점엔 없었어요 |
| (선행 문서에 없음) | Default 모드 flag 를 켜도 **non-blocking auto-resolve** 함정이 있어요 |
