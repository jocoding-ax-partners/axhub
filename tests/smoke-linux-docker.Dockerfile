# LIMITATION: libsecret-tools CLI path only — Coverage: 40% READ-path
# (axhub-helpers contract). End-to-end ~15% (ax-hub-cli WRITE-path D-Bus is
# upstream). Does NOT validate gnome-keyring-daemon, kwalletd5, or headless
# systemd-keyring user-bus.
#
# Pinned to ubuntu:24.04 multi-arch index digest captured 2026-04-27 via:
#   docker buildx imagetools inspect ubuntu:24.04
#
# When this digest goes stale (180+ days), bump per:
#   1. Run capture command above
#   2. Replace digest below
#   3. Update capture-date comment
#   4. Re-run smoke harness, verify still green

FROM ubuntu:24.04@sha256:c4a8d5503dfb2a3eb8ab5f807da5bc69a85730fb49b5cfca2330194ebcc41c7b

# Install libsecret-tools (provides secret-tool CLI) + dbus-x11 (provides
# dbus-launch shim for D-Bus session bus inside container) + ca-certificates
# (for any HTTPS calls if future smoke extensions reach out).
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsecret-tools \
    dbus-x11 \
    gnome-keyring \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# axhub-helpers binary is mounted at runtime via -v from host bin/.
# Container does NOT bake the binary — keeps Dockerfile reusable across versions.

WORKDIR /work
ENTRYPOINT ["/bin/bash"]
