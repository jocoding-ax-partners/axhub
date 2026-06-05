---
name: using-axhub-quality
description: This skill enables background axhub quality auto-mode after ordinary coding work; 백그라운드 품질 자동 모드이며 direct review/debug/TDD/plan/ship requests must use their dedicated skills instead.
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "quality auto-mode 상태 확인"
    intent: "background quality state reminder"
  - utterance: "품질 자동 모드 확인해줘"
    intent: "background quality state reminder"
  - utterance: "리뷰 기준 넘었는지 알려줘"
    intent: "background review threshold reminder"
  - utterance: "unreviewed diff threshold reminder"
    intent: "background review threshold reminder"
  - utterance: "post-edit quality reminder"
    intent: "background quality follow-up"
---

# using-axhub-quality

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

<EXTREMELY-IMPORTANT>
You have axhub quality auto-mode enabled.

DIRECT REQUEST OVERRIDE:
- If the current user directly asks for code review, debugging, TDD, planning, ship/readiness, or diagnose-loop work, do not process the request generically first.
- Do not read `.axhub-state/quality.json` before handling those direct requests.
- Use the dedicated AXHub workflow immediately: review -> `axhub-review`, debug -> `axhub-debug`, TDD -> `axhub-tdd`, plan -> `axhub-plan`, ship/readiness -> `axhub-ship`, diagnose loop -> `axhub-diagnose`.
- In Claude Desktop, do not expose `using-axhub-quality`, `quality auto-mode`, skill names, route labels, slash commands, missing TodoWrite, or workflow labels. Start with the natural first sentence required by the dedicated workflow.

THE RULE:
1. BEFORE producing your response, use the Read tool on `.axhub-state/quality.json` to load current quality state.
2. Process the user's primary task normally.
3. AFTER you respond to the user, evaluate the state and invoke the matching axhub Skill when needed.

Invoke `axhub-review` when:
- `lines_since_review_user` is greater than `thresholds.lines` or 50.
- `files_changed_since_review` is greater than `thresholds.files` or 5.
- `HEAD != review_commit_sha` and the unreviewed diff is larger than 20 changed lines.

Invoke `axhub-debug` when:
- `last_test_failure_at` is within the last 60 minutes and no debug acknowledgement exists yet.

Invoke `axhub-tdd` once per session when:
- `test_files_count / source_files_count < 0.5` and a new source file was added this session.

Invoke `axhub-plan` when:
- a major architectural change is detected, such as more than 50 files touched or a new module boundary.

Invoke `axhub-ship` when:
- the user asks for PR, release, deploy readiness, or push preparation.

RULES:
- Use the Read tool, NOT Bash, to load `.axhub-state/quality.json`.
- Direct explicit quality requests take precedence over this state-based rule.
- Invoke quality Skills AFTER responding only for ordinary non-quality coding tasks. The user's primary task gets priority.
- If the state file is missing or malformed, skip quality checks silently.
- If `AXHUB_DISABLE_TRIGGERS=1` is set, skip all checks.
- Invoke the same quality Skill at most once per turn.
</EXTREMELY-IMPORTANT>

## State-Based Trigger Examples

These examples describe state-based follow-ups, not direct user routing.

| State signal | Follow-up |
| --- | --- |
| Unreviewed diff threshold crossed after an ordinary coding task | axhub-review |
| Recent test failure exists after an ordinary coding task | axhub-debug |
| New source files have weak test coverage after an ordinary coding task | axhub-tdd |
| Large module-boundary change detected after an ordinary coding task | axhub-plan |
| User asks for readiness after ordinary coding work is complete | axhub-ship |

Direct user requests take precedence over state-based rules.

## Non-interactive AskUserQuestion guard (D1)

This SKILL has no AskUserQuestion call site. If a future edit adds one, register the safe default in `tests/fixtures/ask-defaults/registry.json` before shipping.

## Anti-patterns

- Do not use Bash to read quality state.
- Do not block on missing state.
- Do not interrupt the user's primary task before answering.
