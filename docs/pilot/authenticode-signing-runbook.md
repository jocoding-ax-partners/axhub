# Authenticode signing runbook (Phase 11 US-1104)

Vendor procurement plan for signing `axhub-helpers-windows-amd64.exe` and
`bin/install.ps1` + `hooks/session-start.ps1` with Authenticode certificates.
Goal: legitimize EDR allowlist requests for Windows pilot users.

## Why this matters

v0.1.5+ Windows binary uses inline PowerShell + `Add-Type` PInvoke against
`advapi32!CredReadW`. This is a textbook Mimikatz / SharpDPAPI signature.
EDR (V3, AhnLab, CrowdStrike, Microsoft Defender for Endpoint) will quarantine
unsigned binaries doing this — and corporate SOC will REJECT allowlist requests
for unsigned binaries because they cannot verify provenance.

**Authenticode signing makes the allowlist request legitimate**: SOC can verify
the publisher cert chain → trust the pinned cert → grant exception.

## Vendor procurement steps

### Step 1: Cert vendor selection

| Vendor | OV cost (1yr) | EV cost (1yr) | Issue time | Notes |
|---|---|---|---|---|
| Sectigo (Comodo) | ~$200-300 | ~$350-500 | 1-3 days OV / 1-2 weeks EV | Most common for OSS |
| DigiCert | ~$400-500 | ~$700+ | 1-3 days | Premium pricing |
| GlobalSign | ~$250-350 | ~$500-700 | 2-5 days | Mid-tier |
| SSL.com | ~$160-260 | ~$300-500 | 1-3 days | Cheapest, less brand recognition |

**Recommendation**: Sectigo OV (Organization Validation) cert for v0.1.8.
EV (Extended Validation) only if SmartScreen reputation rampup is critical
(EV gets immediate trust, OV needs ~1000 downloads to build reputation).

### Step 2: Identity verification

OV requires:
- Business registration (회사 사업자등록증)
- D-U-N-S Number (free from Dun & Bradstreet, takes 1-2 weeks if not yet registered)
- Phone verification call (English-speaking required)
- DNS TXT record on domain (jocodingax.ai)

EV adds:
- In-person notarization OR video conference verification
- Bank account confirmation
- 2 reference checks (executive + bank)

### Step 3: Signing key storage

NEVER ship private key in repo. Three options:

**Option A: GitHub Actions encrypted secret (cheapest, OV only)**
- Convert `.pfx` → base64
- `gh secret set AUTHENTICODE_PFX_BASE64 < cert.pfx.b64`
- `gh secret set AUTHENTICODE_PASSWORD --body '<password>'`
- Workflow decodes at runtime, signs, never persists

**Option B: Azure Key Vault HSM (recommended for EV)**
- HSM-backed, never exports key
- $1-5/month for vault
- GitHub Actions OIDC federation: no long-lived secret
- Required for EV per CA/Browser Forum baseline

**Option C: Certum / SignPath cloud signing**
- $20-50/month subscription
- Web-based signing, no key management
- Good for low-volume releases

### Step 4: GitHub Actions workflow

Reference template: `.github/workflows/sign-windows.yml.template`

When AXHUB_SIGNING_STUB=1 (until vendor procurement clears):
- Workflow runs `signtool verify /pa <file>` with `continue-on-error: true`
- Emits telemetry event `windows.signing.skipped` via existing telemetry.ts channel
- Release pipeline acknowledges unsigned-binary state without breaking

When vendor procurement clears + secret configured:
- Remove `AXHUB_SIGNING_STUB=1` env
- Remove `continue-on-error: true` on verify step
- Workflow signs binary + verifies + uploads signed artifact

**Day procurement clears, integration is one PR**:
```bash
sed -i '/AXHUB_SIGNING_STUB=1/d' .github/workflows/sign-windows.yml
sed -i '/continue-on-error: true/d' .github/workflows/sign-windows.yml
```

### Step 5: Cosign integration retention

Phase 6 already ships cosign keyless signatures via sigstore for cross-platform
artifact verification. Authenticode is ADDITIVE for Windows-specific EDR trust:

- Cosign sig: cross-platform supply-chain integrity (already shipped)
- Authenticode: Windows-specific user trust (this runbook adds)

Both signatures coexist on `windows-amd64.exe`. Authenticode is verified
by Windows SmartScreen + EDR; cosign is verified by `verify-release.sh`
across all platforms.

## Timeline estimate

- Phase 11: stub workflow + runbook shipped (this US-1104) ✓
- Phase 12 (~2-3 weeks): vendor selection + cert procurement
- Phase 13 (~1 week post-cert): integration PR + first signed release
- Phase 14: pilot user EDR allowlist requests using signed binary

## Honest tradeoff

This is paperwork + executable scaffold. It does NOT add signing TODAY. It
shortens the day-after-procurement integration to one PR (~10 lines changed)
instead of starting from scratch. Vendor procurement timeline is outside
engineering control.

Until then: vibe coders on locked-down Win10/11 with EDR continue using
`AXHUB_TOKEN` env var fallback per Phase 9 keychain-windows.ts:ERR_EDR
4-part Korean message.
