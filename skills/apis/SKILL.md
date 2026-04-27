---
name: apis
description: 이 스킬은 사용자가 axhub API 엔드포인트를 탐색하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "API 뭐 있어", "어떤 API 있어", "API 목록 봐", "API 리스트", "API 카탈로그", "쓸 수 있는 API", "호출 가능한 API", "엔드포인트 뭐 있어", "API 보여줘", "사내 API 뭐 있어", "API 목록 보여주세요", "사용 가능한 API 알려주세요", "API 카탈로그 확인해주세요", "어떤 엔드포인트가 있나요", "apis", "list apis", "api catalog", "available apis", "available endpoints", "endpoints", "services", 또는 axhub API 탐색 의도. 기본 scope 는 현재 앱이며 다른 팀 경계를 넘기 전에 사용자에게 동의 확인합니다.
---

# APIs Catalog (read-only, current-app scoped)

List axhub APIs available to the current app. **Privacy-first**: default scope is the current app only (per PLAN §16.17 / E13 fix). Cross-team listing requires explicit opt-in via AskUserQuestion.

## Workflow

To list APIs:

1. **Pre-flight.** Confirm auth:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json
   ```

   On `auth_ok: false`, halt and route to "exit 65" template.

2. **Resolve current app.** Read `$CURRENT_APP` (set by recent-context cache from prior deploy/status interactions). If absent:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent apis --user-utterance "$ARGS" --json
   ```

   On `app_id: null`, ask the user via AskUserQuestion to pick an app from `axhub apps list --json` (top 5 by recency).

   **Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — app pick → `top_recent` (가장 최근 deploy 한 앱), cross-team opt-in → `stay` (privacy-first 기본), pagination → `stop` (첫 10개 만).

3. **DEFAULT — current-app scope:**

   ```bash
   axhub apis list --app-id "$CURRENT_APP" --json
   ```

   Render top 10 endpoints with `method path — description (auth_required)`:

   ```
   paydrop (id=42)에서 호출 가능한 API 10개:
     ① POST /v1/payments — 결제 생성 (oauth required)
     ② GET  /v1/payments/{id} — 결제 조회 (oauth required)
     ③ POST /v1/refunds — 환불 (oauth + scope: refund.write)
     ...
   ```

4. **Cross-team opt-in.** If the user explicitly says "다른 팀 API도 보고싶어", "회사 전체 API", "all apis", or asks after seeing the scoped list, surface AskUserQuestion:

   ```json
   {
     "question": "다른 팀 API도 보시겠어요? 권한 있는 모든 endpoint를 보여드릴 수 있지만, 보통 현재 앱이 호출하는 것만 보면 충분해요.",
     "options": [
       {"label": "네, 전체 보기", "value": "cross_team", "description": "권한 있는 모든 팀의 API 카탈로그"},
       {"label": "현재 앱만 충분해요", "value": "stay", "description": "현재 앱 scope 유지"},
       {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
     ]
   }
   ```

5. **On `cross_team` only**, run without `--app-id`:

   ```bash
   axhub apis list --json
   ```

   Audit log this elevation locally to `~/.cache/axhub-plugin/cross-team-list.ndjson` so admins can review (PLAN row 46 audit requirement).

6. **Render long lists in pages.** For >20 entries, paginate via AskUserQuestion ("다음 10개 / 검색 / 그만"). Never dump >100 rows at once.

7. **Redact service base URLs** of cross-team APIs even when listed. Helper handles this:

   ```bash
   axhub apis list --json | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers redact
   ```

8. **On non-zero exit**, route to `error-empathy-catalog.md` by code (65 / 67 / 68 / 1).

## NEVER

- NEVER call `axhub apis list` without `--app-id` unless the user explicitly opted in (E13 / F4 privacy fix — Phase 6 §16.17).
- NEVER echo cross-team `service_base_url` raw — always pass through the redact filter.
- NEVER cache the cross-team catalog locally (pulls back stale once team membership changes).
- NEVER drop `--json`.

## Additional Resources

For Korean trigger lexicon (apis intent): `../deploy/references/nl-lexicon.md`.
For 4-part Korean exit templates: `../deploy/references/error-empathy-catalog.md`.
For cross-team API scope filter and audit log format, see `references/privacy-filter.md`.
For PLAN reference: §16.17 (apis list privacy / E13 fix), row 46 (cross-team audit log).
