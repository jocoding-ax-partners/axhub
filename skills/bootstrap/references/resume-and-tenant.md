# Init Resume And Tenant Reference

Load this reference when Step 0.5 resume state, pending GitHub device-flow recovery, or tenant selection needs more detail than the top-level skill.

## Resume Route

After `axhub plugin-support preflight --json` succeeds, check repo-local state before listing templates:

```bash
axhub plugin-support init-resume route --json
```

Expected shape is `{route(fresh|watch_status|resume_last), reason, state_stale, requires_status_authority, args{status_command, resume_command}}`. If route is `watch_status` or `resume_last` and `clone_done=false`, ask:

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

If the user chooses resume, use the route enum and `args.*_command` returned by the CLI. Do not reconstruct raw IDs in the skill.

- `watch_status`: run `args.status_command`. Current shape is `axhub apps bootstrap-status "$BOOTSTRAP_ID" --watch --watch-timeout 9m --json`.
- `resume_last`: run `args.resume_command` as the base argv. Append `--tenant "$AXHUB_TENANT"` only when `$AXHUB_TENANT` is set and the base command lacks tenant context. Current shape is `axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --tenant "$AXHUB_TENANT" --execute --resume-last --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json`.
- stale/broken/fresh: say "이전 기록을 찾지 못해서 새로 시작할게요." and continue to template registry.

## Resume Device-Flow Recovery

If resume fails with `no pending github device flow`, do not declare hard failure immediately. First re-check the read-only account surface:

```bash
axhub github accounts list --json
```

Only when the selected GitHub owner is confirmed installed (`installed=true` or `installation_id`) may the skill run one recovery execute using the same template/name/slug/subdomain/github-owner/repo-name/idempotency-key, without `--resume-last`:

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub apps bootstrap --template "$TEMPLATE" --name "$APP_NAME" --slug "$APP_SLUG" --subdomain "$SUBDOMAIN" --github-owner "$GITHUB_OWNER" --repo-name "$APP_SLUG" --repo-private --tenant "$AXHUB_TENANT" --execute --watch --watch-timeout 9m --idempotency-key "$IDEMPOTENCY_KEY" --json
```

If `device_code_pending` remains, respect `retry_after_secs` and retry the emitted `resume_command` until success or expiry. If owner installation is not confirmed, do not run fresh execute; show the install URL once and stop with the GitHub App install resume phrase.

## Tenant Resolve L1

`axhub plugin-support tenant-resolve` owns risky tenant logic. The skill is a thin resolver and cache reader. Explicit `AXHUB_TENANT` wins. Otherwise call once and persist only a resolved tenant:

```bash
TENANT_CACHE=".axhub/state/tenant.json"
NEEDS_PICK="false"
CANDIDATES_JSON="[]"
if [ -z "${AXHUB_TENANT:-}" ]; then
  eval "$(axhub plugin-support tenant-resolve --field-expr '"AXHUB_TENANT=" + (.tenant // "" | @sh), "_NEEDS_PICK_RAW=" + (.needs_pick // false | tostring | @sh), "CANDIDATES_JSON=" + ((.candidates // []) | tojson | @sh), "_FIRST_CANDIDATE=" + ((.candidates // [])[0].id // (.candidates // [])[0].slug // "" | @sh)' 2>/dev/null)"
  : "${AXHUB_TENANT:=}"
  : "${_NEEDS_PICK_RAW:=false}"
  : "${CANDIDATES_JSON:=[]}"
  : "${_FIRST_CANDIDATE:=}"
  if [ "$_NEEDS_PICK_RAW" = "true" ]; then
    if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
      AXHUB_TENANT="$_FIRST_CANDIDATE"
      echo "여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant($AXHUB_TENANT)로 진행해요"
    else
      NEEDS_PICK="true"
    fi
  fi
fi
if [ -n "${AXHUB_TENANT:-}" ] && [ "$NEEDS_PICK" = "false" ]; then
  mkdir -p "$(dirname "$TENANT_CACHE")"
  printf '{"tenant":"%s","source":"resolved","ts":%s}\n' "$AXHUB_TENANT" "$(date +%s 2>/dev/null || echo '0')" > "$TENANT_CACHE"
fi
export AXHUB_TENANT
export NEEDS_PICK
export CANDIDATES_JSON
```

If `AXHUB_TENANT` is still empty, check preflight `auth_ok` and `current_team_id`, then guide with `다시 로그인해줘`.

## Tenant Picker L2

Only when `NEEDS_PICK=true` and the host is interactive TTY, ask once:

```json
{
  "questions": [{
    "question": "어떤 tenant 로 진행할까요?",
    "header": "Tenant",
    "multiSelect": false,
    "options": [
      {"label": "<tenant name/slug/id>", "description": "이 tenant 로 진행"}
    ]
  }]
}
```

Write the chosen tenant to `.axhub/state/tenant.json` as `{tenant, source:"picker", ts}`. In non-interactive/D1, skip L2 because L1 already used first-candidate fallback or left tenant empty for login guidance.

## Fence Re-Read

Every later command fence should re-read tenant because env may not survive:

```bash
AXHUB_TENANT="${AXHUB_TENANT:-$(axhub plugin-support tenant-resolve --field-expr '.tenant // empty' 2>/dev/null || true)}"
```
