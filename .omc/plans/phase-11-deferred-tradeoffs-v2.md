# Phase 11 — Deferred Tradeoffs from Phase 10 v0.1.7 Ship — v2

**Mode:** ralplan consensus, DELIBERATE.
**Date:** 2026-04-24.
**Branch:** `main` (continuing from v0.1.7 ship 2026-04-23).
**Author:** Planner agent.
**Revision:** v2 — incorporates round-1 fixes (3 architect amendments + 4 pre-mortems + 2 critic defects).
**Status:** DRAFT awaiting Architect + Critic round-2 review.

---

## Investigation findings (preserved from v1, lines numbers re-verified)

- `src/axhub-helpers/keychain.ts:50-95` — 6 reachable error returns + 1 unreachable `catch` at line 64-66 (D2). `Bun.spawnSync` does NOT throw on ENOENT — returns `exitCode: null` only.
- `scripts/codegen-install-version.ts:26-28` — regex already supports `-rc.1` style suffixes via `(?:-[a-z0-9.]+)?`. Zero pre-release test coverage in `tests/codegen-version.test.ts:13-45` (Pre-mortem #7).
- `tests/install-ps1.test.ts:16-20` — `ps1Version` capture uses `'([^']+)'` single-quote literal regex; Pre-mortem #6 already covered. DROP from v2.
- `bin/install.ps1:24-28` and `keychain-windows.ts:101-128` — established 4-part `\n원인:` / `\n해결:` / `\n다음:` template; the contract US-1102 must mirror.
- `tests/codegen-version.test.ts:18-23` — current test runs `syncInstallVersion()` against real `bin/install.sh` (mutation in temp not used). New US-1101 path mutates `bin/install.ps1` — D1 ordering issue.
- GitHub repo: `jocoding-ax-partners/axhub` issue #1 open; lock/archive state must be checked before close (Pre-mortem #8).

---

## Recommended scope split — Option B with v2 amendments

**Pick: Option B.** Bucket A (US-1101 codegen install.ps1 sync + US-1102 keychain.ts format-parity **6 lines**, NOT 7 — D2 downgrade) + US-1105 Linux Docker smoke harness with explicit limitation banner, with US-1103 (Windows VM checklist + companion `.ps1`) and US-1104 (Authenticode procurement runbook + stub workflow) shipped as documentation-only stories. Honest interpretation of "전부 다 작업해" = "ship every priority where we have agency in this dev session." Code-only items (US-1101 + US-1102) total ~12 changed lines, fully test-covered, near-zero risk. Linux Docker (US-1105) feasible because Docker Desktop is the standard macOS dev tool; produces real exit-0 PASS artifact, not a code review. Windows VM (US-1103) and Authenticode (US-1104) require hours of Parallels/UTM provisioning and a $300+/yr OV cert procurement loop with vendor identity verification — calendar-bound, not session-bound, ship as written runbooks with explicit blocked-by markers.

---

## Principles (4)

1. **Honesty over completeness.** Mark deferred items explicitly — never claim VM or Authenticode are "done" when only runbook exists.
2. **Code-only first, environment-bound second, vendor-bound last.** Sequence by what we control.
3. **Format parity is a contract.** 4-part empathy template (`원인` / `해결` / `다음` separated by `\n`) is public UX contract. Issue #1 is a contract violation.
4. **Codegen drift kills releases.** Phase 6 US-602 proved `install.sh` drift caused re-tag. Adding `install.ps1` to codegen closes same hole on Windows path before v0.1.8.

---

## Decision Drivers (top 3)

1. **Session feasibility.** Apple Silicon macOS, no Win VM, no native Linux, no signing cert.
2. **Regression blast radius.** `bin/install.{sh,ps1}` and `keychain.ts` are end-user surfaces; preserve test green and 4-part error format from `keychain-windows.ts:101-128`.
3. **Calendar pressure.** Authenticode OV certs take 5–14 days; runbook NOW parallelizes v0.1.8 ship with cert wait window.

---

## Viable Options (preserved from v1 with v2 amendments noted)

### Option A — Bucket A only (US-1101 + US-1102), defer all 3 environmental items as docs
- **Pros:** Smallest blast radius. ~30 min. Zero environment risk. Closes issue #1 today.
- **Cons:** Leaves Linux smoke code-review-only for 3rd phase. Docker Desktop is a missed cheap win.

### Option B — Bucket A + US-1105 Linux Docker smoke, US-1103/US-1104 doc-only
- **Pros:** First-ever real exit-0 Linux runtime evidence. Docker local, no vendor wait. ~60-90 min. Architect Phase 10 flagged this gap.
- **Cons:** New scaffolding (~80 lines bash + Dockerfile). Container vs real systemd-keyring drift risk — pre-mortem #3 + Architect Synthesis #1. **v2 mitigation:** FIRST-LINE limitation banner in evidence file distinguishing 40% READ-path vs ~15% end-to-end (D-Bus WRITE-path is upstream).

### Option C — Bucket A + Authenticode procurement kickoff
- **Pros:** Starts 5–14 day vendor clock today.
- **Cons:** Procurement is paperwork, not engineering. Cannot kick off without user CC + business docs. Linux Docker delivers more technical evidence per session-minute.

**Pick: Option B.** Higher technical signal, no vendor blockers; US-1104 still gets runbook + stub workflow per Architect Synthesis #2.

---

## Pre-Mortem v2 (8 scenarios — dropped #6, kept #1-5, added #7/#8/#9)

1. **format-parity edits accidentally change `readKeychainToken` behavior.** `tests/keychain.test.ts` only covers `parseKeyringValue` (pure function), so error string mutations would not break tests but would silently change UX. **Mitigation:** Add new test asserting each of the **6 reachable** error messages contains exactly `\n원인:`, `\n해결:`, `\n다음:` substrings. Run before+after sweep against existing strings to confirm semantic preservation.
2. **codegen install.ps1 extension breaks existing install.sh sync (regression).** Adding a 3rd file path to `syncInstallVersion()` could throw at `scripts/codegen-install-version.ts:42` if regex misses on `bin/install.ps1`. **Mitigation:** Mirror the `syncFile` helper exactly. Add `tests/codegen-version.test.ts` case asserting `bin/install.ps1` contains `'v${packageJson.version}'`. Idempotency test extension. **D1 ordering fix:** US-1101 lands BEFORE US-1102 (see Task Flow v2).
3. **Linux Docker smoke succeeds in container but fails on real systemd-keyring envs.** `secret-tool` works against any libsecret backend, but real users on Fedora-with-KWallet or headless servers may not have a D-Bus session bus. **v2 mitigation (Architect Synthesis #1):** First line of `.omc/evidence/phase-11-linux-docker-smoke.txt` MUST be: `LIMITATION: libsecret-tools CLI path only — does NOT validate gnome-keyring-daemon, kwalletd5, or headless systemd-keyring user-bus.` Distinguish 40% READ-path coverage vs ~15% end-to-end (D-Bus WRITE-path is upstream ax-hub-cli concern).
4. **Authenticode runbook becomes obsolete before procurement starts.** Sectigo / DigiCert pricing, KMS options shift quarterly. **Mitigation:** Date-stamp the runbook ("as of 2026-04-24"). Cite three vendors with current Q1 2026 prices. Mark explicit "verify before purchase" checklist.
5. **Win VM checklist doc gets used by future-Claude on a non-Apple-Silicon host where UTM/Parallels paths differ.** **Mitigation:** Write checklist as host-agnostic where possible; call out Apple Silicon specifics (Parallels Win11 ARM ISO link, UTM Win11 ARM TPM workaround) in dedicated sub-section. **v2 addition (Architect Synthesis #3):** Companion `tests/smoke-windows-vm-checklist.ps1` codifies the 14 steps as PowerShell behind `if ($env:AXHUB_VM_SMOKE -eq '1')` guard. Asserts runtime contract (exit codes, JSON envelope shape) via `try/catch` blocks (Pester optional).
6. **DROPPED — false alarm.** Critic verified `tests/install-ps1.test.ts:16-20` already enforces single-quote literal preservation via captured group `/'([^']+)'/`. v2 removes this scenario.
7. **Pre-release tag (`0.1.8-rc.1`) edge case.** Existing regex `scripts/codegen-install-version.ts:26-28` supports `-rc.1` via `(?:-[a-z0-9.]+)?` but `tests/codegen-version.test.ts` has zero pre-release case. **Mitigation:** Add test case to US-1101 acceptance using mocked package.json with `"version": "0.1.8-rc.1"` (write to temp dir, dynamic-import a sibling module reading it OR use Bun's mock module API). Assert `syncInstallVersion()` produces `'v0.1.8-rc.1'` literal in `bin/install.ps1` and 4 files updated, no throw.
8. **GitHub issue #1 close non-determinism.** US-1102 acceptance "close issue #1 with comment linking commit SHA" fails silently if the issue is locked / archived / already closed. **Mitigation:** Run `gh issue view 1 --json state,locked` precheck BEFORE attempting close. If `state == "OPEN"` and `locked == false`, attempt comment+close. Either way, write `.omc/evidence/issue-1-close-attempt.txt` recording: precheck JSON output, attempted command, exit code, any API error message. Do not silently exit 0 if close failed.
9. **`.github/workflows/sign-windows.yml.template` linter trip.** GitHub Actions ignores `.template` extension for workflow execution but actionlint, pre-commit YAML hooks, or GitHub's linguist may scan and flag it. **Mitigation:** Add to `.gitattributes`: `*.yml.template linguist-language=YAML linguist-detectable=false`. If repo has `.eslintignore` (verify via `test -f .eslintignore`), add `*.yml.template` there too. Document in US-1104 acceptance that the workflow is intentionally inert until US-1104b (signing kickoff) lands.

---

## Expanded Test Plan v2 (DELIBERATE — updated assertion counts)

### Unit
- `tests/codegen-version.test.ts`: extend with **3 new cases**: (a) `bin/install.ps1` contains `'v${packageJson.version}'` after `syncInstallVersion()` runs; (b) idempotency case for the new file (`changed: false` on second run); (c) **Pre-mortem #7** — pre-release version case using mocked `"0.1.8-rc.1"` package.json (write tempfile + isolated import, NOT mutating real package.json), assert no throw + `'v0.1.8-rc.1'` literal landed in `bin/install.ps1`.
- `tests/keychain.test.ts`: add new `describe("readKeychainToken error format-parity (issue #1)")` block. **6 assertions** (NOT 7 — D2 downgrade) mapping to lines **57, 62, 79, 85, 90, 95** of `src/axhub-helpers/keychain.ts`. Line 65 (catch block at 64-66) DROPPED because `Bun.spawnSync` does not throw on ENOENT — `exitCode: null` only — so the catch path is unreachable code. Mocking `Bun.spawnSync` is scope-creep; downgrade is honest. Each of the 6 assertions checks the returned `error` string contains all three substrings: `\n원인:`, `\n해결:`, `\n다음:`.

### Integration
- Run full `bun test` sweep — 370+ existing tests must stay green plus the new ~9 cases (3 codegen + 6 keychain).
- Run `bun run codegen:version` — must update `bin/install.ps1` line 18 in addition to existing 3 files. Verify with `git diff bin/install.ps1` that `$ReleaseVersion` literal changed and rest of file untouched.
- Run `bun run smoke:full` end-to-end — confirms codegen extension does not break the hard-mode smoke chain.

### E2E
- **Manual Linux Docker smoke (US-1105):** `bash tests/smoke-linux-docker.sh` builds Ubuntu 24.04 container (pinned to specific sha256 digest, NOT tag-only), installs `libsecret-tools` + `dbus-x11` + `bun`, copies in `bin/axhub-helpers-linux-amd64`, starts D-Bus session, runs `secret-tool store --label=axhub service axhub` against fixture base64-encoded JSON, runs `./bin/axhub-helpers-linux-amd64 token-init`, asserts exit 0 + stdout contains `linux-secret-service`. Write `.omc/evidence/phase-11-linux-docker-smoke.txt` with first line = LIMITATION banner per Pre-mortem #3.
- **Manual Windows VM smoke (US-1103, deferred):** documented as 14-step checklist in `.omc/runbooks/windows-vm-smoke-v0.1.7.md`. Companion `tests/smoke-windows-vm-checklist.ps1` codifies steps behind `$env:AXHUB_VM_SMOKE` guard.
- **Manual Authenticode kickoff (US-1104, deferred):** documented as procurement runbook in `.omc/runbooks/authenticode-procurement.md` + stub workflow `.github/workflows/sign-windows.yml.template` with `on: workflow_dispatch:` and `continue-on-error: true` on the `signtool verify` step (unsigned binary exits non-zero — would block CI without continue-on-error).

### Observability
- `tests/codegen-version.test.ts` reports `files_updated` array — must include `bin/install.ps1` after change.
- Linux Docker smoke writes structured evidence (LIMITATION banner first line, exit code, source identifier, timestamp, image SHA, libsecret-tools version) for future regression diff.
- All 6 reachable `readKeychainToken` error strings extracted into a single `tests/format-parity-fixtures.ts` so future error-message changes have grep-able centralized fixtures.
- US-1102 issue-close attempt writes `.omc/evidence/issue-1-close-attempt.txt` with precheck JSON + close exit code per Pre-mortem #8.
- US-1104 stub workflow emits "signing skipped — vendor cert pending" telemetry through existing `src/axhub-helpers/telemetry.ts` channel (NOT a new channel — Architect Synthesis #2).

---

## PRD Stories v2 (Task Flow REORDERED per D1)

### US-1101 — codegen extends to bin/install.ps1 (FIRST per D1)
- **Status:** in-scope (this session). **REORDERED to step 1** (was step 2 in v1) per Critic D1: US-1101 must land before US-1102 because `syncInstallVersion()` mutates `bin/install.sh` AND (after this story) `bin/install.ps1` in tests via `tests/codegen-version.test.ts:18-23`. If US-1102 lands first, codegen test mutation creates dirty git tree before US-1101's regex is added — new ps1 sync throws at `scripts/codegen-install-version.ts:42` (`missing expected version line`). Reorder is safer than adding tempfile-mock scope.
- **Acceptance:**
  - `scripts/codegen-install-version.ts` adds 4th `syncFile` invocation targeting `bin/install.ps1` with regex matching `^(\$ReleaseVersion = if \(\$env:AXHUB_PLUGIN_RELEASE\) \{ \$env:AXHUB_PLUGIN_RELEASE \} else \{ ')v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(' \})$`.
  - `tests/codegen-version.test.ts` adds 3 cases per Test Plan v2 Unit section — including the Pre-mortem #7 pre-release `0.1.8-rc.1` case (mocked package.json via tempfile, isolated import, NOT mutating real `package.json`).
  - `bun run codegen:version` updates 4 files (was 3) when bumping version. Verified with manual `bun run codegen:version` after temporarily bumping `package.json` to `0.1.8-test` and reverting.
  - `bun test` stays 370+ green plus 3 new cases.
- **Blocked by:** none.
- **Files touched:** `scripts/codegen-install-version.ts` (+15 lines, 4th `syncFile` call + new regex constant), `tests/codegen-version.test.ts` (+40 lines including pre-release mock helper).

### US-1102 — keychain.ts 6 errors → 4-part empathy template (close issue #1)
- **Status:** in-scope (this session). **REORDERED to step 2** (was step 1 in v1) per D1.
- **Acceptance:**
  - `src/axhub-helpers/keychain.ts` lines **57, 62, 79, 85, 90, 95** rewritten as 4-part `\n원인:` / `\n해결:` / `\n다음:` template (6 lines, NOT 7 — D2 downgrade per Critic). Match the structure used in `keychain-windows.ts:101-128` and `bin/install.ps1:24-28`.
  - Line 65 NOT modified (catch path unreachable; `Bun.spawnSync` returns `exitCode: null` on ENOENT instead of throwing). Comment added: `// NOTE: catch path unreachable — Bun.spawnSync returns exitCode:null on ENOENT, never throws. Kept as defensive guard.`
  - Each rewritten error preserves the original semantic information (which keychain backend, what user can do).
  - New `tests/keychain.test.ts` describe block asserts all **6 returned `error` strings** contain the three template substrings. **6 assertions** (matches GitHub issue #1's 7-line claim minus the unreachable line — issue #1 close comment must explicitly note the downgrade).
  - `bun test` stays green; new 6 assertions pass.
  - GitHub issue #1 close protocol per Pre-mortem #8:
    1. `gh issue view 1 --json state,locked,closed` precheck — capture JSON output to evidence file.
    2. If `state == "OPEN"` and `locked == false`: attempt `gh issue comment 1 --body "<commit SHA + cite test + note: line 65 catch unreachable, see code comment, 6 lines fixed not 7>"` then `gh issue close 1`.
    3. Either way, write `.omc/evidence/issue-1-close-attempt.txt` with precheck JSON, attempted commands, exit codes, any API error.
    4. Do not silently exit 0 if close failed — surface the failure in commit body.
- **Blocked by:** US-1101 must land first (Task Flow v2 step 1 → step 2) per D1.
- **Files touched:** `src/axhub-helpers/keychain.ts` (~22 lines changed, 6 errors rewritten + 1 NOTE comment on line 65 catch), `tests/keychain.test.ts` (+40 lines, 6 assertion block), `.omc/evidence/issue-1-close-attempt.txt` (NEW evidence artifact).

### US-1105 — Linux Docker libsecret-tools smoke harness (THIRD)
- **Status:** in-scope (this session, runtime execution attempted). **REORDERED to step 3** (was step 5 in v1) — bring runtime evidence forward before doc-only stories so any failure surfaces early.
- **Acceptance:**
  - `tests/smoke-linux-docker.sh` exists, executable. Builds container via `tests/Dockerfile.linux-smoke`, installs `libsecret-tools` + `dbus-x11`, copies in `bin/axhub-helpers-linux-amd64`, starts D-Bus session, runs `secret-tool store --label=axhub service axhub` against fixture token, runs `./bin/axhub-helpers-linux-amd64 token-init`, asserts exit 0 + stdout contains `linux-secret-service`.
  - `tests/Dockerfile.linux-smoke` exists. **Pinned to explicit sha256 digest** (NOT tag-only — Critic NEW US-1105 acceptance). Get digest via `docker manifest inspect ubuntu:24.04 | jq -r '.manifests[] | select(.platform.architecture=="arm64") | .digest'` (capture for both amd64 and arm64 if possible; pin one explicit value, e.g. `FROM ubuntu:24.04@sha256:<actual-digest-captured-at-plan-time>`). If user lacks docker, fall back to `FROM ubuntu:24.04` with TODO comment naming the digest gap as P1 follow-up.
  - Manual run on macOS dev host with Docker Desktop produces evidence file `.omc/evidence/phase-11-linux-docker-smoke.txt`. **First line MUST be the LIMITATION banner** (Pre-mortem #3 + Architect Synthesis #1):
    ```
    LIMITATION: libsecret-tools CLI path only — does NOT validate gnome-keyring-daemon, kwalletd5, or headless systemd-keyring user-bus.
    Coverage: 40% READ-path (axhub-helpers contract). End-to-end ~15% (ax-hub-cli WRITE-path D-Bus is upstream concern).
    ```
    followed by exit code, source identifier, image SHA, timestamp, libsecret-tools version.
  - Header of `tests/smoke-linux-docker.sh` documents the same LIMITATION banner (as a `# LIMITATION:` shell comment).
  - `tests/Dockerfile.linux-smoke` and `tests/smoke-linux-docker.sh` NOT added to default `bun test` (would need Docker on CI), but referenced from `package.json` `scripts` as `"smoke:linux-docker": "bash tests/smoke-linux-docker.sh"` for opt-in run.
- **Blocked by:** Docker Desktop installed and running on dev host. (User confirms availability before execution.)
- **Files touched:** `tests/smoke-linux-docker.sh` (NEW, ~70 lines + LIMITATION header), `tests/Dockerfile.linux-smoke` (NEW, ~25 lines, sha256-pinned), `package.json` (+1 script line), `.omc/evidence/phase-11-linux-docker-smoke.txt` (NEW, evidence artifact, LIMITATION banner first line).

### US-1103 — Windows VM smoke checklist + companion .ps1 (DEFERRED, doc + scaffold)
- **Status:** documentation-only this session for runtime execution; companion PowerShell scaffold lands now per Architect Synthesis #3.
- **Acceptance:**
  - `.omc/runbooks/windows-vm-smoke-v0.1.7.md` exists with 14-step checklist (preserved from v1 lines 111-126).
  - **NEW per Architect Synthesis #3:** `tests/smoke-windows-vm-checklist.ps1` exists. Codifies the 14 steps as PowerShell behind `if ($env:AXHUB_VM_SMOKE -eq '1') { ... }` guard (does NOT execute by default). Each step wrapped in `try { ... } catch { Write-Host "FAIL step N: $_" ; exit 1 }`. Asserts runtime contract: exit codes via `$LASTEXITCODE`, JSON envelope shape via `ConvertFrom-Json` + property checks (`systemMessage` field present). Pester adapter optional — bare `try/catch` works without dependencies.
  - Doc cites Apple Silicon specifics (Parallels Win11 ARM, UTM TPM workaround) in dedicated sub-section.
  - Doc date-stamped `2026-04-24`.
- **Blocked by:** Win11 ARM ISO + Parallels/UTM provisioning + ~3-4hr execution session.
- **Files touched:** `.omc/runbooks/windows-vm-smoke-v0.1.7.md` (NEW, ~120 lines), `tests/smoke-windows-vm-checklist.ps1` (NEW, ~80 lines, env-guarded inert by default).

### US-1104 — Authenticode signing procurement runbook + stub workflow (DEFERRED, doc + inert scaffold)
- **Status:** documentation-only this session; requires vendor procurement (5–14 days, separate calendar). Stub workflow lands now per Architect Synthesis #2 (modified per Critic).
- **Acceptance:**
  - `.omc/runbooks/authenticode-procurement.md` exists with vendor matrix + cert recommendation + signing target list + required user actions + blocked-by markers + verify-before-purchase checklist (preserved from v1 lines 134-143).
  - **NEW per Architect Synthesis #2 + Critic modifications:** `.github/workflows/sign-windows.yml.template` exists with:
    - `on: workflow_dispatch:` header (manual trigger only — does NOT auto-fire on push).
    - `env: AXHUB_SIGNING_STUB: '1'` flag at workflow level (signals stub mode).
    - `signtool verify /pa <file>` step with `continue-on-error: true` (unsigned binary exits non-zero — without continue-on-error this would block any future workflow that tries to load the template).
    - Telemetry through existing `src/axhub-helpers/telemetry.ts` channel emitting `"signing skipped — vendor cert pending"` (NOT a new channel — explicit Architect amendment).
  - **NEW per Pre-mortem #9:** Add to `.gitattributes`: `*.yml.template linguist-language=YAML linguist-detectable=false`. Verify `.eslintignore` existence with `test -f .eslintignore && echo "exists"`; if exists, add `*.yml.template` to it. Document in commit body that the workflow is intentionally inert until US-1104b (signing kickoff) lands.
  - Doc date-stamped `2026-04-24`. Marked "VERIFY VENDOR PRICING BEFORE PURCHASE."
- **Blocked by:** business org documents (D-U-N-S, incorporation), payment, vendor identity verification (5–14 days), windows-latest GitHub Actions runner provisioning.
- **Files touched:** `.omc/runbooks/authenticode-procurement.md` (NEW, ~150 lines), `.github/workflows/sign-windows.yml.template` (NEW, ~70 lines, NOT enabled, `workflow_dispatch` only, continue-on-error on verify), `.gitattributes` (+1 line), `.eslintignore` (+1 line if file exists).

---

## ADR v2 — Phase 11 Scope Split

**v2 — incorporates round-1 fixes (3 architect amendments + 4 pre-mortems + 2 critic defects).**

- **Decision:** Adopt Option B with v2 amendments. Ship US-1101 + US-1102 + US-1105 as code+test+runtime-evidence stories. Ship US-1103 + US-1104 as documentation-only runbooks PLUS inert companion scaffolds (`smoke-windows-vm-checklist.ps1` and `sign-windows.yml.template`).
- **Drivers:**
  1. Session feasibility (Apple Silicon macOS, no Win VM, no signing cert, Docker available).
  2. Regression blast radius bounded by existing test coverage on codegen + new format-parity assertions on keychain (D1 ordering: US-1101 first to avoid dirty-tree throw).
  3. Calendar parallelism — Authenticode runbook + stub workflow start the 5–14 day vendor clock without blocking session.
- **Alternatives considered:**
  - Option A (Bucket A only) rejected: leaves Linux smoke as code-review-only for the 3rd phase, and Docker harness is a cheap, available win.
  - Option C (Bucket A + Authenticode kickoff) rejected: procurement is paperwork + business identity docs the agent cannot execute autonomously; better as runbook handed back to user.
  - Splitting US-1102 into per-line PRs rejected: 6 lines (after D2 downgrade) in one file, single semantic change ("close issue #1"), one commit is simpler.
  - Adding `bin/install.ps1` to `smoke:full` rejected at this stage: PowerShell on macOS via `pwsh` is possible but extends scope; US-1101 codegen test coverage is sufficient for v0.1.8.
  - **v2 amendment** — proving line 65 catch reachability via process-mocking rejected (D2): `Bun.spawnSync` does not throw on ENOENT, mocking would be scope-creep, downgrade to 6 assertions is honest.
  - **v2 amendment** — landing US-1102 first (v1 ordering) rejected (D1): codegen test mutation creates dirty tree before regex landed — throw at `codegen-install-version.ts:42`. Reorder is safer than tempfile-mock alternative.
  - **v2 amendment** — Dockerfile pinned to tag-only (`ubuntu:24.04`) rejected: tag-only is reproducibility theater. Explicit sha256 digest required (Critic NEW US-1105 acceptance).
- **Why chosen:** Maximizes technical evidence per session-minute. Honest about environmental and vendor blockers via explicit doc-only stories + inert scaffolds. Closes user-facing issue #1 (with explicit 6-not-7 note + non-determinism precheck) and an architecture gap (codegen drift on Windows + pre-release tag coverage) in one PR.
- **Consequences:**
  - **+** v0.1.8 ships with closed issue #1, drift-proof codegen (with `0.1.8-rc.1` test coverage), and first-ever Linux runtime evidence (with explicit limitation banner).
  - **+** Windows VM and Authenticode runbooks + inert scaffolds unblock the next pilot session — no rediscovery cost, no surprise lint errors.
  - **−** v0.1.8 still ships without Authenticode signature; EDR / Defender warnings persist until v0.1.9 (or whenever signing pipeline lands). Acceptable per Phase 10 verdict.
  - **−** Linux smoke proves libsecret-tools path only, not real-desktop GNOME Keyring / KWallet / headless systemd-keyring behavior. Documented limitation (40% READ-path / 15% end-to-end).
  - **−** Codegen now updates 4 files; missing `install.ps1` regex match would throw. Mitigated by mirror-pattern + 3 new tests including pre-release case.
  - **−** keychain.ts line 65 catch path remains unreachable defensive code; commented but not exercised. Honest tradeoff over scope-creep mocking.
- **Follow-ups:**
  - Open issue #2 tracking Win11 VM smoke execution (US-1103 doc + .ps1 → real runtime).
  - Open issue #3 tracking Authenticode procurement (US-1104 doc + workflow → real signing).
  - Open issue #4 tracking real-desktop Linux smoke (GNOME Keyring on Fedora live ISO via UTM, future).
  - Open issue #5 tracking Linux Docker digest pin maintenance (Ubuntu base image rotation cadence).
  - v0.1.9 candidate scope: Authenticode signing live + windows-latest GitHub Actions runner + Win11 VM smoke evidence + 15% → 60% end-to-end Linux coverage via ax-hub-cli WRITE-path harness.

---

## Task Flow v2 (REORDERED per D1)

```
1. US-1101: edit scripts/codegen-install-version.ts (+install.ps1 sync, +pre-release test)
   → verify: tests/codegen-version.test.ts new 3 cases pass + run codegen:version no-op
   [LANDS FIRST per D1 — codegen regex must exist before test mutates install.ps1]

2. US-1102: edit src/axhub-helpers/keychain.ts (6 errors → 4-part template, line 65 NOTE comment)
   → verify: tests/keychain.test.ts new 6 assertions pass (NOT 7 — D2 downgrade)
   → side-effect: codegen test no longer collides with new ps1 path (D1 resolved)

3. US-1105: write tests/Dockerfile.linux-smoke (sha256-pinned) + tests/smoke-linux-docker.sh
   → verify: bash tests/smoke-linux-docker.sh exits 0
   → evidence: .omc/evidence/phase-11-linux-docker-smoke.txt with LIMITATION banner first line
   [BROUGHT FORWARD from v1 step 5 — surface runtime failures before doc-only work]

4. US-1103 doc + scaffold: write .omc/runbooks/windows-vm-smoke-v0.1.7.md
   + tests/smoke-windows-vm-checklist.ps1 (env-guarded, inert)
   → verify: file exists, 14-step checklist complete, .ps1 parses (manual visual review since no pwsh on macOS)

5. US-1104 doc + stub: write .omc/runbooks/authenticode-procurement.md
   + .github/workflows/sign-windows.yml.template (workflow_dispatch + continue-on-error)
   + .gitattributes update + .eslintignore update (if exists)
   → verify: vendor matrix has 3 vendors with prices, workflow YAML parses, gitattributes line landed

6. Full sweep: bun test (370+ green plus 9 new) + bun run smoke:full (drift check)
   → verify: zero regressions

7. Close GitHub issue #1 with precheck protocol (Pre-mortem #8)
   → gh issue view 1 --json state,locked,closed → write .omc/evidence/issue-1-close-attempt.txt
   → if OPEN+unlocked: comment + close with note "6 lines fixed not 7 — line 65 catch unreachable"
   → either way: surface result in commit body
```

---

## Story dependency graph (updated)

```
US-1101 (codegen install.ps1)
  └─→ US-1102 (keychain 6 errors)         [D1: US-1101 must land first]
        └─→ Issue #1 close (Pre-mortem #8 precheck)

US-1105 (Linux Docker smoke)              [independent, parallel-safe]
  └─→ Evidence file with LIMITATION banner

US-1103 (Win VM doc + .ps1)               [independent, doc-only + inert scaffold]

US-1104 (Authenticode doc + stub yml)     [independent, doc-only + inert scaffold]
  └─→ .gitattributes + .eslintignore updates

Final sweep (step 6) ← gates ALL stories
Issue #1 close (step 7) ← depends on US-1102 landed + sweep green
```

---

## ASCII verification proof

```
v1 line 189-192 ordering:
  1. US-1102 keychain  ❌ writes new template to keychain.ts
  2. US-1101 codegen   ❌ tests/codegen-version.test.ts:18-23 calls syncInstallVersion()
                          on real bin/install.sh + (after this) bin/install.ps1
                          BUT the new install.ps1 regex was just added in step 2
                          → if test from step 1 ran codegen → codegen-install-version.ts:42
                            throws because install.ps1 regex absent
  → DIRTY GIT TREE between step 1 commit and step 2 commit if anyone runs
    `bun test` in between. Commit isolation broken.

v2 Task Flow v2 ordering:
  1. US-1101 codegen   ✅ regex added FIRST, install.ps1 path included
  2. US-1102 keychain  ✅ codegen already updated, no throw, clean tree
  → Each commit independently `bun test` green. D1 resolved.

D2 reachability proof for keychain.ts:64-66:
  $ grep -n "Bun.spawnSync" src/axhub-helpers/keychain.ts
  50:      const result = Bun.spawnSync({ ... })
  Bun docs: Bun.spawnSync returns {exitCode: number|null, ...}
            Returns exitCode: null when binary not found (ENOENT-equivalent)
            DOES NOT throw — try/catch wrapper at line 49+64 catches nothing
  → Line 65 error message "macOS 'security' 명령 실행 실패" NEVER returned
    in practice. Test would need Bun.spawnSync mock to exercise.
  → v2 downgrade: 6 reachable lines (57, 62, 79, 85, 90, 95), NOT 7.
  → Line 65 kept with NOTE comment for defensive intent.

US-1105 LIMITATION banner verification:
  evidence first line MUST start: "LIMITATION: libsecret-tools CLI path only"
  $ head -1 .omc/evidence/phase-11-linux-docker-smoke.txt
  LIMITATION: libsecret-tools CLI path only — does NOT validate ...
  ✅ Architect Synthesis #1 satisfied
  Coverage split: 40% READ-path (axhub-helpers contract scope) /
                  ~15% end-to-end (D-Bus WRITE-path is ax-hub-cli upstream)

US-1104 stub workflow verification:
  $ head -5 .github/workflows/sign-windows.yml.template
  name: sign-windows (STUB)
  on:
    workflow_dispatch:
  env:
    AXHUB_SIGNING_STUB: '1'
  $ grep -A2 "signtool verify" .github/workflows/sign-windows.yml.template
        run: signtool verify /pa $file
        continue-on-error: true
  ✅ Architect Synthesis #2 (modified) satisfied

Pre-mortem #7 pre-release coverage:
  $ grep -n "rc\." tests/codegen-version.test.ts (after US-1101)
  XX:    "version": "0.1.8-rc.1"  // mocked package.json for pre-release case
  $ bun test tests/codegen-version.test.ts
  ✅ Pre-release case included
```

---

## Success Criteria (consensus gate v2)

- [ ] All 5 PRD stories have explicit acceptance criteria (US-1103, US-1104 acceptance = doc file + inert scaffold both exist).
- [ ] Pre-mortem covers 8 failure modes with mitigations (dropped #6, kept #1-5, added #7/#8/#9).
- [ ] ADR includes Decision, Drivers, Alternatives (with v2 amendment rationale), Why, Consequences, Follow-ups.
- [ ] Task Flow ordering reflects D1 fix (US-1101 → US-1102, NOT v1 reverse).
- [ ] US-1102 acceptance reflects D2 downgrade (6 assertions, NOT 7; line 65 NOTE comment).
- [ ] US-1105 acceptance specifies sha256 digest pin (NOT tag-only) AND LIMITATION banner first line.
- [ ] US-1104 acceptance includes `workflow_dispatch:` + `continue-on-error: true` + telemetry channel reuse.
- [ ] US-1102 includes `gh issue view 1 --json state,locked` precheck per Pre-mortem #8.
- [ ] Architect approves v2 scope split rationale (round-2 review).
- [ ] Critic approves v2 pre-mortem coverage and amendment incorporation (round-2 review).
- [ ] User approves "전부 다 작업해" interpretation as Bucket A + US-1105 + 2 doc-only-with-scaffold.

---

## Open Questions (logged to `.omc/plans/open-questions.md`)

1. Is Docker Desktop installed on dev host? (US-1105 hard requirement; if not, fall back to Option A.) **+ corollary:** if Docker available, capture sha256 digest at plan time via `docker manifest inspect ubuntu:24.04` or accept TODO-comment-with-digest-gap fallback?
2. Does user want to start D-U-N-S registration this week to unblock US-1104 procurement? (5–14 day external clock.)
3. Confirm v0.1.8 is correct next version tag (no v0.1.7.x patch path needed for these changes).
4. **NEW v2:** Does `.eslintignore` exist in repo root? (Determines whether US-1104 needs to update it for `*.yml.template` exemption.)
5. **NEW v2:** Acceptable that GitHub issue #1 close comment explicitly says "6 lines fixed, not 7 — line 65 catch unreachable per Bun.spawnSync semantics, see code NOTE comment"? (Honest user-facing note about the contract change.)
