# Implementation Plan: Trace 스킬 evidence-source 재설계 (R3γ + R2 + R1)

**Branch**: `005-verify-trace-skill` | **Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-verify-trace-skill/spec.md`

## Summary

검증 결과 trace 의 헤드라인 기능(빌드 실패 원인 → empathy 매칭)이 현행 axhub backend 와 깨졌어요 (F3, CLI/API 소스 확정 — build-log API 부재, `deploy logs` 는 app 런타임 로그 NDJSON 반환). 사용자 결정(γ)에 따라 trace 를 **런타임 로그 기반**으로 재정의해요: `deploy logs` NDJSON 의 `message` 필드를 파싱해 런타임 에러를 매칭하고, **빌드 단계** 실패는 로컬 `event_log.failure_reason` 으로 안내. 부수적으로 F2(필터 reachability)를 R3 위에서 고치고, R1(문서 라벨)을 정리해요.

기술 접근: helper `RealTraceProbes` 의 로그 probe 를 NDJSON-aware 로 바꾸고(`message` unwrap), `trace_helper` 의 매칭을 display-필터와 분리(매칭은 전체 message, 표시는 ERROR/WARN max 5 유지 — SKILL NEVER 규칙 보존), needle 정밀화, SKILL/catalog 문구를 런타임 소스로 갱신.

## Technical Context

**Language/Version**: Rust 2024 edition (workspace), Bun/TypeScript (skill 게이트)

**Primary Dependencies**: `serde`/`serde_json` (NDJSON 파싱), `thiserror`, 기존 `axhub_helpers::{event_log, audit, axhub_cli}` 모듈. 새 외부 crate 불필요.

**Storage**: 로컬 NDJSON `deploy-events/<id>.jsonl` (event_log, 읽기 전용). 백엔드 로그는 `axhub deploy logs` 경유 (네트워크).

**Testing**: `cargo test -p axhub-helpers` (unit in-file + `tests/cli_e2e.rs` integration), `bun test` (skill 불변식 `tests/trace-skill.test.ts`), `bun run skill:doctor --strict` + `lint:tone` + `lint:keywords` (SKILL 게이트).

**Target Platform**: macOS/Linux/Windows CLI helper 바이너리 (cross-arch).

**Project Type**: Rust CLI helper library (`crates/axhub-helpers`) + Claude Code skill (`skills/trace`).

**Performance Goals**: trace probe 5s timeout/소스 유지 (`DEFAULT_AXHUB_TIMEOUT`), 평균 15s 상한. NDJSON 파싱은 max 100 라인 (helper `--limit`).

**Constraints**: fail-open (hook 계약 무관하지만 helper 는 panic 금지 — `unwrap`/`expect` 회피). SKILL NEVER 규칙 (raw stderr 미노출, ERROR/WARN max 5 인용) 보존. SKILL 계약(`trace --json` TraceReport JSON)은 외부 호환 유지.

**Scale/Scope**: helper 1-2 함수 + trace_helper 매칭 경로 + e2e/unit fixture + SKILL.md/catalog 문구. 신규 파일 없음, 외부 API 변경 없음.

**Unknowns**: 없음 (evidence-source fork 는 γ 로 확정). 단 **D5 가정**: event_log `failure_reason` 의 상세도 — coarse 하면 빌드-단계 매칭 변별력 제한 (research.md D5 참조, deploy hook reason 작성부는 이 spec 범위 밖, 별도 후속).

## Constitution Check

*GATE: Phase 0 전 통과, Phase 1 후 재확인.*

`.specify/memory/constitution.md` 는 미작성 템플릿(placeholder)이라 **비준된 명시 원칙이 없어요**. 따라서 프로젝트 자체 표준(CLAUDE.md + `~/.claude/rules/rust/*`)을 게이트로 적용:

- **Test-First**: 회귀 fixture(raw 무태그 / NDJSON / 오탐)를 구현 전에 추가 → RED → GREEN. ✅ 계획 반영.
- **No panic on input**: NDJSON 파싱은 `serde_json` 실패를 `unwrap` 없이 graceful 처리(라인 skip + warning). ✅
- **Surgical change**: helper probe + 매칭 경로 + 문구만 변경, 무관 리팩터 금지. ✅
- **SKILL 계약 보존**: `trace --json` 외부 JSON 불변, NEVER 규칙 유지. ✅
- **해요체 tone / keyword baseline**: SKILL 문구 변경 시 `lint:tone`/`lint:keywords` 통과. ✅

게이트 위반 없음 → Complexity Tracking 비움.

## Project Structure

### Documentation (this feature)

```text
specs/005-verify-trace-skill/
├── plan.md              # 이 파일
├── research.md          # Phase 0 — 설계 결정 (γ, NDJSON, 매칭 분리, needle, fallback)
├── data-model.md        # Phase 1 — AppLogLine / TraceReport / significant-line
├── quickstart.md        # Phase 1 — 빌드·테스트·수동 검증
├── contracts/
│   └── trace-evidence-contract.md   # trace --json 출력(불변) + NDJSON 입력 파싱 계약
├── spec.md              # 검증 + 보완 (F1/F2/F3, R1/R2/R3)
└── tasks.md             # /speckit-tasks 출력 (이 명령은 생성 안 함)
```

### Source Code (repository root)

```text
crates/axhub-helpers/
├── src/
│   ├── trace_helper.rs      # extract_error_lines / match_error_patterns / trace() — 매칭·필터 분리, needle 정밀화
│   ├── main.rs              # RealTraceProbes::axhub_build_log (NDJSON message 파싱) — runtime-log 의미로
│   └── event_log.rs         # (읽기) failure_reason — 빌드-단계 fallback 소스
└── tests/
    └── cli_e2e.rs           # fake_axhub_logs → NDJSON fixture + raw 무태그 + 오탐 회귀

skills/trace/
├── SKILL.md                 # 3-source 문구를 runtime-log 로 갱신 (R3) + 라벨 정리 (R1)
└── references/error-patterns.md   # 런타임 적용 패턴 재정렬 (필요 시)
```

**Structure Decision**: 기존 단일 Rust 워크스페이스 crate(`crates/axhub-helpers`) + skill(`skills/trace`) 구조 유지. 신규 모듈/crate 없음 — 변경은 probe 파싱 + 매칭 경로 + 문구에 국한.

## Complexity Tracking

> Constitution Check 위반 없음 — 비움.
