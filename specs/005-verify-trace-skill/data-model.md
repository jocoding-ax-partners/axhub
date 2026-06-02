# Phase 1 Data Model: Trace evidence-source 재설계

**Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

이 변경은 신규 영속 엔터티를 만들지 않아요. 데이터 흐름에 관여하는 기존 타입만 정리해요.

## 소비(입력): `AppLogLine` (NDJSON, axhub CLI)

`axhub --json deploy logs` 가 line 당 1개씩 emit 하는 런타임 로그 라인. (출처: `ax-hub-cli/crates/axhub-api/src/deploy.rs`)

| 필드 | 타입 | 비고 |
|---|---|---|
| `ts` | `Option<String>` | RFC3339 타임스탬프 |
| `message` | `String` | **매칭/표시 대상** — 실제 로그 텍스트 |
| `insert_id` | `Option<String>` | dedupe 키 (helper 는 미사용) |
| `container` | `Option<String>` | 컨테이너명 |

- CLI JSON envelope: `{"type":"log","source":null,"ts":...,"container":...,"insert_id":...,"message":...}`.
- **파싱 규칙**: helper 는 각 라인을 파싱해 `message` 만 추출. 파싱 실패 라인은 skip + warning(panic 금지). 빈 `lines` → `runtime_log_unavailable` warning.

## 가공(내부): significant-line 판정 + 매칭

- **매칭 입력**: 파싱된 전체 `message` 리스트 (display 5-라인 cap 과 분리 — D3).
- **`extract_error_lines(messages, 5)`** (표시용): `ERROR`/`FATAL`/`WARN` 토큰 message 중 앞 5개. 계약 불변(NEVER 규칙).
- **`match_error_patterns(messages)`**: 전체 message 에 needle substring 매칭. needle 정밀화(D4) 적용.

## 산출(출력): `TraceReport` — 외부 계약 불변

`trace --json` 출력. (출처: `crates/axhub-helpers/src/trace_helper.rs`) 필드 **변경 없음** — 외부 호환 유지.

| 필드 | 타입 | 이번 변경의 영향 |
|---|---|---|
| `deploy_id` | `String` | 불변 |
| `last_phase` | `String` | 불변 (event_log) |
| `failure_reason` | `Option<String>` | 빌드-단계 fallback 안내 강화 (D5) |
| `phase_durations` | `Vec<PhaseDuration>` | 불변 (event_log) |
| `build_log_errors` | `Vec<String>` | **의미 갱신**: 빌드 로그 → 런타임 로그의 ERROR/WARN message (max 5). 필드명/JSON 키는 호환 위해 유지 가능, 문서로 의미 명시 |
| `routing_context` | `Option<RoutingContext>` | 불변 (audit) |
| `matched_patterns` | `Vec<String>` | **도달성 수정**: 전체 message 매칭 (D3) |
| `warnings` | `Vec<String>` | `runtime_log_unavailable`/`runtime_log_parse_warning` 추가 |

## 참조(읽기): `event_log::DeployEvent`

로컬 `deploy-events/<id>.jsonl`. `phase`/`duration_ms`/`reason`. 빌드 단계 실패 fallback 의 `failure_reason` 출처(D5). 이번 변경에서 **읽기 전용**.
