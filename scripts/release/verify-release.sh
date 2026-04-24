#!/usr/bin/env bash
# Phase 3 US-204: User-side release verification.
#
# Downloads release assets + .sig sidecars, verifies cosign signatures, and
# cross-checks each binary's sha256 against manifest.json.
#
# Usage:
#   bash scripts/release/verify-release.sh <tag>
#   e.g.  bash scripts/release/verify-release.sh v0.1.1
#
# Exit 0 if all signatures + checksums valid. Exit 1 on any mismatch.
#
# Requires: gh, cosign, jq, shasum (or sha256sum)
set -euo pipefail

TAG="${1:-}"
if [ -z "$TAG" ]; then
  echo "Usage: $0 <tag>" >&2
  echo "Example: $0 v0.1.1" >&2
  exit 1
fi

REPO="${AXHUB_REPO:-jocoding-ax-partners/axhub}"
WORKDIR=$(mktemp -d)
trap "rm -rf $WORKDIR" EXIT
cd "$WORKDIR"

echo "Downloading release assets from $REPO@$TAG..."
gh release download "$TAG" --repo "$REPO" \
  --pattern "axhub-helpers-*" \
  --pattern "manifest.json*" \
  --pattern "checksums.txt*"

# Verify manifest signature first — it's the trust anchor.
echo "Verifying manifest.json signature (cosign keyless)..."
COSIGN_EXPERIMENTAL=1 cosign verify-blob \
  --signature manifest.json.sig \
  --certificate manifest.json.pem \
  --certificate-identity-regexp "^https://github.com/${REPO}/" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  manifest.json

echo "Manifest signature OK."

# Use sha256sum on Linux, shasum on macOS
if command -v sha256sum >/dev/null 2>&1; then
  SHA256_CMD="sha256sum"
else
  SHA256_CMD="shasum -a 256"
fi

# Verify each binary's signature + sha256 matches manifest entry.
echo "Verifying each binary signature + checksum..."
fail=0
for entry in $(jq -r '.binaries[] | @base64' manifest.json); do
  decoded=$(echo "$entry" | base64 --decode)
  filename=$(echo "$decoded" | jq -r '.filename')
  expected_sha=$(echo "$decoded" | jq -r '.sha256')

  if [ ! -f "$filename" ]; then
    echo "FAIL: $filename not in release assets"
    fail=$((fail+1))
    continue
  fi

  COSIGN_EXPERIMENTAL=1 cosign verify-blob \
    --signature "${filename}.sig" \
    --certificate "${filename}.pem" \
    --certificate-identity-regexp "^https://github.com/${REPO}/" \
    --certificate-oidc-issuer https://token.actions.githubusercontent.com \
    "$filename" >/dev/null 2>&1 || {
      echo "FAIL: cosign verify rejected $filename"
      fail=$((fail+1))
      continue
    }

  actual_sha=$($SHA256_CMD "$filename" | awk '{print $1}')
  if [ "$actual_sha" != "$expected_sha" ]; then
    echo "FAIL: sha256 mismatch on $filename (expected $expected_sha, got $actual_sha)"
    fail=$((fail+1))
    continue
  fi

  echo "OK: $filename"
done

if [ "$fail" -gt 0 ]; then
  echo "---"
  echo "VERIFICATION FAILED: $fail issue(s)" >&2
  exit 1
fi

echo "---"
echo "All release assets verified for $REPO@$TAG"
