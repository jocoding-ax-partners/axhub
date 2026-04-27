# Phase 11 — Deferred Tradeoffs from Phase 10 v0.1.7 Ship

**Mode:** ralplan consensus, DELIBERATE (auth/security + release pipeline + public API surface).
**Date:** 2026-04-24.
**Branch:** `main` (continuing from v0.1.7 ship 2026-04-23).
**Author:** Planner agent.
**Status:** DRAFT awaiting Architect + Critic review.

---

## Recommended scope split

**Pick: Option B — Bucket A (US-1101 codegen install.ps1 sync + US-1102 keychain.ts format-parity 7 lines) + US-1105 Linux Docker smoke harness, with US-1103 (Windows VM checklist doc) and US-1104 (Authenticode procurement doc) shipped as documentation-only stories.** Rationale: "전부 다 작업해" honestly maps to "ship every priority where we have agency in this dev session." Code-only items (US-1101, US-1102) are ~12 lines total, fully test-covered, near-zero risk. Linux Docker (US-1105) is feasible because Docker Desktop is the standard macOS dev tool and `libsecret-tools` Ubuntu image + manual `secret-tool store` populates the same path our code reads — we can produce a real exit-0 PASS artifact, not a code review. Windows VM (US-1103) and Authenticode (US-1104) require hours of Parallels/UTM provisioning and a $300+/yr OV cert procurement loop with vendor identity verification — those are calendar-bound, not session-bound, so they ship as written checklists with explicit blocked-by markers so the next pilot session has a runbook instead of restarting discovery.

---

## Principles (4)

1. **Honesty over completeness.** Mark "checklist doc" deferred items explicitly — never claim Windows VM or Authenticode are "done" when only the runbook exists. v0.1.7 already taught us users notice the gap.
2. **Code-only first, environment-bound second, vendor-bound last.** Sequence work by what we control: source edits → local container smoke → external vendor procurement.
3. **Format parity is a contract, not cosmetics.** The 4-part empathy template (`원인` / `해결` / `다음` separated by `\n`) is now a public UX contract — every error message users hit must follow it. Issue #1 is a contract violation.
4. **Codegen drift kills releases.** Phase 6 US-602 proved `install.sh` version drift caused a re-tag. Adding `install.ps1` to the codegen scope closes the same hole on the Windows path before v0.1.8 ships.

---

## Decision Drivers (top 3)

1. **Session feasibility.** Apple Silicon macOS dev host, no Win VM, no native Linux box, no signing cert. We must split scope along environmental boundaries, not wishful ones.
2. **Regression blast radius.** `bin/install.sh` and `bin/install.ps1` are end-user contact surfaces. `keychain.ts` is the auth read path. Any edit must preserve test green (`bun test` 370/370) and the existing 4-part error format on `keychain-windows.ts:101-128`.
3. **Calendar pressure.** Authenticode OV certs take 5–14 days vendor verification. Starting the procurement runbook NOW (even as a doc) parallelizes the v0.1.8 ship with the cert wait window.

---

## Viable Options for the in-session scope

### Option A — Bucket A only (US-1101 + US-1102), defer all 3 environmental items as docs
- **Pros:** Smallest blast radius. Two code edits, both test-covered. Done in ~30 min. Zero environment risk. Zero new test scaffolding. Closes issue #1 today.
- **Cons:** Leaves Linux smoke as code-review-only forever (Phase 8 shipped without runtime evidence). User explicitly listed it; deferring it 3rd time looks like avoidance. Docker Desktop on macOS is a known-good harness; not using it is a missed cheap win.

### Option B — Bucket A + US-1105 Linux Docker smoke, US-1103/US-1104 as doc-only
- **Pros:** Produces a real exit-0 binary run on real Linux libsecret-tools env — first runtime evidence Phase 8 actually works. Closes the highest-value environmental gap. Docker is local, no vendor wait. ~60-90 min total session. Architect Phase 10 verdict explicitly flagged this gap.
- **Cons:** Docker harness is new test scaffolding (~80 lines bash + Dockerfile). Risk: smoke succeeds in container but real systemd-keyring envs (GNOME Keyring, KWallet) behave differently — pre-mortem #3. Mitigation: pick official Ubuntu + libsecret-tools, document the limitation explicitly.

### Option C — Bucket A + Authenticode procurement kickoff
- **Pros:** Starts the 5–14 day vendor verification clock today. Parallelizes wall-clock for v0.1.8.
- **Cons:** Procurement is paperwork (org docs, D-U-N-S, payment), not engineering. Cannot kick off in autonomous session without user CC + business identity docs. Better as written runbook + user action item handed back. Linux Docker delivers more technical evidence per session-minute.

**Pick: Option B.** Higher technical signal, no vendor blockers, US-1104 still gets a runbook story (US-1104 doc) so Authenticode kickoff is scheduled, not lost.

---

## Pre-Mortem (5 scenarios)

1. **format-parity edits accidentally change `readKeychainToken` behavior.** `tests/keychain.test.ts` only covers `parseKeyringValue` (pure function), so error string mutations would not break tests but would silently change UX. **Mitigation:** Add new test asserting each of the 7 error messages contains exactly `\n원인:`, `\n해결:`, `\n다음:` substrings (template structure check). Run before+after sweep against the existing strings to confirm semantic preservation.
2. **codegen install.ps1 extension breaks existing install.sh sync (regression).** `syncInstallVersion` is called by `smoke:full` and tests/codegen-version.test.ts. Adding a 3rd file path could throw if regex misses on `install.ps1` line 18. **Mitigation:** Mirror the `syncFile` helper exactly. Add `tests/codegen-version.test.ts` case asserting `bin/install.ps1` contains `$ReleaseVersion = if ($env:AXHUB_PLUGIN_RELEASE) { $env:AXHUB_PLUGIN_RELEASE } else { 'v${packageJson.version}' }`. Idempotency test extension.
3. **Linux Docker smoke succeeds in container but fails on real systemd-keyring envs.** `secret-tool` works against any libsecret backend, but real users on Fedora-with-KWallet or headless servers may not have a D-Bus session bus. Container test gives false confidence. **Mitigation:** Document the limitation in the smoke harness header — "validates code path against libsecret-tools CLI, NOT against GNOME Keyring or KWallet daemon presence." Add this caveat to `phase-11-linux-smoke-evidence.md`.
4. **Authenticode runbook becomes obsolete before procurement starts.** Sectigo / DigiCert pricing, KMS options (Azure Key Vault, AWS CloudHSM, GitHub Actions Trusted Signing) shift quarterly. **Mitigation:** Date-stamp the runbook ("as of 2026-04-24"). Cite three vendors with current Q1 2026 prices. Mark explicit "verify before purchase" checklist.
5. **Win VM checklist doc gets used by future-Claude on a non-Apple-Silicon host where UTM/Parallels paths differ.** **Mitigation:** Write checklist as host-agnostic where possible; call out Apple Silicon specifics (Parallels Win11 ARM ISO link, UTM Win11 ARM TPM workaround) in a separate sub-section.

---

## Expanded Test Plan (DELIBERATE)

### Unit
- `tests/codegen-version.test.ts`: extend with one case asserting `bin/install.ps1` `$ReleaseVersion` literal matches `v${packageJson.version}` after `syncInstallVersion()` runs. Add idempotency case for the new file.
- `tests/keychain.test.ts`: add new `describe("readKeychainToken error format-parity (issue #1)")` block. For each of platform=darwin (mock `Bun.spawnSync` to fail), platform=linux (mock fail), and parse-fail paths, assert the returned `error` string contains all three substrings: `\n원인:`, `\n해결:`, `\n다음:`. 7 assertions total mapping to lines 57, 62, 65, 79, 85, 90, 95 of the new keychain.ts.

### Integration
- Run full `bun test` sweep — 370+ existing tests must stay green plus the new ~8 cases.
- Run `bun run codegen:version` — must update `bin/install.ps1` line 18 in addition to existing 3 files. Verify with `git diff bin/install.ps1` that `$ReleaseVersion` literal changed and rest of file untouched.
- Run `bun run smoke:full` end-to-end — confirms codegen extension does not break the hard-mode smoke chain.

### E2E
- **Manual Linux Docker smoke (US-1105):** `docker run --rm -it ubuntu:24.04 bash -c "<harness>"` script that: installs `libsecret-tools` + `bun`, copies in `bin/axhub-helpers-linux-amd64`, runs `secret-tool store --label=axhub service axhub` with a fixture base64-encoded JSON, runs `./bin/axhub-helpers-linux-amd64 token-init`, asserts exit code 0 and stdout contains `linux-secret-service` source identifier. Write evidence to `.omc/evidence/phase-11-linux-docker-smoke.txt`.
- **Manual Windows VM smoke (US-1103, deferred):** documented as 14-step checklist in `.omc/runbooks/windows-vm-smoke-v0.1.7.md`. Acceptance = doc exists with VM provisioning, AXHUB_TOKEN export, hook execution, evidence capture instructions.
- **Manual Authenticode kickoff (US-1104, deferred):** documented as procurement runbook in `.omc/runbooks/authenticode-procurement.md`. Acceptance = doc exists with vendor matrix, signing pipeline GitHub Actions skeleton, blocked-by tags.

### Observability
- `tests/codegen-version.test.ts` reports `files_updated` array — must include `bin/install.ps1` after change.
- Linux Docker smoke writes structured evidence (exit code, source identifier, timestamp, image SHA) for future regression diff.
- All 7 `readKeychainToken` error strings extracted into a single `tests/format-parity-fixtures.ts` so future error-message changes have grep-able centralized fixtures.

---

## PRD Stories

### US-1101 — codegen extends to bin/install.ps1
- **Status:** in-scope (this session).
- **Acceptance:**
  - `scripts/codegen-install-version.ts` adds 4th `syncFile` invocation targeting `bin/install.ps1` with regex matching `^(\$ReleaseVersion = if \(\$env:AXHUB_PLUGIN_RELEASE\) \{ \$env:AXHUB_PLUGIN_RELEASE \} else \{ ')v\d+\.\d+\.\d+(?:-[a-z0-9.]+)?(' \})$`.
  - `tests/codegen-version.test.ts` adds 2 cases: (a) `bin/install.ps1` contains `'v${packageJson.version}'` after sync, (b) idempotency on second run reports `changed: false`.
  - `bun run codegen:version` updates 4 files (was 3) when bumping version. Verified with manual `bun run codegen:version` after temporarily bumping `package.json` to `0.1.8-test` and reverting.
  - `bun test` stays 370+ green plus 2 new cases.
- **Blocked by:** none.
- **Files touched:** `scripts/codegen-install-version.ts` (+15 lines), `tests/codegen-version.test.ts` (+25 lines).

### US-1102 — keychain.ts 7 errors → 4-part empathy template (close issue #1)
- **Status:** in-scope (this session).
- **Acceptance:**
  - `src/axhub-helpers/keychain.ts` lines 57, 62, 65, 79, 85, 90, 95 rewritten as 4-part `\n원인:` / `\n해결:` / `\n다음:` template. Match the structure used in `keychain-windows.ts:101-128` and `bin/install.ps1:24-28`.
  - Each error preserves the original semantic information (which keychain backend, what user can do).
  - New `tests/keychain.test.ts` describe block asserts all 7 returned `error` strings contain the three template substrings. 7 assertions.
  - `bun test` stays green; new 7 assertions pass.
  - GitHub issue #1 (https://github.com/jocoding-ax-partners/axhub/issues/1) closed with comment linking the commit SHA and citing the new test as proof.
- **Blocked by:** none.
- **Files touched:** `src/axhub-helpers/keychain.ts` (~25 lines changed), `tests/keychain.test.ts` (+45 lines, mock spawnSync helper).

### US-1103 — Windows VM smoke checklist (DEFERRED, doc-only)
- **Status:** documentation-only this session; runtime execution requires VM provisioning (~hours, separate session).
- **Acceptance:**
  - `.omc/runbooks/windows-vm-smoke-v0.1.7.md` exists with 14-step checklist:
    1. Provision Win11 ARM VM (Parallels Desktop or UTM on Apple Silicon — both options documented with download links).
    2. Install Bun, PowerShell 7+, Git for Windows.
    3. Clone axhub plugin repo.
    4. Set `$env:AXHUB_TOKEN = 'axhub_pat_<dev-pat>'`.
    5. Run `bin/install.ps1` manually; assert `bin/axhub-helpers.exe` materialized.
    6. Run `hooks/session-start.ps1`; assert exit 0 and stdout JSON contains `systemMessage`.
    7. Run `bin/axhub-helpers.exe token-init`; assert exit 0 and `windows-credman` source.
    8. Test offline path: disable network, re-run; assert correct error empathy template.
    9. Test 407 corp proxy path: configure proxy with auth, assert error matches `bin/install.ps1:65-68`.
    10. Test MAX_PATH path: clone into deeply nested Korean profile path, assert error matches `bin/install.ps1:80-85`.
    11. Test Defender quarantine: enable real-time protection, run, assert error matches `bin/install.ps1:100-105`.
    12. Test PowerShell 5.1 fallback (legacy Windows 10): assert correct fallback message from `keychain-windows.ts:113-115`.
    13. Capture evidence to `.omc/evidence/phase-11-windows-vm-smoke-evidence.txt` (exit codes, stdout JSON, timestamp).
    14. Open GitHub issue #2 with results (PASS/FAIL per step).
  - Doc cites Apple Silicon specifics (Parallels Win11 ARM, UTM TPM workaround) in dedicated sub-section.
  - Doc date-stamped `2026-04-24`.
- **Blocked by:** Win11 ARM ISO + Parallels/UTM provisioning + ~3-4hr execution session.
- **Files touched:** `.omc/runbooks/windows-vm-smoke-v0.1.7.md` (NEW, ~120 lines).

### US-1104 — Authenticode signing procurement runbook (DEFERRED, doc-only)
- **Status:** documentation-only this session; requires vendor procurement (5–14 days, separate calendar).
- **Acceptance:**
  - `.omc/runbooks/authenticode-procurement.md` exists with:
    - **Vendor matrix** (as of 2026-04-24): Sectigo OV (~$199/yr 1-yr OV), DigiCert OV (~$474/yr), SSL.com OV with eSigner cloud HSM (~$249/yr + $130 eSigner). Cite each vendor's URL.
    - **Cert type recommendation:** OV (not EV) for v0.1.8 — EV requires hardware token shipped to physical address; OV via cloud HSM unblocks GitHub Actions automation. Note: SmartScreen reputation builds slower with OV vs EV; flag as accepted tradeoff.
    - **Signing target list:** `bin/axhub-helpers-windows-amd64.exe`, `bin/install.ps1`, `hooks/session-start.ps1`, `hooks/token-init-prompt.ps1`. List every `.ps1` and `.exe` artifact shipped in v0.1.7 release.
    - **GitHub Actions workflow skeleton** (`.github/workflows/sign-windows.yml.template`): windows-latest runner, Azure Key Vault or SSL.com eSigner CSR fetch, `signtool sign /tr http://timestamp.sectigo.com /td sha256 /fd sha256 /a <file>`, `Set-AuthenticodeSignature` for `.ps1` files, Authenticode verification step before upload.
    - **Required user actions** (paperwork): D-U-N-S registration (free, 1-2 weeks), org documentation (incorporation cert, IRS letter or equivalent), vendor identity verification call.
    - **Blocked-by markers:** `BLOCKED-BY: vendor-OV-cert-issuance`, `BLOCKED-BY: GitHub-Actions-windows-latest-runner-setup` (currently only self-hosted Linux ARM64).
    - **Verification before purchase checklist:** confirm pricing current, confirm eSigner supports unattended GitHub Actions usage, confirm cert covers `.ps1` script signing.
  - Doc date-stamped `2026-04-24`. Marked "VERIFY VENDOR PRICING BEFORE PURCHASE."
- **Blocked by:** business org documents (D-U-N-S, incorporation), payment, vendor identity verification (5–14 days), windows-latest GitHub Actions runner provisioning.
- **Files touched:** `.omc/runbooks/authenticode-procurement.md` (NEW, ~150 lines), `.github/workflows/sign-windows.yml.template` (NEW, ~60 lines, NOT enabled).

### US-1105 — Linux Docker libsecret-tools smoke harness
- **Status:** in-scope (this session, runtime execution attempted).
- **Acceptance:**
  - `tests/smoke-linux-docker.sh` exists, executable. Builds Ubuntu 24.04 container with `libsecret-tools`, `dbus-x11`, copies in `bin/axhub-helpers-linux-amd64`, starts D-Bus session, runs `secret-tool store --label=axhub service axhub` against fixture token, runs `./bin/axhub-helpers-linux-amd64 token-init`, asserts exit 0 + stdout contains `linux-secret-service`.
  - `tests/Dockerfile.linux-smoke` exists. Pinned to `ubuntu:24.04` digest.
  - Manual run on macOS dev host with Docker Desktop produces evidence file `.omc/evidence/phase-11-linux-docker-smoke.txt` with: exit code 0, source identifier, image SHA, timestamp, libsecret-tools version.
  - Header of `tests/smoke-linux-docker.sh` documents the limitation: "validates code path against libsecret-tools CLI ONLY. Does NOT validate against GNOME Keyring daemon, KWallet, or systemd-keyring user-bus presence — those require GUI session and are out of scope for headless container smoke."
  - `tests/Dockerfile.linux-smoke` and `tests/smoke-linux-docker.sh` NOT added to default `bun test` (would need Docker on CI), but are referenced from `package.json` `scripts` as `"smoke:linux-docker": "bash tests/smoke-linux-docker.sh"` for opt-in run.
- **Blocked by:** Docker Desktop installed and running on dev host. (User confirms availability before execution.)
- **Files touched:** `tests/smoke-linux-docker.sh` (NEW, ~70 lines), `tests/Dockerfile.linux-smoke` (NEW, ~25 lines), `package.json` (+1 script line), `.omc/evidence/phase-11-linux-docker-smoke.txt` (NEW, evidence artifact).

---

## ADR — Phase 11 Scope Split

- **Decision:** Adopt Option B. Ship US-1101 + US-1102 + US-1105 as code+test+runtime-evidence stories. Ship US-1103 + US-1104 as documentation-only runbooks.
- **Drivers:**
  1. Session feasibility (Apple Silicon macOS, no Win VM, no signing cert, Docker available).
  2. Regression blast radius bounded by existing test coverage on codegen + new format-parity assertions on keychain.
  3. Calendar parallelism — Authenticode runbook starts the 5–14 day vendor clock without blocking session.
- **Alternatives considered:**
  - Option A (Bucket A only) rejected: leaves Linux smoke as code-review-only for the 3rd phase, and Docker harness is a cheap, available win.
  - Option C (Bucket A + Authenticode kickoff) rejected: procurement is paperwork + business identity docs the agent cannot execute autonomously; better as runbook handed back to user.
  - Splitting US-1102 into per-line PRs rejected: 7 lines in one file, single semantic change ("close issue #1"), one commit is simpler.
  - Adding `bin/install.ps1` to `smoke:full` rejected at this stage: PowerShell on macOS via `pwsh` is possible but extends scope; US-1101 codegen test coverage is sufficient for v0.1.8.
- **Why chosen:** Maximizes technical evidence per session-minute. Honest about environmental and vendor blockers via explicit doc-only stories. Closes user-facing issue #1 and an architecture gap (codegen drift on Windows) in one PR.
- **Consequences:**
  - **+** v0.1.8 ships with closed issue #1, drift-proof codegen, and first-ever Linux runtime evidence.
  - **+** Windows VM and Authenticode runbooks unblock the next pilot session — no rediscovery cost.
  - **−** v0.1.8 still ships without Authenticode signature; EDR / Defender warnings persist until v0.1.9 (or whenever signing pipeline lands). Acceptable per Phase 10 verdict.
  - **−** Linux smoke proves libsecret-tools path only, not real-desktop GNOME Keyring / KWallet behavior. Documented limitation.
  - **−** Codegen now updates 4 files; missing `install.ps1` regex match would throw. Mitigated by mirror-pattern + new test.
- **Follow-ups:**
  - Open issue #2 tracking Win11 VM smoke execution (US-1103 doc → real runtime).
  - Open issue #3 tracking Authenticode procurement (US-1104 doc → real signing).
  - Open issue #4 tracking real-desktop Linux smoke (GNOME Keyring on Fedora live ISO via UTM, future).
  - v0.1.9 candidate scope: Authenticode signing live + windows-latest GitHub Actions runner + Win11 VM smoke evidence.

---

## Task Flow

```
1. US-1102: edit src/axhub-helpers/keychain.ts (7 errors → 4-part template)
   → verify: tests/keychain.test.ts new assertions pass
2. US-1101: edit scripts/codegen-install-version.ts (+install.ps1 sync)
   → verify: tests/codegen-version.test.ts new cases pass + run codegen:version no-op
3. US-1103 doc: write .omc/runbooks/windows-vm-smoke-v0.1.7.md
   → verify: file exists, 14-step checklist complete
4. US-1104 doc: write .omc/runbooks/authenticode-procurement.md + .github/workflows/sign-windows.yml.template
   → verify: vendor matrix has 3 vendors with prices, workflow skeleton parses as YAML
5. US-1105: write tests/Dockerfile.linux-smoke + tests/smoke-linux-docker.sh
   → verify: bash tests/smoke-linux-docker.sh exits 0, evidence file written
6. Full sweep: bun test (370+ green) + bun run smoke:full (drift check)
   → verify: zero regressions
7. Close GitHub issue #1 with commit SHA + cite test
   → verify: issue closed
```

---

## Success Criteria (consensus gate)

- [ ] All 5 PRD stories have explicit acceptance criteria (US-1103, US-1104 acceptance = doc file exists with required sections).
- [ ] Pre-mortem covers 5 failure modes with mitigations.
- [ ] ADR includes Decision, Drivers, Alternatives, Why, Consequences, Follow-ups.
- [ ] Architect approves scope split rationale.
- [ ] Critic approves pre-mortem coverage and Authenticode vendor honesty.
- [ ] User approves "전부 다 작업해" interpretation as Bucket A + US-1105 + 2 doc-only.

---

## Open Questions (logged to `.omc/plans/open-questions.md`)

1. Is Docker Desktop installed on dev host? (US-1105 hard requirement; if not, fall back to Option A.)
2. Does user want to start D-U-N-S registration this week to unblock US-1104? (5–14 day external clock.)
3. Should US-1102 land as a single commit closing issue #1, or split with US-1101 as one Phase 11 PR?
4. Confirm v0.1.8 is correct next version tag (no v0.1.7.x patch path needed for these changes).
