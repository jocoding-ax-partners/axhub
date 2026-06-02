# axhub 라우팅 decoupling 명세

> 출처: `/ouroboros:interview` 세션 `interview_20260602_013939` (ambiguity 0.09, seed-ready).
> 이 문서는 `ooo seed` 입력용 요구사항 명세예요. 구현 전 단일 진실 소스로 써요.

## Goal (한 줄)

axhub 라우팅을 로컬 `axhub.yaml` 마커(git-root walk-up)로 게이팅해서 — 마커 없으면 모호한 "배포해"는 pass-through 하고 `"axhub"` 명시·`/deploy` 같은 explicit 의도만 동작, 마커 있어도 `"vercel"` 등 타깃 키워드가 있으면 axhub 가 양보(named-target-wins) — 사용자가 Vercel 등 다른 배포 타깃을 자연스럽게 쓸 수 있게 해요.

## 문제 (강결합 레이어)

세션이 axhub 으로 끌려가는 원인은 **독립적인 트리거 경로 2개**예요.

1. **prompt-route hook** (`bin/axhub-helpers prompt-route`, `hooks/hooks.json` UserPromptSubmit) — 매 프롬프트마다 nl-lexicon 매칭 후 axhub 라우팅 nudge(`systemMessage`) 주입.
2. **deploy SKILL `description:` 프론트매터** — Claude Code 모델의 skill-selection 을 트리거. hook 과 **무관하게** 발화. bare `"배포"/"deploy"/"ship"` + 캐치올 *"현재 브랜치를 axhub 라이브로 push 하고 싶다는 모든 의도"* 를 클레임. trigger 어구는 `lint:keywords --check` baseline-lock 이라 **삭제 불가** → 런타임 게이트만 가능.

보조로 **session-start** 가 모든 프로젝트에서 eager axhub infra(token-init/warmup/quality-context)를 무조건 실행해요(현재 marker 체크 0).

## 핵심 규칙

> **named target wins; 무명시면 marker 가 결정.**

### Marker 정의

| 항목 | 결정 |
|------|------|
| 파일 | `axhub.yaml` |
| 탐색 | cwd → 상위 walk-up |
| 종료 | `.git` 만나면 정지, git 아니면 filesystem root fallback |
| 비용 | 로컬 fs check only, **네트워크 없음** (hot path, 5s timeout) |

### prompt-route 분기 (우선순위 순)

| 순위 | 조건 | 결과 |
|------|------|------|
| **0** | slash invocation (`/deploy`, `/axhub:deploy`) | → axhub (marker·keyword 무관, 최강 explicit — conflict 보다 위) |
| a | `"axhub"` AND foreign 키워드 둘 다 | AskUserQuestion 으로 타깃 disambiguation |
| b | `"axhub"` 포함 | → axhub (marker 무관, explicit) |
| c | foreign 키워드 포함 | → axhub 양보 (pass-through) |
| d | 무명시("배포해") + marker 있음 | → axhub |
| e | 무명시 + marker 없음 | → ignore + 조건부 grace 경고¹ |

¹ grace 경고: 무명시 deploy 의도 + marker 없음 + axhub 인증 유저 → **1회성(once-per-project)** `systemMessage` (마이그레이션 안내: `/init` 또는 `"axhub 배포"` 명시). 1회성 보장은 상태 파일로(megaskill 경고 패턴 참조). **이중 노출 정책**: 같은 케이스에서 deploy SKILL preflight disambiguation 도 뜰 수 있음 → 의도적 이중으로 두되 grace 의 once-per-project 로 겹침을 첫 회로 bound (grace=educate 1회, preflight=block 매번, 이후엔 preflight 만).

### Foreign 타깃 키워드 (하드코딩)

`vercel`, `netlify`, `cloudflare`, `fly`, `render`, `railway` — helper 내 상수 배열. 느리게 변하는 집합이라 외부화 안 함.

### 공유 routing-decision 함수 (single source of truth)

라우팅 결정을 helper 의 **단일 함수**로 추출 → hook 과 deploy preflight 가 **둘 다** 호출(로직 drift 근절).

- 입력: prompt text, `marker_present`(walk-up), `authed`(token-file `.exists()` stat), `explicit_invocation`(slash command 여부 — deploy preflight 가 감지해 전달).
- 출력: `decision ∈ {axhub, yield, ignore, ask}` — 위 분기표 rule 0~e + fail-open 을 그대로 인코딩. **rule 0(slash → axhub)이 최우선**(conflict 보다 위). 4개 input modality(axhub-keyword / foreign-keyword / slash / bare-NL) 전부 커버.
- 소비자별 action 매핑:
  - **hook** (UserPromptSubmit, 먼저 실행): `axhub`→neutral(skill-selection 진행 허용), `yield`→침묵, `ignore`→침묵(+authed 면 grace), `ask`→**neutral**(충돌 disambiguation 은 preflight 소유 — hook 은 tool 실행 불가, systemMessage 만 가능).
  - **deploy preflight Step 0** (skill 선택됐을 때): `axhub`→진행, `yield`→일반 흐름 양보, `ignore`/`ask`→disambiguation. **`decision==axhub` 일 때만 axhub deploy 진행.**

> ⚠️ 이게 없으면: hook 이 'vercel' 에 양보해도 preflight 가 marker-present 만 보고 axhub 로 진행 → 유저 원래 불만("Vercel 쓰고 싶은데 axhub 로 라우팅") 재현. marker 있는 repo(개발자 본인 repo 는 항상 해당)에서 터지므로 edge case 아님. 두 레이어가 같은 함수를 써야 named-target-wins 가 일관.

## 범위 — 전 레이어 marker 게이트

| 레이어 | marker 없으면 |
|--------|--------------|
| **session-start** | eager infra(token-init / Gatekeeper warmup / quality-context 주입) **스킵**. ⚠️ helper 바이너리 auto-download 는 게이트 **제외**(prompt-route 가 helper 라 prerequisite). |
| **prompt-route** | 위 분기표 적용 |
| **classify-exit** | 동일 게이트 |
| **deploy SKILL preflight (Step 0)** | **공유 routing-decision 함수**(위)를 auth/resolve **전에** 호출. `decision==axhub` 일 때만 진행; `yield`(foreign keyword) → 일반 흐름 양보, `ignore`(no marker) → "axhub 맞아요? 어느 타깃?" disambiguation, `ask`(충돌) → disambiguation. ← description-driven skill-selection 을 막는 **실제 레버**, **그리고 foreign-keyword 케이스도 여기서 잡힘**(marker 있어도 'vercel' 명시면 yield). deploy 에 한정(apps/recover 등은 이번 범위 밖). |

## explicit 경로 (marker 무관)

- `"axhub"` 키워드 또는 `/deploy` 슬래시 → **항상 동작**.
- auth/token → **lazy bootstrap** (eager session-start 에서 호출 시점으로 이동).
- lazy auth 실패 → 명확한 에러 + `axhub auth login` 안내, **멈춤**. silent fallback 없음(사용자가 명시했으니 조용히 안 빠짐).

## 안전장치 / 충돌

- false-positive 라우팅(`"axhub"` keyword-only 선택의 수용된 tradeoff, 예: "axhub 플러그인 지워줘") → deploy SKILL 의 AskUserQuestion preview card + HMAC consent gate 가 실제 배포 차단 → 실질 피해 = dismiss 가능한 카드.
- 키워드 충돌(`"axhub"`+foreign 둘 다) → AskUserQuestion disambiguation (deploy = high-stakes).

## Backward Compatibility

| 기존 유저 시나리오 | 동작 |
|-------------------|------|
| `axhub.yaml` 있음 + "배포해" | 기존과 동일 (axhub) |
| `axhub.yaml` 없음 + "배포해" | **변경**: ignore + grace 경고 |
| `axhub.yaml` 없음 + "axhub 배포" | 기존과 동일 (explicit → axhub) |
| `axhub.yaml` 없음 + `/deploy` | 기존과 동일 (explicit → axhub) |

실질 기능 손실 = **implicit nudge 소실뿐**. (deploy 는 현재 `axhub.yaml` 을 안 쓰고 live profile/app resolution 으로 타깃 해석하므로, marker 없는 profile-기반 유저가 회귀 대상.)

## 검증 (acceptance)

1. **routing-audit** (기존 `routing-audit-*.jsonl` + `routing-stats` skill) — 공유 함수의 decision(`axhub`/`yield`/`ignore`/`ask`) + keyword-driven(rule 1~3) vs marker-driven(rule 4~5) 플래그를 기록 → "non-axhub 프로젝트에서 ignore 율" 측정.
2. **test matrix** — `marker × keyword × prompt` 조합으로 prompt-route + deploy preflight 동작 고정.

## 구현 노트 (의식적 제약)

- **fail-open 방향 (auth 조건부)**: marker walk-up 체크 자체가 에러날 때(fs 권한/레이스), 모호한 "배포해"의 기본값 = **axhub-authed 유저 → marker-present(axhub), 비인증 → pass-through**. 이유: 기존 `axhub.yaml` 유저가 transient 에러로 "배포해"→axhub 를 조용히 잃는 회귀를 막으면서 비-axhub 유저는 zero-footprint 유지. hook 은 항상 `exit 0`, panic 금지 (`hook_safety` 계약, `docs/HOOKS.md`) — 단 `exit 0` 은 "어느 라우팅 결과로 빠지느냐"를 안 정하므로 이 방향을 명시.
- **helper-download 비게이트**: prompt-route 자체가 helper 이므로 바이너리 존재는 prerequisite. "eager infra 스킵" 을 download 까지 확장하면 helper 가 안 깔려 게이트 자체가 불가 → 브릭. token-init/warmup/quality-context 만 게이트.
- **trigger 어구 baseline-lock**: deploy `description:` 의 "배포" 등은 `lint:keywords` baseline 으로 잠겨 있어 편집 금지. 게이트는 반드시 런타임(in-body preflight).
- **auth-read 원시값 (순환 회피, load-bearing)**: fail-open·grace·deploy-preflight 의 "axhub-authed?" 판정은 **cheap token-file `.exists()` stat** 으로만 함 (helper 의 auth/delegation token-file `~/.config/axhub-plugin/token`, in-process). `axhub auth status` CLI 스폰이나 token-init bootstrap 을 **트리거하면 안 됨** — 안 그러면 "auth 읽기 → bootstrap → marker 게이트" 순환. 주의: token-present 는 authed 의 proxy(CLI-authed-but-no-helper-token 은 not-authed 로 읽힘 → pass-through; error-path 에서 허용되는 under-detection). `consent/jwt.rs` 의 `token_file_path` 는 HMAC consent 토큰으로 **별개** — 혼동 금지.

## 추정 구현 표면

- `crates/axhub-helpers/src/` — prompt-route 분기 + foreign 키워드 상수 + walk-up marker 탐색 + routing-audit 결정타입 기록.
- `hooks/session-start.sh` / `.ps1` — eager infra 를 marker 조건부로 (download 는 제외).
- `skills/deploy/SKILL.md` — in-body preflight Step 0 에 marker 체크 + disambiguation 분기.
- `crates/axhub-helpers/src/audit.rs` — 결정타입 enum 확장.
- `tests/` — marker × keyword × prompt 매트릭스.
