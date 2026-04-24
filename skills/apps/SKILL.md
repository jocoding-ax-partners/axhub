---
name: apps
description: This skill should be used when the user wants to list or browse axhub apps registered to their team. Activates on "내 앱 보여줘", "내 앱 봐", "앱 뭐 있어", "앱 목록 봐", "어떤 앱 있어", "앱 리스트", "운영 중인 앱 뭐 있어", "등록된 앱 봐", "회사 앱 뭐 있어", "우리 앱 봐", "앱 슬러그 봐", "앱 ID 봐", "앱 목록 보여주세요", "어떤 앱이 있나요", "제 앱들 보여주세요", "운영 중인 앱 보여주세요", "apps", "list apps", "my apps", "available apps", "app catalog", "which apps", "app list", or any read-only app catalog query. Filters output to the current team scope and offers expansion on demand.
---

# Apps List (read-only, team-scoped)

Show registered axhub apps for the current team. Read-only — never triggers a mutation, never needs consent token.

## Workflow

To list apps:

1. **Pre-flight (lightweight).** Confirm auth before the list call:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json
   ```

   On `auth_ok: false`, halt and route to `../deploy/references/error-empathy-catalog.md` ("exit 65"). Suggest the auth skill via "다시 로그인해줘".

2. **Fetch apps:**

   ```bash
   axhub apps list --json
   ```

3. **Filter to current team scope.** Drop entries whose `team_id` does not match `$AXHUB_TEAM_ID` (or the team derived from `axhub auth status --json`). Do NOT dump cross-team apps to the user — they are surface noise that breaks the F4 privacy guarantee.

4. **Render top 10 in Korean.** Format as a numbered list with `slug (id=N) — <status>` per row:

   ```
   현재 팀 앱 10개 (전체 N개):
     ① paydrop (id=42) — production: succeeded (12분 전)
     ② paydrop-staging (id=43) — staging: succeeded (1시간 전)
     ③ checkout-svc (id=44) — production: failed (어제)
     ...
   ```

5. **Offer expansion.** If the filtered list exceeds 10, surface AskUserQuestion:

   ```json
   {
     "question": "앱이 더 있어요. 전체 목록 보시겠어요?",
     "options": [
       {"label": "네, 전체 보기", "value": "show_all", "description": "현재 팀의 모든 앱"},
       {"label": "지금은 그대로", "value": "skip", "description": "상위 10개로 충분"},
       {"label": "검색 (slug 입력)", "value": "search", "description": "특정 slug 검색"}
     ]
   }
   ```

6. **On `validation.app_list_truncated`** (>100 apps server-side): route to `error-empathy-catalog.md` ("exit 64 + validation.app_list_truncated"); ask user to provide a numeric `--app <id>` directly.

7. **On non-zero exit**, route to `error-empathy-catalog.md` by exit code (65 / 67 / 68 / 1). Read paths may auto-retry once on exit 1.

## NEVER

- NEVER list cross-team apps without explicit user opt-in (F4 privacy guarantee).
- NEVER dump >10 rows in the first response (overwhelms vibe coders).
- NEVER drop `--json` (parsing depends on it).
- NEVER cache app_id locally for use in mutation paths — the deploy skill must live-resolve.
- NEVER echo internal endpoint URLs of cross-team apps even if visible in stdout.

## Additional Resources

For Korean trigger lexicon (apps intent): `../deploy/references/nl-lexicon.md`.
For 4-part Korean exit templates: `../deploy/references/error-empathy-catalog.md`.
For privacy filter rules (cross-team scope, NFKC normalize): see the redact subcommand in `axhub-helpers` and PLAN §16.17.
