# axhub Plugin — Open Questions

Append-only log of unresolved decisions across plans.

## bug-hunt-systematic - 2026-04-23

- [ ] Q1: Drop `allowed-tools` from `deploy/SKILL.md`, or add to all 11 skills? — PLAN row 53 says drop (Phase 6 Validator HIGH); current state is split (1 of 11 has it). Need confirmation that Claude Code's loader actually ignores it (or rejects it).
- [ ] Q2: Populate `references/` directories now (extract content from `src/axhub-helpers/catalog.ts` + PLAN.md §16), or strip `references/X.md` mentions from all 11 SKILL.md bodies? — Affects ~3 days of work either direction. Currently 11 skills reference 5+ files that do not exist on disk.
- [ ] Q3: Commit prebuilt multi-arch binaries (cosign-signed per PLAN §16.9) for the 5 platforms (`darwin-arm64/amd64`, `linux-amd64/arm64`, `windows-amd64`), or rely on a SessionStart fallback that runs `bun run build` if `bin/axhub-helpers` is missing AND bun is on PATH? — Spec says option 1; option 2 is faster but breaks for non-bun users.
- [ ] Q4: Is `axhub auth login` actually destructive enough to require an HMAC consent token? — `parseAxhubCommand` flags it as such (action `auth_login`), but OAuth Device Flow is itself a consent surface. If we keep it, the auth skill's flow MUST mint a token before the bash call. Currently it does NOT, which means PreToolUse denies every login. Decide: remove `auth_login` from destructive enum, OR add `consent-mint --action auth_login` step to auth SKILL.md workflow.
- [ ] Q5: `deploy_logs_kill` action gate is unreachable in v0.1.0 CLI — there is no `--kill` flag; process kill happens via signal from outside. Re-implement against actual signal-kill mechanism (likely a Stop hook on the bash subprocess), or remove from destructive enum entirely?

## bug-hunt-systematic — Analyst-deferred items (none yet, pre-architect-review)

(Will be appended after analyst gap-analysis pass on this plan.)
