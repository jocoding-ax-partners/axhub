# claude `-p --output-format json` schema freeze (v2.1.121)

> Phase 22.0 prep — SB-3 / Pre-mortem #4. claude CLI v2.1.121 의 `-p --output-format json` stdout schema baseline. lib/assert.sh 의 jq path + cap-hit detector graceful abort sentinel 이 의존.

## Status

**TENTATIVE** — Phase 22.5.5 baseline measurement 단계에서 실측 후 lock. 현재 v2.1.121 의 stable schema (claude CLI source 또는 docs 미공개) 이라 22.5.5 dry-run 으로 캡처.

## Expected top-level fields (assumption, lock 전)

```jsonc
{
  "session_id": "<uuid>",
  "model": "claude-sonnet-4-6",
  "stop_reason": "end_turn" | "abort" | "user_cancelled" | "max_tokens" | "stop_sequence" | "tool_use",
  "stop_sequence": null,
  "usage": {
    "input_tokens": <number>,
    "output_tokens": <number>,
    "cache_creation_input_tokens": <number>,
    "cache_read_input_tokens": <number>
  },
  "cost_usd": <number>,
  "messages": [
    {
      "role": "assistant" | "user" | "system",
      "content": [
        { "type": "text", "text": "..." },
        { "type": "tool_use", "name": "Skill", "input": { "skill": "<slug>", "args": "..." } },
        { "type": "tool_result", "tool_use_id": "...", "content": "..." }
      ]
    }
  ]
}
```

## Phase 22 harness 가 의존하는 paths

| jq path | 검증 용도 |
|---------|-----------|
| `.stop_reason` | graceful abort sentinel (BUDGET_EXCEEDED 분기 회피) |
| `.cost_usd` | per-case actual cost — baseline-cost.json |
| `.messages[] | .content[] | select(.type=="tool_use" and .name=="Skill") | .input.skill` | Routed SKILL strict 단일 매칭 |
| `.messages[] | .content[] | select(.type=="text") | .text` | Korean phrase / forbidden phrase grep target |
| `.usage.output_tokens` | 비용 회귀 detection |

## Cap-hit detector graceful abort sentinel (line 337 plan)

```bash
# lib/assert.sh
is_budget_exceeded() {
  local stdout_path="$1"
  local exit_code="$2"

  # 첫 조건: timeout
  [ "$exit_code" -eq 124 ] || return 1

  # 둘째 조건: stdout 100 byte 미만
  [ "$(wc -c < "$stdout_path")" -lt 100 ] || return 1

  # 셋째 조건 (graceful abort 회피): stop_reason ∈ {abort, user_cancelled, end_turn} 이면 graceful, NOT budget exceeded
  local stop_reason
  stop_reason=$(jq -r '.stop_reason // empty' "$stdout_path" 2>/dev/null)
  case "$stop_reason" in
    abort|user_cancelled|end_turn)
      return 1  # graceful, NOT budget exceeded
      ;;
  esac

  # 모두 통과 → BUDGET_EXCEEDED
  return 0
}
```

## Verification (Phase 22.5.5)

```bash
# 측정 1회로 schema 실측
claude -p --output-format json --no-session-persistence \
  --max-budget-usd 0.05 --plugin-dir "${CLAUDE_PLUGIN_ROOT}" \
  "ping" > /tmp/claude-schema-sample.json

# stop_reason 실재 enum 값 확인
jq -r '.stop_reason' /tmp/claude-schema-sample.json
# 기대: "end_turn" 또는 "stop_sequence"

# cost_usd 필드 존재 확인
jq -r '.cost_usd' /tmp/claude-schema-sample.json
# 기대: <number>
```

22.5.5 측정 후 본 파일을 lock 으로 update + Phase 22 진행.

## Drift 정책

- minor bump (`2.1.121 → 2.1.122`): schema diff 후 본 파일 update + harness 영향 검토
- major bump (`2.1 → 2.2`): 전체 22.5.5 measurement 재실행 + spawn.sh 영향 평가
- breaking change: harness 일부 case rewrite 필요할 수 있음, ralplan-DR 별도 round
