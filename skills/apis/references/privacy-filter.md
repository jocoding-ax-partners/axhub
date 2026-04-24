# APIs Privacy Filter — Cross-Team Scope Isolation

Default-deny scope rules for `axhub apis list`, plus the audit-log contract that fires whenever the user crosses the team boundary. Implements PLAN §16.17 (revised E13 fix) and the row 46 cross-team audit requirement.

All user-facing copy is Korean. All commands assume `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers` is on PATH.

---

## 1. 왜 격리하는가 (Phase 6 §16.17 first-customer-audit risk)

`axhub apis list` is the highest-leak surface in v0.1: a single broad-scope token can enumerate every endpoint the company exposes. Phase 6 audit (E13, finding F4) classified this as **CRITICAL** with the note "first regulated customer = security incident if shipped as-is."

The risk has three concrete failure modes the filter must close:

1. **Silent broad enumeration.** Vibe coder asks "어떤 API 있어?", token has cross-team read scope, CLI returns 200+ endpoints with `service_base_url` exposed. The user never opted in; the audit log never recorded the access; the other team's on-call never sees that their internal hostnames just shipped to a Claude transcript.
2. **Homoglyph trust transfer.** User intended `paydrop` (latin), token resolves to `pаydrop` (Cyrillic 'а', §16.11). Without scope filtering, the cross-team list returns the homoglyph team's endpoints with the same render template — the user reads them as their own.
3. **Stale-team leak.** User changed teams two weeks ago; cached tokens still carry the old team's scope. Without the live `current_team` resolve in step 2 below, the previous team's catalog leaks every time.

The fix is structural, not advisory: **default scope is the current app, cross-team requires AskUserQuestion + audit log every time, and the adapter redacts `service_base_url` for any out-of-team result even when the user opted in.**

---

## 2. Default scope 동작 (`--app-id` 결정 로직)

The skill never calls `axhub apis list` without an explicit scope flag. The decision tree:

```
Does $CURRENT_APP exist in recent-context cache?
├─ YES → use it directly: axhub apis list --app-id "$CURRENT_APP" --json
└─ NO  → resolve live (no cache, no inference per PLAN §16.13 + Phase 6 row 13)
         │
         ├─ Run: axhub-helpers resolve --intent apis --user-utterance "$ARGS" --json
         │   Returns: { "app_id": "...", "team_id": "...", "source": "git_remote|auth_status|prompt", ... }
         │
         ├─ Source resolution priority (helper-internal):
         │   ① auth_status: token's primary_app_id field if exactly one
         │   ② git_remote: parse `git remote get-url origin`, match against axhub apps list
         │   ③ prompt: AskUserQuestion to pick from `axhub apps list --json` (top 5 by recency)
         │
         └─ On app_id resolved, cache to recent-context for the session and proceed
```

The helper's `resolve` step is required to be live (not cached across sessions). PLAN row 13 was reversed in Phase 6 specifically to prevent stale-team leakage: the previous heuristic of "infer from history" was rejected because team membership can change mid-session and the user has no signal that the cached team is stale.

The default scope command is always:

```bash
axhub apis list --app-id "$CURRENT_APP" --json | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers redact
```

The `redact` filter is mandatory in the pipeline even on the default scope path — it strips `service_base_url` for any accidentally-cross-team entry the CLI returns (defense-in-depth in case the CLI honors the flag loosely on a future version).

---

## 3. Cross-team 요청 처리 (AskUserQuestion KR copy + audit log format)

Cross-team listing fires only when the user's utterance contains an explicit cross-team marker AFTER seeing the scoped list. Acceptable triggers (whitelist, exact lexicon — see `../../deploy/references/nl-lexicon.md`):

- "다른 팀 API도 보고싶어"
- "회사 전체 API"
- "전 팀 API 목록"
- "all apis"
- "across teams"
- "show every endpoint"

Implicit phrasings ("뭐 더 있어?", "다른 거?") do NOT fire cross-team — the skill must re-prompt with the exact AskUserQuestion below. This is intentional friction: the audit-log entry needs the user's affirmative act, not the model's interpretation of ambiguity.

### AskUserQuestion (Korean)

```json
{
  "question": "다른 팀 API도 보시겠어요? 권한 있는 모든 endpoint를 보여드릴 수 있지만, 보통 현재 앱이 호출하는 것만 보면 충분해요. 한 번 보시면 회사 보안팀 감사 로그에 기록돼요 (정상 절차).",
  "options": [
    {"label": "네, 전체 보기", "value": "cross_team", "description": "권한 있는 모든 팀의 API 카탈로그 (감사 로그 1줄 추가)"},
    {"label": "현재 앱만 충분해요", "value": "stay", "description": "현재 앱 scope 유지 (감사 로그 없음)"},
    {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
  ]
}
```

### Audit log format (`~/.cache/axhub-plugin/cross-team-list.ndjson`)

Append-only, one JSON line per cross-team query. The file is created with mode 0600 on first write. Helper handles rotation (>10MB → rename to `cross-team-list.ndjson.1`, start fresh).

```json
{"ts":"2026-04-23T10:14:32Z","user_email":"vibe@example.com","app_id":"paydrop","team_id":"team_42","utterance_sha256":"3a7f9c...","cross_team_consent":"cross_team","result_count":214,"redacted_count":189,"cli_version":"0.1.3","plugin_version":"0.1.0","session_id":"sess_abc123"}
```

Field contract:

| Field | Required | Source | Notes |
|---|---|---|---|
| `ts` | ✓ | helper clock | UTC, ISO 8601 with `Z` suffix |
| `user_email` | ✓ | `axhub auth status --json` | the consenting account |
| `app_id` | ✓ | `$CURRENT_APP` | the app the user was on at time of consent |
| `team_id` | ✓ | resolve step | the user's current team (NOT the cross-team set) |
| `utterance_sha256` | ✓ | `sha256(user_utterance)` | hashed for privacy; admins can correlate without seeing the raw words |
| `cross_team_consent` | ✓ | AskUserQuestion result | always `"cross_team"` here; `stay`/`abort` do NOT write a row |
| `result_count` | ✓ | length of returned API list | |
| `redacted_count` | ✓ | count of out-of-team rows whose `service_base_url` was stripped | |
| `cli_version` | ✓ | `axhub --version --json .version` | |
| `plugin_version` | ✓ | plugin manifest | |
| `session_id` | ✓ | Claude Code session | for cross-correlating with deploy/auth events |

The audit log NEVER records the raw utterance, the API list itself, or the redacted URLs — only the metadata sufficient for an auditor to reconstruct that an opt-in occurred. If the helper cannot write the audit log (disk full, permission denied), the skill MUST refuse the cross-team call with a Korean message:

> "감사 로그를 기록할 수 없어서 다른 팀 API 조회를 진행할 수 없어요. 디스크 공간을 확인하시거나 `~/.cache/axhub-plugin/` 권한을 봐주세요."

This is fail-closed by design — silent cross-team enumeration without an audit trail is the original incident this filter exists to prevent.

---

## 4. Adapter layer redaction (cross-team `service_base_url` 제거 OR Punycode 변환)

Even when the user opted into cross-team, the adapter still rewrites the response before it reaches the user. Two transformations apply, in order:

### 4a. `service_base_url` redaction (out-of-team)

For every API entry where `entry.team_id != current_team`:

```ts
// Inside ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers redact
if (entry.team_id !== ctx.current_team) {
  entry.service_base_url = "[redacted: cross-team — ask owning team for endpoint]";
  entry._redacted_team_id = entry.team_id; // kept so audit log can count
}
```

Render template treats redacted entries identically to in-team entries in every other field — method, path, auth, scope, description all show. Only the host is hidden. This lets the user discover capability ("팀 X has a payments-refund API") without giving them a one-click call surface.

### 4b. Punycode display for non-ASCII slugs (PLAN §16.11)

For every `app_slug`, `team_slug`, or display name in the rendered list, the helper passes the string through NFKC normalize and Punycode encode if any non-ASCII codepoint remains after normalization:

```
Display rule:
  raw slug "paydrop"        → render "paydrop" (no transform)
  raw slug "pаydrop" (Cyrillic 'а') → render "xn--pydrop-vqf (homoglyph: ASCII에 없는 문자 포함, Punycode 표시)"
  raw slug "결제팀"           → render "결제팀 (xn--bj1bx7lu89a)"
```

The Punycode hint is REQUIRED for every non-ASCII slug, even legitimate Hangul. This is not a security-vs-friendliness tradeoff: the Phase 6 §16.11 audit rejected "show Punycode only on suspicious chars" because the model can't reliably classify suspicious vs legitimate at render time. Always show both forms; let the user pattern-match.

If NFKC normalization changed the string (e.g., full-width digit `１` → `1`, fancy quotes), prepend a warning in Korean:

```
⚠ "ｐaydrop" → "paydrop" 으로 자동 정규화됨 (NFKC). 의도하신 게 맞나요?
```

The corpus tests `T-UNI-1` (Cyrillic homoglyph), `T-UNI-2` (zero-width joiner), `T-UNI-3` (Bidi override) gate this transformation; without them passing, the apis skill is blocked from M1 release.

---

## Cross-flow rules

- **Default-deny is structural, not advisory.** The skill MUST NOT call `axhub apis list` without `--app-id` or `--team-id` unless step 3's AskUserQuestion returned `cross_team`. PreToolUse hook denies the bare command as defense-in-depth.
- **Audit log is fail-closed.** No log write → no cross-team call. Never silent.
- **Redaction is mandatory on both paths.** Default scope still pipes through `redact` (defense-in-depth against CLI scope-flag bugs).
- **Punycode is for every non-ASCII slug**, not only suspicious ones (Phase 6 §16.11 rule).
- **NEVER cache the cross-team catalog** locally — see `../SKILL.md` NEVER list. Team membership changes invalidate it instantly.

For the auth-side companion (token scope vs. team scope), see `../../deploy/references/recovery-flows.md` ("headless-auth").
For the Korean lexicon that decides what counts as a cross-team trigger, see `../../deploy/references/nl-lexicon.md`.
For PLAN reference: §16.17 (apis list privacy / E13 fix), §16.11 (Unicode hardening), Phase 6 row 46 (audit log requirement).
