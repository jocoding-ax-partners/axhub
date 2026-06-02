# Source Audit 007 — ax-hub-cli v0.17.3 command/source evidence

이 감사는 `specs/007-vibe-skill-gapfill` 계획을 최신 CLI에 맞추기 위해 수행한 source-grounding 기록이에요.

## 기준 고정

| 항목 | 값 |
|---|---|
| Reference repo | `/Users/wongil/Desktop/work/jocoding/ax-hub-cli` |
| 최신 기준 | `origin/main` = tag `v0.17.3` |
| Commit | `a5310b6 chore(release): axhub-cli 0.17.3` |
| Snapshot | `/tmp/ax-hub-cli-origin-main-007` |
| Build/schema evidence | `cargo run --manifest-path /tmp/ax-hub-cli-origin-main-007/Cargo.toml --bin axhub -- --help`; `axhub --json-schema > /tmp/axhub-schema-007.json` |
| Local checkout caveat | current checkout is `fix/manifest-filename-axhub-yaml` at `382e145`, workspace `Cargo.toml` shows `0.17.2`, with uncommitted changes. It is not the latest source of truth for this spec. |

## Audited source set

- `/tmp/ax-hub-cli-origin-main-007/axhub/src/cli.rs`
- `/tmp/ax-hub-cli-origin-main-007/axhub/src/commands/**/*.rs`
- `/tmp/ax-hub-cli-origin-main-007/axhub/tests/**/*.rs` where command behavior is asserted
- `/tmp/ax-hub-cli-origin-main-007/docs/cli-reference.html` as generated docs evidence
- Current axhub skills: `/Users/wongil/Desktop/work/jocoding/axhub/skills/*/SKILL.md`
- Current axhub slash commands: `/Users/wongil/Desktop/work/jocoding/axhub/commands/*.md`

## Count summary

- Public command tree from `--json-schema`: **39 top-level commands**
- Rust `Command` enum variants before `Unknown`: **43**
- Hidden root commands excluded from public schema/help: `comment`, `ctxdeadline-lint`, `debug`, `like`

## Source-grounded correction log

| Finding | Evidence | Spec impact |
|---|---|---|
| `publish --watch` is not implemented despite help text | `publish.rs` `run_backend` reads `--app` and `--note` only, then POSTs review request | `publish` skill must not promise polling |
| `dev` is not a persistent local proxy | `dev.rs` requires target/flag and prints `axhub dev proxy target=... port=...`, then exits | no `dev` skill in current wave; defer |
| `manifest check --baseline` has no success path | `manifest.rs` returns validation_error for `Some("check")` | `inspect` uses `manifest validate` only |
| `apps detect` absent in v0.17.3 origin/main | `AppsCmd` enum has no Detect variant in snapshot/schema | remove from current migrate/browse plan; branch-only future |
| `deploy create --branch` invalid | `deploy/create.rs` Args has `--app`, `--commit`, `--force-rebuild`, `--no-retry`, `--dry-run`, `--execute` | existing deploy/migrate R0 refactor |
| `deploy create` needs `--execute` for mutation | `DryRun::from_flags(args.dry_run, args.execute, ())` default dry-run true | existing deploy/migrate R0 refactor |
| `deploy cancel --yes` invalid | `deploy/cancel.rs` Args lacks `--yes`, has `--execute` | deploy cancel workflow refactor |
| `apps create --yes` invalid | `apps CreateArgs` lacks `--yes` | apps workflow refactor |
| `apps update --field` invalid | `apps UpdateArgs` exposes explicit update flags, no generic `--field` | apps workflow refactor |
| top-level `github` is not deprecated | `github.rs` exposes `accounts list` and `installations repos` | github skill must restore read surface |
| top-level `status` is general CLI status | `status.rs` prints profile/endpoint/logged_in/apps_count | inspect/status disambiguation |
| `tables` includes column removal and grants | `tables.rs` has `ColumnsCmd::Remove`, `GrantsCmd::{List,Issue,Revoke}` | tables skill includes these |
| `apps bootstrap/bootstrap-status` is public and real | `apps/bootstrap.rs` has `BootstrapArgs`, dry-run/execute, watch/watch-timeout/watch-interval, and `BootstrapStatusArgs` | existing `init` skill owns this saga; plan must mention it |
| `apps owned/workspace` are public read commands | `apps.rs` has `OwnedArgs`, `WorkspaceArgs`, and request paths | existing apps/my-resources read coverage; plan must mention them |
| `access` subcommands are dynamic/trailing-var but real | `access.rs` dispatches `grant`, `check`, `revoke`, `invite`, `uninvite` in `run_backend`; schema shows only `access :: args` because clap cannot statically expose trailing var subcommands | `team` skill may cover these current commands, but must not invent unsupported flags like `--user` on `grant`/`revoke` |
| `deploy doctor` is public diagnostic read | `deploy/doctor.rs` has `DoctorArgs` and `run` | inspect/deploy diagnostic coverage |
| `deploy fleet` is public multi-app mutation | `deploy/fleet.rs` has `--apps`, `--commit`, `--concurrency`, dry-run/execute | explicit defer/operator until fleet consent model exists |

## Generated public command tree

The tree below is generated from `/tmp/axhub-schema-007.json`. Descriptions are often empty, so behavior still comes from Rust source.

```text
access :: args
admin
admin templates
admin templates create :: description, dry-run, execute, folder-name, name, resource-tier, sort-order
admin templates get :: template_id
admin templates list
admin templates update :: active, description, dry-run, execute, inactive, name, resource-tier, sort-order, template_id
admin users
admin users revoke-all :: confirm-self, dry-run, execute, uid
agent :: args
apps
apps bootstrap :: device-code, dry-run, execute, github-owner, installation-id, name, repo-name, repo-private, repo-public, resume-last, slug, subdomain, template, tenant, watch, watch-interval, watch-timeout
apps bootstrap-status :: bootstrap, tenant, watch, watch-interval, watch-timeout
apps check-availability :: slug, subdomain, tenant
apps create :: auth-mode, category, data-scopes, deploy-method, from-file, interactive, name, resource-tier, slug, subdomain, tenant
apps delete :: app, dry-run, execute
apps discover :: category, created-within-days, limit, page, per-page, q, sort, tenant
apps fork :: dry-run, execute, name, repo-public, slug, source, subdomain, template, tenant
apps get :: app
apps git
apps git connect :: app, branch, device-code, dry-run, execute, installation-id, repo, resume-last
apps git disconnect :: app, dry-run, execute
apps git status :: app
apps git update :: app, branch, dry-run, execute
apps list :: all, category-id, operating-status, operating-status-in, page, page-size, per-page, q, review-status, review-status-in, sort, status, tenant, visibility
apps members :: app, page, per-page
apps mine :: page, per-page
apps owned
apps purge :: app, dry-run, execute
apps resume :: app, dry-run, execute
apps search :: all, category, page, per-page, query, sort, visibility
apps sign-icon-upload :: app, content-type, dry-run, execute, file, pre-create, slug, tenant, variant
apps suspend :: app, dry-run, execute
apps templates
apps templates list
apps update :: app, auth-mode, category-id, clear-category, clear-subdomain, data-scopes, description, icon-dark-url, icon-url, name, resource-tier, subdomain, visibility
apps workspace
audit :: args
auth
auth idp
auth idp create :: audience, client-id, client-secret, dry-run, enforced, execute, group-role, issuer-url, jit-provisioning, name, provider, tenant, tenant-resolution-strategy, user-type
auth idp disable :: dry-run, execute, provider_id, tenant
auth idp enable :: dry-run, execute, provider_id, tenant
auth idp list :: tenant
auth idp providers :: tenant
auth idp test :: provider_id, tenant
auth login :: device-code, force, no-browser, resume-last, scopes, tenant
auth logout :: dry-run
auth oauth
auth oauth client
auth oauth client create :: app, auth-method, copy, dry-run, execute, grant-type, name, redirect-uri, scope, type
auth oauth client get :: client_id
auth oauth consent
auth oauth consent revoke :: client_id, dry-run, execute
auth oauth revoke :: client-id, client-secret, client-secret-file, dry-run, execute, token, token-type-hint
auth pat
auth pat issue :: expires-in-days, name, no-save, show-token, use
auth pat list :: reconcile
auth pat revoke :: dry-run, execute, id
auth pat rotate :: expires-in-days, name, show-token
auth pat unset
auth pat use :: id
auth pat whoami
auth refresh :: no-browser, scopes
auth status
auth whoami
authz
authz grants
authz grants grant :: actions-file, actions-json, dry-run, execute, grant_id, reason, tenant
authz grants list :: granted-only, kind, limit, resource-tag-id, subject-tag-id, tenant
authz grants revoke :: dry-run, execute, grant_id, reason, tenant
authz grants show :: grant_id, tenant
authz grants update-actions :: actions-file, actions-json, dry-run, execute, grant_id, reason, tenant
authz subjects
authz subjects create :: attributes-file, attributes-json, dry-run, execute, name, parent-id, sort-order, tenant, type, user-id
authz subjects delete :: dry-run, execute, subject_id, tenant
authz subjects list :: parent-id, tenant
authz subjects move :: dry-run, execute, parent-id, root, subject_id, tenant
authz subjects tag-attach :: dry-run, execute, subject_id, tag-id, tenant
authz subjects tag-detach :: dry-run, execute, subject_id, tag-id, tenant
authz subjects update :: attributes-file, attributes-json, dry-run, execute, name, sort-order, subject_id, tenant
authz tags
authz tags create :: description, dry-run, execute, kind, name, sensitivity, tenant
authz tags delete :: dry-run, execute, tag_id, tenant
authz tags list :: kind, tenant
authz tags update :: description, dry-run, execute, name, sensitivity, tag_id, tenant
cache
cache clear :: all, app
catalog
catalog connectors :: tenant
catalog get :: connector, path, tenant
catalog invoke :: action, connector, dry-run, execute, params-file, params-json, path, row-limit, sql, sql-file, tenant
catalog kinds
catalog resources :: all, connector, connector-id, cursor, kind, limit, search, tenant
catalog search :: all, connector, connector-id, cursor, kind, limit, search, tenant
categories
categories create :: color, description, display-order, icon-url, name, slug, tenant
categories delete :: dry-run, execute, id, tenant, yes
categories get :: id, tenant
categories list :: all, page, per-page, tenant
categories update :: color, description, display-order, dry-run, execute, icon-url, id, name, slug, tenant
completion
completion bash
completion fish
completion powershell
completion zsh
completion-data
completion-data idps :: tenant
config
config explain
connectors
connectors create :: config-file, config-json, credentials-file, credentials-stdin, description, dry-run, engine, execute, name, tenant
connectors credentials-set :: connector_id, credentials-file, credentials-stdin, dry-run, execute, tenant
connectors delete :: connector_id, dry-run, execute, tenant
connectors discover :: connector_id, tenant
connectors list :: enabled-only, tenant
connectors update :: config-file, config-json, connector_id, description, disabled, dry-run, enabled, execute, tenant
data
data count :: app, filter, page, per-page, select, sort, table
data delete :: app, dry-run, execute, id, table
data get :: app, id, table
data insert :: app, batch, body, dry-run, execute, table
data list :: app, filter, page, per-page, select, sort, table
data update :: app, body, dry-run, execute, id, table
deploy :: explain
deploy cancel :: app, deployment_id, dry-run, execute
deploy codes
deploy create :: app, commit, dry-run, execute, force-rebuild, no-retry
deploy doctor :: app
deploy explain :: app
deploy fleet :: apps, commit, concurrency, dry-run, execute, tenant
deploy git
deploy git configure :: app, branch, dry-run, execute, installation-id, repo, state, update
deploy git connect :: app
deploy git disconnect :: app, dry-run, execute
deploy git status :: app
deploy list :: all, app, ndjson, page, page-size, per-page
deploy logs :: app, deployment_id, follow, limit, reconnect-attempts, since, source, until
deploy rollback :: app, dry-run, execute, from-deployment
deploy status :: app, deployment_id, watch, watch-interval, watch-timeout
deploy watch :: app, deployment_id, no-tui, reconnect-attempts, source, watch-interval, watch-timeout
dev :: args
doctor :: args, dry-run, fix, offline, send-report
email-domains
email-domains add :: domain, tenant
email-domains check :: email, tenant
email-domains list :: tenant
email-domains remove :: domain, dry-run, execute, tenant, yes
engines
engines list
env
env delete :: app, dry-run, execute, key
env get :: app, key
env list :: app
env set :: app, from-stdin, key, plain, secret, stage, value
env update :: app, from-stdin, key, plain, secret, stage, value
feedback :: bug, bug-critical, suggest
gateway
gateway query :: allow-non-select, connector-id, dry-run, execute, params-file, params-json, path, row-limit, sql, sql-file, tenant
github
github accounts
github accounts list
github installations
github installations repos :: installation-id, page, per-page
init :: args
invitations
invitations bulk :: dry-run, execute, from-file, role, strict, tenant
invitations cancel :: dry-run, execute, id, tenant, yes
invitations list :: expires-within, status, tenant
invitations resend :: dry-run, execute, id, role, tenant, yes
invitations send :: email, role, tenant
manifest :: args
members
members deactivate :: dry-run, execute, member, tenant, yes
members list :: tenant
members me :: tenant
members reactivate :: dry-run, execute, member, tenant, yes
members resolve :: email, tenant
members set-role :: dry-run, execute, member, role, tenant
open :: args, logs, metrics
profile
profile add :: default, endpoint, name
profile current
profile list
profile remove :: name
profile use :: name
publish :: args
resources
resources bulk-register :: connector-id, dry-run, execute, include-columns, items-file, items-json, tenant
resources delete :: cascade, dry-run, execute, resource_id, tenant
resources list :: parent-id, tenant
resources move :: dry-run, execute, parent-id, resource_id, root, tenant
resources namespace
resources namespace create :: dry-run, execute, name, parent-id, tenant
resources rename :: dry-run, execute, name, resource_id, tenant
resources tag-attach :: dry-run, execute, resource_id, tag-id, tenant
resources tag-detach :: dry-run, execute, resource_id, tag-id, tenant
review
review approve :: note, request_id
review get :: request_id
review history :: app, page, page-size, tenant
review list
review reject :: reason, request_id
status
support
support diagnose :: include-logs, output
tables
tables check-availability :: app, table
tables column-types :: app
tables columns
tables columns add :: app, default, dry-run, execute, name, nullable, table, type
tables columns remove :: app, dry-run, execute, name, table
tables create :: app, column, description, dry-run, execute, owner-column, schema, table
tables drop :: app, confirm, dry-run, execute, force, table
tables get :: app, table
tables grants
tables grants issue :: actions, app, dry-run, execute, principal-id, principal-type, table
tables grants list :: app, table
tables grants revoke :: app, dry-run, execute, grant-id, table
tables list :: all, app, page, per-page
tables rows :: app, page, per-page, table
tenants
tenants create :: name, slug
tenants delete :: dry-run, execute, slug_or_id, yes
tenants get :: slug_or_id, tenant
tenants icon
tenants icon clear :: dry-run, execute, tenant
tenants icon set :: file, icon-url, tenant
tenants icon sign :: content-type, tenant
tenants list :: all
tenants update :: description, dry-run, execute, name, slug_or_id
tenants whoami :: tenant
update
update apply :: dry-run, execute, force, yes
update check
whatsnew
```

## Dynamic trailing-var command audit

Some public commands use `trailing_var_arg`, so schema extraction intentionally collapses them to `:: args`. These were manually audited in Rust source:

| Root | Source | Real subcommands / flags |
|---|---|---|
| `access` | `access.rs` `run_backend` | `access check --app <id>`, `access grant --app <id>`, `access revoke --app <id> --execute`, `access invite --app <id> --user <uuid> --execute`, `access uninvite --app <id> --user <uuid> --execute` |

## Subcommand classification addendum

These public subcommands were easy to miss because top-level command rows hide them. They are explicitly classified here.

| Subcommand | Classification | Owner/Disposition |
|---|---|---|
| `apps bootstrap` | existing covered, needs R0 recheck | `init` skill saga |
| `apps bootstrap-status` | existing covered, needs R0 recheck | `init` skill watch/resume path |
| `apps owned` | existing read | `apps` / `my-resources` inventory |
| `apps workspace` | existing read | `apps` / `my-resources` inventory |
| `deploy doctor` | read diagnostic | `inspect` + deploy/doctor boundary |
| `deploy fleet` | defer/operator | no skill until fleet consent model exists |

## Existing axhub skill coverage inventory

Current `skills/` contains `_template` plus 32 active skill directories:

```text
apps auth axhub-debug axhub-diagnose axhub-plan axhub-review axhub-ship axhub-tdd
clarify data deploy doctor enable-statusline env github init install-cli
karpathy-guidelines logs migrate my-resources open profile recover routing-stats
setup status trace update upgrade using-axhub-quality verify
```

Slash-command docs currently exist only for:

```text
commands/apps.md
commands/deploy.md
commands/doctor.md
commands/help.md
commands/login.md
commands/logs.md
commands/status.md
commands/update.md
commands/배포.md
```

Implication: many existing skills are NL-routed only, and new skill work may need slash docs only when user-facing slash coverage is desired.
