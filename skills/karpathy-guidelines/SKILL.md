---
name: karpathy-guidelines
description: This skill injects Andrej Karpathy style LLM coding reminders for 작은 diff, 테스트 우선, evidence first, and no overconfidence.
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "작은 diff 로 가"
    intent: "apply coding reminder"
  - utterance: "테스트 우선"
    intent: "apply testing reminder"
  - utterance: "과신 금지"
    intent: "apply uncertainty reminder"
  - utterance: "evidence first"
    intent: "apply evidence reminder"
  - utterance: "keep changes small"
    intent: "apply small diff reminder"
---

# karpathy-guidelines

This vendored reminder summarizes practical LLM coding guidance inspired by Andrej Karpathy's public advice.

- Keep changes small and inspectable.
- Prefer tests and concrete evidence over intuition.
- Read the existing code before editing.
- Avoid broad rewrites when a focused patch solves the bug.
- Verify the exact claim before saying work is complete.
- When uncertain, make the uncertainty explicit and gather evidence.

## Non-interactive AskUserQuestion guard (D1)

This SKILL has no AskUserQuestion call site. If a future edit adds one, register the safe default in `tests/fixtures/ask-defaults/registry.json` before shipping.

## License

MIT attribution is recorded in `THIRD-PARTY-NOTICES.md`.
