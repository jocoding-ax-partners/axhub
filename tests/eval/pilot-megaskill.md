---
name: using-axhub-quality
description: axhub quality auto-mode directive. 다음 응답 전에 quality state 를 읽고 axhub-review, axhub-debug, axhub-tdd, axhub-plan, axhub-ship 호출을 권장해요.
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: sonnet
license: MIT
---

# using-axhub-quality

<EXTREMELY-IMPORTANT>
You have axhub quality auto-mode enabled.

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
- Invoke quality Skills AFTER responding, not before. The user's primary task gets priority.
- If the state file is missing or malformed, skip quality checks silently.
- If `AXHUB_DISABLE_TRIGGERS=1` is set, skip all checks.
- Invoke the same quality Skill at most once per turn.
</EXTREMELY-IMPORTANT>

## Korean Trigger Examples

| Korean utterance | Skill |
| --- | --- |
| 리뷰해줘, 코드 봐줘, PR 검토 | axhub-review |
| 디버그해, 왜 안 돼, 에러 원인 | axhub-debug |
| 배포 준비, PR 만들어, 릴리즈 | axhub-ship |
| TDD 로 가, 테스트 먼저 | axhub-tdd |
| 플랜 짜줘, 계획 세워 | axhub-plan |

State-based rules take precedence over trigger examples.

## Anti-patterns

- Do not use Bash to read quality state.
- Do not block on missing state.
- Do not interrupt the user's primary task before answering.
