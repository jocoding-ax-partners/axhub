# Contract: trace evidence 파이프라인

**Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md)

## 외부 출력 계약 — `axhub-helpers trace --json` (불변)

`TraceReport` JSON 스키마는 **변경하지 않아요** (하위 호환). CI 의 `cli_e2e.rs` 기존 assert (`deploy_id`/`last_phase`/`failure_reason`/`phase_durations`/`build_log_errors`/`matched_patterns`) 유지.

```jsonc
{
  "deploy_id": "dep-...",
  "last_phase": "failed",
  "failure_reason": "build command failed",   // event_log 출처
  "phase_durations": [ { "phase": "...", "duration_ms": 20, "step": 0 } ],
  "build_log_errors": ["ERROR ...", "WARN ..."],   // 의미: 런타임 로그 ERROR/WARN message, max 5
  "matched_patterns": ["env_not_found"],            // 전체 message 매칭 (도달성 수정)
  "routing_context": { "...": "..." },
  "warnings": ["runtime_log_unavailable"]           // 신규 warning 허용 (#[serde(default)])
}
```

- **호환성**: 필드 추가만 허용(warning 종류). 기존 키 제거/리네임 금지. `build_log_errors` 키는 유지하되 **의미는 런타임 로그**로 문서화(D3).

## 입력 계약 — `axhub --json deploy logs` (소비)

helper 가 의존하는 현행 CLI 출력. (truth: `ax-hub-cli/axhub/src/commands/deploy/logs.rs::emit_app_log`)

- **NDJSON**: 1 라인 = 1 JSON object `{"type":"log","source":null,"ts":..,"container":..,"insert_id":..,"message":..}`.
- helper 는 `message` 만 사용. `--source`/positional deploy-id 는 현행 CLI 가 **무시** → 의존 금지.
- 빈 `lines`(앱 미기동/빌드 단계 실패) → 정상. `runtime_log_unavailable` warning + event_log fallback.

## 매칭 계약 — reachability (수정 핵심)

- `matched_patterns` = `match_error_patterns(전체 parsed messages)` — display 5-라인 cap 과 **분리**.
- 패턴 needle 이 `ERROR`/`WARN`/`FATAL` 토큰 없는 라인(`env: …`, `npm ERR! …`)에서도 발화해야 함.
- needle 정밀화(D4): `oom`/`exit code 1` 등 substring 오탐 차단.

## 표시 계약 — SKILL NEVER 규칙 (보존)

- `build_log_errors`(사용자 표시) = ERROR/FATAL/WARN-태그 message **max 5**. raw 전체 미노출.
- 이 5-라인 cap 은 매칭과 무관하게 유지 (display 를 넓히지 않음).

## 회귀 테스트 계약 (필수)

| 테스트 | 입력 | 기대 |
|---|---|---|
| NDJSON 파싱 | `{"type":"log","message":"env: FOO not found"}` 라인 | `matched_patterns ⊇ [env_not_found]` |
| raw 무태그 도달성 | `npm ERR! code ELIFECYCLE` (태그 없음) | `dependency_install_failed` 발화 |
| 태그 happy-path (기존) | `ERROR build command failed` / `WARN network timeout` | 기존 assert 유지 (green) |
| 오탐 차단 | `zoom meeting scheduled` / `src/room/x` | `oom` **미발화** |
| 빈 로그 fallback | `{"lines":[]}` | `warnings ⊇ [runtime_log_unavailable]`, generic |
| 결합 경로 | raw 멀티라인 → extract→match | 단위 테스트가 추출 단계 포함 (현재 :247-275 는 우회) |
