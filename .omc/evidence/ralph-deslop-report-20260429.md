# AI slop cleanup report — Ralph Rust-primary cutover

Scope: changed files from this Ralph session only.

Behavior lock before cleanup:
- Rust prompt-route/CLI behavior: `cargo test -p axhub-helpers --test cli_e2e`
- TS prompt-route parity: `bun test tests/axhub-helpers.test.ts`
- Full regression gates: see `.omc/evidence/ralph-rust-primary-cutover-20260429.md`

Cleanup plan:
1. Dead code deletion: inspect generated build artifacts and remove accidental `target/` diffs from the worktree.
2. Duplicate/noisy evidence: keep explicit evidence files but centralize the final proof in one markdown summary.
3. Naming/error handling cleanup: ensure prompt-route contexts state the exact skill and prevent deploy/release ambiguity.
4. Test reinforcement: lock all prompt-route skill contexts and harden T1 interactive budgets to 90s.

Passes completed:
1. Dead code deletion — cleaned `target/` and `fuzz/target` generated artifacts from git status.
2. Duplicate removal — no code-level duplicate safe to remove without widening scope; TS and Rust prompt-route tables intentionally mirror runtime/fallback parity.
3. Naming/error handling cleanup — deploy routing context now explicitly says not to use repository release workflow, `bun run release`, or git tag flow.
4. Test reinforcement — TS and Rust prompt-route tests cover deploy/apps/apis/auth/logs/status/recover/update/upgrade/clarify plus no-route, and T1 case budgets were aligned to 90s.

Quality gates:
- Regression tests: PASS
- Lint: PASS
- Typecheck: PASS
- Static/security scan: PASS

Remaining risks:
- TS/Rust prompt-route route tables are duplicated by design until TS fallback deletion.
- External staging token, Windows V3/AhnLab cohort, and 24h fuzz remain outside this session.
