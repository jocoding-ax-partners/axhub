# Hook Payload Fixtures

Pinned JSON payloads representing the Claude Code hook input schema as of **2026-04-23**.

> See `PLAN.md` Phase 6 (E11 critical) for the audit decision that mandated pinning these payload schemas. Without pinning, the helper would error on every Bash call across the session whenever Claude Code ships a payload-shape change.

## Schema Version

**v0** = current Claude Code hook payload shape (as of 2026-04-23).

`bin/axhub-helpers` (M1+) parses ONLY payloads matching the v0 schema. On shape
mismatch (unexpected field set or missing required fields), it exits 0 (no-op) and
writes a warning to stderr. This defends against future Claude Code harness updates
that change payload shape.

## Versioning Policy

When Claude Code ships a new payload shape:

1. Create `tests/hook-fixtures/v1/` with new fixtures matching the updated schema.
2. Bump `HOOK_SCHEMA_VERSION` constant in `src/axhub-helpers/index.ts` from `"v0"` to `"v1"`.
3. Update parser/validator logic in the affected subcommands (`preauth-check`, `classify-exit`, `session-start`).
4. Keep `v0/` fixtures in place — they serve as regression tests for the old shape.

## Fixture Index (v0/)

| File | Hook Event | Tool | Purpose |
|------|-----------|------|---------|
| `pretooluse-bash-axhub-deploy.json` | PreToolUse | Bash | Destructive `axhub deploy create` — consent gate must intercept |
| `pretooluse-non-axhub.json` | PreToolUse | Bash | Non-axhub command — helper must early-return (exit 0) |
| `pretooluse-non-bash.json` | PreToolUse | Read | Non-Bash tool — helper must early-return (exit 0) |
| `posttooluse-bash-success.json` | PostToolUse | Bash | Successful `axhub apps list` (exit 0) |
| `posttooluse-bash-exit65.json` | PostToolUse | Bash | `axhub deploy create` failed with `auth.expired` (exit 65) |
| `posttooluse-bash-exit64-in-progress.json` | PostToolUse | Bash | `axhub deploy create` blocked by `validation.deployment_in_progress` (exit 64) |
| `sessionstart.json` | SessionStart | — | Session startup — no tool_name / tool_input fields |

## Common Fields (all events)

```
session_id        UUID string
transcript_path   Absolute path to session transcript
cwd               Working directory at hook fire time
permission_mode   "ask" | "allow"
hook_event_name   "PreToolUse" | "PostToolUse" | "SessionStart" (PascalCase)
```

PreToolUse / PostToolUse add: `tool_name`, `tool_input`.
PostToolUse adds: `tool_response` (stdout, stderr, exit_code, interrupted).
SessionStart omits `tool_name` and `tool_input` entirely.

## Manual Testing

```bash
# Pipe a fixture directly into the binary subcommand:
cat tests/hook-fixtures/v0/pretooluse-bash-axhub-deploy.json | ./bin/axhub-helpers preauth-check
cat tests/hook-fixtures/v0/posttooluse-bash-exit65.json      | ./bin/axhub-helpers classify-exit
cat tests/hook-fixtures/v0/sessionstart.json                 | ./bin/axhub-helpers session-start

# Validate JSON syntax (all fixtures must parse cleanly):
for f in tests/hook-fixtures/v0/*.json; do jq -e '.' < "$f" > /dev/null && echo "OK $f"; done
```

## Lint Rule

Each fixture must parse via `jq -e '.' < fixture.json` with exit 0. Add this check
to CI alongside the binary smoke tests to catch accidental fixture corruption.
