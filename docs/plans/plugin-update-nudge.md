# 플러그인/CLI 업데이트 알림 강화 (기존 #175 위에)

> ⚠️ **이 문서는 기존 feature 를 강화하는 plan 이에요.** 처음엔 "SessionStart nudge 가
> 미구현" 으로 잘못 알고 from-scratch plan 을 썼는데, CEO 리뷰 system audit 에서
> **이미 main 에 shipped (#175 `843cd31c`)** 임을 발견했어요. 기존 canonical 설계:
> `docs/plans/plugin-update-proactive-nudge.md` (main). 이 문서는 그 위의 delta 예요.

## 발견 (premise 교정)

- **`crates/axhub-helpers/src/plugin_update.rs` (608줄, #175) 이미 존재** — proactive
  plugin-drift nudge 완성품. ureq GitHub releases fetch, 24h atomic 캐시, semver guard,
  per-version marker, opt-out, non-interactive guard, agent-facing AUQ + user-facing fallback.
- 트리거 = **UserPromptSubmit (prompt-route)**, SessionStart 아님. 기존 설계가 이미
  "SessionStart additionalContext 는 advisory → AUQ 강제 못 함 (T0)" 을 분석하고 기각.
  → 내 첫 plan 의 SessionStart 선택은 **이미 열등 판정난 surface**.
- fetch = **ureq (순수 Rust)**, curl 아님. → 내 curl 선택도 열등 (helper HTTP dep 경계는
  #175 가 ureq 로 이미 정함).

**브랜치 위생 (선결):** 현 브랜치 `fix/hide-route-hint-from-user` 는
- merge-base `d1efa248` (0.9.35) — **#175 이전에 분기** → `plugin_update.rs` 자체가 없음
- 유일 커밋 `41cc07e8` 은 main 의 #178 (`7b9590bc`) 과 동일 → 이미 머지됨, redundant
- → **강화 작업은 main 기준 fresh 브랜치에서** 해야 함 (이 브랜치엔 강화할 대상 코드가 없음)

## gstack 대비 진짜 gap (강화 후보)

| 기능 | gstack | axhub #175 | 강화 |
|---|---|---|---|
| Plugin drift nudge | ✓ | ✓ 이미 우수 | — |
| **CLI drift nudge** | ✓ | ✗ (`onboarding_detect::detect_has_update` 온보딩 한정) | **A** |
| 스누즈 escalating backoff | ✓ 24h→48h→7d | ✗ (버전당 1회 후 영구 침묵 + opt-out) | **B** |
| What's-new (업글 후) | ✓ JUST_UPGRADED + changelog | ✗ | **C** |
| 자동 업그레이드 | ✓ auto_upgrade | ✗ (plugin self-mod 금지) | 제외 |
| 캐시 TTL 전략 | 60m 최신 / 12h drift | 24h flat | minor, 제외 |

### A. CLI drift nudge (최우선 — 사용자가 명시한 "CLI + 플러그인 둘 다")
- 현재 `plugin_update.rs` 는 **plugin** drift 만. CLI binary drift 는 `onboarding_detect`
  에만 있고 온보딩 스캔에서만 발동 → 평상시 안 뜸.
- 강화: `axhub update check --json` 결과를 `plugin_drift_context` 와 같은 prompt-route
  채널로 surface. 기존 nudge 인프라 (캐시/marker/opt-out/semver/non-interactive) 재사용,
  CLI 채널만 추가. fetch 는 `axhub update check` (이미 backend hit, bg fetch 패턴 재사용).
- 메시지: `"axhub CLI 새 버전 v{cur}→v{latest}. \"업데이트 확인해줘\"로 안내해요."`

### B. 스누즈 escalating backoff (선택 — 재nag 정책 변경)
- 현재: 버전당 1회 표시 후 그 버전은 영구 침묵 (marker). dismiss 한 사용자는 영영 재알림 없음.
- gstack: "나중에" → 24h→48h→7d backoff 로 재nag. dismiss 했지만 나중에 업글할 사용자 포착.
- tradeoff: axhub 현 정책이 **덜 naggy** (장점). backoff 는 conversion ↑ 하지만 피로 ↑.
- 도입 시 marker 를 `{version, snooze_level, until_epoch}` 로 확장 (gstack `update-snoozed` 등가).

### C. What's-new post-upgrade (선택 — 업글 후 축하/변경사항)
- 현재: 업글 후 아무 안내 없음. gstack: `JUST_UPGRADED v{old}→v{new}` + CHANGELOG bullets.
- 강화: 업글 감지 시 (`CARGO_PKG_VERSION` > 마지막 nudge 한 버전) prompt-route 로
  "v{new} 로 업그레이드됐어요. 변경사항: …" 1회. CHANGELOG.md 파싱 또는 release notes.

## 제약 (기존과 동일 — 재확인)
- fail-open: prompt-route/fetch-bg 어떤 실패도 exit 0, panic 금지, `hook_safety::is_hook_disabled` 첫 줄
- 자동 업그레이드 금지 (plugin self-mod). 알림 전용
- 새 env → §10.6 `AXHUB_DISABLE_*` + docs/HOOKS.md + tests/hooks-kill-switch
- 기존 자산 재사용 (재구현 금지): `plugin_update.rs` 인프라, `onboarding_detect::detect_has_update`,
  `hook_output::session_start_context`, `~/.cache/axhub-plugin/upgrade-prompts.ndjson`, atomic 캐시

## 결정 (CEO 리뷰 확정)

| # | 결정 | 선택 |
|---|---|---|
| D1 | 강화 범위 | **A 단독 (CLI drift nudge)** — B(스누즈 backoff)/C(what's-new) 는 NOT in scope |
| D2 | 브랜치 | **main 기준 fresh 브랜치** — 현 브랜치는 #175 이전 분기라 대상 코드 부재 |
| D3 | 코드 구조 | **generic drift channel 추출** (DRY) — plugin/CLI 가 cache/marker/optout/should_nudge/semver 공유, 채널별로 fetch-source + nudge text + paths + kill-switch 만 분기 |

## ⚠️ Premise 정직성 (#175 는 plugin-only 를 의도적으로 선택)

기존 `plugin-update-proactive-nudge.md` 의 NOT in scope 가 명시:
- `CLI binary 업데이트 자동화 — skills/update 가 별도 처리`
- `통합 plugin-health 카드 (CLI+plugin+auth+install) — 별도 PR 거부 (C안)`

→ #175 저자는 **의도적으로 plugin proactive nudge 만** 만들고, CLI 는 reactive `skills/update`
(+ `cmd_prompt_route` 의 `update_check_intent_present` — 사용자 프롬프트에 update 의도 있을 때 발동)
에 맡겼어요. 따라서 **"CLI drift nudge" 는 명백한 누락(gap)이 아니라, 그 스코프 결정을 다시 여는
사용자 주도 확장**이에요. 사용자가 "CLI+플러그인 둘 다" 를 명시했으니 정당하지만, #175 의
plugin-only 선택과의 의도적 차이임을 기록해요.

### CLI 채널 비대칭 (알려진 limitation)
- plugin: ureq → GitHub public releases (무인증, CLI 불필요)
- CLI: `axhub update check --json` → **`axhub` CLI 존재 필수** + 백엔드 round-trip (인증 여부는
  CLI repo 소관). CLI 미설치(온보딩 전)/네트워크 fail → nudge 안 뜸 (fail-open, 업데이트할 게
  없으니 정상). plugin nudge 와 달리 CLI 의존성을 가짐 — 문서화.
- 중복 방지: `update_check_intent_present` (reactive) 는 사용자가 물을 때, CLI drift nudge
  (proactive) 는 먼저 알림. 충돌 없음 — 단 사용자가 막 update 물은 턴엔 nudge 억제 검토.

## 구현 설계 (A: CLI drift nudge)

### 핵심: 기존 plugin-drift 를 generic 화 (재구현 금지)
현재 `plugin_update.rs` 는 plugin 전용. CLI 채널 추가 시 300줄 복붙 대신 공유 로직 추출:

```
DriftChannel (공유)
  ├─ LatestCache { latest, fetched_at } + TTL(24h) + atomic write
  ├─ should_nudge(cache, current, now, marker, optout, non_interactive)  ← 순수, 그대로 재사용
  ├─ is_newer (semver guard)                                              ← 그대로 재사용
  └─ marker/optout 경로 패턴
채널별 분기:
  ├─ plugin: GitHub releases (ureq) / skill=upgrade / kill=plugin-drift / cache=plugin-latest
  └─ cli:    axhub update check --json / skill=update / kill=cli-drift / cache=cli-latest
```

### 파일별 변경
- **`crates/axhub-helpers/src/plugin_update.rs`**: **무손상** — `should_nudge`/`is_newer`/`read_cache`/`write_cache`/마커·optout 헬퍼를 `pub(crate)` 로 노출만 (plugin arm 로직 0 수정).
- **`crates/axhub-helpers/src/cli_drift.rs`** (신규): `cli_drift_context()` / `cli_drift_nudge()` / `cmd_cli_latest_fetch_bg()` / `cmd_cli_drift_optout()`.
  - CLI latest 소스 = `axhub update check --json` → `{current, latest, has_update}`. **backend `has_update` 권위 source** (CE-1: 로컬 `is_newer` 우회). `current`=CLI 설치 버전 (plugin CARGO_PKG_VERSION 아님).
  - **Note 반영**: `onboarding_detect::detect_has_update`/`parse_update_check` 는 main 에서 **private** → `pub(crate)` 화 (작은 shipped edit, 명시) 또는 ~14줄 parse 복제.
  - `CliLatestCache { current, latest, has_update, fetched_at }` (CE-1: `current` 저장).
  - nudge_text → `update` 스킬 라우팅: `"axhub CLI 새 버전 v{cur}→v{latest}. \"업데이트 확인해줘\"."`
  - kill switch `AXHUB_DISABLE_HOOK=cli-drift`, optout `cli-drift-optout`, marker `cli-drift-nudge-<ver>`
- **`crates/axhub-helpers/src/runtime_paths.rs`**: `cli_latest_cache_path` / `cli_drift_nudge_marker_path` / `cli_drift_optout_path` (plugin 등가 미러)
- **`crates/axhub-helpers/src/main.rs`**: `cmd_prompt_route` 에서 `plugin_drift_context()` 옆에 `cli_drift_context()` 호출. **둘 다 fire 가능 → 턴당 1개 cap** (plugin 우선, 없으면 cli — double-nag 방지). USAGE + dispatch arm `cli-latest-fetch-bg` / `cli-drift-optout` 추가
- **`hooks/session-start.sh`**: `plugin-latest-fetch-bg` fork 옆에 **별도** `cli-latest-fetch-bg` fork. **CE-4 반영: 단일 fork 통합 금지** — CLI fork 는 `axhub` shell 이라 `command -v axhub` 가드 필수 (auth-refresh-bg 와 동일). plugin fork 는 ureq 라 미가드 유지. 병합하면 CLI 부재(온보딩 전) 사용자에 exit-127 (가드 비대칭 깸).
- **테스트**: `should_nudge`/`is_newer` 공유 테스트 유지, CLI 채널 cache/marker/optout/non-interactive 단위 + `cmd_prompt_route` CLI nudge e2e + `tests/hooks-kill-switch.test.ts` 에 `cli-drift` case
- **`docs/HOOKS.md`**: §1 표 `cli-latest-fetch-bg` / kill switch `cli-drift` 추가

### 턴당 1-nudge cap + starvation 완화 (CE-3 반영)
plugin + CLI 둘 다 drift 면 한 턴에 nudge 2개 = 피로. plugin 우선, plugin nudge 없을 때만 cli.
**CE-3 (a) 반영**: plugin 무조건 우선 + 1-cap 이면 빠른 plugin cadence (repo log 0.9.34→0.9.37 며칠)
에서 CLI nudge 영구 후순위(starvation). → **완화: plugin 이 이번 턴 nudge 를 claim 했으면
다음 자격 턴은 CLI 에 양보 (round-robin marker)**. 둘 다 per-version marker 라 새 버전엔 독립.
**CE-3 (b) scope 근거 명문화**: proactive CLI nudge 의 유일 대상 = 설치O+backend도달O+안물어봄.
물어보는 사용자는 기존 reactive `update_check_intent_present`(main.rs:1857) 가 이미 처리.
이 narrow residual 을 위한 비용임을 known-tradeoff 로 기록 (사용자가 A 명시 선택).

## NOT in scope (deferred)
- **B: 스누즈 escalating backoff** — 현 "버전당 1회 후 침묵" 이 덜 naggy (장점). backoff 는
  피로 tradeoff. 사용자가 A 만 선택 → 후속 별도 검토.
- **C: What's-new post-upgrade** — JUST_UPGRADED + CHANGELOG. 가치 있지만 A 범위 밖. 후속.
- **자동 업그레이드** — plugin self-mod 금지 (#175 결정 유지).
- **캐시 TTL escalating** (60m/12h) — 24h flat 유지. minor.

## What already exists (재사용)
- `plugin_update.rs` 전체 nudge 인프라 (#175) — cache/marker/optout/should_nudge/semver/non-interactive
- `onboarding_detect::detect_has_update` — `axhub update check --json` 파싱
- `cmd_prompt_route` (UserPromptSubmit) — 검증된 nudge surface
- `hook_output::session_start_context`, atomic 캐시, `hook_safety::is_hook_disabled`

## Failure modes (전부 fail-open, exit 0 — plugin 채널과 동일)
| codepath | failure | 처리 | 사용자 |
|---|---|---|---|
| cli-latest-fetch-bg | `axhub` 부재/127 | skip, 캐시 미갱신 | 조용 |
| cli-latest-fetch-bg | update check timeout/non-zero | skip, 캐시 유지 | 조용 |
| cli-latest-fetch-bg | JSON malformed | skip | 조용 |
| cli_drift_context | 캐시 없음/stale/<=current | None | 조용 |
| cli_drift_context | marker/optout/non-interactive | None | 조용 |

## Eng-review 결정 (확정)

| # | 결정 | 선택 | 근거 |
|---|---|---|---|
| E1 | CLI 채널 구조 | ~~generic channel 추출 (enum DriftKind)~~ **CE 리뷰서 폐기 → cli_drift.rs sibling** (아래 CE-2) | ~~DRY~~ — CE-2: blast radius 위험, 실제 공유 코드는 pub(crate) 로 충분 |
| E2 | CLI latest 소스 | **`axhub update check --json`** (질문 아님 — 자명) | `detect_has_update` 가 이미 사용 = CLI authoritative 계약. ureq→ax-hub-cli releases 는 중복+URL 하드코딩 |
| E3 | turn-cap | **턴당 1 nudge (plugin 우선)** + `update_check_intent_present` 턴엔 CLI nudge 억제 | double-nag + reactive 중복 방지 |

## ⚠️ CE Adversarial Review 최종 반영 (E1 폐기 — supersedes Eng-review)

독립 CE 리뷰가 self-review 2개(CEO+Eng)가 놓친 2 HIGH 발견. **E1(enum DriftKind 추출) 폐기.**

### CE-1 (HIGH): version-source trap — CLI 는 backend `has_update` 사용, `is_newer`/`LatestCache` 우회
- `plugin_drift_nudge()` 는 `current = env!("CARGO_PKG_VERSION")` (**플러그인** 버전) 하드코딩 → `is_newer(cache.latest, current)`. `LatestCache { latest, fetched_at }` 에 `current` 필드 없음.
- "should_nudge/is_newer 그대로 재사용" 을 곧이곧대로 하면 CLI arm 이 **플러그인 버전 vs CLI latest** 비교 → 무의미.
- **수정**: CLI baseline = `axhub update check --json` 의 `current` (CLI 설치 버전). backend 가 이미 `has_update` 주므로 **CLI 는 backend `has_update` 를 권위 source 로 사용 — 로컬 `is_newer` semver 게이트 우회**. `is_newer` 는 plugin 전용으로 남김.

### CE-2 (HIGH): E1 폐기 → 얇은 `cli_drift.rs` sibling (blast radius 0)
공유 가능한 `should_nudge`/`is_newer` 는 **이미 순수+파라미터화** → `pub(crate)` 노출만으로 재사용. enum 추출은 안 공유되는 채널-고유 코드만 churn (최대 blast, 최소 dedup). → **`plugin_update.rs` 무손상**.

## 코드 구조 (CE 반영 — cli_drift.rs sibling)
```
plugin_update.rs (#175, 무손상)
  pub(crate) should_nudge / is_newer        // ← pub(crate) 노출만 (plugin arm 0 수정)
  pub(crate) read_cache / write_cache / 마커·optout 헬퍼 (제네릭 재사용 가능 부분)

cli_drift.rs (신규, 얇음)
  struct CliLatestCache { current, latest, has_update, fetched_at }   // ← current 저장 (CE-1)
  cmd_cli_latest_fetch_bg()   // axhub update check --json → {current,latest,has_update} 캐시
  cli_drift_nudge() -> Option // backend has_update 사용 (is_newer 우회) + should_nudge 의
                              //   marker/optout/non-interactive 게이트만 pub(crate) 재사용
  nudge_text/system_message   // update 스킬 라우팅
  paths: cli-latest-cache / cli-drift-nudge-<ver> / cli-drift-optout / kill=cli-drift
```
plugin_update.rs **byte-for-byte 무손상** → #175 plugin nudge 회귀 위험 0. cli_drift.rs 단독 테스트.

## ✅ CE flag 검증 (ax-hub-cli repo `axhub/src/commands/update.rs` 확인)

실제 소스 + 실행 (`axhub 0.18.1`) 확인 결과:
- **인증 불필요 (CE flag 해소)**: `fetch_latest_tag()` 는 `{feed_base}/version.txt` 를 USER_AGENT 헤더만으로 GET — **Authorization/token 없음** (`// external (non-AxHub) HTTPS`). → CE-3(b) "로그아웃+설치 사용자 silence" 우려는 **기우**. 모든 설치 사용자(인증 무관)가 체크됨 → served population CE 예상보다 넓음.
- 실측: `axhub update check --json` → `{"current":"v0.18.1","latest":"v0.18.2","has_update":true,"is_downgrade":false,"disabled":false}` exit 0.
- **CE-4 guard 재확인 (유효)**: auth 아니라 **binary 존재** 의존 — `axhub` shell 이라 `command -v axhub` 가드 필수. 그대로 유지.

### 새 요구사항 (소스 확인서 발견)
1. **`disabled:true` 존중 (필수)**: `AXHUB_DISABLE_AUTOUPDATE` 설정 시 check 가 `{disabled:true, has_update:false}` 반환. CLI 가 자체 auto-update 를 끈 사용자에겐 **nudge 안 함** (CLI 레벨 opt-out 이미 표명). → `cli_drift_nudge` 게이트에 `disabled` 체크 추가.
2. **`v` prefix 정규화**: `current`/`latest` 가 `v0.18.1` 형태 (v 접두). 비교/표시 전 strip.
3. **fail-soft fallback 인지**: 네트워크 실패 시 `latest = FALLBACK_LATEST`(`v0.14.0` baked) — 에러 아님. → 오프라인 캐시가 fallback(설치본보다 구버전)일 수 있음 → `has_update:false`, 무해. 단 TTL 재fetch 가 교정.

## Test coverage (CE + ax-hub-cli 확인 반영 — 21 신규 branch)
- **★ CE-1 version-source (HIGH — 필수)**: CLI `current` = `axhub update check` 의 `current` (≠ plugin CARGO_PKG_VERSION) 단언 + backend `has_update=true/false` 가 nudge 결정 (로컬 is_newer 우회) + `v` prefix strip — 3
- **fetch (cmd_cli_latest_fetch_bg)**: kill-switch / cache-fresh-skip / axhub부재(127) / timeout / non-zero / malformed / success-write — 7
- **cli_drift_nudge**: has_update=true→nudge / =false→None / **disabled:true→None (AXHUB_DISABLE_AUTOUPDATE)** / cache없음·stale / dedup-marker / optout / non-interactive — 7
- **cmd_prompt_route turn-cap**: plugin+CLI 동시→1(plugin)+다음턴 CLI 양보(CE-3) / CLI only→CLI / update_check_intent→억제 — 3
- **optout marker write** — 1
- **NO plugin regression (CE-2)**: plugin_update.rs 무손상이라 plugin nudge 회귀 위험 0. `pub(crate)` 노출만 — 기존 #175 테스트 그대로 green (signature 불변 확인)
- **kill-switch**: `tests/hooks-kill-switch.test.ts` 에 `cli-drift` case
- **QA 시나리오**: CLI drift context 주입 → 에이전트 update-skill AUQ 발동 (eval 신규 불필요 — #175 검증 surface 재사용)

## Failure modes (전부 fail-open, exit 0)
| codepath | failure | 처리 | 사용자 | 테스트 |
|---|---|---|---|---|
| cli-latest-fetch-bg | axhub 부재(127) | skip | 조용 | Y |
| cli-latest-fetch-bg | timeout/non-zero | skip, 캐시 유지 | 조용 | Y |
| cli-latest-fetch-bg | JSON malformed | skip | 조용 | Y |
| cli_drift_context | 캐시 없음/stale/<=current | None | 조용 | Y |
| cli_drift_context | marker/optout/non-interactive | None | 조용 | Y |
| cmd_prompt_route | plugin+CLI 동시 | 1 nudge (plugin 우선) | nudge 1개 | Y |

→ silent+untested+unhandled 조합 **0개** (critical gap 없음).

## 병렬화 (worktree)
**Sequential** — E1 의 generic 추출이 전부 게이트 (plugin_update.rs 재구조화 후에만 CLI arm 추가 가능).
이후 fetch·nudge·prompt-route 통합 모두 같은 `drift.rs` + `main.rs` 터치 → 병렬 lane 없음.
추출 → CLI arm → prompt-route 통합 → 테스트 순.

## 검증 (CLAUDE.md self-check)
- [ ] `cargo test --workspace` (기존 #175 plugin 테스트 green = regression guard) / `bun test` / `bunx tsc --noEmit`
- [ ] `skill:doctor --strict` / `lint:tone` / `lint:keywords`
- [ ] `gitnexus_impact` on `cmd_prompt_route` / `plugin_drift_nudge` 추출 전
- [ ] fail-open exit 0 / `tests/hooks-kill-switch.test.ts` 에 `cli-drift` 추가
- [ ] `docs/HOOKS.md` 갱신 (`cli-latest-fetch-bg` + kill-switch `cli-drift`)
- [ ] 17 신규 branch 전부 테스트 + plugin/CLI 비간섭 regression

## NOT in scope (eng-review 재확인)
- B(스누즈 backoff) / C(what's-new) / 자동 업그레이드 / 캐시 TTL escalating — CEO 리뷰서 deferred
- ureq→ax-hub-cli releases 대안 CLI 소스 — E2 에서 기각 (axhub update check 가 authoritative)

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 1 | clean | premise 교정(#175 기존), scope A lock |
| Eng Review | `/plan-eng-review` | Architecture & tests | 1 | superseded | E1 generic 추출 → CE 가 폐기 |
| **CE Adversarial** | `ce-adversarial-document-reviewer` | 독립 stress-test | 1 | **2 HIGH + 2 MED 반영** | CE-1 version-source trap, CE-2 E1 폐기, CE-3 starvation, CE-4 guard |
| Design Review | `/plan-design-review` | UI/UX | 0 | — | UI scope 없음 |

- **UNRESOLVED:** 0 (CE 4 findings + Note 전부 반영: E1→cli_drift.rs / backend has_update / 2-fork guard / pub(crate) / starvation 완화)
- **VERDICT:** CE-hardened — 구현 준비 완료. self-review 2개가 놓친 version-source trap 을 독립 리뷰가 잡음. fresh 브랜치(main 기준) 선결.
- **KEY LESSON:** 같은 에이전트 self-review (CEO+Eng) 는 비독립이라 "reuse 의 mechanics" 만 검증, "무엇을 reuse 하는가(CLI current ≠ plugin version)" 는 못 봄. 독립 CE 리뷰가 헤드라인 버그 포착.
- **NOTE:** gstack review-log/dashboard 는 이 repo(비-gstack)에 미설치 — 리포트는 plan 문서에만 기록.
