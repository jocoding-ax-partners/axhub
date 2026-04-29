# Release Process — axhub plugin

axhub plugin uses sigstore cosign keyless signing for supply-chain integrity. Every release tag automatically:

1. Builds 5 cross-arch Rust helper binaries (darwin-arm64/amd64, linux-arm64/amd64, windows-amd64) via Cargo/cross in a GitHub Actions matrix
2. Generates `manifest.json` (binary → arch → sha256 mapping) + `checksums.txt`
3. Signs each binary + the manifest with cosign keyless (sigstore OIDC, no long-lived secrets)
4. Uploads everything to a GitHub Release: binaries, .sig sidecars, manifest.json, checksums.txt

---

## For maintainers (cutting a release)

### One-time setup (per repo, by admin)

1. **GitHub Actions write permissions** (Settings → Actions → General):
   - Workflow permissions: **Read and write permissions**
   - Allow GitHub Actions to create and approve pull requests: enabled

2. **Sigstore OIDC**: nothing to configure. Keyless signing uses the workflow's GitHub OIDC token (`id-token: write` in the workflow file).

3. **Rust release matrix**:
   - GitHub-hosted runners build the helper with Rust 1.94.1: ubuntu-latest for Linux amd64, `cross` on ubuntu-latest for Linux arm64, macos-13 for Intel Mac, macos-14 for Apple Silicon Mac, and windows-latest for Windows amd64.
   - Bun is still installed in the signing job only to run `scripts/release/manifest.ts`; it is no longer used to compile helper binaries.
   - Cosign keyless signing remains on ubuntu-latest with `id-token: write`.

4. **Optional staging gate secrets/vars** for `.github/workflows/rust-staging-gates.yml`:
   - Secrets: `AXHUB_E2E_STAGING_TOKEN`, `AXHUB_E2E_STAGING_ENDPOINT`, `AXHUB_E2E_STAGING_APP_ID`
   - Secret or repo var: `AXHUB_CLI_INSTALL_COMMAND` (installs the real `axhub` CLI on Ubuntu runners)
   - Optional repo var: `AXHUB_E2E_ALLOW_PROXY=1` when staging uses a managed proxy or non-production TLS endpoint.

### Cutting a release (Phase 19 v0.1.19+ — 자동 버전 범프)

`commit-and-tag-version` (D2 per `.versionrc.json`) 가 모든 절차를 한 줄로 묶어요. Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`) 기반 자동 bump.

```bash
# 0. clean working tree 확인
git status

# 1. release 한 줄 (auto-bump from commit history)
bun run release
# 또는 명시적:
bun run release -- --release-as patch    # 0.1.18 → 0.1.19
bun run release -- --release-as minor    # 0.1.18 → 0.2.0
bun run release -- --release-as major    # 0.1.18 → 1.0.0
bun run release -- --release-as 0.1.20   # 명시 version

# 자동 수행:
#  ✓ package.json + plugin.json + marketplace.json (3 files) 버전 bump
#  ✓ postbump hook: codegen:version (install.sh/ps1/index.ts/telemetry.ts 동기화)
#                   + generated version file staging
#                   + release:check (Rust host artifact + release matrix/version assert)
#  ✓ CHANGELOG.md 자동 entry generation (Conventional Commits → Added/Fixed sections)
#  ✓ git commit + git tag vX.Y.Z

# 2. CHANGELOG narrative 본문 확인
# tag 생성 뒤에는 amend 하지 않습니다. 필요한 narrative 는 release 전 별도 docs commit 으로 반영해요.
git show --stat --oneline HEAD

# 3. push
git push origin main --tags
# release.yml workflow tag push 시 자동 fire — Rust 5 binary + cosign 서명 + GH release upload
```

### Hotfix workflow (긴급 fix mid-Phase)

```bash
# 1. fix commit
git commit -am "fix: <urgent issue>"

# 2. patch bump + ship 즉시
bun run release -- --release-as patch
git push origin main --tags
```

### Manual fallback (commit-and-tag-version 사용 안 할 때)

```bash
# vim package.json + .claude-plugin/plugin.json + .claude-plugin/marketplace.json
bun run release:check    # MANDATORY (Rust helper stale binary / release matrix drift 방지)
bun test                 # 회귀
git commit -am "chore: bump version to X.Y.Z"
git tag vX.Y.Z
git push origin main --tags
```

The `release.yml` workflow auto-fires on the tag push. Watch progress:

```bash
gh run list --workflow release.yml --limit 1
gh run watch <run-id>
```

### Rust staging gate workflow

Use this before deleting the TypeScript fallback or when a Rust helper change touches auth, deploy listing, TLS, prompt routing, or release packaging:

```bash
gh workflow run rust-staging-gates.yml \
  -f run_staging=true \
  -f require_credentials=true \
  -f fuzz_minutes=1 \
  -f run_windows_smoke=false
```

What it proves:

1. Rebuilds the Rust helper with Cargo and runs local regression gates.
2. Installs the real `axhub` CLI using `AXHUB_CLI_INSTALL_COMMAND`.
3. Runs read-only staging E2E with `AXHUB_E2E_STAGING_TOKEN` and endpoint.
4. Runs the Rust helper against staging via `bin/axhub-helpers list-deployments --app-id "$AXHUB_E2E_STAGING_APP_ID"`.
5. Optionally runs parser fuzz (`fuzz_minutes=1440` for the 24h gate) and GitHub Windows smoke.

The Windows V3/AhnLab cohort still needs the target Windows/EDR environment; the GitHub Windows job is only a smoke gate for the Rust binary and `CredReadW` bridge.

### What gets uploaded to the release

After the workflow finishes, the GitHub Release at `https://github.com/jocoding-ax-partners/axhub/releases/tag/v0.1.1` contains:

| Asset | Purpose |
|---|---|
| `axhub-helpers-darwin-arm64` | Apple Silicon Mac binary |
| `axhub-helpers-darwin-amd64` | Intel Mac binary |
| `axhub-helpers-linux-amd64` | Linux x86_64 binary |
| `axhub-helpers-linux-arm64` | Linux ARM64 (Codespaces, Apple M-series Linux VM, etc.) |
| `axhub-helpers-windows-amd64.exe` | Windows binary |
| `manifest.json` | sha256 + arch + size for each binary above |
| `checksums.txt` | shasum -a 256 output for everything (manual verify fallback) |
| `*.sig` | Cosign signature sidecar for each artifact |

---

## For users (verifying a release)

### Quick verify

```bash
# Requires: gh, cosign, jq
bash scripts/release/verify-release.sh v0.1.1
```

Verifies:
1. `manifest.json` cosign signature is valid (trust anchor)
2. Each binary's cosign signature is valid
3. Each binary's sha256 matches its manifest entry
4. All assets are present

Exit 0 if all pass; exit 1 with details on any failure.

### Manual verification (no scripts/release/verify-release.sh available)

```bash
# Download artifacts for v0.1.1
gh release download v0.1.1 --repo jocoding-ax-partners/axhub --pattern "axhub-helpers-*" --pattern "manifest.json*"

# Verify manifest signature
COSIGN_EXPERIMENTAL=1 cosign verify-blob \
  --signature manifest.json.sig \
  --certificate-identity-regexp "^https://github.com/jocoding-ax-partners/axhub/" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  manifest.json

# Verify a specific binary (example: macOS arm64)
COSIGN_EXPERIMENTAL=1 cosign verify-blob \
  --signature axhub-helpers-darwin-arm64.sig \
  --certificate-identity-regexp "^https://github.com/jocoding-ax-partners/axhub/" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  axhub-helpers-darwin-arm64

# Cross-check sha256 against manifest
expected=$(jq -r '.binaries[] | select(.filename=="axhub-helpers-darwin-arm64") | .sha256' manifest.json)
actual=$(shasum -a 256 axhub-helpers-darwin-arm64 | awk '{print $1}')
[ "$expected" = "$actual" ] && echo "OK" || echo "MISMATCH"
```

---

## When verification fails

`COSIGN_REQUIRE` mode (env var `AXHUB_REQUIRE_COSIGN=1`) on the user's session causes the helper to **warn but not block** if no `.sig` sidecar is found alongside the helper binary at runtime. Hard failure (cosign verify rejects a .sig) is reported via the existing `update.cosign_verification_failed` error in `error-empathy-catalog.md` (exit 66).

If you see `update.cosign_verification_failed`:

1. **DO NOT** override with `AXHUB_ALLOW_UNSIGNED=1` (IT-only escape hatch, see PLAN row 59).
2. Notify your IT/security team. The binary may have been tampered with in transit.
3. Use the previous known-good version until the release is republished.

---

## Out of scope (this PR)

- **Cosign key generation**: not needed for keyless signing. If you want to switch to long-lived keys (some org policies require this), generate via `cosign generate-key-pair` and modify `release.yml` to use `--key`. This requires storing `COSIGN_PASSWORD` + `COSIGN_PRIVATE_KEY` as repo secrets — see [cosign docs](https://docs.sigstore.dev/cosign/key_management/signing_with_self-managed_keys/).
- **Marketplace publish**: separate step after first signed release lands. Currently the plugin is consumed via `/plugin marketplace add jocoding-ax-partners/axhub`; no signed-marketplace-asset workflow exists yet (Phase 4 deferred).
