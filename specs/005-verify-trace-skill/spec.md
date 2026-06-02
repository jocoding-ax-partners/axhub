# Feature Specification: Trace 스킬 동작 검증 (skills/trace)

**Feature Branch**: `005-verify-trace-skill`

**Created**: 2026-06-02

**Status**: **Implemented (R3γ+R2+R1, 2026-06-02)** — F3 RESOLVED (evidence-source 를 런타임 로그로 재설계). cargo trace/full + clippy + skill gates(doctor/tone/keywords) + trace-skill 전부 green, **내 변경 0 regression** (bun 전체 18 fail 은 worktree pre-existing — stash-baseline 으로 입증). · 원 검증 기록: authoring/contract PASS · 핵심 기능 BROKEN→복구 · F2 reachability 수정 완료.

**Input**: User description: "skills/trace 에 있는 스킬이 제대로 동작하는지 검증해주고 안맞다면 보완 계획 작성해줘."

**Verified commit**: `origin/main` 36253cb (v0.9.23). `skills/trace/` 는 사용자의 main 체크아웃과 byte-identical (`diff -rq` 확인). 격리 워크트리 `worktree-prancy-chasing-valiant` 에서 검증.

---

## Summary (검증 결론 먼저)

`skills/trace` 스킬은 **authoring/contract 계층(frontmatter·gate·registry·tone)은 정상**이지만, **핵심 기능인 "빌드 실패 원인 → empathy 매칭"이 현행 axhub backend 대비 동작하지 않아요 (F3, CLI/API 소스로 확정)**.

근거 (primary source, ax-hub-cli):
- `crates/axhub-api/src/deploy.rs` 에 로그 라우트는 `list_app_logs` → `GET /api/v1/apps/{app}/logs` **하나뿐** — **build-log 또는 deploy-scoped 로그 엔드포인트가 없어요.**
- `axhub/src/commands/deploy/logs.rs`: `--source` 는 파싱만 되고 미사용(help "app logs are not server-side filtered"), positional `deployment_id` 는 legacy 라 `run_once` 가 안 읽어요 → `deploy logs` 는 **app-level 런타임 로그 NDJSON**(`{ts,container,message}`)을 반환.
- `watch.rs`: `--source build` 는 **표시 라벨일 뿐** 모든 source 가 같은 `list_app_logs` 호출.
- helper(`main.rs:2281`)는 `deploy logs <id> --app --source build` 를 호출하고 raw NDJSON 을 `extract_error_lines` 에 그대로 먹여요.

결과: **빌드 단계** 실패 시 앱이 안 떠서 런타임 로그가 비고 → `build_log_errors` 빈 배열 → `matched_patterns` 빈 배열 → trace 가 generic "자동 매칭 실패" fallback 으로 떨어져요. 즉 헤드라인 기능이 현행 backend 에서 non-functional.

여전히 동작하는 것: 로컬 `event_log`(phase 타임라인 + `failure_reason`)와 audit context — CLI 비의존이라 OK.

F2(ERROR/WARN 필터 reachability)는 실재하지만 **F3 하위**의 부차 버그 — 소스가 틀려서 올바른 데이터가 안 들어오는 상황이라, 필터만 고쳐도 빌드 에러는 안 잡혀요. R1(cosmetic 라벨)은 그대로.

**한 줄 verdict**: 스킬 자체는 잘 만들어졌지만, 의존하는 `axhub deploy logs` 가 build-log 를 더 이상 제공 안 해서 핵심 기능이 깨졌어요. 보완은 filter 튜닝이 아니라 **evidence-source 재설계**(R3)예요.

---

## Clarifications

### Session 2026-06-02

- Q: 검증 spec 다음 단계 범위? → A: R2 + R1 모두 진행 (reachability 기능 수정 + cosmetic 라벨). /speckit-plan 으로 R2 구현 계획 수립.
- Q: R2 접근법 / A1(upstream 로그 포맷) 처리? → A: R2 를 **format-robust** 로 구현(A1 비의존)하고, A1 경험 확인은 로컬 설치된 axhub CLI 로 **R2 구현 단계에서** 수행. (clarify 중 시도한 live probe 는 inconclusive — 설치 CLI 0.17.2 가 helper supported range 밖이라 결과를 스킬에 귀속 불가. Assumptions A1 참조.)
- Q(소스 확인 후 premise 정정): build-log 부재 확정(F3) → trace evidence-source 를 어디서? → A: **γ 런타임 로그 재정의** — `deploy logs` NDJSON `message` 매칭으로 런타임 실패 추적, 빌드 단계 실패는 event_log `failure_reason`. (R2 는 F3/R3 하위로 강등.)

---

## User Scenarios & Testing *(mandatory)*

검증 대상 스킬이 보장해야 하는 사용자 여정. 각 여정은 독립적으로 테스트 가능해요.

### User Story 1 - vibe coder 가 배포 실패 원인을 묻기 (Priority: P1)

배포가 실패한 사용자가 "왜 실패했어" / "원인 알려줘" 라고 물으면, 스킬이 event_log + build_log(R3 후 runtime_log) + audit 3 source 를 통합해서 phase 타임라인 + 마지막 에러 + 4-part empathy 안내를 1 화면으로 보여줘요.

**Why this priority**: 스킬의 존재 이유. 이게 안 되면 스킬 자체가 무의미.

**Independent Test**: SKILL 본문이 `axhub-helpers trace --deploy-id=$ID --app "$APP" --json` 를 호출하고, 결과의 `matched_patterns` 로 `references/error-patterns.md` 의 entry 를 출력하는지 검증. (`tests/trace-skill.test.ts` 가 자동 검증.)

**Acceptance Scenarios**:

1. **Given** 마지막 배포가 `env: STRIPE_KEY not found` 로 실패, **When** 사용자가 "왜 실패했어" 입력, **Then** helper 가 `matched_patterns: ["env_not_found"]` 반환 + SKILL 이 env_not_found 4-part empathy entry 출력.
2. **Given** preflight 에 `current_app` + `last_deploy_id` 존재, **When** 스킬 실행, **Then** 추가 질문 없이 그 deploy 를 추적.

### User Story 2 - 비대화형 / CI 환경에서 안전하게 동작 (Priority: P2)

`claude -p`, CI, headless subprocess 에서 추적 대상이 모호할 때 (Failed 후보 여러 개) AskUserQuestion 을 건너뛰고 안전 기본값으로 진행해요.

**Why this priority**: 잘못된 deploy_id 로 trace 호출 시 빈 결과 + 사용자 혼란 방지. fail-safe 계약.

**Independent Test**: `registry.json` 의 `trace` 채널 safe_default 가 `"abort"` 인지 + D1 guard 가 SKILL 본문에 존재하는지 검증.

**Acceptance Scenarios**:

1. **Given** 비대화형 환경 + Failed 후보 2개 이상, **When** 스킬이 추적 대상 결정 단계 도달, **Then** AskUserQuestion 생략 + `abort` (추적 중단) 으로 안전하게 종료.
2. **Given** 대화형 환경 + Failed 후보 2개 이상, **When** 같은 단계 도달, **Then** "가장 최근 / 직접 입력" AskUserQuestion 제시.

### User Story 3 - helper transport/auth 실패 시 복구 라우팅 (Priority: P3)

helper 가 인증/전송 실패를 반환하면 (`auth_ok=false`, `auth_error_code`) 스킬이 적절한 복구 슬래시 스킬로 안내해요 (`cli_not_found`→install-cli, `cli_config_corrupted`→auth, `cli_too_old`→upgrade).

**Why this priority**: 추적 자체가 막혀도 사용자를 막다른 길에 두지 않음. graceful degradation.

**Independent Test**: SKILL 본문의 preflight 해석 분기가 `recover/SKILL.md` Step 7 라우팅 표를 참조하는지 검증.

**Acceptance Scenarios**:

1. **Given** helper 가 `cli_too_old`, **When** 스킬 preflight 평가, **Then** `/axhub:upgrade` 안내.
2. **Given** helper 가 치명적이지 않은 경고, **When** 스킬 실행, **Then** 워크플로 계속 진행.

### Edge Cases

- 추적할 Failed 배포가 0개 → "추적할 실패 배포 없음" 안내 + 종료. (SKILL Step 1)
- `axhub logs` hang (5s timeout 초과) → evidence 불완전 상태로 안내, hang 무시 금지. (SKILL NEVER 3)
- build_log 에 매칭 패턴 없음 → `(no_pattern_match)` generic fallback entry 출력. (catalog 마지막 entry)
- raw build_log stderr → ERROR/WARN 최대 5 줄만 인용 (Vibe Coder Visibility). (SKILL NEVER 1)

---

## Requirements *(mandatory)*

검증 기준이 된 계약. 각 FR 은 "스킬이 제대로 동작한다" 의 정의이자 acceptance criteria 예요. 우측 상태는 이번 검증 결과.

### Functional Requirements

- **FR-001**: SKILL 은 frontmatter 에 `multi-step: true`, `needs-preflight: true`, `model: sonnet`, `allows-dependency-execution: false` 를 선언해야 해요. — ✅ PASS
- **FR-002**: SKILL 은 load-time `!command` 주입 없이 in-body preflight 블록 (`PREFLIGHT_JSON=$("$HELPER" preflight --json` signature + helper-pick fallback) 을 포함해야 해요. — ✅ PASS
- **FR-003**: SKILL 은 Step 0 에서 TodoWrite 체크리스트를 렌더하고 종료 시 전부 `completed` 로 만들어야 해요. — ✅ PASS
- **FR-004**: SKILL 은 비대화형 환경용 D1 guard (`! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]`) 를 명시해야 해요. — ✅ PASS
- **FR-005**: 모든 AskUserQuestion 의 question text 는 `tests/fixtures/ask-defaults/registry.json` 에 safe_default + allowed_safe_defaults + rationale 가 등록돼야 해요. trace 의 safe_default = `abort`. — ✅ PASS
- **FR-006**: SKILL 은 evidence 수집에 `axhub-helpers trace --deploy-id=$ID --app "$APP" --json` 를 호출해야 하고, 그 CLI verb 가 helper 바이너리에 wire 돼야 해요. — ✅ PASS (`main.rs` USAGE 에 `trace --deploy-id <id> [--app <app>] [--json]` 존재)
- **FR-007**: SKILL 은 대상 식별 fallback 으로 `axhub-helpers list-deployments --app "$APP" --limit 5 --json` 를 호출해야 하고, 그 verb 가 wire 돼야 해요. — ✅ PASS (`list-deployments` verb + `ListDeploymentsArgs/Result` 존재)
- **FR-008**: `references/error-patterns.md` 의 패턴 key **집합** 과 `trace_helper.rs::ERROR_PATTERNS` 가 내보내는 key 집합이 일치해야 해요 (drift 금지). — ✅ PASS (8 key 완전 일치: env_not_found / oom / module_not_found / network_timeout / dependency_install_failed / docker_image_pull_failed / port_already_in_use / build_command_failed). ※ key **alignment** 만 보장 — 각 패턴의 실제 도달 가능성은 FR-012 참조.
- **FR-009**: helper 의 `TraceReport` JSON 필드가 SKILL 이 문서화한 출력 (deploy_id / last_phase / failure_reason / phase_durations / build_log_errors / matched_patterns) 을 포함해야 해요. — ✅ PASS (TraceReport ⊇ 문서 필드; routing_context + warnings 는 추가 필드)
- **FR-010**: 모든 한글 텍스트는 해요체여야 해요 (`lint:tone --strict` 0 err). — ✅ PASS
- **FR-011**: nl-lexicon trigger 어구는 frontmatter `description:` 에만 존재하고 baseline 과 일치해야 해요 (`lint:keywords --check`). — ✅ PASS
- **FR-012**: 8개 error pattern 각각이 실제 build_log 파이프라인 (`extract_error_lines` → `match_error_patterns`) 에서 **도달 가능** 해야 해요 — 즉 needle 을 담은 로그 라인이 `extract_error_lines` 의 `ERROR`/`FATAL`/`WARN` 필터 (trace_helper.rs:142) 를 통과해야 매칭돼요. — ⚠️ CONDITIONAL (F2 참조). e2e fixture 는 ERROR/WARN-태그된 라인만 먹여서 happy-path 만 입증. raw 무태그 라인 (`env: X not found`, `npm ERR! ...`) 은 필터에서 drop → 해당 패턴 미발화. upstream 로그 포맷이 태그를 보장하는지 미확인. (※ R3 후 **runtime-log** 파이프라인 기준 — Assumptions 의 Terminology 참조.)
- **FR-013** (R3, headline): trace 는 실패 evidence 를 **현행 backend 에서 실제 가용한 소스**(런타임 로그 `deploy logs` NDJSON `message` + 로컬 event_log `failure_reason`)에서 얻어야 해요. build-log 부재(F3) 시 generic fallback + `runtime_log_unavailable` warning 으로 정직하게 degrade. — ⚠️ 미구현 (R3γ; tasks Phase 3/R3).

### Key Entities

- **TraceReport**: helper 가 합성하는 추적 결과. 필드 = deploy_id, last_phase, failure_reason, phase_durations[], build_log_errors[], routing_context?, matched_patterns[], warnings[].
- **Error Pattern Catalog**: `references/error-patterns.md` 의 8 + fallback entry. 각 entry = 4-part (감정 / 원인 / 해결 / 버튼). key 로 Rust `ERROR_PATTERNS` 와 매핑.
- **Ask-Defaults Registry**: `registry.json` 의 `trace` 채널. question text → { safe_default: "abort", allowed_safe_defaults, rationale }.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `bun run skill:doctor --strict` exit 0 — trace 의 D1 / TodoWrite Step 0 / in-body preflight / model routing / step-numbering 5개 항목 전부 ✓. — ✅ 달성
- **SC-002**: `bun run lint:tone --strict` 0 error / 0 warning (46 files). — ✅ 달성
- **SC-003**: `bun run lint:keywords --check` baseline diff 0. — ✅ 달성
- **SC-004**: trace 관련 bun test 100% pass (`tests/trace-skill.test.ts` + `tests/skill-doctor.test.ts` = 6 pass / 0 fail). — ✅ 달성
- **SC-005**: error-pattern key parity 100% (catalog ↔ Rust ↔ SKILL Step 3 목록, 8/8 일치). — ✅ 달성 (alignment)
- **SC-006**: untagged-risk 패턴(env_not_found / dependency_install_failed / oom / docker_image_pull_failed / build_command_failed)이 raw 무태그 fixture 로 발화 입증 + 오탐 회귀(`zoom`/`room` → oom 미발화). 전수 8개 fixture 는 비목표 — 대표 + risk 패턴 우선. — ✅ 달성 (cli_e2e: env_not_found(untagged) / dependency_install_failed(npm err!, untagged) 발화 + trace_helper unit: zoom/room → oom 미발화. R2 구현 완료)

---

## Verification Results (검증 실행 증거)

| # | 검증 항목 | 명령 / 방법 | 결과 |
|---|---|---|---|
| V1 | 스킬 파일 무결성 | `diff -rq skills/trace <main checkout>` | IDENTICAL |
| V2 | contract gate | `bun run skill:doctor --strict` | exit 0 ✅ |
| V3 | 해요체 톤 | `bun run lint:tone --strict` | 0 err / 0 warn / 46 files ✅ |
| V4 | trigger baseline | `bun run lint:keywords --check` | no diff ✅ |
| V5 | 스킬 불변식 | `bun test tests/trace-skill.test.ts tests/skill-doctor.test.ts` | 6 pass / 0 fail ✅ |
| V6 | CLI verb wiring | `main.rs` USAGE grep (`trace` / `list-deployments` / `preflight`) | 3/3 존재 ✅ |
| V7 | JSON 계약 | `trace_helper.rs::TraceReport` 필드 ⊇ SKILL 문서 출력 | 충족 ✅ |
| V8 | pattern key **alignment** | `ERROR_PATTERNS` ↔ catalog ↔ SKILL Step 3 (8 key) | 완전 일치 ✅ |
| V9 | non-interactive 안전값 | `registry.json` trace.safe_default | `"abort"` ✅ |
| V10 | pattern **reachability** | `cli_e2e.rs::cli_trace_json_reads_events_and_build_log_patterns` fixture (`fake_axhub_logs`) 분석 + `extract_error_lines` 로직 | ⚠️ happy-path only — fixture 가 ERROR/WARN-prefixed (`build_command_failed`·`network_timeout` 만 입증). raw 무태그 needle 미검증 |
| V11 | **evidence-source 계약** (CLI/API 소스) | `ax-hub-cli` 의 `axhub-api/src/deploy.rs`(로그 라우트) + `commands/deploy/logs.rs`(`--source`·deploy-id 처리) + `watch.rs` + helper `main.rs:2281` 교차 분석 | ❌ **BROKEN** — build-log 엔드포인트 부재, `deploy logs` 가 source/deploy-id 무시하고 app 런타임 로그 반환 (F3) |

### Findings

- **F3 (functional — ✅ RESOLVED via R3γ, 2026-06-02)**: trace 의 evidence source 가 현행 axhub backend 와 깨졌던 결함 — 런타임 로그 재설계로 복구. helper 가 `axhub deploy logs` NDJSON `message` 를 파싱하고, 매칭을 display 와 분리, 빌드 단계 실패는 event_log `failure_reason` fallback. 소스 확정 (원 결함 근거):
  - `axhub-api/src/deploy.rs`: 로그 API 는 `list_app_logs` → `GET /api/v1/apps/{app}/logs` (app-level) **하나뿐**. build-log / deploy-scoped 로그 엔드포인트 없음.
  - `commands/deploy/logs.rs`: `--source` 미사용(파싱만), `deployment_id` legacy 미사용 → `deploy logs` = app 런타임 로그 NDJSON.
  - `commands/deploy/watch.rs`: `--source build` 는 표시 라벨, 데이터는 동일 `list_app_logs`.
  - helper `main.rs:2281`: `deploy logs <id> --app --source build` 호출 + raw NDJSON 을 `extract_error_lines` 에 그대로 투입 (NDJSON `message` unwrap 도 없음).
  - **귀결**: 빌드 단계 실패 → 앱 미기동 → 런타임 로그 empty → `build_log_errors`/`matched_patterns` empty → 헤드라인 매칭 기능 non-functional. (event_log 의 phase/`failure_reason` 는 로컬이라 생존.)
  - **F2 와의 관계**: F2(필터 reachability)는 F3 하위. 소스가 틀린 상태라 필터를 고쳐도 빌드 에러 라인이 애초에 안 들어와요. F3 해소(evidence-source 재설계) 후에야 F2 가 의미 있어요.
  - **정정 노트**: clarify 단계에서 이 신호를 "out-of-range pairing artifact, out-of-scope" 로 미뤘던 판단을 **철회**해요. CLI/API 소스가 현행 contract 임을 확정 — version-range 핑계가 아니라 helper 가 CLI 의 log-API 변경을 못 따라간 실제 결함이에요. plugin 0.9.8 이 최신이라 `/axhub:upgrade` 로도 안 풀려요.

- **F1 (cosmetic, non-blocking)**: SKILL.md 본문 (D1 guard 단락) 이 추적 대상 질문 채널을 `trace_target_selection` 이라는 라벨로 지칭해요. 하지만 registry 와 `tests/ux-ask-fallback-registry.test.ts` 는 question **text** (`"최근 Failed 배포가 여러 개예요. 어떤 거 추적할까요?"`) 를 key 로 써요. `trace_target_selection` 문자열은 repo 어디에도 실제 key 로 존재하지 않아요 (grep 0건). 테스트 green, 동작 무관 — 순수 문서 표현 drift.

- **F2 (functional, conditional — 사용자가 요청한 "안 맞다면" 핵심 항목)**: error-pattern 매칭은 2단 파이프라인이에요 — `extract_error_lines` (raw build log 에서 `ERROR`/`FATAL`/`WARN` substring 을 가진 라인만 최대 5줄 추림, trace_helper.rs:138-150) → `match_error_patterns` (그 생존 라인에 needle substring 매칭, :172). 문제는 `ERROR_PATTERNS` needle 중 다수가 자신의 자연스러운 raw 형태에 `ERROR`/`WARN`/`FATAL` 토큰을 안 가진다는 점이에요:
  - `env: ` (env_not_found), `npm err!` (dependency_install_failed — "ERR!" 는 "ERROR" 부분문자열 아님), `oom`/`out of memory` (oom), `docker pull` (docker_image_pull_failed), bare `exit code 1`/`build command failed`, bare `connection refused`/`address already in use` → 이 라인들은 1단 필터에서 **silently drop** 되어 needle 이 발화 못 해요.
  - 반면 Node 계열은 `Error: Cannot find module` / `Error: listen EADDRINUSE` 처럼 `Error:` 접두가 붙어 "ERROR" 를 포함 → module_not_found / port_already_in_use 는 도달 가능.
  - 즉 8개 중 약 5개 패턴의 발화 여부가 **upstream `axhub deploy logs --source build` 가 라인을 ERROR/WARN/FATAL 로 태그하는지** 에 달려 있어요. e2e fixture (`fake_axhub_logs`) 는 라인을 미리 `ERROR …`/`WARN …` 로 태그해서 (cli_e2e.rs:3468-3470) 이 의존성을 가려요 — 그래서 happy-path 만 green.
  - **내부 일관성 신호**: needle 집합은 raw 무태그 형태 (`env: `, `npm err!`) 를 기대하는데 추출 필터는 태그된 라인만 통과시켜요. error-patterns.md catalog 도 `env: <KEY> not found` (무태그) 로 기술하지만 SKILL Example 은 `ERROR env: ...` (태그) 로 보여줘서, 저자 의도와 구현 사이에 미세한 어긋남이 있어요.
  - **F3 로 갱신**: upstream 이 build-log 자체를 안 주는 게 확정돼서, 이 reachability 리스크는 **F3 의 하위 항목으로 흡수**돼요. F2 필터 수정은 F3(evidence-source 재설계)로 올바른 데이터가 들어온 **뒤에** 적용해야 의미가 있어요.

---

## Remediation Plan (보완 계획)

CLI/API 소스로 **확정 결함 1건 (F3 → R3, P1)** 이 드러났어요. 우선순위 재정렬: **R3 (evidence-source 재설계, primary)** → R2 (필터 reachability, R3 이후에만 의미) → R1 (cosmetic).

> **상태 변경 (2026-06-02, CLI 소스 확인 후)**: clarify 때 "R2 가 주 작업" 으로 본 전제는 무효예요. 진짜 원인은 필터(F2)가 아니라 evidence-source(F3)라서, 주 작업은 **R3** 로 바뀌어요. R3 에는 사용자 결정이 필요한 **설계 fork** 가 있어요 (아래 ⬥).

### R3 (functional, P1, primary) — evidence-source 재설계 (build-log 부재 대응)

- **대상**: helper `RealTraceProbes::axhub_build_log` (`main.rs:2269-2299`) + `trace_helper.rs` (matching 입력 소스) + `skills/trace/SKILL.md` (3-source 계약 문구) + `references/error-patterns.md` (필요 시).
- **문제 (F3)**: `axhub deploy logs --source build` 가 현행 backend 에서 build-log 를 안 줌(app 런타임 로그 NDJSON 반환). 빌드 실패 시 런타임 로그 empty → 매칭 불가.
- **⬥ 설계 fork → 결정 (2026-06-02): γ 런타임 로그 재정의.** trace 의 evidence 를 런타임 로그(`deploy logs` NDJSON 의 `message`)로 재정의하고, **빌드 단계** 실패는 로컬 event_log `failure_reason` 으로 안내. 후보 기록:
  - **α. 로컬 event_log `failure_reason` 매칭 (권장 후보)**: deploy 시점에 로컬 `deploy-events/<id>.jsonl` 에 이미 기록됨(CLI 비의존). build-log probe 제거, `match_error_patterns` 를 `failure_reason` 에 적용. 단 reason 이 coarse("build command failed")면 8패턴 변별력이 떨어질 수 있어 — reason 작성부(deploy hook)도 같이 봐야 함.
  - **β. deployment status API**: `get_deployment` → `current_stage`/`stage`/`cloud_build_id` 로 실패 단계 식별 + reason. 네트워크 의존.
  - **γ. 런타임 로그로 재정의**: `deploy logs` 를 런타임 로그로 인정, NDJSON `message` 파싱 후 런타임 에러 매칭. trace 의 의미를 "빌드 실패"에서 "런타임 실패"로 재범위화 (빌드 실패는 event_log reason 으로).
  - **δ. backend build-log API 대기**: 곧 build-log 엔드포인트가 추가될 예정이면 helper 만 새 라우트로 갱신 + 그때까지 명시적 degrade 안내.
- **공통 작업 (fork 무관)**: (a) helper 가 NDJSON 을 받으면 raw 가 아니라 `message` 필드를 unwrap 해서 매칭/표시 (현재 raw JSON 노출 — Vibe Coder Visibility NEVER 위반 소지). (b) 로그 미가용 시 `runtime_log_unavailable` warning + generic fallback 으로 정직하게 degrade. (c) SKILL "3-source(event_log+build_log+audit)" 문구를 실제 소스로 갱신.
- **검증**: 선택한 소스에 대한 unit/e2e + (가능 시) in-range axhub 로 실패 배포 1건 end-to-end.
- **우선순위**: **P1** — 헤드라인 기능 복구. fork 결정 전엔 구현 착수 불가.

### R2 (functional, P2 — **R3 이후 적용**) — error-pattern reachability 확정 + lock

- **대상**: `crates/axhub-helpers/src/trace_helper.rs::extract_error_lines` + `ERROR_PATTERNS` needle 정밀화 + `crates/axhub-helpers/tests/cli_e2e.rs` / 단위 fixture.
- **문제**: F2 참조 — needle 매칭이 추출된 5라인(ERROR/WARN/FATAL)만 보므로, 태그 없는 라인(`env: `, `npm err!` 등)의 패턴이 발화 못 해요.
- **접근 (plan/research D3 확정: match/display 분리)**: `extract_error_lines`(표시 5라인)는 그대로 두고 **매칭만** 전체 message 로 분리해요. display 를 넓히면 NEVER 규칙(비태그 raw 노출 금지) 위반이라 union 확장은 **기각**(research D3).
- **단계**:
  1. `matched_patterns` 를 **전체 parsed message** 기준으로 계산 (display 5-라인 필터와 분리). `build_log_errors`(표시)는 ERROR/FATAL/WARN max 5 **불변** → upstream 태그 여부와 무관하게 패턴 도달, NEVER 규칙 보존.
  2. **needle 정밀화** — `oom` (3자 substring → `room`/`zoom` 오탐), bare `exit code 1` 등을 word-boundary/맥락으로 강화. 필터를 union 으로 완화하면 오탐 표면적이 커지므로 필수 동반 작업.
  3. **회귀 lock** — `cli_e2e.rs` 에 raw 무태그 fixture (`env: STRIPE_KEY not found`, `npm ERR! code ELIFECYCLE`) 추가 → `matched_patterns` 에 `env_not_found`/`dependency_install_failed` assert. + `trace_helper.rs` 에 `extract_error_lines` → `match_error_patterns` **결합** 경로 단위 테스트 추가 (현재 :247-275 는 `match_error_patterns` 를 직접 호출해 추출 단계를 우회함).
  4. ~~A1 경험 확인 (build log 캡처)~~ — **삭제**: F3 으로 build-log 부재 확정. R2 는 R3 가 정한 소스의 라인 텍스트에 대해 적용해요.
- **영향 범위**: helper 1-2 함수 + e2e/unit fixture. (SKILL 3-source 문구 갱신은 R3/D6 에서 처리 — R2 자체는 SKILL.md 무변경.)
- **검증**: `cargo test -p axhub-helpers trace` + `cargo test -p axhub-helpers --test cli_e2e` green + 새 raw-fixture + 오탐 회귀 케이스.
- **우선순위**: P2 — happy-path 는 동작하지만 "모든 실패 자동 추적" 약속의 신뢰도를 결정.

### R1 (optional, cosmetic, P3) — SKILL 본문의 채널 라벨 명확화

- **대상**: `skills/trace/SKILL.md` D1 guard 단락의 "`trace_target_selection` safe_default 는 ..." 문장.
- **문제**: `trace_target_selection` 은 코드/registry 어디에도 없는 설명용 라벨이라, 독자가 실제 registry key 로 오해할 수 있어요.
- **수정안**: 라벨을 실제 매칭 기준 (registry 의 `trace` 채널 + question text) 으로 바꾸거나, "(question text 로 매칭)" 한 구절 추가.
- **영향 범위**: 문서 1줄. 기능/테스트 영향 0. `bun run skill:doctor --strict` + `bun run lint:tone --strict` 재실행으로 충분.
- **우선순위**: P3 (안 고쳐도 동작엔 문제 없음).

---

## Implementation (2026-06-02)

`/speckit-implement` 로 R3γ + R2 + R1 (23 tasks) 구현 완료.

**변경 파일**:
- `crates/axhub-helpers/src/trace_helper.rs`: 매칭/표시 분리 (`matched_patterns` = 전체 message, `build_log_errors` = ERROR/WARN max5 불변) + 빌드 단계 event_log `failure_reason` fallback + needle 정밀화 (`oom` word-boundary + `oomkilled`, `exit code 1` 경계). 단위 테스트 2 추가.
- `crates/axhub-helpers/src/main.rs`: `RealTraceProbes` 가 `axhub deploy logs --app --limit` (`--source`/deploy-id 제거) 호출 + NDJSON `message` 파싱 + `runtime_log_*` warning.
- `crates/axhub-helpers/tests/cli_e2e.rs`: fake → NDJSON, `fake_axhub_app_logs` 헬퍼 + e2e 3 (untagged env, npm err!, empty→fallback).
- `skills/trace/SKILL.md` + `references/error-patterns.md`: 3-source 문구 build_log→runtime_log, `--source build` 전제 제거, 라벨 정리.
- `tests/trace-skill.test.ts`: assertion runtime_log 로 갱신.

**검증 (내 scope green, 0 regression)**:
| gate | 결과 |
|---|---|
| `cargo test -p axhub-helpers trace` | 8 e2e(신규 3) + unit + integration, 0 fail |
| `cargo test -p axhub-helpers` (full) | 0 fail |
| `cargo clippy -p axhub-helpers --all-targets` | 0 warn |
| `bunx tsc --noEmit` | clean |
| `skill:doctor --strict` / `lint:tone` / `lint:keywords` | 전부 pass |
| `bun test tests/trace-skill.test.ts` | 3 pass |

**Out-of-scope (pre-existing, 내 변경 무관)**: `bun test` 전체 18 fail (README release-summary / ux-autowire-cli / token-freshness-gate / plan-consistency / hooks-kill-switch). stash-baseline 비교로 **내 변경 없이도 동일 실패** 확인 — worktree/version drift, 별도 이슈.

---

## Assumptions

- **A1 (RESOLVED via CLI/API 소스 — moot)**: "upstream 이 실패 라인을 ERROR/WARN 로 태그하는가" 라는 원래 질문은 **무의미해졌어요** — 소스 확인 결과 현행 backend 엔 build-log 가 아예 없어요(`list_app_logs` 만 존재). 태그 여부가 아니라 **소스 자체가 빌드 로그가 아님**이 문제예요(F3). 따라서 매칭 대상을 바꿔야 해요(R3).
- **정정 (이전 "out-of-scope artifact" 판단 철회)**: clarify 때 `deploy logs --source build` drift 를 "out-of-range pairing artifact, spec 범위 밖" 으로 미뤘는데, **사용자 지시대로 CLI 소스를 직접 읽어 철회**해요. `commands/deploy/logs.rs` + `axhub-api/src/deploy.rs` 가 이게 현행 CLI/backend contract 임을 확정 — version-range 핑계가 아니라 helper 가 못 따라간 실제 결함(F3). 이 spec 의 1차 verdict 로 승격했어요.
- **검증 스냅샷**: origin/main 36253cb (v0.9.23) 기준. 사용자의 작업 중 브랜치 `feat/decouple-routing` 의 in-progress 변경은 `skills/trace/` 2개 파일을 건드리지 않아서 스킬 verdict 는 동일하게 유지돼요. 단, 그 브랜치가 `trace_helper.rs` 의 audit/routing 의존부를 바꿨다면 impl-level 동작은 재검증이 필요할 수 있어요 (스킬 계약 자체는 무관).
- **검증 범위 경계 (실행 안 한 것)**: (a) cargo unit/e2e 테스트 (`trace_helper_test.rs`, `cli_e2e.rs` 의 trace 케이스) — 바이너리 미빌드. impl 계약은 정적 분석 (V7·V8) 으로 이미 입증, release commit 이라 green 전제. (b) 실제 바이너리를 live 실패 배포에 대해 end-to-end 실행 — live 배포 상태 필요. 둘 다 "스킬이 계약대로 구성됐는가" 라는 이번 검증 질문 밖이에요.
- 검증은 read-only + 기존 test/lint harness 만 사용했고 스킬 파일을 수정하지 않았어요.
- **Terminology (build_log → runtime_log)**: spec 상단(Summary/F3/R3)과 plan/research/tasks 는 **runtime_log** 로 정정됐어요. 하단 AS-FOUND 서술(US1 "3-source event_log+build_log+audit", FR-009 `build_log_errors`, Edge Cases 의 `build_log`)은 **현행 스킬이 실제 그렇게 적혀 있다**는 검증 기록이라 build_log 표기를 보존하되, R3γ 구현(D6/T016) 후 SKILL 문구는 runtime_log 로 바뀌어요. `build_log_errors` JSON 키는 하위호환 위해 유지하되 의미는 런타임 로그(data-model.md 참조).
