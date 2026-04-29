# Redact parity mapping

| TypeScript test | Rust coverage |
| --- | --- |
| fullwidth NFKC collapse | `redact_matches_typescript_secret_and_unicode_contract` |
| zero-width stripping | `redact_matches_typescript_secret_and_unicode_contract` |
| bidi stripping | `redact_matches_typescript_secret_and_unicode_contract` |
| Bearer token redaction and short-token non-redaction | `redact_matches_typescript_secret_and_unicode_contract` |
| `AXHUB_TOKEN=` redaction | `redact_matches_typescript_secret_and_unicode_contract` |
| `axhub_pat_*` redaction | `redact_matches_typescript_secret_and_unicode_contract` |
| ANSI stripping | `redact_matches_typescript_secret_and_unicode_contract` |
