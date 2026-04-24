# Release Process — axhub plugin

axhub plugin uses sigstore cosign keyless signing for supply-chain integrity. Every release tag automatically:

1. Builds 5 cross-arch helper binaries (darwin-arm64/amd64, linux-arm64/amd64, windows-amd64) via Bun
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

3. **Self-hosted runner setup** (required — workflow uses `runs-on: [self-hosted, Linux, ARM64]`):
   - Provision a Linux ARM64 machine with: bun ≥1.1, git, cosign installer compatibility (curl + bash). 4 CPU / 8GB RAM is plenty for `bun build:all` (5 cross-arch compiles, ~10s total).
   - Bun cross-compiles all 5 targets (darwin-arm64/amd64, linux-arm64/amd64, windows-amd64) from a single ARM64 host — no need to provision multiple architectures.
   - Settings → Actions → Runners → "New self-hosted runner" → follow GitHub's installer instructions on the runner host. Default labels `self-hosted` + `Linux` + `ARM64` are added automatically (no extra label needed).
   - Verify: Settings → Actions → Runners shows the runner as **Idle** (green dot).
   - **Why self-hosted**: keeps cosign signing material + sigstore OIDC token exchange on owned infra (회사 보안 정책 호환), avoids GitHub-hosted runner queue times during release windows, fixed cost vs. per-minute billing on busy weeks.
   - **Hardening checklist**: ephemeral runner OR run with `--ephemeral` flag (fresh state per job), restricted firewall (outbound only — fulcio.sigstore.dev, rekor.sigstore.dev, github.com, api.github.com, registry.npmjs.org, registry.bun.sh), runner user has no sudo, log retention ≥ 30d for audit.

4. **Optional: AXHUB_E2E_STAGING_TOKEN** + `AXHUB_E2E_STAGING_ENDPOINT` repository secrets for the gated E2E job (see US-206).

### Cutting a release

```bash
# 1. Bump version in package.json + .claude-plugin/plugin.json + .claude-plugin/marketplace.json
#    (must all match — see tests/manifest.test.ts cross-consistency assertions)
vim package.json    # "version": "0.1.1"
vim .claude-plugin/plugin.json
vim .claude-plugin/marketplace.json

# 2. Run regression locally
bun test
bun run typecheck
bun run smoke:full

# 3. Commit + tag
git commit -am "chore: bump version to 0.1.1"
git tag v0.1.1
git push origin main --tags
```

The `release.yml` workflow auto-fires on the tag push. Watch progress:

```bash
gh workflow run --watch
```

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
- **Marketplace publish**: separate step after first signed release lands. Currently the plugin is consumed via `/plugin marketplace add jocoding-ax-partners/axhub-plugin-cc`; no signed-marketplace-asset workflow exists yet (Phase 4 deferred).
