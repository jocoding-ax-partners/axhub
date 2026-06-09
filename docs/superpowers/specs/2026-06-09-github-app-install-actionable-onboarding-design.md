# GitHub App 설치 surface — onboarding actionable 전진배치 설계

- 날짜: 2026-06-09
- 범위: `skills/onboarding/SKILL.md` (새 `Step 2.5` — branch-independent actionable surface) + `tests/fixtures/ask-defaults/registry.json` (onboarding 새 question 등록) + `docs/superpowers/specs/2026-06-08-github-app-install-surface-design.md` 비-목표 개정
- 접근: DETECT_ALL 직후 단일 branch-independent 지점에 install_url 을 무조건 노출 + actionable 설치 제안
- 출처: `/deep-interview` (사용자 스크린샷 2장 + 2개 결정 확정)

## deep-interview 메타데이터

- Threshold: 0.2 (20%) / Source: default
- Type: brownfield
- Rounds: 1 (Round 0 topology + 1 결정 라운드, 강한 사전 컨텍스트로 조기 수렴)
- Final Ambiguity: 9.3% (≤20% PASSED)
- Clarity: Goal 0.92 / Constraint 0.90 / Criteria 0.90 / Context 0.90

## 문제

사용자 스크린샷 두 장은 같은 onboarding 흐름의 다른 지점이에요:

| | 위치 | install_url |
|---|---|---|
| **Part 1** | onboarding `Step 2` DETECT_ALL 요약 카드 ("GitHub App 설치됨 (3계정)") | ❌ 없음 |
| **Part 2** | `init` `Step 2.5` ("GitHub App 설치·계정 추가 링크: …") | ✅ 있음 |

이미 GitHub App 이 설치된 사용자가 **빈 폴더**에서 onboarding 을 돌리면, helper 의 `first_gap` 이 `no_manifest_empty` 라 state machine 이 바로 `Step 7` (첫 앱 만들래요?) → `init` 으로 라우팅해요. 그 결과 install_url 을 노출하는 두 지점을 **둘 다 우회**해요:

- `Step 6` (github gap): `github.state` 가 `uninstalled`/`empty` 일 때만 발화 → 설치된 사용자는 skip
- `Step 10` (VIBE_READY ready card): gap 이 0개일 때만 도달 → 빈 폴더 gap 이 있어 미도달

그래서 install_url 은 한참 뒤 `init` `Step 2.5` (Part 2) 에서야 처음 등장해요. 사용자 요구:

1. **"다른 곳에서도 깃헙 앱 설치하고 싶은데 깃헙 앱 설치하는 플로우가 없음"** — 첫 요약 카드에 설치 진입 수단이 없어요.
2. **"이렇게 나중에 뜨긴 하는데 맨 처음에 install url 이 나왔으면 좋겠음"** — install_url 을 흐름 맨 앞에서 보고 싶어요.

## whack-a-mole 경고 (git 이력)

이 install_url surface 는 한 feature + 세 follow-up fix 로 반복 패치돼 왔어요:

- #173 feat: 상시 노출 (init + onboarding)
- #176 fix: ready card 링크 안 뜨는 문제
- #177 fix: init route hint 가 Step 2.5 건너뛰는 문제
- #179 fix: install_url 상시 노출 (again)

매 site 가 **conditional** (Step 6 / Step 10 / init Step 2.5) 이라, 사용자가 어느 state-machine branch 에 떨어지느냐에 따라 노출이 갈려요. routing 이 바뀔 때마다 새 구멍이 뚫리고, 사용자가 발견한 이번 건이 4번째예요. 2026-06-08 spec line 5 의 deferral 트리거("helper 추출은 3번째 사이트 등장 시점")가 이미 발동했어요.

## 결정 (사용자 확정 — deep-interview)

| 항목 | 결정 | 근거 |
|---|---|---|
| 동작 방식 | **actionable** (표시 전용 ❌) | "플로우가 없음" annotation 을 문자 그대로 구현 — 단순 링크 줄이 아니라 설치 제안 AskUserQuestion + 재개 phrase |
| 적용 범위 | **DETECT 직후 무조건 1지점 (branch 독립)** | github.state·first_gap 과 무관하게 모든 onboarding 경로가 지나는 단일 지점 → whack-a-mole 종결 |

### 2026-06-08 spec 비-목표 개정 (의식적 변경)

이전 spec 은 다음을 비-목표로 못박았어요:

- ~~"onboarding 변경 안 해요 (#171 이 충족)"~~ → **개정**: 스크린샷이 그 가정(설치+빈폴더 경로는 #171 로 충족)을 반증했어요. onboarding 에 branch-independent surface 를 추가해요.
- ~~"새 AskUserQuestion 없음 / 비차단"~~ → **부분 개정**: actionable 결정에 따라 새 AskUserQuestion 1개를 추가하되, **default 옵션이 onboarding 을 그대로 이어가게(비차단)** 설계해요.

## 핵심 사실 (확정)

1. **helper 가 `github.install_url` 을 이미 무조건 채워요.** onboarding SKILL.md Step 2 계약: "`install_url` 은 GitHub 조회가 성공하면 (`installed`/`mixed`/`uninstalled`/`empty`) 설치 여부·계정 수와 무관하게 항상 채워져요 (계정이 0개여도 app-level 링크로 fallback)." → 데이터는 이미 DETECT_JSON 에 있어요. **Rust/helper 변경 불필요.** 순수 SKILL.md (agent-facing 지시) 변경이에요.
2. **`github.install_url` 이 null 인 경우는 `auth_error`/`unavailable` 뿐.** 이때만 Step 2.5 를 생략하고, `auth_error` 면 "다시 로그인해줘" 로 낮춰요.
3. **step-numbering collision (FU-3) 회피.** init 이 이미 `2.5.` sub-step 패턴을 쓰고(top-level `^N. \*\*` regex 비충돌), onboarding 도 `2.5.` 로 미러해요. Step 3~10 renumber 불필요.
4. **D1 비대화형 guard 적용.** 새 AskUserQuestion 은 `claude -p`/CI/headless 에서 skip 하고 registry safe_default 로 진행해야 해요.

## 설계 — onboarding Step 2.5 (actionable, branch-independent, 비차단)

위치: `Step 2` (DETECT_ALL) 직후, `Step 3` (gap state machine) **이전**. 모든 경로가 gap 라우팅 전에 이 지점을 지나요.

```
2.5. GitHub App 설치·계정 추가 surface (branch-independent, 비차단).

  Step 2 helper JSON 의 github 를 그대로 써요 (accounts list 재호출 X).
  github.install_url 이 null 이 아니면 (state ∈ installed/mixed/uninstalled/empty)
  설치 여부·계정 수와 first_gap 과 무관하게 항상 이 블록을 실행해요.

  (a) install_url 한 줄 무조건 표시 (annotation 2 충족):
      "GitHub App 설치·계정 추가 링크: <github.install_url>
       이미 설치돼 있어도 다른 org/계정을 더 붙일 수 있어요."
      github.installed_logins 가 있으면 "이미 연결된 계정: <login...>" 덧붙여요.
      installation_id 등 internal 값 echo 금지, login + install_url 만.

  (b) actionable 설치 제안 (annotation 1 충족) — AskUserQuestion:
      질문: "다른 org/계정에도 GitHub App 을 설치할래요?"
      header: "GitHub App"
      옵션:
        - "아니요, 계속" (default/safe) → gap 라우팅(Step 3)으로 그대로 진행. 비차단.
        - "설치할래요" → github.install_url 보여주고 브라우저 열기.
                         "설치했어"/"온보딩 계속" 재개 phrase 안내 → 말하면 Step 2 재감지 1회.

  github.install_url 이 null (auth_error/unavailable) 이면 이 블록 전체 생략.
  auth_error 면 "다시 로그인해줘" 안내로 낮춰요.

  D1: 비대화형(claude -p/CI/headless)에서는 (b) AskUserQuestion skip,
      registry safe_default("아니요, 계속")로 진행. (a) 표시 줄은 그대로 출력.
```

### 왜 표시(a) + 액션(b) 둘 다인가

- (a) 무조건 표시 줄이 annotation 2("맨 처음에 url")를 보장해요 — 사용자가 (b) 를 skip 해도 URL 은 맨 앞에서 봤어요.
- (b) AskUserQuestion 이 annotation 1("설치 플로우")을 충족해요.
- 둘을 **하나의 branch-independent 지점(Step 2.5)** 에 묶어서, Step 6/Step 10 의 conditional 노출에 의존하지 않아요 → routing 이 또 바뀌어도 이 지점은 항상 지나므로 whack-a-mole 종결.

## 비-목표

- init/apps 진입점은 이번 범위 밖이에요 (사용자 "1지점" 결정). init Step 2.5 는 read-only 로 이미 존재.
- CLI/helper/Rust 변경 안 해요 (install_url 데이터는 이미 제공됨).
- Step 6/Step 10 의 기존 install_url 문구는 제거하지 않아요 — Step 2.5 가 guarantee 를 추가할 뿐, 기존 surface 와 공존해요.
- 차단 gate 아니에요. default 가 onboarding 을 이어가요.

## 후속 권고 (이번 PR 밖, 명시적 surface)

4번째 site 등장으로 2026-06-08 spec line 5 의 deferral 트리거가 발동했어요. install_url 노출 문구가 이제 onboarding(Step 2.5/6/10) + init(Step 2.5) 4곳에 분산돼 prose drift 위험이 있어요. **공유 reference(예: `skills/deploy/references/github-install-surface.md`) 추출**을 후속으로 권고해요. 이번 fix 는 surgical 하게 onboarding Step 2.5 만 추가하고, 추출은 별도 판단으로 남겨요.

## 제약 — Phase 17/18 skill 게이트 유지

- onboarding `needs-preflight: false`, in-body 구조 유지.
- 신규 한글 텍스트 전부 해요체 (`lint:tone --strict` 0 err): 합니다/입니다/당신 금지, 해요/예요/이에요 사용.
- frontmatter `description:` 불변 (`lint:keywords --check` no diff) — body 에만 추가, trigger 어구 변경 금지.
- step-numbering collision(FU-3) 회피 — `2.5.` sub-step.
- 새 AskUserQuestion 은 `tests/fixtures/ask-defaults/registry.json` 의 `onboarding` 키에 등록 필수:
  - question: "다른 org/계정에도 GitHub App 을 설치할래요?"
  - safe_default: "아니요, 계속"
  - rationale: 비대화형에서 GitHub App 설치 브라우저 흐름을 자동 시작하면 안 되고, 설치 안내는 init/connect 가 reactive 로 책임지므로 onboarding 을 그대로 이어가요.
  - allowed_safe_defaults: ["아니요, 계속", "설치할래요"]

## 검증

- [ ] `bun run skill:doctor --strict` exit 0 (D1 sentinel / TodoWrite / step-numbering)
- [ ] `bun run lint:tone --strict` 0 err
- [ ] `bun run lint:keywords --check` no diff
- [ ] `bun test` regression 0 fail (특히 `tests/ux-ask-fallback-registry.test.ts` — 새 question 등록 lock)
- [ ] `bunx tsc --noEmit` clean
- [ ] 수동 재현: 설치된 계정 ≥1 + 빈 폴더에서 onboarding → Step 2.5 에서 install_url + 설치 제안이 gap 라우팅 전에 노출되는지 확인

## Acceptance Criteria

- [ ] 이미 설치된 사용자 + 빈 폴더 경로에서 install_url 이 onboarding 맨 앞(Step 2.5, gap 라우팅 전)에 노출돼요.
- [ ] "다른 org/계정에 설치할래요?" AskUserQuestion 이 제공되고, "설치할래요" 시 브라우저 열기 + 재개 phrase 안내가 동작해요.
- [ ] "아니요, 계속" (또는 비대화형 default) 은 기존 onboarding 흐름을 그대로 이어가요 (비차단).
- [ ] github.install_url 이 null(auth_error/unavailable) 이면 Step 2.5 가 생략되고 auth_error 는 재로그인 안내로 낮춰져요.
- [ ] 새 question 이 registry 에 등록돼 `ux-ask-fallback-registry` 가 green.
