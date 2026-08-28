# Init Errors And Follow-Ups Reference

Load this reference for long error routing, final result wording, and carry-over-safe next actions.
Branch before provider errors: resolved apps use `axhub apps get <app> --json`, fresh apps use `axhub apps git-backend --tenant <tenant> --json`. When `git_backend.backend=selfhosted`, ignore GitHub/device/installation recovery rows and never surface their copy; use only backend-neutral app/deploy failures.


## Result Card

Use saga response only for verification. User-facing result should be short Korean lines. URL and public state must be read from the app record through `axhub apps get` CLI only and never synthesized from dry-run subdomain. Do not use `App get (axhub)`, `App list`, deployment MCP, or app connector tools even if Claude Desktop shows them as available:

```bash
PUBLIC_URL="$(axhub apps get "$APP_SLUG" --no-input --field-expr '.access_url // .data.access_url // empty' 2>/dev/null || true)"
VISIBILITY="$(axhub apps get "$APP_SLUG" --no-input --field-expr '.visibility // .data.visibility // empty' 2>/dev/null || true)"
REVIEW_STATUS="$(axhub apps get "$APP_SLUG" --no-input --field-expr '.review_status // .data.review_status // empty' 2>/dev/null || true)"
```

If `PUBLIC_URL` exists and `VISIBILITY=public` and `REVIEW_STATUS=approved`:

```text
인터넷에 올라갔어요: <confirmed-public-url>
친구한테 바로 보여줄 수 있어요.
```

If `PUBLIC_URL` exists but `VISIBILITY=private` or `REVIEW_STATUS!=approved`, do not call it public:

```text
배포는 끝났어요: <confirmed-access-url>
지금은 로그인 후 볼 수 있는 private 앱이에요.
```

If the user asked for a public app or to share with anyone, submit the publication request with the top-level publish command, not an app metadata update:

```bash
axhub publish --app "$APP_SLUG" --visibility public --execute --json
axhub review history --app "$APP_SLUG" --json
```

Report the result as "공개 신청은 들어갔고 승인 대기 중이에요" when the request is pending. Never try `axhub apps update "$APP_SLUG" --visibility public` before approval; the backend rejects visibility widening until the review is approved.

If local preview is alive, add "로컬 미리보기도 떠 있어요: `<localhost-url>`". If no confirmed URL exists, lower the claim:

```text
인터넷 배포가 시작됐어요. "방금 배포 어디까지 됐어?" 라고 물으면 이어서 확인할게요.
```

Then offer concise natural next actions:

- 코드 고치고 "다시 배포해줘"
- "방금 배포 어디까지 됐어?"
- 데이터 읽기는 template 에 설치된 `@ax-hub/sdk` 를 써요.

## Error Routing

Never show raw JSON/stderr unless `AXHUB_INIT_VERBOSE=1`. Map failures:

- `conflict` / `ambiguous_installation` / CLI exit 9: show install_url if available, ask owner again with the GitHub account picker, then retry Step 7 with the same idempotency key and `--github-owner "$GITHUB_OWNER"`. In non-interactive mode, retry only if `AXHUB_GITHUB_OWNER` exists; otherwise stop with `취소`.
- `github.installation_missing` / `github.repo_create_failed`: say `GitHub 연결 다시 해줘` and preserve resume phrase.
- `validation.template_not_found`: go back to backend template list.
- `validation.slug_collision`: go back to app name/slug once.
- `auth` / CLI exit 4: subcode `github_relogin_required` 면 GitHub 계정 연동이 없거나 만료된 상태라 device-flow fast path(`axhub github link` 승인 → `axhub github accounts list --json`)로 이어가요 — 코드를 놓쳤거나 만료됐으면 `axhub github link --fresh` 로 새 코드를 받아요 (저장된 pending link 는 죽은 코드를 그대로 돌려줘요). `--fresh` 가 exit 64 로 거부되면 그 플래그를 모르는 구 CLI 라 플래그 없이 한 번만 다시 실행하고 update 스킬로 CLI 를 올리도록 안내해요 — axhub 재로그인으로는 안 풀려요. 연동이 살아 있으면 이 경로 자체가 안 나와요. 그 외에는 `다시 로그인해줘`.
- `forbidden` / `tenant_scope` / CLI exit 12 or 8: explain permission/workspace admin issue.
- missing `repo_full_name`: do not clone; say `설치 상태 진단해줘` can inspect.
- anything else: say `설치 상태 진단해줘`.

Use `../../deploy/references/error-empathy-catalog.md` when a longer 4-part Korean exit-code message is needed.

## Optional Code Analysis Follow-Up

After successful creation, ask once whether to recommend tables/env needed by the cloned code. In non-interactive/D1, safe default is `아니요`.

```json
{
  "questions": [{
    "question": "방금 만든 코드에서 필요한 테이블·환경변수를 추천받을래요?",
    "header": "사전 점검",
    "multiSelect": false,
    "options": [
      {"label": "아니요", "description": "지금은 넘어가요"},
      {"label": "네, 추천받기", "description": "코드 분석으로 필요한 테이블·env 를 추천받아요"}
    ]
  }]
}
```

If the user chooses analysis, infer from cloned scaffold code. Include same-conversation connector/table results only when actual results are visible in context. Do not claim carry-over from memory or intent alone; follow `../../deploy/references/session-carryover.md`.

## Carry-Over Contract

Same-conversation carry-over is allowed only with concrete evidence: previous `connector_query`, `connector_resources`, `row_list`, `table_list`, or onboarding Ready card in the current conversation. The conditional ack is allowed only in interactive D1:

```text
방금 본 <리소스> 데이터 반영할게요.
```

If evidence is absent, do a cold template flow and do not invent resources, tables, env vars, or user requirements.

## Resume Phrases

When stopping for user action, leave a phrase that resumes the correct lane:

- CLI/auth: `다시 로그인해줘`
- GitHub App install: `설치했어` or `다시 만들어줘`
- Device-flow fallback after browser-open failure or expiry: `다시 만들어줘`
- In-progress deploy: `방금 배포 어디까지 됐어?`
- Diagnostics: `설치 상태 진단해줘`

Do not tell Desktop users to type slash commands unless they explicitly ask for slash syntax.
