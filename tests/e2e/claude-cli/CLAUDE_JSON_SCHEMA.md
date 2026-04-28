# claude `-p --output-format json` schema (v2.1.121, **LOCKED**)

> Phase 22.5.5 baseline measurement 직접 측정 후 LOCK. lib/assert.sh 의 jq path + cap-hit detector 가 본 schema 의존.

## Status

**LOCKED — measured 2026-04-28 against `claude 2.1.121 (Claude Code)`.** Sample artifacts: `tests/e2e/claude-cli/output/baseline-samples/*.stdout.json`. Sample run = 5 case dry-run, $1.32 USD total cost, all `error_max_budget_usd` (cap=0.05 USD intentional).

## Top-level shape (실측)

```jsonc
{
  "type": "result",
  "subtype": "error_max_budget_usd" | "success" | "error_*",
  "duration_ms": <number>,
  "duration_api_ms": <number>,
  "is_error": true | false,
  "num_turns": <number>,
  "stop_reason": "end_turn" | "tool_use" | "stop_sequence" | "max_tokens" | …,
  "session_id": "<uuid>",
  "total_cost_usd": <number>,
  "usage": {
    "input_tokens": <number>,
    "cache_creation_input_tokens": <number>,
    "cache_read_input_tokens": <number>,
    "output_tokens": <number>,
    "server_tool_use": {
      "web_search_requests": <number>,
      "web_fetch_requests": <number>
    },
    "service_tier": "standard",
    "cache_creation": {
      "ephemeral_1h_input_tokens": <number>,
      "ephemeral_5m_input_tokens": <number>
    },
    "inference_geo": "",
    "iterations": [],
    "speed": "standard"
  },
  "modelUsage": {
    "<model_id>": {
      "inputTokens": <number>,
      "outputTokens": <number>,
      "cacheReadInputTokens": <number>,
      "cacheCreationInputTokens": <number>,
      "webSearchRequests": <number>,
      "costUSD": <number>,
      "contextWindow": <number>,
      "maxOutputTokens": <number>
    }
  },
  "permission_denials": [],
  "fast_mode_state": "off" | "on",
  "uuid": "<uuid>",
  "errors": ["Reached maximum budget ($X.XX)"] | []
}
```

## Plan SB-3 cap-hit detector — 정의 정정 (baseline measurement 결과)

**v3 plan 의 detector 정의 부정확**:
- 가정: `(exit==124) AND (stdout<100byte) AND NOT (stop_reason ∈ {abort, user_cancelled, end_turn})`
- 실측: cap-hit 시에도 `stop_reason: "end_turn"` 으로 동일하게 emit. graceful abort 와 시그널 구분 불가능.

**v4 정정 (실측 기반)**:

```bash
# lib/assert.sh
classify_case_state() {
  local stdout_path="$1"
  local exit_code="$2"

  # SUCCESS — exit 0 + is_error false
  if [ "$exit_code" -eq 0 ] && [ "$(jq -r '.is_error // empty' "$stdout_path" 2>/dev/null)" = "false" ]; then
    echo "PASS"
    return 0
  fi

  # BUDGET_EXCEEDED — exit 1 + subtype "error_max_budget_usd" 명시 marker
  local subtype
  subtype=$(jq -r '.subtype // empty' "$stdout_path" 2>/dev/null)
  if [ "$subtype" = "error_max_budget_usd" ]; then
    echo "BUDGET_EXCEEDED"
    return 0
  fi

  # TIMEOUT — exit 124 (kernel timeout signal)
  if [ "$exit_code" -eq 124 ]; then
    echo "TIMEOUT"
    return 0
  fi

  # SKIP — exit 0 + skip marker
  if [ "$exit_code" -eq 0 ] && grep -q '"action": "skip"' "$stdout_path"; then
    echo "SKIP"
    return 0
  fi

  # FAIL — 그 외 모든 비정상 (exit != 0 + 알 수 없는 subtype)
  echo "FAIL"
  return 0
}
```

**핵심 변화**:
- ❌ `stop_reason` 기반 graceful 판정 폐기 (cap-hit 도 `end_turn` 이라 useless)
- ✅ `subtype === "error_max_budget_usd"` 가 cap-hit 결정적 marker
- ✅ `is_error` 와 `subtype` 조합으로 5-state 판정 (PASS / FAIL / SKIP / TIMEOUT / BUDGET_EXCEEDED)

## Plan SB-3 cap ratchet (baseline measurement 결과)

**측정 결과**:

| case | cost_usd | wall_s |
|------|----------|--------|
| smoke-ping | $0.40 | 3.94s |
| smoke-help-slash | $0.18 | 10.14s |
| smoke-doctor-ko | $0.25 | 5.25s |
| smoke-status-noauth | $0.25 | 6.82s |
| smoke-empty | $0.24 | 5.25s |

- max observed cost = $0.40 (smoke-ping; cap-hit 였음, 실 사용은 더 클 수도)
- median ~$0.25
- max observed wall = 10.14s (cap 30s 안)
- total wall = 31.4s for 5 cases (parallel 시 < 10.14s)

**Plan v3 SB-3 cap=0.30 USD 부적정**: 측정치 0.40 + cap-hit 양산 위험.

**v4 권고**: cap=0.50 USD (max observed × 1.25 안전 마진). 그래도 PR-blocking T1+T2 19 case × 0.50 = max $9.50 가 PR 마다 burn — 비용 우려. 대안:
- (a) 모델 다운그레이드 (`--model haiku`) — claude-opus 대신 claude-haiku 4.5. cost ~10x 절감 추정.
- (b) cap 유지 + flake retry 1회 만.
- (c) Tier 1 case 만 claude-opus, Tier 2 helper-bin 은 모델 안 씀.

권고: (a) + (c) 조합. spawn.sh 에 `--model claude-haiku-4-5` 추가, cap 0.10 USD. 19 PR-case × $0.10 = $1.90/PR. 수용 가능.

## stop_reason 실측 enum

baseline 5 case 에서 관측된 값:
- `end_turn` (3 cases — graceful 끝, cap-hit 도 포함 — single 시그널 useless)
- `tool_use` (2 cases — agent 가 도구 호출하다 cap-hit)

추가 가능 (claude SDK docs 추론, 미측정):
- `stop_sequence`, `max_tokens`, `pause_turn`, `refusal`, `model_context_window_exceeded`

## Verification command

```bash
# 22.5.5 dry-run 재실행
bun scripts/measure-claude-baseline.ts

# schema sample 직접 검사
jq '.' tests/e2e/claude-cli/output/baseline-samples/smoke-empty.stdout.json
```

## Drift 정책

- minor bump (`2.1.121 → 2.1.122`): 본 schema diff 검증
- major bump (`2.1 → 2.2`): 22.5.5 baseline 재측정 + 본 파일 LOCK 갱신
- breaking change: harness 일부 case rewrite 가능성, ralplan-DR 별도 round
