# 플러그인 자동 업데이트 알림 (proactive nudge)

> CEO 리뷰 산출물 (`/plan-ceo-review`). Mode: **HOLD SCOPE** (최소 설계를 bulletproof).
> 코드 변경 전 설계 문서 — eng-review 가 세부 구현 확정.

## 문제

플러그인 버전 드리프트(설치본 < 최신)를 사용자가 직접 "플러그인 최신인지 봐줘" 라고
물어야만 알 수 있어요 (reactive). 자동으로 먼저 알려주는 흐름이 없어요.

## 결정 (CEO 리뷰)

| # | 결정 | 선택 | 근거 |
|---|---|---|---|
| D1 | 트리거 방식 | **A: SessionStart 버전당 1회 비차단 nudge, AUQ 로 표시** | prompt 피로 없음, 기존 upgrade 스킬+dedup 재사용, 최소 diff |
| D2 | latest 소스 | **A: 백그라운드 GitHub releases + 24h 캐시, fail-open** | 세션 시작 블로킹 0, 신뢰 latest, auth-refresh-bg 패턴 미러 |
| D3 | fetch 구현 | **B: ureq dep 추가 (순수 Rust)** | 모든 환경 결정론적 동작 + 테스트 용이. TLS backend(rustls vs native-tls)는 핀된 ureq 버전 기준으로 feature flag 확인 필요 — OpenSSL 시스템 의존 회피 목표. (eng-review) |
| D4 | trigger surface | **UserPromptSubmit (prompt-route), SessionStart 아님** | **T0 해소**: SessionStart additionalContext 는 advisory (axhub `session_start_megaskill_context` 가 증거 — "Suggested" 제안일 뿐). user turn 이전이라 AUQ 강제 못 함. UserPromptSubmit 은 실제 턴 steer (prompt-route 검증됨). per-version dedup 으로 세션 첫 프롬프트에 1회. (ralph 실행 단계) |

## 핵심 제약 (load-bearing)

1. **hook 은 AskUserQuestion 을 직접 못 띄워요.** fail-open subprocess, exit 0 강제.
   → AUQ 는 **에이전트**가 쏘고, hook 은 `additionalContext` 트리거만 주입해요.
   → ⚠️ **검증 필요 (T0 blocking)**: SessionStart 주입이 에이전트의 자율 AUQ 발동을
     신뢰성 있게 유도하는지, 그리고 *언제* (사용자 첫 프롬프트 턴을 가로채는지) 미검증.
     기존 `prompt-route` 는 **UserPromptSubmit** 라 *그 턴*을 steer 하는 증거일 뿐,
     SessionStart pre-prompt 주입을 증명하지 못해요. T0 spike 로 먼저 확인.
   → **Fallback**: SessionStart 가 AUQ 를 안 띄우면, 트리거를 **UserPromptSubmit**
     (세션 첫 프롬프트, per-version marker 로 1회 가드) 로 이동. 이건 검증된 surface
     (prompt-route 자체) 라 실제 턴에서 자연히 발동해요.
2. **`latest_version` 이 현재 `null`** — 설치본은 자기 버전만 알아요.
   → 실제 latest 소스(캐시된 network check)가 선결과제. 이게 feature 의 feasibility 코어.
3. **session-start 는 fail-open 계약** (`docs/HOOKS.md`). 어떤 실패도 exit 0, panic 금지.

## 아키텍처

```
  ┌─ SessionStart hook (hooks/session-start.sh) ───────────────────────┐
  │                                                                     │
  │  1. nohup "$HELPER" plugin-latest-fetch-bg &   (auth-refresh-bg 미러)│
  │       └→ GitHub releases/latest 폴링(timeboxed) → 24h TTL 캐시 기록  │
  │                                                                     │
  │  2. exec "$HELPER" session-start                                    │
  │       └→ 캐시 읽기 → 설치본(plugin.json) vs 캐시 latest 비교         │
  │          drift && !nudged-this-version                              │
  │            → additionalContext 주입 + nudge marker 기록              │
  └─────────────────────────────────────────────────────────────────────┘
                              │ additionalContext
                              ▼
  ┌─ 에이전트 (첫 턴) ──────────────────────────────────────────────────┐
  │  주입 context 읽음 → AskUserQuestion "플러그인 업데이트 할까요?"      │
  │    → 기존 skills/upgrade 흐름 라우팅 (버전비교·노트·/plugin update)   │
  └─────────────────────────────────────────────────────────────────────┘
```

### Data flow (shadow paths)

```
  CACHE READ ──▶ PARSE latest ──▶ SEMVER COMPARE ──▶ INJECT context
      │              │                  │                  │
      ▼              ▼                  ▼                  ▼
  [없음?]        [malformed?]      [downgrade?]      [non-interactive?]
   → skip,        → skip,           current>latest    claude -p/CI
   조용,          fail-open         → skip(프리뷰)    → skip AUQ
   fetch 만                                          (upgrade D1 guard)
  [권한오류?]    [버전파싱실패?]    [동일?] → skip    [already nudged?]
   → skip        → skip                              → skip (marker)
```

## 재사용 (기존 자산 — 재구현 금지)

| 필요 | 기존 자산 |
|---|---|
| "업데이트 할까요?" AUQ + 노트 + /plugin update 안내 | `skills/upgrade/SKILL.md` (DX-6/row28) |
| 재nag 방지 telemetry | `~/.cache/axhub-plugin/upgrade-prompts.ndjson` |
| 세션당-1회 marker 패턴 | `session-start.sh` megaskill-superpowers-warning marker |
| 백그라운드 fetch 패턴 | `session-start.sh` auth-refresh-bg (nohup + disown) |
| fail-open + kill switch | `hook_safety::is_hook_disabled` (`crates/axhub-helpers/src/hook_safety.rs`) |
| **현 플러그인 버전 (공짜)** | `env!("CARGO_PKG_VERSION")` = `PLUGIN_VERSION` (telemetry.rs:13) — 런타임 read 불필요 |
| **drift nudge 패턴** | `session_start_megaskill_context() -> Option<String>` (main.rs:2666) 미러 |
| **additionalContext emit** | `hook_output::session_start_context(text)` (hook_output.rs:3) — 이미 존재 |
| **atomic 캐시 쓰기** | `atomic_jsonl.rs` (temp+rename, half-write 방지) |

## Failure modes registry (전부 fail-open, exit 0)

| codepath | failure | 처리 | 사용자 |
|---|---|---|---|
| plugin-latest-fetch-bg | 네트워크 down | skip, 캐시 미갱신 | 조용 (다음 세션 재시도) |
| plugin-latest-fetch-bg | GitHub 403/429 rate-limit | skip, 캐시 유지 | 조용 |
| plugin-latest-fetch-bg | releases JSON malformed | skip | 조용 |
| session-start 비교 | 캐시 파일 손상/권한 | skip | 조용 |
| session-start 비교 | 버전 파싱 실패 | skip | 조용 |
| 주입 | non-interactive (`claude -p`/CI) | AUQ skip (upgrade D1 guard) | 조용 |

silent 알림 누락은 **의도된 fail-open** — drift 알림은 best-effort 라 안전.

## 보안 (Section 3)

- **release notes 원문을 additionalContext 에 주입 금지.** 검증된 semver 버전 번호만 주입.
  release 본문은 attacker-influenceable → prompt-injection 표면. 버전 숫자만 통과.
- fetch 는 read-only GitHub API. 토큰 불필요 (public releases).

## DX (devex-review)

**Persona**: vibe coder — 자연어만, slash command·env var 안 씀, 반복 인터럽트 내성 낮음.
**Mode**: DX POLISH. **Product type**: Claude Code Plugin.

**DX-1 결정 (A)**: nudge 는 opinionated default 지만 vibe coder 가 쓸 escape hatch 가
env var 뿐이라 부재였음. → **자연어 영구 opt-out + 완전 카피**.

nudge `additionalContext` → 에이전트가 AskUserQuestion 으로 물음 (problem + delta + fix + escape):

```
[axhub] 플러그인 새 버전이 나왔어요: v<CURRENT> → v<LATEST>
(에이전트: AskUserQuestion 으로 물어요 — upgrade 스킬 라우팅)

AUQ 옵션:
  · 네, 업데이트          → upgrade 스킬
  · 릴리즈 노트 보기       → 변경사항
  · 지금은 그대로          → 이번만 skip
  · 그만 볼래요 (다시 안 봄) → 영구 opt-out marker 기록  ← DX-1 escape hatch
```

escape hatch 동작:
- per-version dedup (새 버전만 nudge, 같은 버전 재nag 0) — 기존 메커니즘.
- **영구 opt-out**: AUQ 옵션 "그만 볼래요" 선택 → helper 가 영구 marker 기록
  (`$XDG_STATE_HOME/axhub-plugin/plugin-drift-optout`). drift-check 가 marker 있으면 미주입.
  AUQ 옵션 = 결정론적(선택→write), nl-lexicon baseline 변경 0, 라우팅 도박 회피 (advisor).
- DX 원칙: "decide for me, **let me override**" — override 가 인터럽트 순간에 한 클릭으로 discoverable.

## 구현 태스크

- [x] **T0 (P0, BLOCKING) — 해소됨** — SessionStart additionalContext 는 advisory (강제 AUQ
      아님; axhub `session_start_megaskill_context` 가 "Suggested" 제안으로 쓰는 게 증거) +
      user turn 이전이라 발동 안 됨. → **trigger = UserPromptSubmit (prompt-route)**, 검증된 surface.
      결정 D4. T2/T4 가 prompt-route 로 재정의됨.
- [ ] **T1 (P1)** — `plugin-latest-fetch-bg` helper subcommand: **ureq** (rustls) 로
      GitHub releases/latest GET (timeboxed) → serde_json 파싱 → **atomic** 캐시 쓰기
      (temp+rename, `atomic_jsonl.rs` 재사용) → 24h TTL. fail-open, panic 금지.
      `crates/axhub-helpers/Cargo.toml` 에 `ureq` 추가 (rustls feature).
  - Verify: `cargo test`; 캐시 생성 + TTL 만료 재fetch + atomic write 단위테스트
- [ ] **T2 (P1)** — `plugin_drift_context() -> Option<String>` 추가. 현 버전 =
      `env!("CARGO_PKG_VERSION")`, 캐시 latest 와 semver 비교, downgrade/동일 skip,
      per-version nudge marker + per-session marker (첫 프롬프트 1회), **drift-optout marker 있으면 None** (DX-1),
      non-interactive None. **완전 카피** (DX 섹션) emit.
      **trigger = `prompt-route` (UserPromptSubmit)** — D4. `cmd_prompt_route` 가 호출,
      `hook_output` UserPromptSubmit additionalContext 로 emit. (SessionStart 아님.)
  - Verify: latest>/==/< + dedup(version+session) + optout + non-interactive 각각 단위테스트 green
- [ ] **T3 (P1)** — fail-open + kill switch: `is_hook_disabled("plugin-drift")` 게이트 +
      `AXHUB_DISABLE_HOOK=plugin-drift` 추가. `docs/HOOKS.md` §1 표 + 매트릭스 갱신.
  - Verify: `cargo test hook_safety` + `tests/hooks-kill-switch.test.ts` 매트릭스 케이스
- [ ] **T4 (P1)** — `session-start.sh` 에 `nohup "$HELPER" plugin-latest-fetch-bg &` 스폰
      (auth-refresh-bg 패턴 미러, `AXHUB_*` opt-out + non-interactive guard).
      **split**: fetch 는 세션 시작에 캐시 warm (1회), nudge 는 첫 prompt-route 에서 캐시 read (D4).
  - Verify: 세션 시작 블로킹 없음 확인, opt-out env 동작
- [ ] **T5 (P2)** — per-version nudge marker + 기존 upgrade-prompts.ndjson "다시 묻지 않기"
      pref 존중. 같은 버전 재nag 0 회 보장.
  - Verify: 같은 버전 2세션 연속 → 1회만 주입 테스트
- [ ] **T6 (P2)** — 에이전트 라우팅 instruction: 주입 context 가 upgrade 스킬 AUQ 로
      확실히 유도되도록 문구 강화 ([MAGIC KEYWORD]/prompt-route 패턴 참고).
  - Verify: 주입 context 로 AUQ 발동되는 QA 시나리오
- [ ] **T8 (P2, DX-1)** — 영구 opt-out 을 **nudge AUQ 4번째 옵션 "그만 볼래요 (다시 안 봄)"**
      으로 전달 (nl-lexicon trigger 아님). 선택 시 helper 가 `plugin-drift-optout` marker 기록.
      **이유**: AUQ 옵션 = 결정론적(선택→write), baseline 재캡처 0, 라우팅 도박(T0 리스크) 회피.
      자연어 trigger phrase 는 no-AUQ surface 가 필요할 때만 fallback. (A 의 UX 동일, mechanism 만 개선)
  - Verify: "그만 볼래요" 선택 → marker 생성 → 다음 세션 nudge 미주입 QA
- [ ] **T7 (P1)** — 테스트 100% 커버리지 (eng-review 다이어그램 14 분기). fetch
      ok/네트워크fail/rate-limit/malformed/캐시권한fail + drift latest>/==/</dedup/non-interactive
      + kill switch. ureq 는 mock HTTP 또는 로컬 fixture 로.
  - Verify: `cargo test` 14 분기 전부 green, fail-open path 패닉 0

## NOT in scope

- 통합 plugin-health 카드 (CLI+plugin+auth+install) — opt-in 확장, 별도 PR (C 안 거부).
- 매 prompt/tool-use 마다 알림 (B 안) — 기술 불가 + prompt 피로로 기각.
- 플러그인 파일 자동 교체 — Claude Code `/plugin update` 소유, v0.1 out of scope (upgrade 스킬 NEVER 규칙).
- CLI binary 업데이트 자동화 — `skills/update` 가 별도 처리.

## 12개월 이상 (Section 10)

- T1 의 캐시된 latest-check 인프라는 향후 통합 health 카드(C 안) 의 기반이 돼요.
- Reversibility: 4/5 — additionalContext 주입 + 캐시는 쉽게 되돌림. marker/캐시 파일만 정리.

## open question (eng-review 후 잔여)

1. ~~GitHub releases vs marketplace repo raw — canonical latest?~~ **해소**: release
   workflow 가 `package.json`+`plugin.json`+`marketplace.json` 을 한 `vX.Y.Z` 태그로
   co-version 해요 (CLAUDE.md Release Workflow). → GitHub releases/latest 태그가 canonical.
2. additionalContext **emit 능력은 코드로 확인** (`hook_output::session_start_context`).
   남은 절반 — 에이전트가 그 context 로 **AUQ 를 실제 쏘는지** 는 **T0 spike 로 선검증** (BLOCKING).
   결과가 트리거 surface(SessionStart vs UserPromptSubmit fallback)를 확정해요.
3. 캐시 파일 경로: `$XDG_CACHE_HOME/axhub-plugin/plugin-latest.json` 적정한가?
4. **ureq supply-chain** (D3=B): 워크스페이스 전체 grep 결과 HTTP/TLS dep **0개**
   (`crates/axhub-helpers` 단일 멤버, `axhub` CLI 는 별도 repo). → ureq 는 **첫 TLS 스택**
   (net-new, 중복 smell 없음 — 재사용할 기존 client 자체가 없음). 핀된 ureq 버전의 TLS
   feature flag 확인 + `cargo deny` advisory/license 통과 + `Cargo.lock` 리뷰 (rust/dependencies.md).

## 병렬화 (worktree)

**Sequential** — T0 가 전부 게이트, T1/T2 둘 다 `axhub-helpers` + `session-start` 를 만져요.
병렬 lane 없음. T0 → (T1·T2·T3·T4) → (T5·T6·T7) 순.
