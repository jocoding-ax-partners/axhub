# Contract: preauth-check PreToolUse 출력 JSON

**Date**: 2026-06-01 | **Plan**: [../plan.md](../plan.md)

`axhub-helpers preauth-check` 는 Claude Code `PreToolUse:Bash` hook 으로, stdin 으로 tool payload 를 받아 stdout 으로 **JSON 한 덩어리**를 emit 해요. 이 문서는 그 출력 계약을 고정해요. (본 작업이 건드리는 유일한 외부 인터페이스 — 나머지는 내부 파일 경로 변경.)

## 입력 (stdin, Claude Code 제공)

```jsonc
{
  "session_id": "cb8c6ed7-...",
  "tool_call_id": "...",
  "tool_name": "Bash",
  "tool_input": { "command": "axhub auth login --force --no-browser --json" }
}
```

## 출력 계약

### 1. allow (비파괴 명령 / 유효 consent / hook 비활성)

```json
{ "hookSpecificOutput": { "hookEventName": "PreToolUse", "permissionDecision": "allow" } }
```

- 항상 **exit 0** (fail-open). `tool_name != "Bash"`, 비파괴 명령, `is_hook_disabled`, 유효 consent 매칭 시 모두 allow.

### 2. deny (파괴 명령 + 유효 consent 없음)

**Before (현재):**
```json
{
  "hookSpecificOutput": { "hookEventName": "PreToolUse", "permissionDecision": "deny" },
  "systemMessage": "이 명령은 사전 승인이 필요해요. 먼저 '로그인해'라고 말해서 승인 카드를 받으세요."
}
```

**After (P2 — additive):**
```jsonc
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "이 명령은 사전 승인이 필요해요. 먼저 '로그인해'라고 말해서 승인 카드를 받으세요."
  },
  "systemMessage": "이 명령은 사전 승인이 필요해요. 먼저 '로그인해'라고 말해서 승인 카드를 받으세요."
}
```

- **불변식**: exit 0 유지. `permissionDecision: "deny"` 유지. `systemMessage` **제거 금지**(기존 채널 보존).
- **추가**: `permissionDecisionReason` 에 동일 사유 텍스트(`format_preauth_deny_hint`). Claude Code hook 계약상 권한 결정 사유 필드이며, `systemMessage` 는 사용자-visible prose 채널 보존용으로 함께 유지해요(research R3).
- 사유 텍스트는 `action` 별 한국어 안내(`parser.rs:302`) — 변경 없음.

## 불변식 요약 (계약 테스트 대상)

| # | 불변식 | 근거 |
|---|---|---|
| C1 | 어떤 입력에서도 **exit 0** | hook fail-open (CLAUDE.md Hook Safety) |
| C2 | 출력은 유효 JSON 한 객체, `hookEventName: "PreToolUse"` 포함 | Claude Code hook 계약 |
| C3 | 비파괴/유효consent → `permissionDecision: "allow"` | 기존 동작 보존 |
| C4 | 파괴+consent부재 → `permissionDecision: "deny"` + 사유 텍스트 노출 | FR-002, FR-006 |
| C5 | `permissionDecision` enum = {allow, deny} 만 | 계약 안정성 |
| C6 | (P1 핵심) mint 프로세스와 hook 프로세스의 `$TMPDIR` 가 달라도 유효 consent → allow | FR-001 |

> C6 은 출력 계약이 아니라 **동작 계약**이에요 — 회귀 테스트(quickstart)가 검증해요.
