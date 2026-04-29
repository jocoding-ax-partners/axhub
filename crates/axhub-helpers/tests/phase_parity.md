# Rust phase parity test ledger

`phase_parity.rs` groups regression coverage by Rust-port phase:

- Phase 1: redact/catalog/telemetry contracts.
- Phase 2: preflight/resolve/list-deployments contracts, including semver prerelease drop and API response mapping.
- Phase 3: consent zero-leeway, binding mismatch, parser hardening, symlink/world-readable defenses, keyring envelope parsing.
- Phase 4: `cli_e2e.rs` covers Rust CLI version/help/redact/classify behavior.

Pending live/manual gates remain documented in `.plan/10-source-mapping.md`.
