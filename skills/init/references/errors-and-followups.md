# Init Errors And Follow-Ups Reference

Load this reference for long error routing, final result wording, optional MCP/setup follow-ups, and carry-over-safe next actions.

## Result Card

Use saga response only for verification. User-facing result should be short Korean lines. Public URL must be read from the app record and never synthesized from dry-run subdomain:

```bash
PUBLIC_URL="$(axhub apps get "$APP_ID" --no-input --field-expr '.access_url // .data.access_url // empty' 2>/dev/null || true)"
```

If `PUBLIC_URL` exists:

```text
인터넷에 올라갔어요: <confirmed-public-url>
친구한테 바로 보여줄 수 있어요.
```

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
- `auth` / CLI exit 4: say `다시 로그인해줘`.
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

## Optional MCP Setup

MCP setup is optional follow-up, not part of the creation gate. Offer it only after app creation/clone result is clear or when the user explicitly asks to connect tools/data sources.

- If required CLI/plugin setup is missing, route to onboarding/update guidance instead of mutating silently.
- Do not install or modify external MCP config in subprocess/headless mode.
- Do not block success on MCP setup. The bootstrap saga and clone are the init completion criteria.
- Keep wording natural: "데이터 연결까지 이어서 볼까요?" rather than naming internal skill labels.

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
