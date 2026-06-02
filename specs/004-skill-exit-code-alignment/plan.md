# Implementation Plan: Skill 복구 라우팅 ↔ 현행 CLI 실패 신호 정합

**Branch**: `004-skill-exit-code-alignment` | **Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-skill-exit-code-alignment/spec.md` (+ [verification-report.md](./verification-report.md), [research.md](./research.md), [data-model.md](./data-model.md))

## Summary

axhub 플러그인의 실패-복구 라우팅이 옛 sysexits-style 코드(`65`/`67`/`68`/`70`)에 묶여 있어, 현행 ax-hub-cli 0.17.2 가 실제로 내보내는 신호(flat slug `unauthenticated`/`not_found`/`rate_limited` + 숫자 `4`/`5`/`6`)와 어긋나 있어요. 결과: 토큰 만료·리소스 없음·rate-limit 이 일반 통신-오류로 falling through 해 사용자가 복구 안내를 못 받아요.

**접근**: 복구 라우팅을 **CLI 의 `error.code` slug 기준으로 정규화**(version-agnostic)하고, 숫자 fallback 을 `4`/`5`/`6` 으로 교정하고, flat(CLI)↔dotted(helper) slug 네임스페이스를 단일점에서 reconcile 해요. (research.md 결정 1·3.) 정합 대상은 catalog.json(source) + codegen + catalog.md + Rust helper(`list_deployments`/`classify-exit`) + 8 skills 의 5개 표면이에요 (research.md 결정 2). 재발 방지로 CLI 계약 pinned snapshot + parity 가드를 추가해요 (spec FR-012).

## Technical Context

**Language/Version**: Rust 2024 edition (axhub-helpers 크레이트) + TypeScript on Bun (codegen/lint/test 하니스). 대조 대상 CLI: ax-hub-cli 0.17.2 (별도 repo).

**Primary Dependencies**: clap(파생) · serde/serde_json(envelope 파싱) · bun(codegen-catalog.ts, 테스트). 신규 의존 없음.

**Storage**: N/A (마크다운 카탈로그 + JSON source + Rust 상수).

**Testing**: `cargo test`(axhub-helpers, exit→slug 매핑 단위) + `bun test`(codegen drift, parity 가드, ux 계약) + `bunx tsc --noEmit`.

**Target Platform**: Claude Code 플러그인 (macOS/Linux/Windows). 크로스-플랫폼 — 새 OS별 분기 없음.

**Project Type**: CLI-adjacent 플러그인 — Rust helper 바이너리 + 마크다운 SKILL + TS 하니스. spec 의 "기능"은 데이터/라우팅 정합이지 새 런타임 기능이 아니에요.

**Performance Goals**: N/A (라우팅은 1회성 분류; hot path 아님).

**Constraints**:
- `description:` frontmatter(nl-lexicon 트리거)는 byte-lock — 변경 금지 (FR-010).
- 모든 한글 텍스트 해요체 (`lint:tone --strict`).
- 새 SKILL 작업 시 scaffold 우회 금지 (해당 시 `skill:doctor`).
- fail-open hook 계약 (해당 시).
- ax-hub-cli **자체는 수정 안 함** — 스킬을 CLI 계약에 맞춤 (spec Out of Scope).

**Scale/Scope**: 5개 표면, 8개 skill, ~14개 CLI 실패 조건. 확정 broken = status·deploy·logs(direct numeric); mixed = recover·init·apps; 모범 = update·auth.

## Constitution Check

*GATE: Phase 0 전 통과, Phase 1 후 재확인.*

`.specify/memory/constitution.md` 는 **미작성 템플릿**(placeholder)이라 프로젝트별 비준 원칙이 없어요 → 형식적 gate 없음. 대신 **de-facto gate = `CLAUDE.md` 계약**:

| 계약 | 적용 | 상태 |
|---|---|---|
| Skill Authoring (Phase 17/18) | 본문 SKILL.md 편집만, 새 SKILL 없음 → scaffold 무관. `description:` byte-lock 준수 | ✅ pass |
| Hook Safety (PR 25.2) | classify-exit 가 hook(PostToolUse) → fail-open/exit-0 유지, kill-switch 보존 | ⚠ 변경 시 준수 (Phase 1 재확인) |
| Release Workflow | 본 작업은 release cut 아님 (별도) | N/A |
| nl-lexicon keyword lock | `description:` 미변경 → baseline 무손상 | ✅ pass |
| tone 해요체 | 카탈로그/스킬 한글 → `lint:tone --strict` | ⚠ 편집 후 검증 |

**위반 없음** — gate 통과. (Complexity Tracking 참조: scope 가 spec Dependencies 보다 큼은 정당화됨.)

## Project Structure

### Documentation (this feature)

```text
specs/004-skill-exit-code-alignment/
├── plan.md              # 이 파일
├── spec.md              # 기능 명세 (outcome-framed)
├── verification-report.md  # gate-zero + 초기 drift 표
├── research.md          # Phase 0 — 아키텍처/전략 결정 (관찰 기반)
├── data-model.md        # Phase 1 — canonical 매핑 (현재→목표)
├── contracts/
│   └── cli-error-envelope.md   # CLI --json 실패 계약 (정합 목표)
├── quickstart.md        # Phase 1 — 검증 절차
└── checklists/requirements.md  # spec 품질 체크리스트 (17/17)
```

### Source Code (repository root — 실제 변경 지점)

```text
crates/axhub-helpers/
├── data/catalog.json                 # [표면1] 복구 카탈로그 source-of-truth (키 재매핑)
├── data/cli-exit-contract.json        # [신규] CLI 실패 계약 pinned snapshot (drift-guard 모집단)
└── src/
    ├── list_deployments.rs            # [표면4] EXIT_LIST_* + exit_to_error_code + exit_to_helper_exit
    ├── main.rs                        # [표면4b] cmd_classify_exit (classify-exit 서브커맨드)
    └── cli_envelope.rs                # error.code/slug 파서 (flat slug 수용 확인)

scripts/codegen-catalog.ts             # [표면2] catalog.json → generated.md (재실행)

skills/deploy/references/
├── error-empathy-catalog.md           # [표면3] hand-written 4-part 템플릿 (재키 + 인용)
└── error-empathy-catalog.generated.md # codegen 산출 (직접 편집 금지)

skills/{status,deploy,logs}/SKILL.md   # [표면5-확정] direct-path 라우팅 교정
skills/{recover,init,apps}/SKILL.md    # [표면5-mixed] 잔여 numeric/slug 정리

tests/
├── codegen.test.ts                    # catalog.json↔generated.md drift (기존)
└── exit-contract-parity.test.ts       # [신규] catalog 키 ↔ pinned CLI 계약 parity (Q2/FR-012)
crates/axhub-helpers/src/...           # [신규 단위테스트] exit→slug 매핑
```

**Structure Decision**: 단일 repo (axhub 플러그인) 안의 Rust 크레이트 + TS 하니스 + 마크다운 스킬 혼합. 새 디렉토리/크레이트 없음 — 기존 5개 표면 in-place 정합.

## Phased approach (tranche)

research.md 의 tranche 를 실행 단계로. `/speckit-tasks` 가 이걸 세분 작업으로 분해해요.

- **T0 — gate-zero 잔여 확정 (편집 전 필수)**: (a) auth(4) slug 문자열 확정 — deauth 후 `axhub auth status --json` 또는 401 경로 live 실행, (b) `rate_limited`(6) slug + `apis.call_consent_required` 실제 출처(ax-hub-cli grep 0건), (c) `error.rs` 변형별 slug 전수 → pinned snapshot 모집단, (d) `EXIT_LIST_*` helper-exit 소비처(skill) 확인, (e) reachability: `8`~`15` 중 어느 코드가 어느 skill 경로에 도달하는지 (Q1 bespoke 대상 확정).
- **T1 — 확정 core (사용자 진입점)**: catalog.json/catalog.md 의 `65→4`/`67→5`/`68→6`/`70→7` 재키 + `2` 제거 + slug 1차 키. status·deploy·logs 의 direct-path 라우팅을 slug 기준으로 교정. codegen 재실행. status 우선 (user 가 가리킨 스킬).
- **T2 — helper 레이어**: `list_deployments.rs`(EXIT_LIST_* + exit_to_error_code + exit_to_helper_exit auth 분기) + `cmd_classify_exit` numeric/slug 교정 + flat↔dotted reconcile (단일 변환점). 단위테스트.
- **T3 — 정합 잠금 + 잔여**: pinned snapshot + parity 가드(Q2/FR-012). mixed skills(recover/init/apps) 잔여 numeric 정리. `66` base 재정의 + `apis.call_consent_required`/`70` 출처 해소. FR-011 인용 추가.
- **T4 — 게이트**: `cargo test` + `bun test`(≥기존 baseline) + `bunx tsc --noEmit` + `lint:tone --strict` + `lint:keywords --check`(무diff) + (해당 시)`skill:doctor --strict`. SC-006 회귀 0.

## Complexity Tracking

> Constitution 위반은 없지만, scope 가 spec Dependencies(catalog.md + 8 skills)를 초과한 점을 정당화.

| 확장 | 왜 필요 | 더 단순한 대안 기각 이유 |
|---|---|---|
| catalog.json + codegen 포함 | catalog.md 는 codegen 산출의 형제 — .md 만 고치면 codegen drift test fail + source(json) 와 불일치 | "catalog.md 만 편집" → `tests/codegen.test.ts` fail + 다음 codegen 이 되돌림 |
| Rust helper(list_deployments/classify-exit) 포함 | helper 가 exit→slug 라우팅의 실제 코드 경로 (catalog 은 텍스트일 뿐). auth slug(`unauthenticated`)가 `starts_with("auth.")` 불일치로 broken (관찰) | "skill/카탈로그만" → helper 경로(recover/status cold-cache)의 auth 라우팅이 계속 broken |
| slug 네임스페이스 reconcile | CLI flat(`not_found`) ↔ helper dotted(`resource.app_not_found`) drift 가 관찰됨 | numeric 만 교정 → slug 경로 drift 잔존 |
| drift-guard 신규 테스트 | 이 버그가 생긴 이유 = 무가드 (spec SC-008) | 무가드 → 다음 CLI 변경에 또 silent drift |
