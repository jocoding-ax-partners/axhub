# Quickstart: Trace 재설계 검증

**Date**: 2026-06-02 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

구현 후 이 순서로 검증해요. (워크트리 루트에서 실행)

## 1. Rust 단위/통합 테스트

```bash
# trace 단위 (extract→match 결합 경로 + needle 정밀화 포함)
cargo test -p axhub-helpers trace

# e2e (NDJSON fixture + raw 무태그 + 오탐 + 빈 로그 fallback)
cargo test -p axhub-helpers --test cli_e2e

# 회귀 케이스 확인 포인트:
#  - cli_trace_json_* : 기존 ERROR/WARN happy-path green 유지
#  - 신규 raw 무태그   : env_not_found / dependency_install_failed 발화
#  - 신규 오탐         : "zoom"/"room" 라인이 oom 미발화
#  - 신규 빈 로그       : runtime_log_unavailable warning + generic fallback
```

## 2. SKILL 게이트 (R3 문구 + R1 라벨)

```bash
bun run skill:doctor --strict       # D1/TodoWrite/preflight/model/step-numbering
bun run lint:tone --strict          # 해요체 0 err
bun run lint:keywords --check        # nl-lexicon baseline 불변 (description trigger 불변 확인)
bun test tests/trace-skill.test.ts   # SKILL 불변식 (3-source 문구 갱신 반영)
```

> 주의: `tests/trace-skill.test.ts` 가 `event_log + build_log + audit` 문자열을 assert 하면, runtime-log 문구 변경에 맞춰 테스트도 함께 갱신해야 해요 (D6).

## 3. 수동 end-to-end (in-range axhub, 선택)

```bash
# 런타임 로그가 있는(기동된) 앱에 대해 — 런타임 에러 매칭 확인
axhub-helpers trace --deploy-id <id> --app <app> --json | jq '{matched_patterns, build_log_errors, warnings}'

# 빌드 단계 실패 배포 — 런타임 로그 비고 event_log fallback 확인
#   → matched_patterns 빈 배열 가능, failure_reason(event_log) 안내 + runtime_log_unavailable warning
```

## 4. 타입/전체 회귀

```bash
bunx tsc --noEmit
bun test          # 전체 (Phase 18 baseline ≥498 pass / 0 fail)
cargo build --release -p axhub-helpers   # 바이너리 정상 빌드
```

## Definition of Done

- [ ] NDJSON `message` 파싱 (raw JSON 미노출)
- [ ] `matched_patterns` 가 전체 message 기준 (raw 무태그 needle 발화)
- [ ] `build_log_errors` display = ERROR/WARN max 5 (NEVER 규칙 보존)
- [ ] needle 정밀화 — 오탐 회귀 green
- [ ] 빌드 단계 빈 로그 → event_log fallback + warning
- [ ] SKILL/catalog 문구 runtime-log 로 갱신 (R3) + 라벨 정리 (R1)
- [ ] 모든 게이트 green (cargo + bun + skill:doctor + lint)
