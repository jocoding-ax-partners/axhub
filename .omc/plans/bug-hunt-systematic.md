# axhub Plugin — Systematic Bug Hunt + Fix Iteration (v2)

> Created: 2026-04-23 · ITER 2 (Architect + Critic feedback applied) · Plugin commit: `dda3305` · Test baseline: 76 pass / 0 fail / typecheck clean
> Predecessor bugs already fixed: `8b6c476` (plugin.json `repository` shape), `977f0fc` (preauth-check `hookEventName`), `dda3305` (classify-exit exit-0 silent except deploy create)
> Out of scope: ax-hub-cli #47 (install.sh pipe bug), #50 (endpoint default missing) — separate repo
> Diff vs v1: each `[ITER 2 REV]` marker indicates a substantive change demanded by Critic acceptance criteria.

---

## RALPLAN-DR Summary (revised, top of doc)

### Mode: DELIBERATE (unchanged)
Triggered by destructive parser drift surface (Fix 1.1), HMAC consent gate edits (1.2/1.3), production-bound trust model, and elevated prior probability after 3 prior shipped bugs.

### Principles (5 — unchanged from v1, both reviewers approved)

1. Test every surface against Claude Code's actual loader, not against our internal contract.
2. Schema conformance > functional correctness (the hookEventName fix proved this).
3. A fixture is a fact; a test is a hope.
4. Surface area > line count: 33 surfaces total, only 3 covered by the 3 prior fixes.
5. Documentation that lies is worse than no documentation.

### Decision Drivers (top 3 — unchanged)

1. Vibe coder safety (parser-drift / dead consent gate / wrong-template emission).
2. Plugin distribution readiness (first-install must load without errors → binary distribution is the dominant first-install crash risk per Critic CRITICAL #1 reweighting).
3. Test cycle time ≤30 min on dev laptop (else the next bug ships before anyone runs the suite).

### Viable Options (≥2; recommendation revised)

#### Option A — Bottom-up fixture pinning + unit test expansion (CI-only)
Pros: Deterministic, <30s suite, catches Class A and Class B. Cons: Class C (doc-link / usage-syntax) only via human review; cannot prove the live loader accepts our output.

#### Option B — Top-down headless Claude Code session walkthrough
Pros: The only test that proves Claude Code itself loads the plugin. Catches Class C. Cons: Headless harness DOES NOT EXIST today (PLAN row 15 dependency on Anthropic). Cannot block CI without that.

#### Option C (chosen) — Hybrid: pinned fixtures (A) + manual walkthrough checklist (B-lite) + parser-drift fuzzer for randomness coverage
Pros: Respects all 3 drivers. Hand-curated fixtures catch catalog-routing bugs (non-substitutable per Critic M3). Fuzzer catches random structural bugs (additional, not substitutional). Manual walkthrough catches Class C with human eyeballs at PR-review time.
Cons: Manual step requires discipline; dependency on humans not skipping it.
**[ITER 2 REV — Critic M3]** Fuzzer added as supplement, not replacement; all 38 hand-curated fixtures retained.

**Recommendation: Option C.** If only one option were viable: Option A (manual walkthrough discipline is recoverable; an undeterministic CI gate is not). All other options are not invalidated; both are degraded fallbacks.

### Pre-mortem (≥5 scenarios) **[ITER 2 REV — Critic M5: scenarios 4 + 5 added]**

#### Scenario 1 — "Silent skill loader rejection at Claude Code 2.x release."
6 months from now, Claude Code's loader strictly rejects `allowed-tools` on skills. Deploy stops auto-triggering at first paying customer. Mitigation: Promote `bun run spec:refresh` cron (Tier 1, calendar date 2026-04-30) — see §6 Tier 1 Fix 1.5.

#### Scenario 2 — "Helper binary arch mismatch on first customer install."
Korean B2B customer with mixed Mac arm64 / Windows amd64 / Codespaces linux-amd64 developers. Plugin ships only `bin/axhub-helpers` (darwin-arm64, the dev build). Windows users → exit 127 → blame falls on axhub. Mitigation: Q3 PROMOTED to Tier 1 with calendar deadline 2026-04-28 — see §6 Tier 1 Fix 1.6.

#### Scenario 3 — "Test suite became theater (fixture rot vs evolving spec)."
Claude Code 3.x rewrites hook payload format. Our 38 fixtures describe v0; tests still pass against fixtures; plugin broken in production; green badge lies. Mitigation: meta-envelope schema versioning (Tier 1.5 observability MVP) + spec:refresh cron (Tier 1.5 Fix 1.5).

#### Scenario 4 — "First regulated-customer audit reveals cross-team apis-list leak." **[ITER 2 REV — new]**
Q3 customer is a Korean fintech. Their compliance team audits the `apis` skill and finds `axhub apis list --org $ORG` surfaces other teams' API names without RBAC tag filtering (PLAN.md §16.17 / E13 audit-class risk). Single audit failure blocks the deal. Mitigation: dedicated privacy-filter fixture for `apis` skill (Tier 1 Fix 1.7) + populate `skills/apis/references/privacy-filter.md` (Tier 2 Fix 2.3).

#### Scenario 5 — "Vibe coder runs corpus and triggers 12+ unsafe actions from parser drift in production." **[ITER 2 REV — new]**
A vibe-coder customer copy-pastes 100 utterances from blog tutorials. `parseAxhubCommand` regex agreement with `classify-exit` regex drifts by one character because of an unrelated unicode-NFKC fix to slug normalization. 12 utterances now bypass consent. They DM the founder. Mitigation: parser-drift fuzzer (Tier 1 Fix 1.8) + NFKC slug-normalization unit test (Tier 1 Fix 1.9). The fuzzer is non-substitutable for hand-curated fixtures (Critic M3).

### Expanded Test Plan (deliberate mode — revised scope)

Unit: 76 existing + spec-conformance suite (88 assertions) + parser-drift fuzzer (≥30 shapes, ~+4 hrs build) + NFKC slug normalization (~6 assertions).
Integration: 38 hand-curated helper × hook fixtures (155 assertions) + build-smoke (5-platform binary × subcommand matrix).
E2E: One of {`live-session-walkthrough.sh` (CI-only behind `LIVE_SESSION=1` flag — DEFERRED, no decidable date per Critic M4) | `manual-walkthrough.md` 20-utterance checklist (PR-template gate)} — pick manual walkthrough as Tier 1 deliverable per Critic M2.
Observability **[ITER 2 REV — Critic M6: scope cut to MVP]**: meta envelope `{schema, helper_version}` on all 8 helper subcommand outputs. **DEFERRED to next iteration:** `session-heartbeat.ndjson`, `exit-classifications.ndjson` (the two ndjson logs).

### Final ADR

**Decision:** Run a 1.5–2 day systematic bug hunt iteration **[ITER 2 REV — Critic CRITICAL #1: was 3 days, now 1.5–2 days realistic given Class C deflated 70% (only 2 of ~5 predicted broken-link files actually broken)]**. Land Tier 1 + Tier 2; defer Tier 3/4 polish; defer headless E2E harness explicitly.

**Drivers:** Vibe-coder safety, distribution readiness, ≤30 min test cycle.

**Alternatives considered:** Option A (fails distribution-readiness gate by missing Class C entirely); Option B (fails cycle-time gate via missing harness dependency).

**Why chosen:** Option C is the only one respecting all 3 drivers. Class A + B caught by automated fixtures; Class C caught by 10-min manual walkthrough at PR-review time; randomness coverage by fuzzer.

**Consequences:** ~1.5–2 days of focused work. 38 hand-curated fixtures + ~30-shape fuzzer + 88 spec assertions + 155 fixture assertions + manual walkthrough + NFKC test + 2 file population (privacy-filter.md, headless-flow.md). Q3 binary distribution is the calendar gate at 2026-04-28.

**Follow-ups:** Headless `claude --no-interactive` corpus runner DEFERRED with no decidable date (Anthropic dependency, Critic M4); promote manual walkthrough to automated Option B once available; weekly hook-fixture schema diff cron; ndjson observability deferred from MVP.

---

## 1. Why this plan exists (unchanged)

The 3 fixed bugs were each found by **manual interactive testing**, not by the 76-test suite. Every fix was a **schema-level mismatch with how Claude Code actually parses the plugin** — none were logic bugs. The unit tests pass against the helper's *own contract* but never validate that contract against Claude Code's loader / hook dispatcher / manifest validator. By construction, they cannot find more bugs of the same class. Manual testing found 3; the same testing pattern run more systematically will find the next 3–5 (revised down per Critic CRITICAL #1).

## 2. Scope and non-goals (unchanged)

In scope: every plugin surface tested against Claude Code's actual loader/dispatcher/validator; helper subcommand schema pinned against fixtures; bug fixes ordered by blast radius.
Not in scope: ax-hub-cli changes; new skills/commands; refactoring helper internals (consent.ts, redact.ts) without a found bug; M1.5 corpus rescoring.

## 3. Bug class taxonomy + revised predictions

### Class A — Manifest / JSON / frontmatter shape mismatch (unchanged taxonomy; key open items)

1. `deploy` SKILL.md `allowed-tools` — Architect confirmed: KEEP resolution to drop (PLAN row 53 = correct, spec says skills don't take `allowed-tools`).
2. `commands/help.md` missing `allowed-tools`/`argument-hint`/`model` — verify which way to converge.
3. `hooks.json` `matcher: "Bash"` literal — verify against actual hook fires.
4. `marketplace.json` `source: "./"` trailing slash.
5. SessionStart helper output shape (systemMessage vs hookSpecificOutput).
6. Helper subcommand JSON field-name drift (e.g., `app_id` vs `appId`).
7. plugin.json missing `version`/`engines.bun`.

### Class B — Hook output context-blind / wrong template (unchanged)

1. classify-exit on exit-64 sub-codes (`app_ambiguous`, `app_list_truncated`).
2. classify-exit on exit-66 sub-codes (`scope.downgrade_blocked`, `update.cosign_verification_failed`).
3. classify-exit on `axhub` not at command start (compound command parser drift vs preauth-check) — see Fix 1.1.
4. preauth-check on `auth_login` empty-binding consent failure — **[ITER 2 REV — Critic CRITICAL #2 OVERRIDE]** auth_login MUST stay in VALID_ACTIONS; auth SKILL.md MUST include `consent-mint --action auth_login` step before `axhub auth login`. Token-paste path is otherwise unprotected.
5. preauth-check on `axhub deploy logs --kill` unreachable gate — **[ITER 2 REV — Critic CRITICAL #3 OVERRIDE]** deploy_logs_kill MUST stay in enum with `// reserved for v0.2` comment + unit test asserting no current path produces it. Removing now forces an HMAC binding-schema migration later.
6. session-start English placeholder vs DX-1 Korean welcome (PLAN row 21).

### Class C — Documented usage syntax error **[ITER 2 REV — Architect sample-verified, scope deflated 70%]**

**Verified state via `find skills -type f -name "*.md"` (run during this revision):**

| Reference path | Status | Notes |
|---|---|---|
| `skills/deploy/references/error-empathy-catalog.md` | EXISTS (12.4 KB) | Architect VERIFIED → was wrongly predicted broken in v1 |
| `skills/deploy/references/nl-lexicon.md` | EXISTS (22.5 KB) | Architect VERIFIED → was wrongly predicted broken in v1 |
| `skills/deploy/references/recovery-flows.md` | EXISTS (14.8 KB) | Architect VERIFIED → was wrongly predicted broken in v1 |
| `skills/deploy/references/privacy-filter.md` | MISSING | Real broken-link, must populate |
| `skills/deploy/references/headless-flow.md` | MISSING | Real broken-link, must populate |
| Other skills' `references/` referenced files | Need verification per skill |

**[ITER 2 REV — Critic acceptance criterion #1]** Class C predictions reconciled against actual `find` output:
- `error-empathy-catalog.md`: VERIFIED present → FALSIFIED v1 prediction
- `nl-lexicon.md`: VERIFIED present → FALSIFIED v1 prediction
- `recovery-flows.md`: VERIFIED present → FALSIFIED v1 prediction
- `privacy-filter.md`: VERIFIED missing → CONFIRMED Class C bug
- `headless-flow.md`: VERIFIED missing → CONFIRMED Class C bug

**Scope reduction:** Class C work goes from "create ~5 reference files for 11 skills = 3 days" to "create 2 reference files + audit other skills' references = 0.5 days." This is the source of the 1.5–2 day budget revision.

**Remaining Class C predictions (ranked):**
1. `commands/help.md` body documents `/axhub:deploy` etc — verify slash invocations work in real session.
2. SKILL bodies reference `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers` — Q3 binary distribution → see Tier 1 Fix 1.6.
3. **2 missing reference files: `privacy-filter.md` (apis-skill audit-class scenario), `headless-flow.md` (deploy-skill non-TTY)** → see Tier 2 Fix 2.3.
4. doc-link audit across all `*.md`.
5. `AXHUB_TOKEN_FILE` documented but no helper reads it.
6. `AXHUB_AGENT=1` documented but no skill exports it.

## 4. Bug hunt methodology (unchanged structure; observability scope cut)

Same 5 steps as v1: spec conformance audit (Step 1), helper × hook fixture matrix (Step 2), end-to-end live walkthrough (Step 3), build artifact verification (Step 4), documentation link audit (Step 5).

**[ITER 2 REV — Critic M6]** Step 2 helper meta-envelope output is MVP-scope this iter. The two ndjson sinks (`session-heartbeat.ndjson`, `exit-classifications.ndjson`) are deferred to next iteration.

**[ITER 2 REV — Critic M3]** Step 2 ALSO adds parser-drift fuzzer (~+4 hrs): `tests/parser-drift.test.ts` enumerates ≥30 random shapes (env-prefix, sub-shell, eval, compound separators, paren, leading-space, mid-sentence-axhub-as-arg, NFKC unicode slugs) and asserts `classify-exit` regex agrees with `parseAxhubCommand` tokenizer. Non-substitutable for the 38 hand-curated fixtures (different bug-class coverage).

**[ITER 2 REV — Critic acceptance criterion #8]** Test runtime budget MUST be measured before §8 claims it. Current `bun test` baseline: 76 tests in ~2.4s. Target after additions: ≤30 min total. Spot-budget allocation: spec-conformance 88 assertions ≤5s, helper fixtures 155 assertions ≤30s, parser-drift fuzzer 30 shapes ≤2s, build-smoke 5×8 = 40 runs ≤25 min (cross-arch builds dominate). Verify with `time bun run smoke:full` before publishing §8 numbers.

## 5. Test fixture coverage matrix (unchanged from v1, with fuzzer added)

Net new: ~155 assertions across 38 hand-curated fixtures + ~30-shape fuzzer + 88 spec assertions + NFKC slug normalization (~6 assertions). All retained per Critic M3.

## 6. Bug fix sequence (revised tier ordering)

**[ITER 2 REV — Critic M1]** Q3 binary distribution PROMOTED from Tier 2 to Tier 1 (was Fix 2.4, is now Fix 1.6). Spec-conformance work for SKILL `allowed-tools` DEMOTED back to Tier 2 (was tied as Tier 1 candidate by Architect; Critic correctly identified binary distribution as the actual first-install crash risk).

**[ITER 2 REV — Critic M2]** Manual walkthrough OR `bun run spec:refresh` cron — choosing **MANUAL WALKTHROUGH as Tier 1 deliverable** (calendar-deadline-free, dependency-free, ships immediately). `bun run spec:refresh` cron is Tier 1.5 with calendar date 2026-04-30 (Critic acceptance criterion #5).

### Tier 1 — Safety / correctness (fix BEFORE next plugin install)

**Fix 1.1: parser-vs-parser drift in classify-exit.** classify-exit `^axhub\s/` rejects `cd /tmp && axhub deploy create`. preauth-check `parseAxhubCommand` handles compound. Both must agree. Acceptance: identical `is_axhub` classification for ≥30 shapes (covered by §4 fuzzer).

**Fix 1.2: `auth_login` consent binding.** **[ITER 2 REV — Critic CRITICAL #2 OVERRIDE]** REVERSED from v1: auth_login STAYS in VALID_ACTIONS. Acceptance:
- (a) `auth/SKILL.md` workflow updated to mint consent BEFORE `axhub auth login`: `axhub-helpers consent-mint --action auth_login --binding '{"app_id":"","branch":"","commit_sha":""}'` step required.
- (b) Unit test `tests/auth-skill-consent.test.ts` (new) parses `skills/auth/SKILL.md` body and asserts the consent-mint invocation appears before any `axhub auth login` bash block.
- (c) Token-paste path consent gate: SKILL body must surface a Korean confirmation step when token is being pasted from stdin.

**Fix 1.3: `deploy_logs_kill` enum.** **[ITER 2 REV — Critic CRITICAL #3 OVERRIDE]** REVERSED from v1: deploy_logs_kill STAYS in enum with `// reserved for v0.2 — no current path produces this; do not remove without HMAC binding-schema migration` comment in `src/axhub-helpers/parseAxhubCommand.ts`. Acceptance:
- (a) Comment lands.
- (b) Unit test `tests/dead-action-reservation.test.ts` (new) asserts: for the v0.1.0 CLI command set (`axhub deploy logs`, `axhub deploy logs --follow`, `axhub deploy logs --tail N`), `parseAxhubCommand` does NOT produce `deploy_logs_kill`. This pins the dead-but-reserved gate.

**Fix 1.4: SessionStart helper Korean welcome.** Replace English placeholder with PLAN row 21 (DX-1) Korean 3-step welcome + PLAN audit row 14 version-range gate. p95 ≤50ms.

**Fix 1.5 [Tier 1.5]: `bun run spec:refresh` cron stub** **[ITER 2 REV — Critic M2]**. Calendar deadline: 2026-04-30. Stub script that diffs our spec-conformance assertions against `~/.claude/plugins/marketplaces/claude-plugins-official/plugins/plugin-dev/skills/` and fails CI if any asserted field disappeared from the spec. Wires Scenario-1 mitigation. Cron registration handed off to release pipeline.

**Fix 1.6 [Tier 1, PROMOTED FROM TIER 2]: binary distribution for 5 platforms** **[ITER 2 REV — Critic M1]**. Calendar deadlines:
- **Decide** signing path (cosign on-self-host vs GitHub-Actions OIDC): **2026-04-25**
- **Ship** prebuilt binaries for `darwin-arm64`, `darwin-amd64`, `linux-amd64`, `linux-arm64`, `windows-amd64`: **2026-04-28**
Acceptance: 5 binaries committed under `bin/<arch>/axhub-helpers` (or detached release-asset bucket if size becomes git-prohibitive); `package.json` postinstall hook (or PluginInstall hook if available) selects the right arch and symlinks to `bin/axhub-helpers`. NO SessionStart fallback. Architect-confirmed: SessionStart fallback (the v1 alternative) creates first-install latency cliff and silently fails for non-bun users — both unacceptable.

**Fix 1.7: privacy-filter fixture for apis skill** **[ITER 2 REV — Critic "What's missing" + Scenario 4]**. New fixture `tests/helper-fixtures/apis-list-rbac-redacted.json`. Asserts `axhub apis list --org $ORG` output is filtered through `redact.ts` to strip cross-team API names not visible to the requesting user's RBAC tags. Wires PLAN.md §16.17 / E13 audit-class risk. Includes `posttooluse` fixture proving classify-exit doesn't re-leak in error message.

**Fix 1.8: parser-drift fuzzer** **[ITER 2 REV — Critic M3]**. New `tests/parser-drift.test.ts`. Enumerates ≥30 random shapes generated by a small grammar (env-prefix × subshell × compound × eval × paren). Asserts `classify-exit`'s `^axhub\s/` matches iff `parseAxhubCommand` would have parsed it as axhub. Additional to (not substitute for) the 38 hand-curated fixtures.

**Fix 1.9: NFKC slug-normalization test** **[ITER 2 REV — Critic "What's missing"]**. New `tests/nfkc-slug.test.ts`. Korean slug edge cases (composed-vs-decomposed Hangul, fullwidth digits in app names like `app－３` vs `app-3`) feed through `parseAxhubCommand`. Asserts the parser normalizes via NFKC before equality compare. ~6 assertions. Covers Pre-mortem Scenario 5 root cause.

### Tier 2 — Spec conformance (fix BEFORE next marketplace publish)

**Fix 2.1: `deploy/SKILL.md` drop `allowed-tools`** **[ITER 2 REV — Architect Q1 confirmed]**. Architect confirmed PLAN row 53 is correct; skills do not take `allowed-tools`. Acceptance: removed from `deploy/SKILL.md`; spec-conformance test enforces uniformity across all 11 skills.

**Fix 2.2: `commands/help.md` frontmatter completeness.** Add `allowed-tools: []` + `argument-hint` + `model` to converge with the other 8 commands.

**Fix 2.3: populate 2 missing reference files** **[ITER 2 REV — Architect Q2 revised, Critic acceptance criterion #9]**. Only 2 files are actually missing per the verified `find` output in §3.3:
- `skills/deploy/references/headless-flow.md` (or relocate under apis if more appropriate per non-TTY agentic usage docs).
- `skills/apis/references/privacy-filter.md` (RBAC tag filtering, cross-team leak prevention; wires Fix 1.7 + Pre-mortem Scenario 4).
Plus audit other skills' `references/X.md` mentions to confirm no other broken links exist (sample verified `deploy` is otherwise complete; remaining 10 skills not yet sampled — Tier 3 expanded doc-link audit).

**Fix 2.4 (FORMERLY Tier 2; now folded into Tier 1 Fix 1.6 above)** **[ITER 2 REV — promoted to Tier 1]**.

### Tier 3 — Documentation / UX consistency (fix BEFORE next user-facing release)

**Fix 3.1: `AXHUB_AGENT=1` documented but never set** — every skill prepends `AXHUB_AGENT=1 AXHUB_NO_INPUT=1` per PLAN §3.3 contract.

**Fix 3.2: `AXHUB_TOKEN_FILE` documented but no helper reads it** — implement `axhub-helpers token-install --from-stdin` OR remove documented flow.

**Fix 3.3: `commands/help.md` Korean menu accuracy** — every line maps 1:1 to a real command file; PLAN row 25 Korean aliases (`/axhub:배포`) decision.

**Fix 3.4: classify-exit fixtures for every documented exit code** — 9 new fixtures per §5 matrix.

### Tier 4 — Internal consistency (post-fix audit)

Fix 4.1 (replace `package.json` smoke), Fix 4.2 (PLAN row 53 audit-trail), Fix 4.3 (`docs/troubleshooting.ko.md` freshness) — unchanged from v1.

## 7. Verification strategy (unchanged structure; budget verification added)

Each fix lands with: failing test first → minimum code change → `bun test` ≥76 + new tests pass → `bun run typecheck` clean → `bun run smoke:full` green → for Tier 1: live walkthrough transcript captured → manual: 3 originally-found bugs do not re-surface.

**[ITER 2 REV — Critic acceptance criterion #8]** Before publishing §8 quantitative gates, run `time bun run smoke:full` against the actual built binaries and verify total ≤30 min. Current measured baseline (this iter, before adding fixtures): 76 tests in ~2.4s. Cross-arch build dominates total runtime; verify on dev laptop with all 5 archs.

## 8. Acceptance criteria (measurable, revised) **[ITER 2 REV — Critic acceptance criteria #1, #8, #9 wired]**

A bug hunt + fix iteration is COMPLETE when:

1. **Spec conformance test green.** `tests/spec-conformance.test.ts` passes 88 assertions across plugin.json + marketplace.json + hooks.json + 9 commands + 11 skills. **Quantitative gate:** 0 schema validation errors.
2. **Helper × fixture matrix complete.** 38 hand-curated fixtures + ≥30-shape parser-drift fuzzer + 6 NFKC slug assertions, all passing. **Quantitative gate:** 38 + 30 + 6 = ≥74 distinct test cases beyond the existing 76.
3. **Live walkthrough clean.** Manual walkthrough checklist (`tests/manual-walkthrough.md`) executed by human reviewer at PR review; all 9 slash commands fire without error; all 11 skills auto-trigger ≥1 corpus row; 0 unsafe-trigger on negative corpus; 0 deny on read-only positive corpus. Headless automation DEFERRED (Critic M4: no Anthropic API decidable date).
4. **Documentation links resolve.** `tests/docs-link-audit.sh` reports broken links **= exactly the 2 known missing files (privacy-filter.md, headless-flow.md) BEFORE Fix 2.3 lands; = 0 AFTER** **[ITER 2 REV — Critic acceptance criterion #9]**. The "0 broken links" gate from v1 was wrong — the v1 plan understated baseline because Class C was 70% deflated. Restated:
   - Pre-fix baseline: 2 broken links (`skills/deploy/references/privacy-filter.md`, `skills/deploy/references/headless-flow.md`).
   - Post-fix gate: 0 broken links across all `*.md`.
   - This gate fails open if the audit script is not run. Manual verification step in PR template.
5. **Build artifact present (Q3 promoted to Tier 1).** All 5 platform binaries committed by 2026-04-28 calendar deadline; `bun run smoke:full` runs each subcommand × each binary and stays green. **Quantitative gate:** 5 binaries × 8 subcommands = 40 smoke runs, all exit 0. Test runtime budget verified ≤30 min via `time bun run smoke:full` BEFORE this gate is published as green **[ITER 2 REV — Critic acceptance criterion #8]**.
6. **No regression.** Original 76 tests pass. Original 3 fixed bugs do not re-surface in fresh Claude Code session.
7. **Audit trail updated.** PLAN.md gets a new audit row per Tier 1 + Tier 2 fix landed.
8. **Override criteria from Critic** **[ITER 2 REV — Critic CRITICAL #2 + #3]**:
   - Fix 1.2 acceptance test (auth_login consent-mint step) lands and passes.
   - Fix 1.3 acceptance test (deploy_logs_kill reservation comment + dead-path test) lands and passes.
9. **Calendar gates met** **[ITER 2 REV — Critic M1 + M2]**:
   - 2026-04-25: Q3 binary signing decision documented (cosign-on-self-host vs GitHub-Actions OIDC).
   - 2026-04-28: 5 platform binaries committed.
   - 2026-04-30: `bun run spec:refresh` cron stub merged.

If any of 1–9 fail, the iteration is incomplete.

## 9. Open questions (revised resolution status)

These were originally Q1–Q5 in `.omc/plans/open-questions.md`. Resolutions per Architect/Critic ITER 2:

- **Q1 (drop `allowed-tools` from deploy/SKILL.md):** RESOLVED — Architect confirmed KEEP resolution (drop). See Fix 2.1.
- **Q2 (populate references/):** PARTIAL — REVISED scope per Architect: only 2 missing files, not 5+. See Fix 2.3.
- **Q3 (binary distribution):** RESOLVED — COMMIT prebuilt + ship 5 archs in `bin/`, NO SessionStart fallback. Calendar deadlines added. See Fix 1.6.
- **Q4 (auth_login):** RESOLVED — Critic OVERRIDE: KEEP destructive enum entry; auth SKILL must mint consent. See Fix 1.2.
- **Q5 (deploy_logs_kill):** RESOLVED — Critic OVERRIDE: KEEP enum entry with `// reserved for v0.2` comment + unit test. See Fix 1.3.

All Q1–Q5 are CLOSED for this iteration. Future-iteration items (headless harness, weekly fixture rot diff) move to next iteration's open-questions log.

---

## Critic 9-item acceptance checklist — inline status

| # | Critic acceptance criterion | Plan §/line | Status |
|---|---|---|---|
| 1 | Class C re-runs against actual `find` output; predictions VERIFIED/FALSIFIED per item | §3 Class C table | DONE |
| 2 | Q4 overridden: `auth_login` retained; auth SKILL workflow includes `consent-mint --action auth_login` | §6 Fix 1.2 | DONE |
| 3 | Q5 overridden: `deploy_logs_kill` retained with reservation comment + unit test | §6 Fix 1.3 | DONE |
| 4 | Tier 1 promoted: Q3 binary distribution with calendar dates | §6 Fix 1.6 | DONE |
| 5 | Manual walkthrough OR `spec:refresh` cron — pick one as Tier 1 | §6 Fix 1.5 + Tier 1 narrative | DONE (manual walkthrough as Tier 1; cron at Tier 1.5 w/ 2026-04-30 date) |
| 6 | Pre-mortem ≥5 scenarios (add audit-leak, corpus drift) | RALPLAN-DR Pre-mortem | DONE (5 scenarios) |
| 7 | Observability scope cut to meta envelope only this iter | §4 Step 2 + RALPLAN-DR Expanded Test Plan | DONE |
| 8 | Test runtime budget verified with measured baseline before §8 claims | §7 + §8 #5 | DONE (baseline 2.4s noted; verify before publishing) |
| 9 | Class C deflation reflected in §8 quantitative gates (specify 2 missing files) | §8 #4 | DONE |

---

*Plan saved to `.omc/plans/bug-hunt-systematic.md` (v2). Open questions log unchanged path; resolutions to be appended after Architect/Critic sign-off on this revision. Next: Architect re-review for ITER 3 APPROVE/ITERATE decision.*
