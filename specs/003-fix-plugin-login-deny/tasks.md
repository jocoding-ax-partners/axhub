---
description: "Task list — 플러그인 로그인 consent deny 수정 (TMPDIR 핸드오프)"
---

# Tasks: 플러그인 로그인 consent deny 수정 (TMPDIR 핸드오프)

**Input**: Design documents from `specs/003-fix-plugin-login-deny/`

**Prerequisites**: plan.md, spec.md (US1/US2), research.md (R1~R5), data-model.md, contracts/preauth-check-output.md, quickstart.md

**Tests**: 포함해요 — spec 의 Independent Test + quickstart 의 fail-before/pass-after 회귀 테스트 + 저장소 mandate(`cargo test` green). TDD 순서: 실패 테스트 먼저.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 가능 (다른 파일, 미완 task 의존 없음)
- **[Story]**: US1(P1) / US2(P2). Setup·Polish 는 라벨 없음.
- 모든 경로는 repo 루트 기준.

## Path Conventions

단일 Rust 크레이트 `crates/axhub-helpers/` — `src/consent/{key.rs,jwt.rs}`, `src/main.rs`, `tests/cli_e2e.rs`.

---

## Phase 1: Setup & Guardrails

**Purpose**: pre-change 베이스라인 고정 + symbol 편집 전 impact mandate.

- [X] T001 베이스라인: `cargo build -p axhub-helpers` + `cargo test -p axhub-helpers` 가 green 인지 확인하고 바이너리 경로 기록 (수정 전 상태 고정)
- [X] T002 [P] `mcp__gitnexus__impact({target:"runtime_root", direction:"upstream"})` 실행 후 blast radius 기록 — CLAUDE.md symbol 편집 mandate (plan 의 수동 4-call-site grep 과 대조: `key.rs:38,41` / `jwt.rs:137-138,209`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 user story 공통 선행 작업.

**없음** — US1 과 US2 는 서로 독립이고(다른 파일·다른 관심사) 공유 차단 인프라가 없어요. Phase 1 완료 후 두 story 모두 즉시 착수 가능.

**Checkpoint**: 베이스라인 green + impact 기록 → user story 착수 가능

---

## Phase 3: User Story 1 - 플러그인 로그인이 부당하게 차단되지 않음 (Priority: P1) 🎯 MVP

**Goal**: mint 프로세스와 hook subprocess 의 `$TMPDIR` 가 달라도(그리고 `XDG_RUNTIME_DIR` 미설정이어도) 유효 consent 를 발견해 `axhub auth login` 이 allow 되도록 consent 저장 경로를 프로세스-무관 경로로 옮겨요.

**Independent Test**: `XDG_RUNTIME_DIR` 제거 + `XDG_STATE_HOME`=tempdir 에서, mint 를 `TMPDIR=A` 로 / preauth-check 를 `TMPDIR=B` 로 실행 → `permissionDecision:"allow"`. (수정 전엔 `"deny"`)

### Tests for User Story 1 ⚠️ (먼저 작성, 반드시 FAIL 확인)

- [X] T003 [US1] 회귀 테스트 `preauth_allows_when_tmpdir_differs_and_xdg_runtime_unset` 추가 in `crates/axhub-helpers/tests/cli_e2e.rs` — 기존 `run_stdin` 은 env 추가만 하므로, `env_remove("XDG_RUNTIME_DIR")` 를 실제로 적용할 수 있는 새 test helper(예: `run_stdin_with_env_overrides`)를 먼저 만들어요. `XDG_STATE_HOME`=tempdir, mint 는 tempdir-owned `TMPDIR=A`, preauth-check 는 다른 tempdir-owned `TMPDIR=B`, `assert allow`, 그리고 매칭 성공 후 pending consent 파일이 삭제됐는지(pending single-use consume 보존, FR-003) assert. **현재 바이너리에서 DENY(fail-before) 임을 먼저 확인** (재현 ≠ 동어반복 증명, quickstart §1)

### Implementation for User Story 1

- [X] T004 [US1] `runtime_root()` 폴백 수정 in `crates/axhub-helpers/src/consent/key.rs` (research R1; T002·T003 후): **경로 형태를 정확히 고정**해요 — `XDG_RUNTIME_DIR` 가 non-empty 면 `PathBuf::from(xdg).join("axhub")`, 없으면 `state_root().join("runtime")` 를 반환. 기존처럼 마지막에 공통 `.join("axhub")` 를 붙이면 fallback 이 `state_root()/runtime/axhub` 가 되어 plan 과 달라지므로 금지.
- [X] T005 [US1] T003 회귀 테스트 PASS + 기존 consent E2E 전부 green 확인: `cargo test -p axhub-helpers` — XDG_RUNTIME_DIR 분기 무회귀(FR-005)

### Hardening for User Story 1

- [X] T006 [P] [US1] 권한 테스트: 폴백 경로 디렉터리 `0700` / consent 파일 `0600` 검증 추가 in `crates/axhub-helpers/tests/cli_e2e.rs` (FR-004)
- [X] T007 [P] [US1] TTL 테스트: `exp<=now` 만료 consent 는 `$TMPDIR` 무관하게 deny 검증 추가 in `crates/axhub-helpers/tests/cli_e2e.rs` (FR-003 보안 계약 보존)
- [X] T008 [US1] FR-007 만료 스윕: `sweep_expired_consent_files(dir, key)` 헬퍼(`decode_token_file` 재사용 + `is_consent_token_path`: `consent-*.json` 매칭, best-effort 실패 무시) 추가 in `crates/axhub-helpers/src/consent/jwt.rs`, mint 경로(`mint_token_to_path` 의 `create_dir_all`/`set_private_dir_mode` 직후)와 preauth claim 경로(`claim_pending_token` 의 `read_dir` 직전)에서 호출 (T004 후)
- [X] T009 [P] [US1] 스윕 테스트: 만료 pending 파일 + 만료 session consent 파일 존재 + mint/preauth 1회 → 해당 만료 파일 제거 / 유효(미만료) pending·session consent 미영향 검증 추가 in `crates/axhub-helpers/tests/cli_e2e.rs` (T008 후)

**Checkpoint**: US1 완료 → 플러그인 로그인이 `$TMPDIR` 차이에도 통과 (MVP, 단독 ship 가능)

---

## Phase 4: User Story 2 - deny 발생 시 사유가 사용자에게 보임 (Priority: P2)

**Goal**: 정당한 deny(카드 없이 login, TTL 만료) 시 한국어 사유와 다음 행동이 사용자에게 표면화되도록 deny 출력에 canonical 사유 필드를 추가해요. **additive — 기존 `systemMessage` 유지.** US1 과 독립(다른 파일 `main.rs`), 병렬 가능.

**Independent Test**: 유효 consent 없이 `axhub auth login` preauth-check 실행 → deny payload 에 사유 필드 노출 + exit 0.

### Tests / Verification for User Story 2 ⚠️

- [X] T010 [US2] 🔬 Claude Code PreToolUse deny 사유 surface 계약 기록 — 공식 hook 문서상 `hookSpecificOutput.permissionDecisionReason` 가 권한 결정 사유 필드임을 implementation note 에 남기고, `systemMessage` 는 사용자-visible prose 채널로 유지한다는 결정을 확인해요. 구현 후 실제 Claude Code deny UI smoke 는 별도 증빙으로 기록해요.

### Implementation for User Story 2

- [X] T011 [US2] `cmd_preauth_check` deny 출력에 `permissionDecisionReason` 추가(`systemMessage` 제거 금지) in `crates/axhub-helpers/src/main.rs` (contracts/preauth-check-output.md §2-After)
- [X] T012 [P] [US2] 계약 테스트: deny payload 에 `permissionDecisionReason` + `systemMessage` 둘 다 존재 + `permissionDecision:"deny"` + exit 0 검증 추가 in `crates/axhub-helpers/tests/cli_e2e.rs` (contract C4)

**Checkpoint**: US1 + US2 둘 다 독립적으로 동작

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: 게이트 + 증빙 + 문서.

- [X] T013 [P] Gate: `cargo clippy --all-targets -- -D warnings` + `bunx tsc --noEmit` clean
- [X] T014 [P] Tone: `bun run lint:tone --strict` 통과 확인 (신규 한글 메시지 없으면 무영향)
- [X] T015 [P] quickstart.md §1 재현 → §3 검증 수동 실행으로 fail-before/pass-after 증빙 기록
- [X] T016 Docs: README.md / README.html 의 consent 경로 설명을 새 계약과 일치시켜요 — `XDG_RUNTIME_DIR` 가 있으면 `$XDG_RUNTIME_DIR/axhub`, 없으면 `${XDG_STATE_HOME:-$HOME/.local/state}/axhub/runtime`; `${XDG_RUNTIME_DIR:-/tmp}/axhub` 디버깅 안내 제거/교체.
- [ ] T017 릴리스 메모: CHANGELOG narrative(해요체) + 디버깅 기록의 다중-`TMPDIR` 우회가 불필요해짐 명시 (`bun run release` 흐름 시)
- [ ] T018 커밋 후 `npx gitnexus analyze` 인덱스 갱신 확인 (PostToolUse hook 자동)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 즉시 시작. T001 → 이후 전부, T002 → T004.
- **Foundational (Phase 2)**: 없음.
- **US1 (Phase 3)**: Phase 1 후 착수. T003(fail) → T004 → T005; T006/T007 → T004 후; T008 → T004 후; T009 → T008 후.
- **US2 (Phase 4)**: Phase 1 후 착수. **US1 과 병렬** (다른 파일). T010 → T011 → T012.
- **Polish (Phase 5)**: US1(+선택 US2) 완료 후.

### User Story Dependencies

- **US1 (P1)**: 다른 story 의존 없음 — MVP.
- **US2 (P2)**: US1 의존 없음 — `main.rs` deny 출력만 건드림. US1 과 독립 ship 가능.

### Within Each Story

- 테스트(실패 확인) → 구현 → green. T003 은 T004 전에 DENY 확인 필수.

### Parallel Opportunities

- T002 [P] 를 T003 작성과 병렬.
- T006 / T007 / T009 / T016 [P] — 서로 다른 테스트 함수/문서, 각 선행(T004/T008) 후 병렬.
- **US2 전체(T010~T012)** 를 US1 전체와 병렬 (key.rs/jwt.rs vs main.rs 분리).
- T013 / T014 / T015 [P].

---

## Parallel Example: User Story 1

```bash
# T004 적용 후, hardening 테스트 병렬:
Task: "권한 테스트(0700/0600) in crates/axhub-helpers/tests/cli_e2e.rs"   # T006
Task: "TTL 만료 deny 테스트 in crates/axhub-helpers/tests/cli_e2e.rs"      # T007
# T008(스윕) 적용 후:
Task: "만료 스윕 테스트 in crates/axhub-helpers/tests/cli_e2e.rs"          # T009

# US2 는 US1 과 통째로 병렬 (다른 파일):
Task: "deny permissionDecisionReason 추가 in crates/axhub-helpers/src/main.rs"  # T011
```

---

## Implementation Strategy

### MVP First (User Story 1 만)

1. Phase 1 (Setup) — T001 베이스라인, T002 impact
2. Phase 3 (US1) — T003 실패 테스트 → T004 수정 → T005 green → T006~T009 hardening
3. **STOP & VALIDATE**: quickstart 로 fail-before/pass-after 증명 + README 경로 설명 갱신 확인 → 로그인 통과 = MVP
4. 단독 ship 가능

### Incremental Delivery

1. Setup → US1(MVP, 로그인 복구) → ship
2. + US2(deny 진단성) → ship
3. Polish 게이트 + 릴리스 메모

### Notes

- [P] = 다른 파일, 의존 없음. [Story] = 추적용.
- T003 은 구현 전 반드시 FAIL 확인 (재현 증명).
- US2 는 P2 — MVP 후순위, 같은 PR 분리 커밋 권장.
- 각 task 후 또는 논리 그룹 후 커밋. 보안 계약(TTL·pending single-use·HMAC·권한) 절대 약화 금지.
- Windows `HOME` 미설정 경계는 범위 외(research R4) — 별도 follow-up.
