# Init Resume And Tenant Reference

Load this reference only when Step 0.5 resume state, pending GitHub device-flow recovery, or tenant selection needs more detail than the top-level skill. The normal fresh bootstrap path should not read this file; the top-level skill already covers tenant selection without plugin-cache reference reads.

Desktop-visible commands here must stay direct and portable: no `rtk`, no generic `ls`/`pwd` probes, and no shell wrapper unless the command is explicitly documented as an execute/resume recovery command. Prefer the CLI-emitted command string only when resuming a prior flow; for fresh flow checks, run the literal `axhub plugin-support ... --json` command shown in each section.

## Resume Route

After `axhub plugin-support preflight --json` succeeds, check repo-local state before listing templates:

```bash
axhub plugin-support init-resume route --json
```

Expected shape is `{route(fresh|watch_status|resume_last), reason, state_stale, requires_status_authority, args{status_command, resume_command}}`.

Before showing the resume question or running either `args.status_command` or `args.resume_command`, choose the backend read from persisted route authority:

- route가 `watch_status`이거나 state에 `bootstrap_id`가 있으면 persisted app id/slug로 app-level choice를 읽어요.

  ```bash
  axhub apps get <app> --json
  ```

- route가 `resume_last`이고 `bootstrap_id`가 없으면 app row가 아직 없어요. 먼저 아래 Tenant Resolve L1로 literal tenant를 확정하고 tenant default를 읽어요.

  ```bash
  axhub apps git-backend --tenant <tenant> --json
  ```

Missing/malformed `git_backend` or the selected read's failure stops fail-closed before any emitted command. If `git_backend.backend=selfhosted`, discard a stale provider-auth `resume_last` route and never execute its emitted command. If a bootstrap id exists, continue through the backend-neutral status path; otherwise rebuild the ownerless selfhosted resume from stored template/name/slug/tenant/idempotency values. Do not surface GitHub/device/install copy. Only confirmed `github` or `legacy_github` may execute a provider-auth resume command.

After this check, if route is `watch_status` or `resume_last` and `clone_done=false`, ask:

```json
{
  "question": "저번에 만들던 앱을 이어서 할까요?",
  "header": "이어서",
  "options": [
    {"label": "이어서 하기", "value": "resume", "description": "이전 생성 흐름을 계속해요"},
    {"label": "새로 시작", "value": "fresh", "description": "이전 기록은 두고 새 앱 생성을 시작해요"}
  ]
}
```

Non-interactive/D1 safe default is `새로 시작`. Do not echo raw `bootstrap_id`, `idempotency_key`, repo, or slug. Humanize only a short app alias if needed.

If the user chooses resume, use the route enum only after the backend check above. Do not reconstruct raw IDs except for the ownerless selfhosted resume described above.

- `watch_status`: run `args.status_command`. Current shape is `axhub apps bootstrap-status "$BOOTSTRAP_ID" --watch --watch-timeout 9m --json`.
- `resume_last`: use `args.resume_command` as the base argv, but never run it verbatim. Append `--tenant "$AXHUB_TENANT"` only when `$AXHUB_TENANT` is set and the base command lacks tenant context. For Desktop device-flow recovery, strip `--watch --watch-timeout <value>` and `--json` from the first Desktop resume so stdout is visible. Do not use background watchers or output-file reads as the resume control plane. Current shape should be `axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --tenant "$AXHUB_TENANT" --execute --resume-last --idempotency-key "$IDEMPOTENCY_KEY"`.
- stale/broken/fresh: say "이전 기록을 찾지 못해서 새로 시작할게요." and continue to template registry.

## Resume Device-Flow Recovery

This whole section is GitHub-only fallback and may be entered only after the Resume Route check confirmed `git_backend.backend=github` or `git_backend.source=legacy_github`; a linked GitHub account never reaches a pending device flow, so the remainder applies only to a missing or expired link.

If resume fails with `no pending github device flow`, do not declare hard failure immediately. First re-check the read-only account surface:

```bash
axhub github accounts list --json
```

Only when the selected GitHub owner is confirmed installed (`installed=true` or `installation_id`) may the skill run one recovery execute using the same template/name/slug/subdomain/github-owner/repo-name/idempotency-key, without `--resume-last`:

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
axhub --no-input apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --github-owner "$GITHUB_OWNER" --repo-name "$APP_SLUG" --repo-private --tenant "$AXHUB_TENANT" --execute --idempotency-key "$IDEMPOTENCY_KEY"
```

If `device_code_pending` remains, respect `retry_after_secs` and retry the emitted `resume_command` until success or expiry. Every pending payload from a cached resume carries `user_code` + `verification_uri`; re-show them in the body on each retry so the user always has something to approve. Do not ask the user to say an approval phrase in chat; the CLI resume result is the only completion signal. If owner installation is not confirmed, do not run fresh execute; show the install URL once and stop with the GitHub App install resume phrase.

## Tenant Resolve L1

`axhub plugin-support tenant-resolve` owns risky tenant logic. The skill is a thin resolver and cache reader. Explicit `AXHUB_TENANT` wins. Otherwise call once and use the resolved tenant as a literal value in later commands. In Codex, the visible tool title for this command must be `앱 설정 확인`; never use `tenanting 확인`, `tenant 확인`, `테넌트 확인`, or `tenant-resolve`.

```bash
axhub plugin-support tenant-resolve --field-expr '.tenant // empty'
```

If the command returns one tenant value, carry it as a literal `--tenant <slug>` value. If the resolver output says multiple tenants need a pick, ask the picker from the resolver/preflight candidates and carry the chosen value as the same literal `--tenant <slug>` value. User-facing text must call these `작업공간`, not `tenant` or `테넌트`. If no tenant can be resolved, check preflight `auth_ok` and `current_team_id`, then guide with `다시 로그인해줘`.

## Tenant Picker L2

Only when the resolver/preflight output says multiple tenants need a pick and the host is interactive TTY, ask once. The surrounding chat sentence should be "작업공간이 여러 개 있어요. 어디에 만들지 골라주세요." Do not say "테넌트가 2개 있어요." or "어떤 tenant 로 진행할까요?"

```json
{
  "questions": [{
    "question": "새 앱을 어느 작업공간에 만들까요?",
    "header": "작업공간",
    "multiSelect": false,
    "options": [
      {"label": "<workspace name/slug/id>", "description": "이 작업공간에 앱을 만들어요"}
    ]
  }]
}
```

Do not write `.axhub/state/tenant.json` from Codex. Do not use `mkdir`, `printf`, redirection, timestamp commands, or shell glue to persist the choice. In non-interactive/D1, use the CLI resolver's first-candidate fallback when it is explicitly returned; otherwise leave tenant empty for login guidance.

## Fence Re-Read

Every later command fence should use the literal tenant already selected in the current conversation. If the value is genuinely missing, re-run the direct resolver command:

```bash
axhub plugin-support tenant-resolve --field-expr '.tenant // empty'
```
