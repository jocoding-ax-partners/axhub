# Ralph external verification evidence — 2026-04-29

## Environment
- cwd: /Users/wongil/Desktop/work/jocoding/axhub
- timestamp_utc: 2026-04-29T06:21:00Z
- AXHUB_E2E_STAGING_TOKEN: unset
- AXHUB_E2E_STAGING_ENDPOINT: unset

## cargo build --workspace
   Compiling proc-macro2 v1.0.106
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.45
   Compiling libc v0.2.186
   Compiling itoa v1.0.18
   Compiling memchr v2.8.0
   Compiling shlex v1.3.0
   Compiling find-msvc-tools v0.1.9
   Compiling stable_deref_trait v1.2.1
   Compiling serde_core v1.0.228
   Compiling autocfg v1.5.0
   Compiling cfg-if v1.0.4
   Compiling bytes v1.11.1
   Compiling dunce v1.0.5
   Compiling pin-project-lite v0.2.17
   Compiling fs_extra v1.3.0
   Compiling serde v1.0.228
   Compiling zmij v1.0.21
   Compiling futures-core v0.3.32
   Compiling num-traits v0.2.19
   Compiling serde_json v1.0.149
   Compiling aws-lc-rs v1.16.3
   Compiling time-core v0.1.8
   Compiling futures-sink v0.3.32
   Compiling smallvec v1.15.1
   Compiling writeable v0.6.3
   Compiling jobserver v0.1.34
   Compiling mio v1.2.0
   Compiling socket2 v0.6.3
   Compiling powerfmt v0.2.0
   Compiling version_check v0.9.5
   Compiling cc v1.2.61
   Compiling litemap v0.8.2
   Compiling zeroize v1.8.2
   Compiling num-conv v0.2.1
   Compiling tokio v1.52.1
   Compiling generic-array v0.14.7
   Compiling deranged v0.5.8
   Compiling time-macros v0.2.27
   Compiling syn v2.0.117
   Compiling getrandom v0.2.17
   Compiling http v1.4.0
   Compiling subtle v2.6.1
   Compiling once_cell v1.21.4
   Compiling icu_properties_data v2.2.0
   Compiling icu_normalizer_data v2.2.0
   Compiling utf8_iter v1.0.4
   Compiling untrusted v0.9.0
   Compiling rustls-pki-types v1.14.1
   Compiling futures-io v0.3.32
   Compiling cmake v0.1.58
   Compiling minimal-lexical v0.2.1
   Compiling slab v0.4.12
   Compiling thiserror v1.0.69
   Compiling futures-task v0.3.32
   Compiling http-body v1.0.1
   Compiling anyhow v1.0.102
   Compiling percent-encoding v2.3.2
   Compiling time v0.3.47
   Compiling typenum v1.20.0
   Compiling aws-lc-sys v0.40.0
   Compiling ring v0.17.14
   Compiling httparse v1.10.1
   Compiling futures-util v0.3.32
   Compiling nom v7.1.3
   Compiling num-integer v0.1.46
   Compiling try-lock v0.2.5
   Compiling rustls v0.23.40
   Compiling base64 v0.22.1
   Compiling tower-service v0.3.3
   Compiling num-bigint v0.4.6
   Compiling want v0.3.1
   Compiling tracing-core v0.1.36
   Compiling futures-channel v0.3.32
   Compiling log v0.4.29
   Compiling utf8parse v0.2.2
   Compiling atomic-waker v1.1.2
   Compiling thiserror v2.0.18
   Compiling anstyle-parse v1.0.0
   Compiling tracing v0.1.44
   Compiling block-buffer v0.10.4
   Compiling crypto-common v0.1.7
   Compiling form_urlencoded v1.2.2
   Compiling sync_wrapper v1.0.2
   Compiling getrandom v0.4.2
   Compiling rusticata-macros v4.1.0
   Compiling anstyle-query v1.1.5
   Compiling colorchoice v1.0.5
   Compiling oid-registry v0.7.1
   Compiling anstyle v1.0.14
   Compiling is_terminal_polyfill v1.70.2
   Compiling ipnet v2.12.0
   Compiling tower-layer v0.3.3
   Compiling anstream v1.0.0
   Compiling digest v0.10.7
   Compiling webpki-roots v1.0.7
   Compiling aho-corasick v1.1.4
   Compiling bitflags v2.11.1
   Compiling synstructure v0.13.2
   Compiling ryu v1.0.23
   Compiling hyper v1.9.0
   Compiling tower v0.5.3
   Compiling heck v0.5.0
   Compiling regex-syntax v0.8.10
   Compiling tinyvec_macros v0.1.1
   Compiling strsim v0.11.1
   Compiling core-foundation-sys v0.8.7
   Compiling iri-string v0.7.12
   Compiling clap_lex v1.1.0
   Compiling zerofrom-derive v0.1.7
   Compiling displaydoc v0.2.5
   Compiling yoke-derive v0.8.2
   Compiling zerovec-derive v0.11.3
   Compiling serde_derive v1.0.228
   Compiling thiserror-impl v1.0.69
   Compiling asn1-rs-derive v0.5.1
   Compiling zerofrom v0.1.7
   Compiling asn1-rs-impl v0.2.0
   Compiling yoke v0.8.2
   Compiling thiserror-impl v2.0.18
   Compiling hyper-util v0.1.20
   Compiling clap_derive v4.6.1
   Compiling zerovec v0.11.6
   Compiling zerotrie v0.2.4
   Compiling regex-automata v0.4.14
   Compiling asn1-rs v0.6.2
   Compiling tower-http v0.6.8
   Compiling tinystr v0.8.3
   Compiling potential_utf v0.1.5
   Compiling icu_collections v2.2.0
   Compiling icu_locale_core v2.2.0
   Compiling simple_asn1 v0.6.4
   Compiling clap_builder v4.6.0
   Compiling iana-time-zone v0.1.65
   Compiling tinyvec v1.11.0
   Compiling pem v3.0.6
   Compiling http-body-util v0.1.3
   Compiling cpufeatures v0.2.17
   Compiling lazy_static v1.5.0
   Compiling data-encoding v2.11.0
   Compiling unicode-normalization v0.1.25
   Compiling sha2 v0.10.9
   Compiling uuid v1.23.1
   Compiling hmac v0.12.1
   Compiling semver v1.0.28
   Compiling der-parser v9.0.0
   Compiling icu_provider v2.2.0
   Compiling icu_normalizer v2.2.0
   Compiling icu_properties v2.2.0
   Compiling x509-parser v0.16.0
   Compiling axhub-codegen v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-codegen)
   Compiling serde_urlencoded v0.7.1
   Compiling jsonwebtoken v9.3.1
   Compiling chrono v0.4.44
   Compiling regex v1.12.3
   Compiling axhub-helpers v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-helpers)
   Compiling idna_adapter v1.2.2
   Compiling idna v1.1.0
   Compiling clap v4.6.1
   Compiling url v2.5.8
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 03s

## Live hub TLS pin + read-only list-deployments probe
Command uses a fake token and app id 1. Expected successful TLS pin followed by auth failure (exit 65). A TLS pin failure would exit 1 with security.tls_pin_failed before HTTP auth.
```
exit=101
stdout=
stderr=
thread 'main' (559283853) panicked at /Users/wongil/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rustls-0.23.40/src/crypto/mod.rs:249:14:

Could not automatically determine the process-level CryptoProvider from Rustls crate features.
Call CryptoProvider::install_default() before this point to select a provider manually, or make sure exactly one of the 'aws-lc-rs' and 'ring' features is enabled.
See the documentation of the CryptoProvider type for more information.
            
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

## bun run test:e2e (staging gated)
$ bun test tests/e2e/
bun test v1.3.10 (30e609e0)

tests/e2e/staging.test.ts:
Skipped: AXHUB_E2E_STAGING_TOKEN not set. See tests/e2e/README.md for how to enable.
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > (unnamed)
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > axhub auth status --json returns valid identity
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > axhub apps list --json returns array (may be empty)
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > parseAxhubCommand → action mapping is consistent with real CLI surface
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > classify-exit produces Korean 4-part template for real exit codes
(pass) ax-hub-cli staging E2E (skipped — no AXHUB_E2E_STAGING_TOKEN) > placeholder: set AXHUB_E2E_STAGING_TOKEN + AXHUB_E2E_STAGING_ENDPOINT to enable [0.59ms]

 1 pass
 5 skip
 0 fail
 1 expect() calls
Ran 6 tests across 1 file. [129.00ms]

## Result summary
- live_tls_probe_exit: 101
- live_tls_probe_interpretation: REVIEW — expected 65 auth failure after TLS pin, got 101.

## cargo audit
```
exit=0
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1060 security advisories (from /Users/wongil/.cargo/advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (241 crate dependencies)
```

## cargo fuzz parser smoke
Command: cargo fuzz run parser -- -max_total_time=60
```
exit=1
error: current package believes it's in a workspace when it's not:
current:   /Users/wongil/Desktop/work/jocoding/axhub/fuzz/Cargo.toml
workspace: /Users/wongil/Desktop/work/jocoding/axhub/Cargo.toml

this may be fixable by adding `fuzz` to the `workspace.members` array of the manifest located at: /Users/wongil/Desktop/work/jocoding/axhub/Cargo.toml
Alternatively, to keep it out of the workspace, add the package to the `workspace.exclude` array, or add an empty `[workspace]` table to the package's manifest.
Error: failed to build fuzz script: ASAN_OPTIONS="detect_odr_violation=0" RUSTFLAGS=" -Cpasses=sancov-module -Cllvm-args=-sanitizer-coverage-level=4 -Cllvm-args=-sanitizer-coverage-inline-8bit-counters -Cllvm-args=-sanitizer-coverage-pc-table -Cllvm-args=-sanitizer-coverage-trace-compares --cfg fuzzing -Cllvm-args=-simplifycfg-branch-fold-threshold=0 -Zsanitizer=address -Cdebug-assertions -Ccodegen-units=1" "cargo" "build" "--manifest-path" "/Users/wongil/Desktop/work/jocoding/axhub/fuzz/Cargo.toml" "--target" "aarch64-apple-darwin" "--release" "--config" "profile.release.debug=\"line-tables-only\"" "--bin" "parser"
```

## cargo fuzz parser smoke retry
Command: cargo fuzz run parser -- -max_total_time=60
```
exit=1
error: failed to run `rustc` to learn about target-specific information

Caused by:
  process didn't exit successfully: `/Users/wongil/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc - --crate-name ___ --print=file-names -Cpasses=sancov-module -Cllvm-args=-sanitizer-coverage-level=4 -Cllvm-args=-sanitizer-coverage-inline-8bit-counters -Cllvm-args=-sanitizer-coverage-pc-table -Cllvm-args=-sanitizer-coverage-trace-compares --cfg fuzzing -Cllvm-args=-simplifycfg-branch-fold-threshold=0 -Zsanitizer=address -Cdebug-assertions -Ccodegen-units=1 --target aarch64-apple-darwin --crate-type bin --crate-type rlib --crate-type dylib --crate-type cdylib --crate-type staticlib --crate-type proc-macro --print=sysroot --print=split-debuginfo --print=crate-name --print=cfg -Wwarnings` (exit status: 1)
  --- stderr
  error: the option `Z` is only accepted on the nightly compiler

  help: consider switching to a nightly toolchain: `rustup default nightly`

  note: selecting a toolchain with `+toolchain` arguments require a rustup proxy; see <https://rust-lang.github.io/rustup/concepts/index.html>

  note: for more information about Rust's stability policy, see <https://doc.rust-lang.org/book/appendix-07-nightly-rust.html#unstable-features>

  error: 1 nightly option were parsed

Error: failed to build fuzz script: ASAN_OPTIONS="detect_odr_violation=0" RUSTFLAGS=" -Cpasses=sancov-module -Cllvm-args=-sanitizer-coverage-level=4 -Cllvm-args=-sanitizer-coverage-inline-8bit-counters -Cllvm-args=-sanitizer-coverage-pc-table -Cllvm-args=-sanitizer-coverage-trace-compares --cfg fuzzing -Cllvm-args=-simplifycfg-branch-fold-threshold=0 -Zsanitizer=address -Cdebug-assertions -Ccodegen-units=1" "cargo" "build" "--manifest-path" "/Users/wongil/Desktop/work/jocoding/axhub/fuzz/Cargo.toml" "--target" "aarch64-apple-darwin" "--release" "--config" "profile.release.debug=\"line-tables-only\"" "--bin" "parser"
```

## cargo +nightly fuzz parser smoke
Command: cargo +nightly fuzz run parser -- -max_total_time=60
```
exit=0
    Updating crates.io index
     Locking 239 packages to latest compatible versions
      Adding generic-array v0.14.7 (available: v0.14.9)
 Downloading crates ...
  Downloaded arbitrary v1.4.2
  Downloaded libfuzzer-sys v0.4.12
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling unicode-ident v1.0.24
   Compiling libc v0.2.186
   Compiling itoa v1.0.18
   Compiling memchr v2.8.0
   Compiling serde_core v1.0.228
   Compiling shlex v1.3.0
   Compiling stable_deref_trait v1.2.1
   Compiling find-msvc-tools v0.1.9
   Compiling autocfg v1.5.0
   Compiling bytes v1.11.1
   Compiling zmij v1.0.21
   Compiling pin-project-lite v0.2.17
   Compiling cfg-if v1.0.4
   Compiling serde v1.0.228
   Compiling num-traits v0.2.19
   Compiling futures-core v0.3.32
   Compiling serde_json v1.0.149
   Compiling smallvec v1.15.1
   Compiling version_check v0.9.5
   Compiling powerfmt v0.2.0
   Compiling litemap v0.8.2
   Compiling writeable v0.6.3
   Compiling time-core v0.1.8
   Compiling num-conv v0.2.1
   Compiling futures-sink v0.3.32
   Compiling time-macros v0.2.27
   Compiling generic-array v0.14.7
   Compiling deranged v0.5.8
   Compiling http v1.4.0
   Compiling untrusted v0.9.0
   Compiling subtle v2.6.1
   Compiling anyhow v1.0.102
   Compiling utf8_iter v1.0.4
   Compiling once_cell v1.21.4
   Compiling zeroize v1.8.2
   Compiling jobserver v0.1.34
   Compiling icu_normalizer_data v2.2.0
   Compiling icu_properties_data v2.2.0
   Compiling mio v1.2.0
   Compiling socket2 v0.6.3
   Compiling getrandom v0.2.17
   Compiling cc v1.2.61
   Compiling syn v2.0.117
   Compiling http-body v1.0.1
   Compiling tokio v1.52.1
   Compiling time v0.3.47
   Compiling rustls-pki-types v1.14.1
   Compiling percent-encoding v2.3.2
   Compiling httparse v1.10.1
   Compiling futures-task v0.3.32
   Compiling thiserror v1.0.69
   Compiling futures-io v0.3.32
   Compiling ring v0.17.14
   Compiling typenum v1.20.0
   Compiling slab v0.4.12
   Compiling minimal-lexical v0.2.1
   Compiling futures-util v0.3.32
   Compiling nom v7.1.3
   Compiling num-integer v0.1.46
   Compiling try-lock v0.2.5
   Compiling rustls v0.23.40
   Compiling tower-service v0.3.3
   Compiling base64 v0.22.1
   Compiling want v0.3.1
   Compiling num-bigint v0.4.6
   Compiling synstructure v0.13.2
   Compiling rusticata-macros v4.1.0
   Compiling tracing-core v0.1.36
   Compiling futures-channel v0.3.32
   Compiling utf8parse v0.2.2
   Compiling thiserror v2.0.18
   Compiling atomic-waker v1.1.2
   Compiling hyper v1.9.0
   Compiling displaydoc v0.2.5
   Compiling zerofrom-derive v0.1.7
   Compiling yoke-derive v0.8.2
   Compiling zerovec-derive v0.11.3
   Compiling serde_derive v1.0.228
   Compiling thiserror-impl v1.0.69
   Compiling zerofrom v0.1.7
   Compiling yoke v0.8.2
   Compiling asn1-rs-derive v0.5.1
   Compiling zerovec v0.11.6
   Compiling zerotrie v0.2.4
   Compiling asn1-rs-impl v0.2.0
   Compiling thiserror-impl v2.0.18
   Compiling anstyle-parse v1.0.0
   Compiling asn1-rs v0.6.2
   Compiling tinystr v0.8.3
   Compiling potential_utf v0.1.5
   Compiling icu_locale_core v2.2.0
   Compiling icu_collections v2.2.0
   Compiling tracing v0.1.44
   Compiling rustls-webpki v0.103.13
   Compiling icu_provider v2.2.0
   Compiling icu_properties v2.2.0
   Compiling icu_normalizer v2.2.0
   Compiling idna_adapter v1.2.2
   Compiling crypto-common v0.1.7
   Compiling block-buffer v0.10.4
   Compiling form_urlencoded v1.2.2
   Compiling sync_wrapper v1.0.2
   Compiling ipnet v2.12.0
   Compiling anstyle v1.0.14
   Compiling anstyle-query v1.1.5
   Compiling tower-layer v0.3.3
   Compiling is_terminal_polyfill v1.70.2
   Compiling colorchoice v1.0.5
   Compiling oid-registry v0.7.1
   Compiling getrandom v0.4.2
   Compiling anstream v1.0.0
   Compiling tower v0.5.3
   Compiling hyper-util v0.1.20
   Compiling digest v0.10.7
   Compiling axhub-codegen v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-codegen)
   Compiling idna v1.1.0
   Compiling tokio-rustls v0.26.4
   Compiling webpki-roots v1.0.7
   Compiling aho-corasick v1.1.4
   Compiling clap_lex v1.1.0
   Compiling iri-string v0.7.12
   Compiling regex-syntax v0.8.10
   Compiling strsim v0.11.1
   Compiling ryu v1.0.23
   Compiling bitflags v2.11.1
   Compiling core-foundation-sys v0.8.7
   Compiling heck v0.5.0
   Compiling tinyvec_macros v0.1.1
   Compiling tinyvec v1.11.0
   Compiling clap_derive v4.6.1
   Compiling iana-time-zone v0.1.65
   Compiling serde_urlencoded v0.7.1
   Compiling clap_builder v4.6.0
   Compiling hyper-rustls v0.27.9
   Compiling regex-automata v0.4.14
   Compiling tower-http v0.6.8
   Compiling simple_asn1 v0.6.4
   Compiling url v2.5.8
   Compiling axhub-helpers v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-helpers)
   Compiling der-parser v9.0.0
   Compiling pem v3.0.6
   Compiling libfuzzer-sys v0.4.12
   Compiling http-body-util v0.1.3
   Compiling cpufeatures v0.2.17
   Compiling data-encoding v2.11.0
   Compiling log v0.4.29
   Compiling lazy_static v1.5.0
   Compiling x509-parser v0.16.0
   Compiling reqwest v0.12.28
   Compiling sha2 v0.10.9
   Compiling jsonwebtoken v9.3.1
   Compiling uuid v1.23.1
   Compiling regex v1.12.3
   Compiling clap v4.6.1
   Compiling chrono v0.4.44
   Compiling unicode-normalization v0.1.25
   Compiling hmac v0.12.1
   Compiling semver v1.0.28
   Compiling arbitrary v1.4.2
   Compiling axhub-helpers-fuzz v0.0.0 (/Users/wongil/Desktop/work/jocoding/axhub/fuzz)
    Finished `release` profile [optimized + debuginfo] target(s) in 1m 45s
    Finished `release` profile [optimized + debuginfo] target(s) in 0.87s
     Running `fuzz/target/aarch64-apple-darwin/release/parser -artifact_prefix=/Users/wongil/Desktop/work/jocoding/axhub/fuzz/artifacts/parser/ -max_total_time=60 /Users/wongil/Desktop/work/jocoding/axhub/fuzz/corpus/parser`
INFO: Running with entropic power schedule (0xFF, 100).
INFO: Seed: 2039338249
INFO: Loaded 1 modules   (443666 inline 8-bit counters): 443666 [0x106c4fc20, 0x106cbc132), 
INFO: Loaded 1 PC tables (443666 PCs): 443666 [0x106cbc138,0x107381258), 
INFO:        0 files found in /Users/wongil/Desktop/work/jocoding/axhub/fuzz/corpus/parser
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: A corpus is not provided, starting from an empty corpus
#2	INITED cov: 4278 ft: 4278 corp: 1/1b exec/s: 0 rss: 65Mb
#3	NEW    cov: 4299 ft: 4306 corp: 2/3b lim: 4 exec/s: 0 rss: 67Mb L: 2/2 MS: 1 InsertByte-
#7	NEW    cov: 4300 ft: 4307 corp: 3/5b lim: 4 exec/s: 0 rss: 72Mb L: 2/2 MS: 4 ChangeByte-ChangeByte-CrossOver-ChangeByte-
	NEW_FUNC[1/10]: 0x000105d553c8 in _RINvMs1_NtCs69vn28XA1JY_9hashbrown3mapINtB6_7HashMapNtNtNtNtCs2BKWDpcBPTM_14regex_automata4util11determinize5state5StateNtNtNtBW_6hybrid2id11LazyStateIDNtNtNtCs3qUFNVap8t0_3std4hash6random11RandomStateE3getShEBW_+0x0 (parser:arm64+0x100f113c8)
	NEW_FUNC[2/10]: 0x000105db640c in _RNvMs0_NtNtCs2BKWDpcBPTM_14regex_automata6hybrid5regexNtB5_5Regex10try_search+0x0 (parser:arm64+0x100f7240c)
#9	NEW    cov: 4491 ft: 4521 corp: 4/8b lim: 4 exec/s: 4 rss: 77Mb L: 3/3 MS: 1 CrossOver-
#10	NEW    cov: 4492 ft: 4522 corp: 5/10b lim: 4 exec/s: 5 rss: 78Mb L: 2/3 MS: 1 ChangeBit-
#11	NEW    cov: 4498 ft: 4529 corp: 6/12b lim: 4 exec/s: 5 rss: 80Mb L: 2/3 MS: 1 CopyPart-
#13	NEW    cov: 4499 ft: 4534 corp: 7/14b lim: 4 exec/s: 6 rss: 82Mb L: 2/3 MS: 2 ShuffleBytes-CrossOver-
#15	NEW    cov: 4500 ft: 4544 corp: 8/17b lim: 4 exec/s: 7 rss: 85Mb L: 3/3 MS: 2 CrossOver-InsertByte-
#16	pulse  cov: 4500 ft: 4544 corp: 8/17b lim: 4 exec/s: 8 rss: 86Mb
#17	NEW    cov: 4501 ft: 4545 corp: 9/19b lim: 4 exec/s: 8 rss: 88Mb L: 2/3 MS: 2 EraseBytes-ChangeBinInt-
#26	NEW    cov: 4501 ft: 4546 corp: 10/20b lim: 4 exec/s: 13 rss: 99Mb L: 1/3 MS: 4 ChangeBinInt-EraseBytes-ShuffleBytes-ChangeBinInt-
#27	NEW    cov: 4501 ft: 4549 corp: 11/23b lim: 4 exec/s: 13 rss: 100Mb L: 3/3 MS: 1 CrossOver-
#32	pulse  cov: 4501 ft: 4549 corp: 11/23b lim: 4 exec/s: 16 rss: 106Mb
#39	NEW    cov: 4501 ft: 4550 corp: 12/25b lim: 4 exec/s: 19 rss: 115Mb L: 2/3 MS: 2 ChangeByte-CopyPart-
#64	pulse  cov: 4501 ft: 4550 corp: 12/25b lim: 4 exec/s: 32 rss: 148Mb
#69	NEW    cov: 4501 ft: 4552 corp: 13/28b lim: 4 exec/s: 34 rss: 154Mb L: 3/3 MS: 5 ChangeBit-ChangeASCIIInt-ChangeBinInt-CrossOver-CopyPart-
	NEW_FUNC[1/3]: 0x000106484b20 in _RINvNvMs2_NtCsfrArhfcn9Tb_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsbR5C9kte1NK_13axhub_helpers+0x0 (parser:arm64+0x101640b20)
	NEW_FUNC[2/3]: 0x000104f20670 in _RINvXs6_NtCsfrArhfcn9Tb_5alloc6stringNtB6_6StringINtNtNtNtCs88k1Tj5ijCL_4core4iter6traits7collect12FromIteratorRcE9from_iterINtNtNtBU_5slice4iter4ItercEECsbR5C9kte1NK_13axhub_helpers+0x0 (parser:arm64+0x1000dc670)
#87	NEW    cov: 4524 ft: 5426 corp: 14/30b lim: 4 exec/s: 29 rss: 177Mb L: 2/3 MS: 3 ChangeByte-CopyPart-InsertByte-
	NEW_FUNC[1/4]: 0x000105ed2ed8 in _RNvXs2_NtNtCs2BKWDpcBPTM_14regex_automata4meta8strategyNtB5_4CoreNtB5_8Strategy12search_slots+0x0 (parser:arm64+0x10108eed8)
	NEW_FUNC[2/4]: 0x000105f0eff0 in _RNvXs_NtNtNtCs2BKWDpcBPTM_14regex_automata4util9prefilter5teddyNtB4_5TeddyNtB6_10PrefilterI4find+0x0 (parser:arm64+0x1010caff0)
#93	NEW    cov: 4604 ft: 5539 corp: 15/34b lim: 4 exec/s: 31 rss: 185Mb L: 4/4 MS: 1 CopyPart-
#100	NEW    cov: 4607 ft: 5558 corp: 16/38b lim: 4 exec/s: 33 rss: 194Mb L: 4/4 MS: 2 InsertByte-InsertByte-
#102	REDUCE cov: 4607 ft: 5558 corp: 16/37b lim: 4 exec/s: 34 rss: 197Mb L: 2/4 MS: 2 CrossOver-CrossOver-
#104	REDUCE cov: 4607 ft: 5558 corp: 16/36b lim: 4 exec/s: 34 rss: 200Mb L: 1/4 MS: 2 ShuffleBytes-EraseBytes-
#116	REDUCE cov: 4607 ft: 5558 corp: 16/35b lim: 4 exec/s: 38 rss: 215Mb L: 1/4 MS: 2 ShuffleBytes-EraseBytes-
#124	NEW    cov: 4610 ft: 5562 corp: 17/37b lim: 4 exec/s: 41 rss: 226Mb L: 2/4 MS: 3 ShuffleBytes-ChangeBinInt-ChangeByte-
#128	pulse  cov: 4610 ft: 5642 corp: 17/37b lim: 4 exec/s: 42 rss: 231Mb
#129	NEW    cov: 4644 ft: 5642 corp: 18/40b lim: 4 exec/s: 43 rss: 232Mb L: 3/4 MS: 4 EraseBytes-EraseBytes-InsertByte-InsertByte-
#134	NEW    cov: 4651 ft: 5657 corp: 19/42b lim: 4 exec/s: 44 rss: 239Mb L: 2/4 MS: 5 ChangeByte-ChangeBit-CopyPart-ChangeBit-ChangeByte-
#136	NEW    cov: 4653 ft: 6567 corp: 20/46b lim: 4 exec/s: 45 rss: 243Mb L: 4/4 MS: 2 InsertByte-CopyPart-
#144	NEW    cov: 4653 ft: 6568 corp: 21/48b lim: 4 exec/s: 48 rss: 255Mb L: 2/4 MS: 3 ChangeBit-ChangeBinInt-CopyPart-
#155	NEW    cov: 4656 ft: 6572 corp: 22/50b lim: 4 exec/s: 51 rss: 270Mb L: 2/4 MS: 1 CrossOver-
#196	NEW    cov: 4657 ft: 6573 corp: 23/53b lim: 4 exec/s: 65 rss: 323Mb L: 3/4 MS: 1 ChangeASCIIInt-
#200	NEW    cov: 4658 ft: 6575 corp: 24/55b lim: 4 exec/s: 66 rss: 328Mb L: 2/4 MS: 4 ShuffleBytes-EraseBytes-CopyPart-CMP- DE: "' "-
#211	NEW    cov: 4658 ft: 6576 corp: 25/58b lim: 4 exec/s: 70 rss: 342Mb L: 3/4 MS: 1 ChangeBit-
#212	NEW    cov: 4658 ft: 6578 corp: 26/62b lim: 4 exec/s: 70 rss: 344Mb L: 4/4 MS: 1 PersAutoDict- DE: "' "-
#233	NEW    cov: 4659 ft: 6579 corp: 27/64b lim: 4 exec/s: 77 rss: 373Mb L: 2/4 MS: 1 InsertByte-
#235	NEW    cov: 4659 ft: 6581 corp: 28/67b lim: 4 exec/s: 78 rss: 376Mb L: 3/4 MS: 2 ChangeBinInt-InsertByte-
#236	NEW    cov: 4659 ft: 7217 corp: 29/71b lim: 4 exec/s: 78 rss: 379Mb L: 4/4 MS: 1 CopyPart-
#237	NEW    cov: 4663 ft: 7221 corp: 30/75b lim: 4 exec/s: 79 rss: 381Mb L: 4/4 MS: 1 InsertByte-
#256	pulse  cov: 4663 ft: 7221 corp: 30/75b lim: 4 exec/s: 85 rss: 394Mb
#258	NEW    cov: 4665 ft: 7223 corp: 31/77b lim: 4 exec/s: 86 rss: 394Mb L: 2/4 MS: 1 ChangeBit-
#272	NEW    cov: 4665 ft: 7227 corp: 32/80b lim: 4 exec/s: 90 rss: 394Mb L: 3/4 MS: 4 ChangeBinInt-ChangeASCIIInt-ChangeByte-PersAutoDict- DE: "' "-
#278	NEW    cov: 4665 ft: 7254 corp: 33/84b lim: 4 exec/s: 92 rss: 395Mb L: 4/4 MS: 1 CrossOver-
#286	NEW    cov: 4666 ft: 7255 corp: 34/85b lim: 4 exec/s: 95 rss: 395Mb L: 1/4 MS: 3 CopyPart-EraseBytes-ChangeByte-
#288	NEW    cov: 4666 ft: 7272 corp: 35/88b lim: 4 exec/s: 96 rss: 395Mb L: 3/4 MS: 2 ChangeBit-ShuffleBytes-
#289	NEW    cov: 4666 ft: 7275 corp: 36/92b lim: 4 exec/s: 96 rss: 395Mb L: 4/4 MS: 1 ChangeBinInt-
#296	NEW    cov: 4667 ft: 7303 corp: 37/96b lim: 4 exec/s: 98 rss: 396Mb L: 4/4 MS: 2 EraseBytes-CopyPart-
#297	NEW    cov: 4667 ft: 7305 corp: 38/100b lim: 4 exec/s: 99 rss: 396Mb L: 4/4 MS: 1 PersAutoDict- DE: "' "-
#303	NEW    cov: 4669 ft: 7308 corp: 39/103b lim: 4 exec/s: 101 rss: 396Mb L: 3/4 MS: 1 InsertByte-
#320	NEW    cov: 4669 ft: 7309 corp: 40/107b lim: 4 exec/s: 106 rss: 397Mb L: 4/4 MS: 2 ChangeBinInt-CopyPart-
#323	NEW    cov: 4669 ft: 7317 corp: 41/110b lim: 4 exec/s: 107 rss: 397Mb L: 3/4 MS: 3 ShuffleBytes-ChangeByte-PersAutoDict- DE: "' "-
#354	NEW    cov: 4669 ft: 7321 corp: 42/114b lim: 4 exec/s: 118 rss: 397Mb L: 4/4 MS: 1 ChangeBit-
#370	REDUCE cov: 4669 ft: 7321 corp: 42/113b lim: 4 exec/s: 123 rss: 398Mb L: 1/4 MS: 1 EraseBytes-
#396	NEW    cov: 4669 ft: 7327 corp: 43/116b lim: 4 exec/s: 132 rss: 398Mb L: 3/4 MS: 1 ShuffleBytes-
#411	NEW    cov: 4669 ft: 7335 corp: 44/118b lim: 4 exec/s: 137 rss: 398Mb L: 2/4 MS: 5 ChangeByte-CrossOver-ChangeByte-CrossOver-CrossOver-
#435	NEW    cov: 4669 ft: 7336 corp: 45/121b lim: 4 exec/s: 145 rss: 400Mb L: 3/4 MS: 4 CopyPart-EraseBytes-PersAutoDict-ShuffleBytes- DE: "' "-
#457	REDUCE cov: 4669 ft: 7336 corp: 45/120b lim: 4 exec/s: 152 rss: 401Mb L: 1/4 MS: 2 ShuffleBytes-EraseBytes-
#482	NEW    cov: 4669 ft: 7338 corp: 46/124b lim: 4 exec/s: 160 rss: 401Mb L: 4/4 MS: 5 ChangeBinInt-ShuffleBytes-ChangeBit-CrossOver-CopyPart-
#483	NEW    cov: 4669 ft: 7340 corp: 47/128b lim: 4 exec/s: 161 rss: 401Mb L: 4/4 MS: 1 ChangeByte-
#490	NEW    cov: 4675 ft: 7346 corp: 48/132b lim: 4 exec/s: 163 rss: 402Mb L: 4/4 MS: 2 InsertByte-ChangeBinInt-
#512	pulse  cov: 4675 ft: 7346 corp: 48/132b lim: 4 exec/s: 128 rss: 402Mb
#523	NEW    cov: 4679 ft: 7350 corp: 49/136b lim: 4 exec/s: 130 rss: 402Mb L: 4/4 MS: 3 ChangeBinInt-ChangeBit-ChangeBit-
#564	NEW    cov: 4679 ft: 7351 corp: 50/138b lim: 4 exec/s: 141 rss: 402Mb L: 2/4 MS: 1 ChangeByte-
#568	NEW    cov: 4679 ft: 7353 corp: 51/142b lim: 4 exec/s: 142 rss: 402Mb L: 4/4 MS: 4 InsertByte-ChangeByte-ChangeBit-CopyPart-
#582	NEW    cov: 4679 ft: 7384 corp: 52/146b lim: 4 exec/s: 145 rss: 404Mb L: 4/4 MS: 4 CopyPart-PersAutoDict-CopyPart-ChangeBinInt- DE: "' "-
#627	NEW    cov: 4683 ft: 7388 corp: 53/150b lim: 4 exec/s: 156 rss: 405Mb L: 4/4 MS: 5 CopyPart-CopyPart-CopyPart-ChangeByte-CopyPart-
#658	NEW    cov: 4683 ft: 7392 corp: 54/153b lim: 4 exec/s: 164 rss: 405Mb L: 3/4 MS: 1 ChangeBit-
#701	REDUCE cov: 4684 ft: 7393 corp: 55/154b lim: 4 exec/s: 175 rss: 406Mb L: 1/4 MS: 3 ChangeBit-CopyPart-ChangeBit-
#782	NEW    cov: 4688 ft: 7397 corp: 56/158b lim: 4 exec/s: 195 rss: 407Mb L: 4/4 MS: 1 CrossOver-
#956	NEW    cov: 4697 ft: 7407 corp: 57/161b lim: 4 exec/s: 191 rss: 407Mb L: 3/4 MS: 4 CrossOver-ShuffleBytes-ChangeBit-CopyPart-
#1024	pulse  cov: 4697 ft: 7407 corp: 57/161b lim: 4 exec/s: 204 rss: 407Mb
#1037	REDUCE cov: 4697 ft: 7407 corp: 57/160b lim: 4 exec/s: 207 rss: 407Mb L: 3/4 MS: 1 EraseBytes-
#1104	REDUCE cov: 4697 ft: 7407 corp: 57/159b lim: 4 exec/s: 220 rss: 407Mb L: 2/4 MS: 2 ShuffleBytes-EraseBytes-
#1144	NEW    cov: 4703 ft: 7413 corp: 58/163b lim: 4 exec/s: 228 rss: 407Mb L: 4/4 MS: 5 ChangeByte-CopyPart-ShuffleBytes-ChangeASCIIInt-ChangeByte-
#1153	NEW    cov: 4705 ft: 7415 corp: 59/166b lim: 4 exec/s: 230 rss: 407Mb L: 3/4 MS: 4 ChangeBit-CrossOver-ChangeBit-CrossOver-
#1164	NEW    cov: 4705 ft: 7419 corp: 60/170b lim: 4 exec/s: 232 rss: 407Mb L: 4/4 MS: 1 ShuffleBytes-
#1235	NEW    cov: 4706 ft: 7420 corp: 61/174b lim: 4 exec/s: 205 rss: 407Mb L: 4/4 MS: 1 ChangeByte-
#1262	NEW    cov: 4706 ft: 7435 corp: 62/178b lim: 4 exec/s: 210 rss: 407Mb L: 4/4 MS: 2 CopyPart-CopyPart-
#1307	NEW    cov: 4706 ft: 7436 corp: 63/181b lim: 4 exec/s: 217 rss: 407Mb L: 3/4 MS: 5 ChangeBinInt-ShuffleBytes-ChangeBit-EraseBytes-InsertByte-
#1398	NEW    cov: 4707 ft: 7437 corp: 64/185b lim: 4 exec/s: 233 rss: 409Mb L: 4/4 MS: 1 ChangeByte-
#1501	NEW    cov: 4707 ft: 7440 corp: 65/189b lim: 4 exec/s: 250 rss: 411Mb L: 4/4 MS: 3 ShuffleBytes-ShuffleBytes-PersAutoDict- DE: "' "-
#1508	NEW    cov: 4707 ft: 7441 corp: 66/192b lim: 4 exec/s: 251 rss: 411Mb L: 3/4 MS: 2 ShuffleBytes-ChangeBit-
#1520	NEW    cov: 4707 ft: 7442 corp: 67/195b lim: 4 exec/s: 253 rss: 411Mb L: 3/4 MS: 2 ShuffleBytes-EraseBytes-
#1608	NEW    cov: 4712 ft: 7447 corp: 68/199b lim: 4 exec/s: 268 rss: 411Mb L: 4/4 MS: 3 InsertByte-ChangeBinInt-InsertByte-
#1678	NEW    cov: 4712 ft: 7448 corp: 69/201b lim: 4 exec/s: 279 rss: 411Mb L: 2/4 MS: 5 InsertByte-EraseBytes-InsertByte-ChangeBit-CopyPart-
#1692	NEW    cov: 4712 ft: 7449 corp: 70/205b lim: 4 exec/s: 282 rss: 411Mb L: 4/4 MS: 4 CrossOver-InsertByte-ChangeBinInt-CopyPart-
#1699	REDUCE cov: 4712 ft: 7449 corp: 70/204b lim: 4 exec/s: 242 rss: 411Mb L: 1/4 MS: 2 ChangeByte-EraseBytes-
#1780	REDUCE cov: 4712 ft: 7449 corp: 70/203b lim: 4 exec/s: 254 rss: 411Mb L: 1/4 MS: 1 EraseBytes-
#1783	NEW    cov: 4712 ft: 7451 corp: 71/207b lim: 4 exec/s: 254 rss: 411Mb L: 4/4 MS: 3 ChangeBit-PersAutoDict-ShuffleBytes- DE: "' "-
#1786	NEW    cov: 4712 ft: 7452 corp: 72/211b lim: 4 exec/s: 255 rss: 411Mb L: 4/4 MS: 3 ChangeBit-CrossOver-CrossOver-
#1788	NEW    cov: 4712 ft: 7454 corp: 73/215b lim: 4 exec/s: 255 rss: 411Mb L: 4/4 MS: 2 CrossOver-CopyPart-
#1905	NEW    cov: 4712 ft: 7455 corp: 74/218b lim: 4 exec/s: 272 rss: 411Mb L: 3/4 MS: 2 ShuffleBytes-ShuffleBytes-
#2012	NEW    cov: 4713 ft: 7458 corp: 75/220b lim: 4 exec/s: 287 rss: 411Mb L: 2/4 MS: 2 EraseBytes-ChangeByte-
#2036	NEW    cov: 4715 ft: 7460 corp: 76/223b lim: 4 exec/s: 290 rss: 411Mb L: 3/4 MS: 4 ChangeBinInt-ChangeByte-CrossOver-CrossOver-
#2048	pulse  cov: 4715 ft: 7460 corp: 76/223b lim: 4 exec/s: 292 rss: 411Mb
#2220	NEW    cov: 4730 ft: 7612 corp: 77/227b lim: 4 exec/s: 277 rss: 411Mb L: 4/4 MS: 3 ChangeByte-ChangeBinInt-ChangeBit-
	NEW_FUNC[1/1]: 0x000105db5eb0 in _RNvMs0_NtNtCs2BKWDpcBPTM_14regex_automata6hybrid3dfaNtB5_3DFA13match_pattern+0x0 (parser:arm64+0x100f71eb0)
#2322	NEW    cov: 4762 ft: 7680 corp: 78/231b lim: 4 exec/s: 290 rss: 411Mb L: 4/4 MS: 1 CopyPart-
#2433	NEW    cov: 4762 ft: 7682 corp: 79/235b lim: 4 exec/s: 304 rss: 411Mb L: 4/4 MS: 1 CopyPart-
#2642	REDUCE cov: 4762 ft: 7683 corp: 80/239b lim: 6 exec/s: 330 rss: 411Mb L: 4/4 MS: 4 ChangeByte-ChangeBit-PersAutoDict-ChangeByte- DE: "' "-
#2648	NEW    cov: 4765 ft: 7694 corp: 81/244b lim: 6 exec/s: 331 rss: 411Mb L: 5/5 MS: 1 CrossOver-
	NEW_FUNC[1/1]: 0x0001064e5b34 in _RNvMNtNtCsiVHoy8e5Glr_12aho_corasick6packed9rabinkarpNtB2_9RabinKarp6verify+0x0 (parser:arm64+0x1016a1b34)
#2649	NEW    cov: 4770 ft: 7699 corp: 82/248b lim: 6 exec/s: 331 rss: 411Mb L: 4/5 MS: 1 ChangeBit-
#2655	NEW    cov: 4770 ft: 7700 corp: 83/253b lim: 6 exec/s: 295 rss: 411Mb L: 5/5 MS: 1 InsertByte-
#2656	NEW    cov: 4770 ft: 7701 corp: 84/258b lim: 6 exec/s: 295 rss: 411Mb L: 5/5 MS: 1 InsertByte-
	NEW_FUNC[1/1]: 0x000105ed13f4 in _RNvXs2_NtNtCs2BKWDpcBPTM_14regex_automata4meta8strategyNtB5_4CoreNtB5_8Strategy11search_half+0x0 (parser:arm64+0x10108d3f4)
#2658	NEW    cov: 4810 ft: 8068 corp: 85/264b lim: 6 exec/s: 295 rss: 411Mb L: 6/6 MS: 2 CrossOver-CrossOver-
#2664	NEW    cov: 4810 ft: 8214 corp: 86/270b lim: 6 exec/s: 296 rss: 411Mb L: 6/6 MS: 1 CrossOver-
#2666	NEW    cov: 4810 ft: 8215 corp: 87/275b lim: 6 exec/s: 296 rss: 411Mb L: 5/6 MS: 2 PersAutoDict-InsertByte- DE: "' "-
#2682	NEW    cov: 4810 ft: 8219 corp: 88/280b lim: 6 exec/s: 298 rss: 411Mb L: 5/6 MS: 1 CrossOver-
#2683	REDUCE cov: 4810 ft: 8219 corp: 88/279b lim: 6 exec/s: 298 rss: 411Mb L: 3/6 MS: 1 EraseBytes-
#2771	NEW    cov: 4810 ft: 8220 corp: 89/285b lim: 6 exec/s: 307 rss: 411Mb L: 6/6 MS: 3 ChangeByte-InsertByte-CopyPart-
#2786	REDUCE cov: 4810 ft: 8379 corp: 90/291b lim: 6 exec/s: 309 rss: 411Mb L: 6/6 MS: 5 InsertByte-ChangeByte-CrossOver-ShuffleBytes-CopyPart-
#2796	NEW    cov: 4810 ft: 9082 corp: 91/297b lim: 6 exec/s: 310 rss: 411Mb L: 6/6 MS: 5 CopyPart-ShuffleBytes-ShuffleBytes-ShuffleBytes-CopyPart-
#2798	NEW    cov: 4810 ft: 9084 corp: 92/303b lim: 6 exec/s: 310 rss: 411Mb L: 6/6 MS: 2 ShuffleBytes-CrossOver-
#2801	NEW    cov: 4810 ft: 9086 corp: 93/308b lim: 6 exec/s: 311 rss: 411Mb L: 5/6 MS: 3 ShuffleBytes-ShuffleBytes-ChangeBinInt-
#2822	NEW    cov: 4810 ft: 9087 corp: 94/312b lim: 6 exec/s: 313 rss: 412Mb L: 4/6 MS: 1 InsertByte-
#2826	REDUCE cov: 4810 ft: 9093 corp: 95/317b lim: 6 exec/s: 314 rss: 412Mb L: 5/6 MS: 4 CrossOver-InsertByte-InsertByte-ChangeBit-
#2853	NEW    cov: 4810 ft: 9096 corp: 96/322b lim: 6 exec/s: 317 rss: 414Mb L: 5/6 MS: 2 CopyPart-InsertByte-
#2856	NEW    cov: 4810 ft: 9097 corp: 97/328b lim: 6 exec/s: 317 rss: 414Mb L: 6/6 MS: 3 InsertByte-ChangeByte-CopyPart-
#2994	NEW    cov: 4811 ft: 9098 corp: 98/334b lim: 6 exec/s: 332 rss: 415Mb L: 6/6 MS: 3 EraseBytes-ShuffleBytes-InsertRepeatedBytes-
#3005	REDUCE cov: 4811 ft: 9098 corp: 98/333b lim: 6 exec/s: 333 rss: 415Mb L: 2/6 MS: 1 EraseBytes-
#3045	NEW    cov: 4811 ft: 9099 corp: 99/339b lim: 6 exec/s: 338 rss: 415Mb L: 6/6 MS: 5 ChangeByte-PersAutoDict-ChangeBit-EraseBytes-InsertRepeatedBytes- DE: "' "-
#3091	NEW    cov: 4817 ft: 9109 corp: 100/343b lim: 6 exec/s: 343 rss: 415Mb L: 4/6 MS: 1 CopyPart-
#3117	NEW    cov: 4817 ft: 9111 corp: 101/348b lim: 6 exec/s: 311 rss: 415Mb L: 5/6 MS: 1 CopyPart-
#3156	NEW    cov: 4824 ft: 9118 corp: 102/354b lim: 6 exec/s: 315 rss: 415Mb L: 6/6 MS: 4 PersAutoDict-ChangeByte-CopyPart-ChangeBit- DE: "' "-
#3347	REDUCE cov: 4824 ft: 9118 corp: 102/353b lim: 6 exec/s: 334 rss: 415Mb L: 5/6 MS: 1 EraseBytes-
#3398	REDUCE cov: 4824 ft: 9118 corp: 102/352b lim: 6 exec/s: 339 rss: 415Mb L: 3/6 MS: 1 EraseBytes-
#3409	NEW    cov: 4824 ft: 9122 corp: 103/358b lim: 6 exec/s: 340 rss: 415Mb L: 6/6 MS: 1 PersAutoDict- DE: "' "-
#3424	NEW    cov: 4824 ft: 9123 corp: 104/364b lim: 6 exec/s: 342 rss: 415Mb L: 6/6 MS: 5 PersAutoDict-ShuffleBytes-CopyPart-CopyPart-CrossOver- DE: "' "-
#3462	NEW    cov: 4824 ft: 9129 corp: 105/370b lim: 6 exec/s: 346 rss: 415Mb L: 6/6 MS: 3 InsertByte-InsertByte-CrossOver-
#3476	NEW    cov: 4824 ft: 9130 corp: 106/376b lim: 6 exec/s: 347 rss: 415Mb L: 6/6 MS: 4 ShuffleBytes-ChangeBit-CrossOver-CopyPart-
#3627	NEW    cov: 4824 ft: 9131 corp: 107/382b lim: 6 exec/s: 329 rss: 419Mb L: 6/6 MS: 1 PersAutoDict- DE: "' "-
#3646	NEW    cov: 4824 ft: 9132 corp: 108/388b lim: 6 exec/s: 331 rss: 419Mb L: 6/6 MS: 4 ChangeByte-ChangeBit-CrossOver-ChangeByte-
#3708	NEW    cov: 4824 ft: 9134 corp: 109/394b lim: 6 exec/s: 337 rss: 419Mb L: 6/6 MS: 2 CopyPart-CrossOver-
#3724	NEW    cov: 4824 ft: 9153 corp: 110/400b lim: 6 exec/s: 338 rss: 419Mb L: 6/6 MS: 1 CrossOver-
#3867	NEW    cov: 4824 ft: 9155 corp: 111/406b lim: 6 exec/s: 322 rss: 419Mb L: 6/6 MS: 3 ShuffleBytes-ChangeBinInt-CrossOver-
#3913	REDUCE cov: 4824 ft: 9155 corp: 111/405b lim: 6 exec/s: 326 rss: 419Mb L: 1/6 MS: 1 EraseBytes-
#3971	NEW    cov: 4824 ft: 9156 corp: 112/411b lim: 6 exec/s: 330 rss: 419Mb L: 6/6 MS: 3 CopyPart-ShuffleBytes-ShuffleBytes-
#3995	REDUCE cov: 4824 ft: 9156 corp: 112/410b lim: 6 exec/s: 332 rss: 419Mb L: 3/6 MS: 4 ShuffleBytes-InsertByte-CrossOver-EraseBytes-
#4008	NEW    cov: 4825 ft: 9161 corp: 113/416b lim: 6 exec/s: 334 rss: 419Mb L: 6/6 MS: 3 InsertByte-InsertByte-ChangeByte-
#4025	NEW    cov: 4825 ft: 9167 corp: 114/422b lim: 6 exec/s: 335 rss: 419Mb L: 6/6 MS: 2 EraseBytes-CMP- DE: "\026\000"-
#4096	pulse  cov: 4825 ft: 9167 corp: 114/422b lim: 6 exec/s: 341 rss: 419Mb
#4112	REDUCE cov: 4825 ft: 9167 corp: 114/421b lim: 6 exec/s: 342 rss: 419Mb L: 3/6 MS: 2 ShuffleBytes-CrossOver-
#4132	NEW    cov: 4837 ft: 9188 corp: 115/427b lim: 6 exec/s: 317 rss: 419Mb L: 6/6 MS: 4 CopyPart-CopyPart-CrossOver-CopyPart-
#4338	NEW    cov: 4845 ft: 9197 corp: 116/435b lim: 8 exec/s: 333 rss: 419Mb L: 8/8 MS: 1 CopyPart-
#4372	NEW    cov: 4845 ft: 9198 corp: 117/443b lim: 8 exec/s: 336 rss: 419Mb L: 8/8 MS: 4 InsertRepeatedBytes-EraseBytes-InsertByte-InsertByte-
#4376	NEW    cov: 4845 ft: 9200 corp: 118/449b lim: 8 exec/s: 336 rss: 419Mb L: 6/8 MS: 4 PersAutoDict-ChangeBit-CMP-PersAutoDict- DE: "' "-"\000\000\000\000"-"' "-
#4404	NEW    cov: 4848 ft: 9203 corp: 119/457b lim: 8 exec/s: 338 rss: 419Mb L: 8/8 MS: 3 ChangeBit-CrossOver-CopyPart-
#4448	NEW    cov: 4848 ft: 9471 corp: 120/465b lim: 8 exec/s: 342 rss: 419Mb L: 8/8 MS: 4 ChangeByte-ChangeBit-CrossOver-CopyPart-
	NEW_FUNC[1/2]: 0x000105d520c4 in _RINvMNtNtCs2BKWDpcBPTM_14regex_automata4util6searchNtB3_5Input8set_spanNtB3_4SpanEB7_+0x0 (parser:arm64+0x100f0e0c4)
	NEW_FUNC[2/2]: 0x000105dc0e64 in _RNvMs1_NtNtCs2BKWDpcBPTM_14regex_automata6hybrid3dfaNtB5_5Cache12search_start+0x0 (parser:arm64+0x100f7ce64)
#4473	REDUCE cov: 4911 ft: 9628 corp: 121/473b lim: 8 exec/s: 344 rss: 419Mb L: 8/8 MS: 5 ChangeByte-ChangeBit-InsertByte-InsertByte-InsertRepeatedBytes-
#4476	NEW    cov: 4911 ft: 9629 corp: 122/481b lim: 8 exec/s: 344 rss: 419Mb L: 8/8 MS: 3 ChangeBinInt-CrossOver-CMP- DE: "\000\000"-
#4477	NEW    cov: 4911 ft: 9631 corp: 123/489b lim: 8 exec/s: 344 rss: 419Mb L: 8/8 MS: 1 CopyPart-
#4517	REDUCE cov: 4911 ft: 9634 corp: 124/496b lim: 8 exec/s: 347 rss: 419Mb L: 7/8 MS: 5 InsertByte-InsertRepeatedBytes-CrossOver-CrossOver-InsertRepeatedBytes-
#4524	NEW    cov: 4911 ft: 9636 corp: 125/504b lim: 8 exec/s: 348 rss: 419Mb L: 8/8 MS: 2 ChangeBit-ChangeBinInt-
#4540	REDUCE cov: 4911 ft: 9636 corp: 125/502b lim: 8 exec/s: 349 rss: 419Mb L: 4/8 MS: 1 EraseBytes-
#4542	NEW    cov: 4911 ft: 9647 corp: 126/508b lim: 8 exec/s: 349 rss: 419Mb L: 6/8 MS: 1 ShuffleBytes-
#4650	NEW    cov: 4916 ft: 9652 corp: 127/513b lim: 8 exec/s: 332 rss: 419Mb L: 5/8 MS: 3 ChangeBinInt-ShuffleBytes-PersAutoDict- DE: "' "-
#4662	NEW    cov: 4916 ft: 9655 corp: 128/519b lim: 8 exec/s: 333 rss: 419Mb L: 6/8 MS: 2 ShuffleBytes-CopyPart-
#4665	NEW    cov: 4918 ft: 9657 corp: 129/527b lim: 8 exec/s: 333 rss: 419Mb L: 8/8 MS: 3 CopyPart-PersAutoDict-CrossOver- DE: "\026\000"-
#4711	NEW    cov: 4918 ft: 9659 corp: 130/532b lim: 8 exec/s: 336 rss: 419Mb L: 5/8 MS: 1 CopyPart-
#4723	NEW    cov: 4918 ft: 9661 corp: 131/540b lim: 8 exec/s: 337 rss: 419Mb L: 8/8 MS: 2 InsertRepeatedBytes-ChangeBinInt-
#4772	NEW    cov: 4918 ft: 9664 corp: 132/548b lim: 8 exec/s: 340 rss: 419Mb L: 8/8 MS: 4 ChangeBit-CrossOver-ChangeBinInt-CopyPart-
#4779	REDUCE cov: 4918 ft: 9664 corp: 132/547b lim: 8 exec/s: 341 rss: 419Mb L: 4/8 MS: 2 EraseBytes-ShuffleBytes-
#4785	NEW    cov: 4918 ft: 9666 corp: 133/555b lim: 8 exec/s: 341 rss: 419Mb L: 8/8 MS: 1 CopyPart-
#4806	NEW    cov: 4919 ft: 10055 corp: 134/563b lim: 8 exec/s: 343 rss: 419Mb L: 8/8 MS: 1 CrossOver-
#4842	NEW    cov: 4920 ft: 10057 corp: 135/571b lim: 8 exec/s: 345 rss: 419Mb L: 8/8 MS: 1 InsertRepeatedBytes-
#4869	NEW    cov: 4920 ft: 10059 corp: 136/579b lim: 8 exec/s: 347 rss: 419Mb L: 8/8 MS: 2 CrossOver-CopyPart-
#4900	NEW    cov: 4920 ft: 10063 corp: 137/587b lim: 8 exec/s: 350 rss: 421Mb L: 8/8 MS: 1 PersAutoDict- DE: "\000\000"-
#5104	REDUCE cov: 4920 ft: 10067 corp: 138/595b lim: 8 exec/s: 340 rss: 421Mb L: 8/8 MS: 4 InsertByte-ChangeBinInt-EraseBytes-InsertRepeatedBytes-
#5155	REDUCE cov: 4920 ft: 10067 corp: 138/594b lim: 8 exec/s: 343 rss: 421Mb L: 3/8 MS: 1 EraseBytes-
#5289	NEW    cov: 4920 ft: 10068 corp: 139/601b lim: 8 exec/s: 352 rss: 421Mb L: 7/8 MS: 4 CopyPart-ShuffleBytes-PersAutoDict-CopyPart- DE: "' "-
#5301	NEW    cov: 4920 ft: 10083 corp: 140/608b lim: 8 exec/s: 353 rss: 421Mb L: 7/8 MS: 2 CopyPart-CopyPart-
#5304	NEW    cov: 4922 ft: 10088 corp: 141/616b lim: 8 exec/s: 353 rss: 421Mb L: 8/8 MS: 3 EraseBytes-PersAutoDict-CopyPart- DE: "\000\000"-
#5305	NEW    cov: 4925 ft: 10094 corp: 142/624b lim: 8 exec/s: 353 rss: 421Mb L: 8/8 MS: 1 PersAutoDict- DE: "' "-
#5356	NEW    cov: 4925 ft: 10098 corp: 143/630b lim: 8 exec/s: 334 rss: 421Mb L: 6/8 MS: 1 CopyPart-
#5377	NEW    cov: 4925 ft: 10099 corp: 144/636b lim: 8 exec/s: 336 rss: 421Mb L: 6/8 MS: 1 CopyPart-
#5428	NEW    cov: 4925 ft: 10105 corp: 145/644b lim: 8 exec/s: 339 rss: 421Mb L: 8/8 MS: 1 CopyPart-
#5564	NEW    cov: 4925 ft: 10106 corp: 146/652b lim: 8 exec/s: 347 rss: 421Mb L: 8/8 MS: 1 ChangeBinInt-
#5565	NEW    cov: 4925 ft: 10107 corp: 147/660b lim: 8 exec/s: 347 rss: 421Mb L: 8/8 MS: 1 CrossOver-
#5715	REDUCE cov: 4925 ft: 10110 corp: 148/668b lim: 8 exec/s: 357 rss: 421Mb L: 8/8 MS: 5 ChangeByte-CrossOver-InsertRepeatedBytes-ShuffleBytes-CopyPart-
#5736	NEW    cov: 4926 ft: 10112 corp: 149/676b lim: 8 exec/s: 337 rss: 421Mb L: 8/8 MS: 1 PersAutoDict- DE: "\026\000"-
#5750	NEW    cov: 4926 ft: 10113 corp: 150/684b lim: 8 exec/s: 338 rss: 421Mb L: 8/8 MS: 4 InsertRepeatedBytes-CopyPart-CrossOver-CrossOver-
#5831	NEW    cov: 4926 ft: 10116 corp: 151/688b lim: 8 exec/s: 343 rss: 421Mb L: 4/8 MS: 1 CopyPart-
#5892	NEW    cov: 4926 ft: 10118 corp: 152/696b lim: 8 exec/s: 346 rss: 421Mb L: 8/8 MS: 1 ChangeBit-
#5988	NEW    cov: 4926 ft: 10119 corp: 153/704b lim: 8 exec/s: 352 rss: 421Mb L: 8/8 MS: 1 CopyPart-
	NEW_FUNC[1/1]: 0x000105d93ca0 in _RINvNtCs88k1Tj5ijCL_4core3ptr14read_unalignedNtNtNtNtB4_9core_arch10arm_shared4neon9uint8x8_tECs2BKWDpcBPTM_14regex_automata+0x0 (parser:arm64+0x100f4fca0)
#6001	NEW    cov: 4931 ft: 10124 corp: 154/712b lim: 8 exec/s: 353 rss: 421Mb L: 8/8 MS: 3 CrossOver-ShuffleBytes-ChangeByte-
#6053	REDUCE cov: 4931 ft: 10124 corp: 154/710b lim: 8 exec/s: 356 rss: 421Mb L: 6/8 MS: 2 ChangeByte-EraseBytes-
#6099	NEW    cov: 4931 ft: 10125 corp: 155/718b lim: 8 exec/s: 358 rss: 421Mb L: 8/8 MS: 1 CMP- DE: "/\000"-
#6117	NEW    cov: 4935 ft: 10129 corp: 156/722b lim: 8 exec/s: 339 rss: 421Mb L: 4/8 MS: 3 CopyPart-ChangeByte-ChangeByte-
#6145	NEW    cov: 4935 ft: 10130 corp: 157/730b lim: 8 exec/s: 341 rss: 421Mb L: 8/8 MS: 3 CopyPart-CopyPart-ChangeBinInt-
#6152	NEW    cov: 4935 ft: 10131 corp: 158/736b lim: 8 exec/s: 341 rss: 421Mb L: 6/8 MS: 2 ChangeByte-CopyPart-
#6159	NEW    cov: 4935 ft: 10132 corp: 159/744b lim: 8 exec/s: 342 rss: 421Mb L: 8/8 MS: 2 ChangeByte-ChangeASCIIInt-
#6169	NEW    cov: 4935 ft: 10133 corp: 160/752b lim: 8 exec/s: 342 rss: 421Mb L: 8/8 MS: 5 EraseBytes-ChangeBinInt-InsertByte-InsertByte-CMP- DE: "\014\000\000\000"-
#6187	NEW    cov: 4935 ft: 10143 corp: 161/760b lim: 8 exec/s: 343 rss: 421Mb L: 8/8 MS: 3 ChangeByte-ChangeBinInt-CopyPart-
#6240	NEW    cov: 4935 ft: 10144 corp: 162/767b lim: 8 exec/s: 346 rss: 421Mb L: 7/8 MS: 3 CrossOver-CrossOver-InsertRepeatedBytes-
#6379	REDUCE cov: 4935 ft: 10144 corp: 162/766b lim: 8 exec/s: 354 rss: 421Mb L: 7/8 MS: 4 CrossOver-CrossOver-ChangeBit-CopyPart-
#6486	NEW    cov: 4940 ft: 10150 corp: 163/772b lim: 8 exec/s: 360 rss: 421Mb L: 6/8 MS: 2 CrossOver-EraseBytes-
#6775	REDUCE cov: 4940 ft: 10150 corp: 163/770b lim: 8 exec/s: 356 rss: 423Mb L: 4/8 MS: 4 InsertByte-ChangeByte-ChangeBit-EraseBytes-
#6946	NEW    cov: 4940 ft: 10156 corp: 164/778b lim: 8 exec/s: 347 rss: 423Mb L: 8/8 MS: 1 ChangeByte-
#7093	REDUCE cov: 4940 ft: 10156 corp: 164/775b lim: 8 exec/s: 322 rss: 423Mb L: 4/8 MS: 2 CopyPart-EraseBytes-
#7095	NEW    cov: 4940 ft: 10158 corp: 165/783b lim: 8 exec/s: 322 rss: 423Mb L: 8/8 MS: 2 ChangeBinInt-CopyPart-
#7208	NEW    cov: 4940 ft: 10160 corp: 166/791b lim: 8 exec/s: 313 rss: 423Mb L: 8/8 MS: 3 ChangeByte-CopyPart-CrossOver-
#7268	NEW    cov: 4940 ft: 10161 corp: 167/799b lim: 8 exec/s: 316 rss: 423Mb L: 8/8 MS: 5 InsertRepeatedBytes-CrossOver-CrossOver-PersAutoDict-CopyPart- DE: "' "-
#7577	NEW    cov: 4940 ft: 10168 corp: 168/808b lim: 11 exec/s: 329 rss: 423Mb L: 9/9 MS: 4 CopyPart-ChangeBinInt-ChangeBinInt-InsertRepeatedBytes-
#7580	NEW    cov: 4940 ft: 10172 corp: 169/818b lim: 11 exec/s: 329 rss: 423Mb L: 10/10 MS: 3 ChangeBit-CrossOver-PersAutoDict- DE: "' "-
#7619	NEW    cov: 4941 ft: 10175 corp: 170/828b lim: 11 exec/s: 317 rss: 423Mb L: 10/10 MS: 4 CrossOver-ShuffleBytes-ChangeBit-PersAutoDict- DE: "\000\000\000\000"-
#7636	NEW    cov: 4941 ft: 10176 corp: 171/836b lim: 11 exec/s: 318 rss: 423Mb L: 8/10 MS: 2 ChangeBit-ShuffleBytes-
#7639	NEW    cov: 4941 ft: 10182 corp: 172/847b lim: 11 exec/s: 318 rss: 423Mb L: 11/11 MS: 3 ChangeBit-PersAutoDict-CrossOver- DE: "\000\000"-
#7728	NEW    cov: 4941 ft: 10183 corp: 173/855b lim: 11 exec/s: 322 rss: 423Mb L: 8/11 MS: 4 ChangeBinInt-ChangeByte-CopyPart-ShuffleBytes-
#7774	REDUCE cov: 4941 ft: 10183 corp: 173/854b lim: 11 exec/s: 323 rss: 423Mb L: 6/11 MS: 1 EraseBytes-
#7871	REDUCE cov: 4941 ft: 10185 corp: 174/865b lim: 11 exec/s: 327 rss: 423Mb L: 11/11 MS: 2 CopyPart-CrossOver-
#7931	NEW    cov: 4941 ft: 10187 corp: 175/876b lim: 11 exec/s: 330 rss: 423Mb L: 11/11 MS: 5 CopyPart-EraseBytes-EraseBytes-ChangeByte-CrossOver-
#7932	NEW    cov: 4941 ft: 10189 corp: 176/887b lim: 11 exec/s: 317 rss: 423Mb L: 11/11 MS: 1 InsertRepeatedBytes-
#7973	NEW    cov: 4941 ft: 10190 corp: 177/896b lim: 11 exec/s: 318 rss: 423Mb L: 9/11 MS: 1 CrossOver-
#7999	NEW    cov: 4941 ft: 10192 corp: 178/899b lim: 11 exec/s: 319 rss: 423Mb L: 3/11 MS: 1 EraseBytes-
#8012	NEW    cov: 4941 ft: 10193 corp: 179/910b lim: 11 exec/s: 320 rss: 423Mb L: 11/11 MS: 3 InsertByte-PersAutoDict-InsertRepeatedBytes- DE: "\026\000"-
#8042	NEW    cov: 4941 ft: 10201 corp: 180/919b lim: 11 exec/s: 321 rss: 423Mb L: 9/11 MS: 5 ShuffleBytes-ChangeBit-ShuffleBytes-InsertByte-CrossOver-
#8070	NEW    cov: 4941 ft: 10204 corp: 181/925b lim: 11 exec/s: 322 rss: 423Mb L: 6/11 MS: 3 CopyPart-CopyPart-CopyPart-
#8102	NEW    cov: 4943 ft: 10206 corp: 182/935b lim: 11 exec/s: 324 rss: 423Mb L: 10/11 MS: 2 InsertByte-InsertByte-
#8138	NEW    cov: 4943 ft: 10404 corp: 183/946b lim: 11 exec/s: 325 rss: 423Mb L: 11/11 MS: 1 CrossOver-
#8192	pulse  cov: 4943 ft: 10404 corp: 183/946b lim: 11 exec/s: 315 rss: 423Mb
#8198	NEW    cov: 4943 ft: 10431 corp: 184/957b lim: 11 exec/s: 315 rss: 423Mb L: 11/11 MS: 5 CrossOver-ShuffleBytes-ChangeByte-InsertByte-CopyPart-
#8205	NEW    cov: 4944 ft: 10434 corp: 185/968b lim: 11 exec/s: 315 rss: 423Mb L: 11/11 MS: 2 InsertByte-InsertRepeatedBytes-
#8326	REDUCE cov: 4944 ft: 10434 corp: 185/967b lim: 11 exec/s: 320 rss: 423Mb L: 10/11 MS: 1 EraseBytes-
#8332	NEW    cov: 4944 ft: 10443 corp: 186/975b lim: 11 exec/s: 320 rss: 423Mb L: 8/11 MS: 1 CopyPart-
#8350	REDUCE cov: 4945 ft: 10444 corp: 187/984b lim: 11 exec/s: 321 rss: 423Mb L: 9/11 MS: 3 CrossOver-ChangeBit-ChangeByte-
#8603	NEW    cov: 4945 ft: 10447 corp: 188/995b lim: 11 exec/s: 318 rss: 423Mb L: 11/11 MS: 3 ChangeBinInt-PersAutoDict-CopyPart- DE: "/\000"-
#8689	NEW    cov: 4945 ft: 10471 corp: 189/1004b lim: 11 exec/s: 321 rss: 423Mb L: 9/11 MS: 1 CopyPart-
#8839	NEW    cov: 4946 ft: 10473 corp: 190/1015b lim: 11 exec/s: 327 rss: 423Mb L: 11/11 MS: 5 ChangeByte-CopyPart-CopyPart-CrossOver-ChangeBinInt-
#8846	NEW    cov: 4946 ft: 10474 corp: 191/1025b lim: 11 exec/s: 327 rss: 423Mb L: 10/11 MS: 2 InsertByte-InsertByte-
#9042	NEW    cov: 4946 ft: 10477 corp: 192/1033b lim: 11 exec/s: 322 rss: 423Mb L: 8/11 MS: 1 CopyPart-
#9096	REDUCE cov: 4946 ft: 10478 corp: 193/1042b lim: 11 exec/s: 324 rss: 423Mb L: 9/11 MS: 4 CopyPart-ShuffleBytes-CrossOver-CopyPart-
#9271	NEW    cov: 4946 ft: 10482 corp: 194/1050b lim: 11 exec/s: 331 rss: 423Mb L: 8/11 MS: 5 EraseBytes-CrossOver-ChangeBit-PersAutoDict-CMP- DE: "' "-"\000\000"-
#9392	REDUCE cov: 4946 ft: 10482 corp: 194/1047b lim: 11 exec/s: 323 rss: 423Mb L: 8/11 MS: 1 EraseBytes-
#9513	NEW    cov: 4946 ft: 10483 corp: 195/1050b lim: 11 exec/s: 328 rss: 423Mb L: 3/11 MS: 1 CopyPart-
#9635	NEW    cov: 4946 ft: 10485 corp: 196/1061b lim: 11 exec/s: 332 rss: 423Mb L: 11/11 MS: 2 CopyPart-InsertRepeatedBytes-
#9657	NEW    cov: 4946 ft: 10486 corp: 197/1067b lim: 11 exec/s: 333 rss: 423Mb L: 6/11 MS: 2 InsertByte-CopyPart-
#9738	REDUCE cov: 4946 ft: 10489 corp: 198/1075b lim: 11 exec/s: 335 rss: 423Mb L: 8/11 MS: 1 ChangeByte-
#9839	REDUCE cov: 4946 ft: 10489 corp: 198/1074b lim: 11 exec/s: 327 rss: 423Mb L: 1/11 MS: 1 EraseBytes-
#9928	NEW    cov: 4946 ft: 10494 corp: 199/1085b lim: 11 exec/s: 320 rss: 423Mb L: 11/11 MS: 4 CrossOver-CrossOver-InsertByte-CrossOver-
#9995	NEW    cov: 4946 ft: 10498 corp: 200/1091b lim: 11 exec/s: 322 rss: 423Mb L: 6/11 MS: 2 CopyPart-ShuffleBytes-
#10086	NEW    cov: 4946 ft: 10500 corp: 201/1102b lim: 11 exec/s: 325 rss: 423Mb L: 11/11 MS: 1 CrossOver-
#10409	NEW    cov: 4947 ft: 10502 corp: 202/1116b lim: 14 exec/s: 325 rss: 423Mb L: 14/14 MS: 3 ShuffleBytes-ChangeBinInt-InsertRepeatedBytes-
#10420	REDUCE cov: 4947 ft: 10502 corp: 202/1115b lim: 14 exec/s: 325 rss: 423Mb L: 9/14 MS: 1 CrossOver-
#10456	NEW    cov: 4947 ft: 10503 corp: 203/1120b lim: 14 exec/s: 326 rss: 423Mb L: 5/14 MS: 1 CopyPart-
#10535	NEW    cov: 4949 ft: 10558 corp: 204/1133b lim: 14 exec/s: 329 rss: 423Mb L: 13/14 MS: 4 ShuffleBytes-EraseBytes-InsertRepeatedBytes-ChangeByte-
#10606	NEW    cov: 4949 ft: 10563 corp: 205/1146b lim: 14 exec/s: 321 rss: 423Mb L: 13/14 MS: 1 CopyPart-
#10611	NEW    cov: 4949 ft: 10564 corp: 206/1159b lim: 14 exec/s: 321 rss: 423Mb L: 13/14 MS: 5 CopyPart-CopyPart-ShuffleBytes-CrossOver-InsertByte-
#10812	REDUCE cov: 4949 ft: 10566 corp: 207/1170b lim: 14 exec/s: 327 rss: 423Mb L: 11/14 MS: 1 CopyPart-
#10949	NEW    cov: 4949 ft: 10572 corp: 208/1178b lim: 14 exec/s: 331 rss: 423Mb L: 8/14 MS: 2 EraseBytes-CopyPart-
#11025	NEW    cov: 4955 ft: 10578 corp: 209/1186b lim: 14 exec/s: 324 rss: 423Mb L: 8/14 MS: 1 ChangeByte-
#11230	NEW    cov: 4955 ft: 10579 corp: 210/1195b lim: 14 exec/s: 330 rss: 423Mb L: 9/14 MS: 5 EraseBytes-EraseBytes-CrossOver-ShuffleBytes-CrossOver-
#11242	NEW    cov: 4955 ft: 10582 corp: 211/1207b lim: 14 exec/s: 330 rss: 423Mb L: 12/14 MS: 2 InsertRepeatedBytes-InsertByte-
#11253	REDUCE cov: 4955 ft: 10582 corp: 211/1203b lim: 14 exec/s: 330 rss: 423Mb L: 4/14 MS: 1 CrossOver-
#11330	NEW    cov: 4955 ft: 10609 corp: 212/1217b lim: 14 exec/s: 333 rss: 423Mb L: 14/14 MS: 2 CrossOver-CopyPart-
#11466	NEW    cov: 4956 ft: 10612 corp: 213/1222b lim: 14 exec/s: 327 rss: 423Mb L: 5/14 MS: 1 CrossOver-
	NEW_FUNC[1/11]: 0x000104e92bb8 in _RINvMs3_NtCsfrArhfcn9Tb_5alloc3stre7replaceReECsbR5C9kte1NK_13axhub_helpers+0x0 (parser:arm64+0x10004ebb8)
	NEW_FUNC[2/11]: 0x000104e998e0 in _RINvMsj_NtCsfrArhfcn9Tb_5alloc3vecINtB6_3VecNtNtB8_6string6StringE16extend_desugaredINtNtNtNtCs88k1Tj5ijCL_4core4iter8adapters4skip4SkipINtNtB6_9into_iter8IntoIterBG_EEECsbR5C9kte1NK_13axhub_helpers+0x0 (parser:arm64+0x1000558e0)
#11492	NEW    cov: 5208 ft: 12969 corp: 214/1228b lim: 14 exec/s: 328 rss: 423Mb L: 6/14 MS: 1 PersAutoDict- DE: "\014\000\000\000"-
#11506	NEW    cov: 5212 ft: 12973 corp: 215/1240b lim: 14 exec/s: 328 rss: 423Mb L: 12/14 MS: 4 EraseBytes-ChangeBit-CMP-CrossOver- DE: "\026\000"-
#11514	NEW    cov: 5212 ft: 12978 corp: 216/1254b lim: 14 exec/s: 328 rss: 423Mb L: 14/14 MS: 3 CrossOver-CrossOver-CrossOver-
#11710	NEW    cov: 5212 ft: 12979 corp: 217/1263b lim: 14 exec/s: 325 rss: 423Mb L: 9/14 MS: 1 CopyPart-
#11718	NEW    cov: 5213 ft: 12982 corp: 218/1274b lim: 14 exec/s: 325 rss: 423Mb L: 11/14 MS: 3 ChangeByte-ChangeBinInt-CrossOver-
#11824	NEW    cov: 5213 ft: 12983 corp: 219/1282b lim: 14 exec/s: 328 rss: 423Mb L: 8/14 MS: 1 ChangeBit-
#11871	NEW    cov: 5213 ft: 13244 corp: 220/1296b lim: 14 exec/s: 329 rss: 423Mb L: 14/14 MS: 2 PersAutoDict-InsertRepeatedBytes- DE: "' "-
#11937	NEW    cov: 5213 ft: 13245 corp: 221/1308b lim: 14 exec/s: 331 rss: 423Mb L: 12/14 MS: 1 InsertRepeatedBytes-
#11946	NEW    cov: 5213 ft: 13246 corp: 222/1322b lim: 14 exec/s: 331 rss: 423Mb L: 14/14 MS: 4 InsertRepeatedBytes-ShuffleBytes-ChangeBinInt-ChangeByte-
#12012	REDUCE cov: 5213 ft: 13246 corp: 222/1321b lim: 14 exec/s: 333 rss: 423Mb L: 5/14 MS: 1 EraseBytes-
#12023	NEW    cov: 5213 ft: 13247 corp: 223/1331b lim: 14 exec/s: 333 rss: 423Mb L: 10/14 MS: 1 CrossOver-
#12026	NEW    cov: 5213 ft: 13248 corp: 224/1345b lim: 14 exec/s: 334 rss: 423Mb L: 14/14 MS: 3 CrossOver-PersAutoDict-CopyPart- DE: "\026\000"-
#12037	NEW    cov: 5213 ft: 13249 corp: 225/1358b lim: 14 exec/s: 334 rss: 423Mb L: 13/14 MS: 1 InsertRepeatedBytes-
#12043	NEW    cov: 5213 ft: 13250 corp: 226/1364b lim: 14 exec/s: 334 rss: 423Mb L: 6/14 MS: 1 EraseBytes-
#12134	NEW    cov: 5213 ft: 13251 corp: 227/1376b lim: 14 exec/s: 327 rss: 423Mb L: 12/14 MS: 1 EraseBytes-
#12215	NEW    cov: 5213 ft: 13252 corp: 228/1385b lim: 14 exec/s: 330 rss: 423Mb L: 9/14 MS: 1 CrossOver-
#12256	NEW    cov: 5213 ft: 13254 corp: 229/1399b lim: 14 exec/s: 331 rss: 423Mb L: 14/14 MS: 1 CopyPart-
#12264	REDUCE cov: 5213 ft: 13255 corp: 230/1403b lim: 14 exec/s: 331 rss: 423Mb L: 4/14 MS: 3 CrossOver-CopyPart-InsertRepeatedBytes-
#12353	NEW    cov: 5213 ft: 13263 corp: 231/1410b lim: 14 exec/s: 325 rss: 423Mb L: 7/14 MS: 4 EraseBytes-CrossOver-EraseBytes-InsertByte-
#12394	NEW    cov: 5213 ft: 13267 corp: 232/1418b lim: 14 exec/s: 326 rss: 423Mb L: 8/14 MS: 1 CopyPart-
#12405	NEW    cov: 5213 ft: 13295 corp: 233/1424b lim: 14 exec/s: 326 rss: 423Mb L: 6/14 MS: 1 ChangeBit-
#12421	NEW    cov: 5213 ft: 13299 corp: 234/1430b lim: 14 exec/s: 326 rss: 423Mb L: 6/14 MS: 1 ChangeByte-
#12508	NEW    cov: 5214 ft: 13300 corp: 235/1442b lim: 14 exec/s: 329 rss: 423Mb L: 12/14 MS: 2 ChangeBit-PersAutoDict- DE: "\014\000\000\000"-
#12564	REDUCE cov: 5214 ft: 13300 corp: 235/1441b lim: 14 exec/s: 330 rss: 423Mb L: 4/14 MS: 1 EraseBytes-
#12590	REDUCE cov: 5214 ft: 13300 corp: 235/1440b lim: 14 exec/s: 331 rss: 423Mb L: 10/14 MS: 1 EraseBytes-
#12611	NEW    cov: 5214 ft: 13548 corp: 236/1454b lim: 14 exec/s: 331 rss: 423Mb L: 14/14 MS: 1 CopyPart-
#12689	NEW    cov: 5235 ft: 13599 corp: 237/1461b lim: 14 exec/s: 333 rss: 423Mb L: 7/14 MS: 3 PersAutoDict-ChangeBinInt-CopyPart- DE: "/\000"-
#12744	REDUCE cov: 5235 ft: 13599 corp: 237/1460b lim: 14 exec/s: 335 rss: 423Mb L: 13/14 MS: 5 ChangeBit-ShuffleBytes-CMP-ShuffleBytes-EraseBytes- DE: "\026\000"-
#12755	NEW    cov: 5235 ft: 13601 corp: 238/1474b lim: 14 exec/s: 335 rss: 423Mb L: 14/14 MS: 1 CrossOver-
#12771	NEW    cov: 5236 ft: 13617 corp: 239/1479b lim: 14 exec/s: 327 rss: 423Mb L: 5/14 MS: 1 EraseBytes-
#12782	NEW    cov: 5236 ft: 13620 corp: 240/1482b lim: 14 exec/s: 327 rss: 423Mb L: 3/14 MS: 1 CrossOver-
#12788	NEW    cov: 5270 ft: 13660 corp: 241/1492b lim: 14 exec/s: 327 rss: 423Mb L: 10/14 MS: 1 InsertRepeatedBytes-
#12862	NEW    cov: 5270 ft: 13661 corp: 242/1505b lim: 14 exec/s: 329 rss: 423Mb L: 13/14 MS: 4 InsertRepeatedBytes-ChangeBit-ChangeBit-CopyPart-
#12869	REDUCE cov: 5270 ft: 13662 corp: 243/1518b lim: 14 exec/s: 329 rss: 423Mb L: 13/14 MS: 2 ChangeBit-CrossOver-
#12888	NEW    cov: 5270 ft: 13663 corp: 244/1524b lim: 14 exec/s: 330 rss: 423Mb L: 6/14 MS: 4 ChangeBinInt-EraseBytes-ChangeBinInt-EraseBytes-
#12925	NEW    cov: 5270 ft: 13674 corp: 245/1538b lim: 14 exec/s: 331 rss: 423Mb L: 14/14 MS: 2 CopyPart-ChangeByte-
#12961	NEW    cov: 5271 ft: 13675 corp: 246/1542b lim: 14 exec/s: 332 rss: 423Mb L: 4/14 MS: 1 CrossOver-
#12968	REDUCE cov: 5271 ft: 13676 corp: 247/1555b lim: 14 exec/s: 332 rss: 423Mb L: 13/14 MS: 2 EraseBytes-InsertRepeatedBytes-
#12999	REDUCE cov: 5271 ft: 13676 corp: 247/1553b lim: 14 exec/s: 333 rss: 423Mb L: 11/14 MS: 1 EraseBytes-
#13052	NEW    cov: 5271 ft: 13682 corp: 248/1567b lim: 14 exec/s: 334 rss: 423Mb L: 14/14 MS: 3 CrossOver-InsertRepeatedBytes-CopyPart-
#13058	NEW    cov: 5271 ft: 13688 corp: 249/1580b lim: 14 exec/s: 334 rss: 423Mb L: 13/14 MS: 1 CopyPart-
#13078	NEW    cov: 5271 ft: 13689 corp: 250/1586b lim: 14 exec/s: 335 rss: 423Mb L: 6/14 MS: 5 CMP-ChangeBinInt-PersAutoDict-ChangeBit-ChangeByte- DE: "( \000\000"-"\014\000\000\000"-
#13085	REDUCE cov: 5271 ft: 13689 corp: 250/1585b lim: 14 exec/s: 335 rss: 423Mb L: 7/14 MS: 2 InsertRepeatedBytes-EraseBytes-
#13132	NEW    cov: 5271 ft: 13692 corp: 251/1592b lim: 14 exec/s: 336 rss: 423Mb L: 7/14 MS: 2 CrossOver-CrossOver-
#13153	REDUCE cov: 5271 ft: 13693 corp: 252/1605b lim: 14 exec/s: 337 rss: 423Mb L: 13/14 MS: 1 CopyPart-
#13208	NEW    cov: 5271 ft: 13694 corp: 253/1619b lim: 14 exec/s: 330 rss: 423Mb L: 14/14 MS: 5 CMP-PersAutoDict-CopyPart-CopyPart-CopyPart- DE: "\001\000\000\000\000\000\000\002"-"/\000"-
#13225	NEW    cov: 5273 ft: 13696 corp: 254/1630b lim: 14 exec/s: 330 rss: 423Mb L: 11/14 MS: 2 CrossOver-CopyPart-
#13291	NEW    cov: 5274 ft: 13697 corp: 255/1642b lim: 14 exec/s: 332 rss: 423Mb L: 12/14 MS: 1 CrossOver-
#13387	NEW    cov: 5274 ft: 13698 corp: 256/1656b lim: 14 exec/s: 334 rss: 423Mb L: 14/14 MS: 1 CopyPart-
#13398	NEW    cov: 5280 ft: 13858 corp: 257/1668b lim: 14 exec/s: 334 rss: 423Mb L: 12/14 MS: 1 CrossOver-
#13429	NEW    cov: 5294 ft: 13911 corp: 258/1682b lim: 14 exec/s: 335 rss: 423Mb L: 14/14 MS: 1 CMP- DE: "\007\000\000\000\000\000\000\000"-
#13440	NEW    cov: 5294 ft: 13912 corp: 259/1689b lim: 14 exec/s: 336 rss: 423Mb L: 7/14 MS: 1 InsertByte-
#13541	NEW    cov: 5309 ft: 13955 corp: 260/1699b lim: 14 exec/s: 330 rss: 423Mb L: 10/14 MS: 1 CopyPart-
#13606	NEW    cov: 5309 ft: 13956 corp: 261/1713b lim: 14 exec/s: 331 rss: 423Mb L: 14/14 MS: 5 ChangeByte-CrossOver-CopyPart-CrossOver-CrossOver-
#13608	NEW    cov: 5313 ft: 13982 corp: 262/1724b lim: 14 exec/s: 331 rss: 423Mb L: 11/14 MS: 2 ChangeBit-CrossOver-
#13626	REDUCE cov: 5315 ft: 14012 corp: 263/1735b lim: 14 exec/s: 332 rss: 423Mb L: 11/14 MS: 3 CrossOver-CrossOver-CrossOver-
#13884	REDUCE cov: 5315 ft: 14012 corp: 263/1734b lim: 14 exec/s: 322 rss: 423Mb L: 2/14 MS: 3 ChangeBit-ChangeBinInt-EraseBytes-
#13912	REDUCE cov: 5315 ft: 14012 corp: 263/1732b lim: 14 exec/s: 323 rss: 423Mb L: 12/14 MS: 3 ShuffleBytes-ChangeBit-EraseBytes-
#14014	NEW    cov: 5317 ft: 14062 corp: 264/1746b lim: 14 exec/s: 325 rss: 423Mb L: 14/14 MS: 2 InsertByte-InsertRepeatedBytes-
#14017	REDUCE cov: 5317 ft: 14064 corp: 265/1760b lim: 14 exec/s: 325 rss: 423Mb L: 14/14 MS: 3 CrossOver-CopyPart-CopyPart-
#14019	NEW    cov: 5317 ft: 14070 corp: 266/1772b lim: 14 exec/s: 326 rss: 423Mb L: 12/14 MS: 2 ChangeBit-CrossOver-
#14130	NEW    cov: 5318 ft: 14083 corp: 267/1786b lim: 14 exec/s: 328 rss: 423Mb L: 14/14 MS: 1 ChangeBinInt-
#14143	NEW    cov: 5318 ft: 14093 corp: 268/1799b lim: 14 exec/s: 328 rss: 423Mb L: 13/14 MS: 3 CopyPart-CopyPart-InsertRepeatedBytes-
#14179	NEW    cov: 5319 ft: 14099 corp: 269/1807b lim: 14 exec/s: 329 rss: 423Mb L: 8/14 MS: 1 EraseBytes-
#14231	REDUCE cov: 5319 ft: 14099 corp: 269/1806b lim: 14 exec/s: 330 rss: 423Mb L: 2/14 MS: 2 CopyPart-EraseBytes-
#14276	NEW    cov: 5324 ft: 14104 corp: 270/1817b lim: 14 exec/s: 324 rss: 423Mb L: 11/14 MS: 5 ChangeByte-EraseBytes-ChangeBinInt-PersAutoDict-CopyPart- DE: "\000\000\000\000"-
#14497	NEW    cov: 5326 ft: 14110 corp: 271/1827b lim: 14 exec/s: 329 rss: 423Mb L: 10/14 MS: 1 ChangeByte-
#14503	NEW    cov: 5326 ft: 14111 corp: 272/1841b lim: 14 exec/s: 329 rss: 423Mb L: 14/14 MS: 1 CopyPart-
#14614	NEW    cov: 5326 ft: 14126 corp: 273/1852b lim: 14 exec/s: 332 rss: 423Mb L: 11/14 MS: 1 CopyPart-
#14660	NEW    cov: 5326 ft: 14127 corp: 274/1864b lim: 14 exec/s: 325 rss: 423Mb L: 12/14 MS: 1 CopyPart-
#14819	REDUCE cov: 5326 ft: 14131 corp: 275/1875b lim: 14 exec/s: 329 rss: 423Mb L: 11/14 MS: 4 InsertByte-InsertRepeatedBytes-CrossOver-ChangeByte-
#14887	NEW    cov: 5326 ft: 14133 corp: 276/1889b lim: 14 exec/s: 330 rss: 423Mb L: 14/14 MS: 3 CrossOver-ChangeASCIIInt-CrossOver-
#15023	NEW    cov: 5326 ft: 14134 corp: 277/1898b lim: 14 exec/s: 326 rss: 423Mb L: 9/14 MS: 1 CopyPart-
#15080	NEW    cov: 5326 ft: 14136 corp: 278/1909b lim: 14 exec/s: 327 rss: 423Mb L: 11/14 MS: 2 ShuffleBytes-EraseBytes-
#15092	NEW    cov: 5326 ft: 14137 corp: 279/1921b lim: 14 exec/s: 328 rss: 423Mb L: 12/14 MS: 2 CopyPart-CrossOver-
#15128	NEW    cov: 5326 ft: 14138 corp: 280/1931b lim: 14 exec/s: 328 rss: 423Mb L: 10/14 MS: 1 EraseBytes-
#15136	NEW    cov: 5326 ft: 14149 corp: 281/1938b lim: 14 exec/s: 329 rss: 423Mb L: 7/14 MS: 3 ChangeBinInt-ShuffleBytes-ChangeByte-
#15253	REDUCE cov: 5326 ft: 14149 corp: 281/1937b lim: 14 exec/s: 331 rss: 423Mb L: 12/14 MS: 2 ChangeBit-EraseBytes-
#15274	NEW    cov: 5326 ft: 14150 corp: 282/1951b lim: 14 exec/s: 332 rss: 423Mb L: 14/14 MS: 1 CopyPart-
#15296	NEW    cov: 5326 ft: 14155 corp: 283/1963b lim: 14 exec/s: 332 rss: 423Mb L: 12/14 MS: 2 PersAutoDict-CrossOver- DE: "/\000"-
#15412	NEW    cov: 5326 ft: 14157 corp: 284/1974b lim: 14 exec/s: 327 rss: 423Mb L: 11/14 MS: 1 ChangeBit-
#15418	NEW    cov: 5326 ft: 14159 corp: 285/1986b lim: 14 exec/s: 328 rss: 423Mb L: 12/14 MS: 1 CopyPart-
#15450	REDUCE cov: 5326 ft: 14159 corp: 285/1985b lim: 14 exec/s: 328 rss: 423Mb L: 2/14 MS: 2 ShuffleBytes-EraseBytes-
#15470	NEW    cov: 5326 ft: 14169 corp: 286/1995b lim: 14 exec/s: 329 rss: 423Mb L: 10/14 MS: 4 ShuffleBytes-ChangeByte-CrossOver-ChangeBit-
#15527	REDUCE cov: 5326 ft: 14171 corp: 287/2009b lim: 14 exec/s: 330 rss: 423Mb L: 14/14 MS: 2 CopyPart-InsertRepeatedBytes-
#15607	REDUCE cov: 5326 ft: 14171 corp: 287/2007b lim: 14 exec/s: 332 rss: 423Mb L: 4/14 MS: 5 ShuffleBytes-ChangeByte-InsertByte-ChangeByte-EraseBytes-
#15695	REDUCE cov: 5326 ft: 14171 corp: 287/2004b lim: 14 exec/s: 326 rss: 423Mb L: 10/14 MS: 3 ChangeBinInt-EraseBytes-CrossOver-
#15700	NEW    cov: 5326 ft: 14175 corp: 288/2016b lim: 14 exec/s: 327 rss: 423Mb L: 12/14 MS: 5 PersAutoDict-CMP-ChangeBinInt-ShuffleBytes-CrossOver- DE: "' "-"\000\000\000\000\000\000\000\004"-
#15701	REDUCE cov: 5326 ft: 14175 corp: 288/2015b lim: 14 exec/s: 327 rss: 423Mb L: 13/14 MS: 1 EraseBytes-
	NEW_FUNC[1/1]: 0x000105e5efc8 in _RNvMsj_NtNtCs2BKWDpcBPTM_14regex_automata4util6searchNtB5_10MatchError4quit+0x0 (parser:arm64+0x10101afc8)
#15802	NEW    cov: 5393 ft: 14280 corp: 289/2027b lim: 14 exec/s: 329 rss: 423Mb L: 12/14 MS: 1 CrossOver-
#15823	NEW    cov: 5393 ft: 14281 corp: 290/2039b lim: 14 exec/s: 329 rss: 423Mb L: 12/14 MS: 1 ChangeBinInt-
#16070	NEW    cov: 5397 ft: 14294 corp: 291/2053b lim: 14 exec/s: 334 rss: 423Mb L: 14/14 MS: 2 ChangeByte-PersAutoDict- DE: "\014\000\000\000"-
#16088	NEW    cov: 5397 ft: 14295 corp: 292/2064b lim: 14 exec/s: 328 rss: 423Mb L: 11/14 MS: 3 CrossOver-PersAutoDict-ShuffleBytes- DE: "( \000\000"-
#16124	NEW    cov: 5399 ft: 14388 corp: 293/2077b lim: 14 exec/s: 329 rss: 423Mb L: 13/14 MS: 1 CrossOver-
#16161	NEW    cov: 5401 ft: 14390 corp: 294/2082b lim: 14 exec/s: 329 rss: 423Mb L: 5/14 MS: 2 ChangeBinInt-EraseBytes-
#16202	NEW    cov: 5401 ft: 14393 corp: 295/2095b lim: 14 exec/s: 330 rss: 423Mb L: 13/14 MS: 1 InsertByte-
#16318	NEW    cov: 5405 ft: 14397 corp: 296/2107b lim: 14 exec/s: 333 rss: 423Mb L: 12/14 MS: 1 ChangeByte-
#16319	NEW    cov: 5405 ft: 14414 corp: 297/2117b lim: 14 exec/s: 333 rss: 423Mb L: 10/14 MS: 1 EraseBytes-
#16321	REDUCE cov: 5406 ft: 14427 corp: 298/2127b lim: 14 exec/s: 333 rss: 423Mb L: 10/14 MS: 2 CrossOver-CrossOver-
#16337	NEW    cov: 5406 ft: 14428 corp: 299/2140b lim: 14 exec/s: 333 rss: 423Mb L: 13/14 MS: 1 ChangeByte-
#16363	NEW    cov: 5407 ft: 14452 corp: 300/2151b lim: 14 exec/s: 333 rss: 423Mb L: 11/14 MS: 1 CopyPart-
#16384	pulse  cov: 5407 ft: 14452 corp: 300/2151b lim: 14 exec/s: 327 rss: 423Mb
#16415	NEW    cov: 5407 ft: 14453 corp: 301/2165b lim: 14 exec/s: 321 rss: 423Mb L: 14/14 MS: 2 CopyPart-CopyPart-
#16720	NEW    cov: 5407 ft: 14455 corp: 302/2178b lim: 14 exec/s: 298 rss: 423Mb L: 13/14 MS: 5 ShuffleBytes-ShuffleBytes-ShuffleBytes-InsertByte-InsertByte-
#16721	NEW    cov: 5410 ft: 14458 corp: 303/2192b lim: 14 exec/s: 298 rss: 423Mb L: 14/14 MS: 1 PersAutoDict- DE: "' "-
#16792	REDUCE cov: 5410 ft: 14458 corp: 303/2191b lim: 14 exec/s: 299 rss: 423Mb L: 3/14 MS: 1 EraseBytes-
#16900	NEW    cov: 5410 ft: 14460 corp: 304/2203b lim: 14 exec/s: 301 rss: 423Mb L: 12/14 MS: 3 InsertRepeatedBytes-ChangeBinInt-CrossOver-
#16916	REDUCE cov: 5410 ft: 14460 corp: 304/2201b lim: 14 exec/s: 302 rss: 423Mb L: 12/14 MS: 1 EraseBytes-
#16977	NEW    cov: 5410 ft: 14461 corp: 305/2213b lim: 14 exec/s: 303 rss: 423Mb L: 12/14 MS: 1 CopyPart-
#17015	NEW    cov: 5410 ft: 14462 corp: 306/2225b lim: 14 exec/s: 298 rss: 423Mb L: 12/14 MS: 3 ShuffleBytes-ChangeBit-CopyPart-
#17144	NEW    cov: 5410 ft: 14463 corp: 307/2239b lim: 14 exec/s: 300 rss: 423Mb L: 14/14 MS: 4 EraseBytes-CopyPart-CrossOver-CopyPart-
#17201	NEW    cov: 5410 ft: 14470 corp: 308/2245b lim: 14 exec/s: 301 rss: 423Mb L: 6/14 MS: 2 CopyPart-CrossOver-
#17500	NEW    cov: 5410 ft: 14471 corp: 309/2255b lim: 14 exec/s: 301 rss: 423Mb L: 10/14 MS: 4 PersAutoDict-ShuffleBytes-ChangeBinInt-EraseBytes- DE: "\026\000"-
#17541	REDUCE cov: 5410 ft: 14471 corp: 309/2254b lim: 14 exec/s: 302 rss: 423Mb L: 10/14 MS: 1 EraseBytes-
#17547	NEW    cov: 5410 ft: 14472 corp: 310/2268b lim: 14 exec/s: 302 rss: 423Mb L: 14/14 MS: 1 CopyPart-
#17604	NEW    cov: 5410 ft: 14473 corp: 311/2276b lim: 14 exec/s: 303 rss: 423Mb L: 8/14 MS: 2 ChangeBinInt-EraseBytes-
#17616	REDUCE cov: 5410 ft: 14473 corp: 311/2275b lim: 14 exec/s: 303 rss: 423Mb L: 8/14 MS: 2 InsertRepeatedBytes-EraseBytes-
#17688	REDUCE cov: 5410 ft: 14473 corp: 311/2270b lim: 14 exec/s: 299 rss: 423Mb L: 8/14 MS: 2 ShuffleBytes-EraseBytes-
#17754	REDUCE cov: 5410 ft: 14474 corp: 312/2277b lim: 14 exec/s: 300 rss: 423Mb L: 7/14 MS: 1 CopyPart-
#17760	NEW    cov: 5410 ft: 14480 corp: 313/2290b lim: 14 exec/s: 301 rss: 423Mb L: 13/14 MS: 1 CopyPart-
#17907	REDUCE cov: 5410 ft: 14480 corp: 313/2289b lim: 14 exec/s: 303 rss: 423Mb L: 8/14 MS: 2 ChangeBit-EraseBytes-
#17956	REDUCE cov: 5410 ft: 14480 corp: 313/2286b lim: 14 exec/s: 304 rss: 423Mb L: 6/14 MS: 4 ChangeByte-CopyPart-ChangeBinInt-EraseBytes-
#17994	REDUCE cov: 5410 ft: 14480 corp: 313/2282b lim: 14 exec/s: 304 rss: 423Mb L: 8/14 MS: 3 ChangeByte-InsertByte-EraseBytes-
#18010	NEW    cov: 5410 ft: 14482 corp: 314/2293b lim: 14 exec/s: 305 rss: 423Mb L: 11/14 MS: 1 ShuffleBytes-
#18024	NEW    cov: 5410 ft: 14484 corp: 315/2307b lim: 14 exec/s: 300 rss: 423Mb L: 14/14 MS: 4 ChangeByte-CopyPart-ShuffleBytes-PersAutoDict- DE: "\000\000"-
#18123	NEW    cov: 5410 ft: 14505 corp: 316/2318b lim: 14 exec/s: 302 rss: 423Mb L: 11/14 MS: 4 PersAutoDict-CopyPart-ChangeBinInt-CrossOver- DE: "' "-
#18240	REDUCE cov: 5410 ft: 14505 corp: 316/2317b lim: 14 exec/s: 304 rss: 423Mb L: 7/14 MS: 2 InsertByte-EraseBytes-
#18296	NEW    cov: 5410 ft: 14605 corp: 317/2331b lim: 14 exec/s: 304 rss: 423Mb L: 14/14 MS: 1 CopyPart-
#18347	REDUCE cov: 5410 ft: 14605 corp: 317/2330b lim: 14 exec/s: 305 rss: 423Mb L: 10/14 MS: 1 EraseBytes-
#18352	DONE   cov: 5410 ft: 14605 corp: 317/2330b lim: 14 exec/s: 300 rss: 423Mb
###### Recommended dictionary. ######
"' " # Uses: 560
"\026\000" # Uses: 212
"\000\000\000\000" # Uses: 143
"\000\000" # Uses: 208
"/\000" # Uses: 170
"\014\000\000\000" # Uses: 133
"( \000\000" # Uses: 50
"\001\000\000\000\000\000\000\002" # Uses: 21
"\007\000\000\000\000\000\000\000" # Uses: 23
"\000\000\000\000\000\000\000\004" # Uses: 14
###### End of recommended dictionary. ######
Done 18352 runs in 61 second(s)
```

## Post-external-fix regression gate
timestamp_utc=2026-04-29T06:31:52Z
```
post_external_fix_regression_exit=0
    Checking memchr v2.8.0
    Checking itoa v1.0.18
    Checking stable_deref_trait v1.2.1
    Checking pin-project-lite v0.2.17
    Checking libc v0.2.186
    Checking bytes v1.11.1
    Checking cfg-if v1.0.4
    Checking zerofrom v0.1.7
    Checking futures-core v0.3.32
    Checking num-traits v0.2.19
    Checking serde_core v1.0.228
    Checking futures-sink v0.3.32
   Compiling num-conv v0.2.1
    Checking litemap v0.8.2
    Checking yoke v0.8.2
    Checking powerfmt v0.2.0
    Checking smallvec v1.15.1
    Checking writeable v0.6.3
   Compiling time-core v0.1.8
    Checking deranged v0.5.8
    Checking zerovec v0.11.6
    Checking mio v1.2.0
    Checking socket2 v0.6.3
   Compiling time-macros v0.2.27
    Checking getrandom v0.2.17
    Checking zerotrie v0.2.4
    Checking http v1.4.0
    Checking untrusted v0.9.0
    Checking once_cell v1.21.4
    Checking zeroize v1.8.2
    Checking utf8_iter v1.0.4
    Checking subtle v2.6.1
    Checking ring v0.17.14
    Checking rustls-pki-types v1.14.1
    Checking percent-encoding v2.3.2
    Checking minimal-lexical v0.2.1
    Checking tokio v1.52.1
    Checking slab v0.4.12
    Checking typenum v1.20.0
    Checking futures-task v0.3.32
    Checking futures-io v0.3.32
    Checking futures-util v0.3.32
    Checking nom v7.1.3
    Checking tinystr v0.8.3
    Checking icu_locale_core v2.2.0
    Checking potential_utf v0.1.5
    Checking icu_collections v2.2.0
    Checking http-body v1.0.1
    Checking generic-array v0.14.7
    Checking icu_properties_data v2.2.0
    Checking time v0.3.47
    Checking icu_normalizer_data v2.2.0
    Checking num-integer v0.1.46
    Checking base64 v0.22.1
    Checking icu_provider v2.2.0
    Checking tower-service v0.3.3
    Checking try-lock v0.2.5
    Checking icu_properties v2.2.0
    Checking rustls-webpki v0.103.13
    Checking want v0.3.1
    Checking icu_normalizer v2.2.0
    Checking num-bigint v0.4.6
    Checking serde v1.0.228
    Checking thiserror v1.0.69
    Checking rusticata-macros v4.1.0
    Checking httparse v1.10.1
    Checking tracing-core v0.1.36
    Checking futures-channel v0.3.32
    Checking zmij v1.0.21
    Checking utf8parse v0.2.2
    Checking atomic-waker v1.1.2
    Checking serde_json v1.0.149
    Checking anstyle-parse v1.0.0
    Checking tracing v0.1.44
    Checking rustls v0.23.40
    Checking block-buffer v0.10.4
    Checking crypto-common v0.1.7
    Checking form_urlencoded v1.2.2
   Compiling anyhow v1.0.102
    Checking sync_wrapper v1.0.2
    Checking tower-layer v0.3.3
    Checking anstyle-query v1.1.5
    Checking anstyle v1.0.14
    Checking ipnet v2.12.0
    Checking colorchoice v1.0.5
    Checking is_terminal_polyfill v1.70.2
    Checking thiserror v2.0.18
    Checking anstream v1.0.0
    Checking digest v0.10.7
    Checking asn1-rs v0.6.2
    Checking webpki-roots v1.0.7
    Checking idna_adapter v1.2.2
    Checking aho-corasick v1.1.4
    Checking idna v1.1.0
    Checking ryu v1.0.23
    Checking regex-syntax v0.8.10
    Checking tinyvec_macros v0.1.1
    Checking strsim v0.11.1
    Checking bitflags v2.11.1
    Checking clap_lex v1.1.0
    Checking iri-string v0.7.12
    Checking core-foundation-sys v0.8.7
    Checking url v2.5.8
    Checking iana-time-zone v0.1.65
    Checking clap_builder v4.6.0
    Checking tinyvec v1.11.0
    Checking serde_urlencoded v0.7.1
    Checking simple_asn1 v0.6.4
    Checking getrandom v0.4.2
    Checking pem v3.0.6
    Checking http-body-util v0.1.3
    Checking cpufeatures v0.2.17
    Checking lazy_static v1.5.0
    Checking log v0.4.29
    Checking der-parser v9.0.0
    Checking oid-registry v0.7.1
    Checking hyper v1.9.0
    Checking tower v0.5.3
    Checking data-encoding v2.11.0
    Checking sha2 v0.10.9
    Checking x509-parser v0.16.0
    Checking jsonwebtoken v9.3.1
    Checking uuid v1.23.1
    Checking unicode-normalization v0.1.25
    Checking tower-http v0.6.8
    Checking hyper-util v0.1.20
    Checking chrono v0.4.44
    Checking hmac v0.12.1
    Checking semver v1.0.28
    Checking axhub-codegen v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-codegen)
    Checking regex-automata v0.4.14
    Checking clap v4.6.1
   Compiling axhub-helpers v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-helpers)
    Checking tokio-rustls v0.26.4
    Checking hyper-rustls v0.27.9
    Checking reqwest v0.12.28
    Checking regex v1.12.3
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.01s
   Compiling axhub-codegen v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-codegen)
   Compiling axhub-helpers v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-helpers)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.90s
     Running unittests src/lib.rs (target/debug/deps/axhub_codegen-b0eff76fcb439a60)

running 5 tests
test tests::extracts_catalog_entries ... ok
test tests::rejects_unterminated_strings_and_invalid_field_names ... ok
test tests::reports_actionable_parse_errors_for_missing_catalog_and_bad_shapes ... ok
test tests::generate_catalog_json_is_pretty_json_for_build_script_consumers ... ok
test tests::extracts_entries_with_comments_commas_escapes_and_quoted_fields ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/axhub_helpers-f986a2bad6baf65c)

running 4 tests
test catalog::tests::generated_catalog_has_expected_entries ... ok
test list_deployments::tests::rustls_crypto_provider_is_unambiguous_without_proxy_override ... ok
test preflight::tests::semver_drops_prerelease_and_build_like_ts ... ok
test redact::tests::strips_unicode_and_redacts_secrets ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/axhub_helpers-b878d96427c6292b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/cli_e2e.rs (target/debug/deps/cli_e2e-73ccc94306f91659)

running 3 tests
test cli_version_help_redact_and_classify_work ... ok
test cli_usage_preflight_resolve_list_and_session_start_paths_are_stable ... ok
test cli_consent_and_preauth_e2e_preserve_permission_contract ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s

     Running tests/phase_parity.rs (target/debug/deps/phase_parity-eea71f694f267d94)

running 16 tests
test catalog_classifies_base_subclassified_and_default_entries ... ok
test keychain_parses_go_keyring_envelope ... ok
test consent_rejects_symlink_and_world_readable_private_files_on_unix ... ok
test resolve_filters_apps_and_preserves_git_context_for_errors ... ok
test preflight_semver_auth_and_exit_precedence_match_ts ... ok
test redact_matches_typescript_secret_and_unicode_contract ... ok
test consent_parser_recognizes_nested_shell_destructive_intents_and_ignores_safe_commands ... ok
test consent_locks_zero_leeway_binding_mismatch_and_parser_hardening ... ok
test keychain_runner_maps_platform_success_missing_parse_error_and_unsupported ... ok
test list_deployments_covers_token_endpoint_http_and_error_matrix ... ok
test list_deployments_maps_auth_not_found_success_and_proxy_skip ... ok
test preflight_covers_auth_shapes_env_cache_and_cli_absence ... ok
test resolve_covers_arg_parsing_auth_parse_ambiguity_and_not_found_paths ... ok
test windows_keychain_runner_covers_success_and_failure_guidance ... ok
test spawn_sync_covers_empty_command_and_successful_child_output ... ok
test telemetry_is_opt_in_private_jsonl_and_error_swallowing ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s

   Doc-tests axhub_codegen

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests axhub_helpers

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

info: cargo-llvm-cov currently setting cfg(coverage); you can opt-out it by passing --no-cfg-coverage
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling unicode-ident v1.0.24
   Compiling libc v0.2.186
   Compiling itoa v1.0.18
   Compiling memchr v2.8.0
   Compiling stable_deref_trait v1.2.1
   Compiling serde_core v1.0.228
   Compiling cfg-if v1.0.4
   Compiling autocfg v1.5.0
   Compiling pin-project-lite v0.2.17
   Compiling shlex v1.3.0
   Compiling find-msvc-tools v0.1.9
   Compiling bytes v1.11.1
   Compiling zmij v1.0.21
   Compiling futures-core v0.3.32
   Compiling cc v1.2.61
   Compiling serde v1.0.228
   Compiling num-traits v0.2.19
   Compiling serde_json v1.0.149
   Compiling time-core v0.1.8
   Compiling futures-sink v0.3.32
   Compiling powerfmt v0.2.0
   Compiling num-conv v0.2.1
   Compiling version_check v0.9.5
   Compiling once_cell v1.21.4
   Compiling writeable v0.6.3
   Compiling smallvec v1.15.1
   Compiling litemap v0.8.2
   Compiling generic-array v0.14.7
   Compiling time-macros v0.2.27
   Compiling deranged v0.5.8
   Compiling ring v0.17.14
   Compiling syn v2.0.117
   Compiling mio v1.2.0
   Compiling socket2 v0.6.3
   Compiling getrandom v0.2.17
   Compiling http v1.4.0
   Compiling tokio v1.52.1
   Compiling zeroize v1.8.2
   Compiling icu_normalizer_data v2.2.0
   Compiling icu_properties_data v2.2.0
   Compiling utf8_iter v1.0.4
   Compiling untrusted v0.9.0
   Compiling subtle v2.6.1
   Compiling rustls-pki-types v1.14.1
   Compiling time v0.3.47
   Compiling http-body v1.0.1
   Compiling futures-task v0.3.32
   Compiling httparse v1.10.1
   Compiling futures-io v0.3.32
   Compiling typenum v1.20.0
   Compiling slab v0.4.12
   Compiling thiserror v1.0.69
   Compiling anyhow v1.0.102
   Compiling minimal-lexical v0.2.1
   Compiling percent-encoding v2.3.2
   Compiling nom v7.1.3
   Compiling futures-util v0.3.32
   Compiling synstructure v0.13.2
   Compiling num-integer v0.1.46
   Compiling try-lock v0.2.5
   Compiling base64 v0.22.1
   Compiling rustls v0.23.40
   Compiling tower-service v0.3.3
   Compiling want v0.3.1
   Compiling num-bigint v0.4.6
   Compiling tracing-core v0.1.36
   Compiling futures-channel v0.3.32
   Compiling rusticata-macros v4.1.0
   Compiling thiserror v2.0.18
   Compiling displaydoc v0.2.5
   Compiling zerofrom-derive v0.1.7
   Compiling yoke-derive v0.8.2
   Compiling zerovec-derive v0.11.3
   Compiling serde_derive v1.0.228
   Compiling thiserror-impl v1.0.69
   Compiling asn1-rs-impl v0.2.0
   Compiling asn1-rs-derive v0.5.1
   Compiling zerofrom v0.1.7
   Compiling getrandom v0.4.2
   Compiling bitflags v2.11.1
   Compiling atomic-waker v1.1.2
   Compiling utf8parse v0.2.2
   Compiling rustls-webpki v0.103.13
   Compiling anstyle-parse v1.0.0
   Compiling hyper v1.9.0
   Compiling yoke v0.8.2
   Compiling thiserror-impl v2.0.18
   Compiling tracing v0.1.44
   Compiling crypto-common v0.1.7
   Compiling block-buffer v0.10.4
   Compiling form_urlencoded v1.2.2
   Compiling sync_wrapper v1.0.2
   Compiling colorchoice v1.0.5
   Compiling asn1-rs v0.6.2
   Compiling anstyle-query v1.1.5
   Compiling ipnet v2.12.0
   Compiling anstyle v1.0.14
   Compiling is_terminal_polyfill v1.70.2
   Compiling zerovec v0.11.6
   Compiling zerotrie v0.2.4
   Compiling tower-layer v0.3.3
   Compiling oid-registry v0.7.1
   Compiling tower v0.5.3
   Compiling anstream v1.0.0
   Compiling hyper-util v0.1.20
   Compiling digest v0.10.7
   Compiling webpki-roots v1.0.7
   Compiling aho-corasick v1.1.4
   Compiling tinystr v0.8.3
   Compiling potential_utf v0.1.5
   Compiling icu_collections v2.2.0
   Compiling icu_locale_core v2.2.0
   Compiling iri-string v0.7.12
   Compiling heck v0.5.0
   Compiling tinyvec_macros v0.1.1
   Compiling clap_lex v1.1.0
   Compiling ryu v1.0.23
   Compiling core-foundation-sys v0.8.7
   Compiling regex-syntax v0.8.10
   Compiling strsim v0.11.1
   Compiling icu_provider v2.2.0
   Compiling clap_builder v4.6.0
   Compiling icu_normalizer v2.2.0
   Compiling icu_properties v2.2.0
   Compiling iana-time-zone v0.1.65
   Compiling tinyvec v1.11.0
   Compiling clap_derive v4.6.1
   Compiling simple_asn1 v0.6.4
   Compiling axhub-codegen v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-codegen)
   Compiling serde_urlencoded v0.7.1
   Compiling tower-http v0.6.8
   Compiling axhub-helpers v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-helpers)
   Compiling pem v3.0.6
   Compiling tokio-rustls v0.26.4
   Compiling http-body-util v0.1.3
   Compiling hyper-rustls v0.27.9
   Compiling cpufeatures v0.2.17
   Compiling data-encoding v2.11.0
   Compiling rustix v1.1.4
   Compiling idna_adapter v1.2.2
   Compiling idna v1.1.0
   Compiling der-parser v9.0.0
   Compiling lazy_static v1.5.0
   Compiling log v0.4.29
   Compiling regex-automata v0.4.14
   Compiling url v2.5.8
   Compiling x509-parser v0.16.0
   Compiling clap v4.6.1
   Compiling sha2 v0.10.9
   Compiling jsonwebtoken v9.3.1
   Compiling chrono v0.4.44
   Compiling unicode-normalization v0.1.25
   Compiling uuid v1.23.1
   Compiling reqwest v0.12.28
   Compiling hmac v0.12.1
   Compiling errno v0.3.14
   Compiling semver v1.0.28
   Compiling fastrand v2.4.1
   Compiling tempfile v3.27.0
   Compiling regex v1.12.3
    Finished `test` profile [unoptimized + debuginfo] target(s) in 23.16s
     Running unittests src/lib.rs (target/llvm-cov-target/debug/deps/axhub_codegen-b0eff76fcb439a60)

running 5 tests
test tests::reports_actionable_parse_errors_for_missing_catalog_and_bad_shapes ... ok
test tests::extracts_catalog_entries ... ok
test tests::rejects_unterminated_strings_and_invalid_field_names ... ok
test tests::extracts_entries_with_comments_commas_escapes_and_quoted_fields ... ok
test tests::generate_catalog_json_is_pretty_json_for_build_script_consumers ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/llvm-cov-target/debug/deps/axhub_helpers-92a0c71f812ec0d3)

running 3 tests
test catalog::tests::generated_catalog_has_expected_entries ... ok
test preflight::tests::semver_drops_prerelease_and_build_like_ts ... ok
test redact::tests::strips_unicode_and_redacts_secrets ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/axhub_helpers-f7e1f874948b5616)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/cli_e2e.rs (target/llvm-cov-target/debug/deps/cli_e2e-97c8965a23071386)

running 3 tests
test cli_version_help_redact_and_classify_work ... ok
test cli_usage_preflight_resolve_list_and_session_start_paths_are_stable ... ok
test cli_consent_and_preauth_e2e_preserve_permission_contract ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s

     Running tests/phase_parity.rs (target/llvm-cov-target/debug/deps/phase_parity-cb80904d55b3370f)

running 16 tests
test catalog_classifies_base_subclassified_and_default_entries ... ok
test keychain_parses_go_keyring_envelope ... ok
test redact_matches_typescript_secret_and_unicode_contract ... ok
test resolve_filters_apps_and_preserves_git_context_for_errors ... ok
test preflight_semver_auth_and_exit_precedence_match_ts ... ok
test consent_rejects_symlink_and_world_readable_private_files_on_unix ... ok
test consent_parser_recognizes_nested_shell_destructive_intents_and_ignores_safe_commands ... ok
test list_deployments_covers_token_endpoint_http_and_error_matrix ... ok
test consent_locks_zero_leeway_binding_mismatch_and_parser_hardening ... ok
test list_deployments_maps_auth_not_found_success_and_proxy_skip ... ok
test spawn_sync_covers_empty_command_and_successful_child_output ... ok
test preflight_covers_auth_shapes_env_cache_and_cli_absence ... ok
test resolve_covers_arg_parsing_auth_parse_ambiguity_and_not_found_paths ... ok
test keychain_runner_maps_platform_success_missing_parse_error_and_unsupported ... ok
test windows_keychain_runner_covers_success_and_failure_guidance ... ok
test telemetry_is_opt_in_private_jsonl_and_error_swallowing ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s

Filename                                  Regions    Missed Regions     Cover   Functions  Missed Functions  Executed       Lines      Missed Lines     Cover    Branches   Missed Branches     Cover
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
axhub-codegen/src/lib.rs                      347                18    94.81%          25                 4    84.00%         245                 8    96.73%           0                 0         -
axhub-helpers/src/catalog.rs                   62                 1    98.39%           8                 0   100.00%          42                 1    97.62%           0                 0         -
axhub-helpers/src/consent/jwt.rs              172                22    87.21%           8                 2    75.00%         135                10    92.59%           0                 0         -
axhub-helpers/src/consent/key.rs              140                21    85.00%          17                 5    70.59%          88                12    86.36%           0                 0         -
axhub-helpers/src/consent/parser.rs           286                33    88.46%          14                 1    92.86%         153                15    90.20%           0                 0         -
axhub-helpers/src/keychain.rs                  93                 9    90.32%           8                 1    87.50%          67                 3    95.52%           0                 0         -
axhub-helpers/src/keychain_windows.rs          69                 6    91.30%           7                 2    71.43%          67                 4    94.03%           0                 0         -
axhub-helpers/src/list_deployments.rs         225                38    83.11%          30                 1    96.67%         210                42    80.00%           0                 0         -
axhub-helpers/src/main.rs                     336                41    87.80%          16                 2    87.50%         178                15    91.57%           0                 0         -
axhub-helpers/src/preflight.rs                263                19    92.78%          35                 2    94.29%         167                17    89.82%           0                 0         -
axhub-helpers/src/redact.rs                    66                 0   100.00%           8                 0   100.00%          27                 0   100.00%           0                 0         -
axhub-helpers/src/resolve.rs                  317                27    91.48%          27                 1    96.30%         246                36    85.37%           0                 0         -
axhub-helpers/src/spawn.rs                     24                 2    91.67%           2                 0   100.00%          11                 0   100.00%           0                 0         -
axhub-helpers/src/telemetry.rs                137                 9    93.43%          15                 4    73.33%          83                 5    93.98%           0                 0         -
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
TOTAL                                        2537               246    90.30%         220                25    88.64%        1719               168    90.23%           0                 0         -
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1060 security advisories (from /Users/wongil/.cargo/advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (241 crate dependencies)
bun test v1.3.10 (30e609e0)

tests/fixtures.test.ts:
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > exactly 40 fixtures present [0.65ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > each fixture has required schema fields [1.22ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture adv-001-env-prefix.json: Env-prefix bypass attempt — must still detect destructive [2.42ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture adv-002-multi-env-prefix.json: Multiple env-prefix assignments [0.14ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture adv-003-sub-shell.json: Sub-shell wrapper $(...) bypass attempt [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture adv-004-eval-prefix.json: Eval prefix bypass attempt [0.05ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture adv-005-and-chain.json: &&-chain — destructive command in second position [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture adv-006-semi-chain.json: ;-chain — destructive command after semicolon [0.03ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture adv-007-pipe-chain.json: Pipe chain — destructive in second position [0.03ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture adv-008-bash-c.json: bash -c wrapper — destructive in shell-string [0.05ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-001-deploy-create-basic.json: Bare deploy create with all required flags [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-002-deploy-create-json.json: Deploy create with --json flag [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-003-deploy-create-equals.json: Deploy create using --flag=value syntax [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-004-deploy-create-numeric-id.json: Deploy create with numeric app id
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-005-deploy-create-feature-branch.json: Deploy create with feature branch (slash in branch name)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-006-update-apply-basic.json: Update apply (CLI version upgrade) [0.01ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-007-update-apply-with-cosign.json: Update apply with cosign required env (default-on per Phase 6 §16.10)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-008-auth-login-basic.json: Auth login (mutates token storage at ~/.config/axhub-plugin/token)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-009-auth-login-print-token.json: Auth login with print-token (still mutates token state)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture destructive-010-deploy-create-tab-separated.json: Deploy create with tab-separated flags (whitespace robustness)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture neg-001-not-axhub-command.json: Random non-axhub bash command — must NOT be destructive
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture neg-002-comment-with-axhub.json: Comment containing 'axhub deploy' — must NOT be destructive
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture neg-003-string-with-axhub.json: String containing 'axhub deploy create' — echo only, NOT executable
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture neg-004-other-tool-named-axhub.json: Different tool whose name starts with 'axhub' (no subcommand match) — NOT destructive
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture prf-001-axhub-profile-env.json: AXHUB_PROFILE env override (staging profile)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture prf-002-bare-axhub-version.json: Bare axhub --version (read-only diagnostic, not destructive)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture prf-003-headless-token-paste.json: Headless token-paste flow (auth login without browser)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture prf-004-axhub-help.json: axhub --help (info-only, not destructive)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-001-apps-list.json: Apps list — pure read, no consent gate [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-002-apps-list-paginated.json: Apps list with pagination
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-003-apis-list-scoped.json: APIs list scoped to current app (default-deny pattern)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-004-deploy-status.json: Deploy status without watch
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-005-deploy-status-watch.json: Deploy status with --watch (still read-only) [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-006-deploy-logs-build.json: Deploy build logs [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-007-deploy-logs-follow.json: Deploy logs with --follow (SSE stream, still read-only)
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-008-auth-status.json: Auth status query (no token mutation) [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-009-deploy-status-watch-explicit.json: Phase 5 US-504: deploy status with --watch — read-only despite long-lived stream [0.02ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture ro-010-deploy-logs-follow-explicit.json: Phase 5 US-504: deploy logs with --follow build source — read-only SSE stream [0.03ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture uni-001-cyrillic-homoglyph.json: Cyrillic 'а' in app slug (homoglyph for ASCII 'a') — parser still flags destructive [0.01ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture uni-002-zwj.json: Zero-width joiner inside app slug — parser still flags destructive [0.04ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture uni-003-fullwidth-digit.json: Full-width digit '１' in commit sha — parser still flags destructive [0.07ms]
(pass) Frozen fixture suite (40 hand-curated parseAxhubCommand cases) > fixture uni-004-nbsp-in-flag.json: Non-breaking space (U+00A0) inside flag value [0.03ms]
(pass) Fixture distribution (curation invariants) > 10 destructive fixtures [0.01ms]
(pass) Fixture distribution (curation invariants) > 10 read-only fixtures (ro-*) — Phase 5 US-504 added 2 explicit deploy status/logs [0.01ms]
(pass) Fixture distribution (curation invariants) > 8 adversarial fixtures (adv-*)
(pass) Fixture distribution (curation invariants) > 4 unicode fixtures (uni-*)
(pass) Fixture distribution (curation invariants) > 4 profile/headless fixtures (prf-*)
(pass) Fixture distribution (curation invariants) > 4 negative fixtures (neg-*)

tests/corpus-schema.test.ts:
(pass) corpus.100.jsonl schema validation (US-203) > exactly 100 rows [0.16ms]
(pass) corpus.100.jsonl schema validation (US-203) > each row has all 7 required fields [0.12ms]
(pass) corpus.100.jsonl schema validation (US-203) > each row id is unique [0.05ms]
(pass) corpus.100.jsonl schema validation (US-203) > lang is one of: ko, en, mixed, slash [0.04ms]
(pass) expected_cmd_pattern coverage (US-203 + architect M2.5 §3) > all destructive rows MUST have non-null expected_cmd_pattern [0.41ms]
(pass) expected_cmd_pattern coverage (US-203 + architect M2.5 §3) > read-only rows with non-null expected_skill MUST have non-null expected_cmd_pattern [0.28ms]
(pass) expected_cmd_pattern coverage (US-203 + architect M2.5 §3) > pure-negative rows (destructive=false AND expected_skill=null) MUST have null expected_cmd_pattern [0.06ms]
(pass) expected_cmd_pattern coverage (US-203 + architect M2.5 §3) > expected_cmd_pattern values compile as valid regex [0.38ms]
(pass) expected_cmd_pattern coverage (US-203 + architect M2.5 §3) > all expected_cmd_pattern values reference 'axhub' command (when non-null) [0.06ms]
(pass) expected_cmd_pattern distribution (curation health) > destructive count is reasonable (10-50 of 100, current curation has many adversarial bypass attempts) [0.05ms]
(pass) expected_cmd_pattern distribution (curation health) > negative-case count is reasonable (10-30 of 100) [0.04ms]
(pass) expected_cmd_pattern distribution (curation health) > expected_skill set covers core skills [0.07ms]

tests/codegen.test.ts:
(pass) catalog → markdown codegen (US-202) > generated markdown file exists [0.31ms]
(pass) catalog → markdown codegen (US-202) > generated markdown starts with auto-generated header [0.25ms]
(pass) catalog → markdown codegen (US-202) > generated markdown contains every catalog key [0.10ms]
(pass) catalog → markdown codegen (US-202) > generated markdown 4-part schema present per entry [0.14ms]
(pass) catalog → markdown codegen (US-202) > idempotency: re-running formatEntry on each entry produces identical output [1.62ms]
(pass) catalog ↔ hand-written markdown drift detection (US-202) > hand-written markdown exists [0.11ms]
(pass) catalog ↔ hand-written markdown drift detection (US-202) > every catalog.ts key has a corresponding section in hand-written markdown [0.93ms]
(pass) catalog ↔ hand-written markdown drift detection (US-202) > hand-written markdown contains the 4-part labels (감정/원인/해결/버튼) [0.13ms]

tests/manifest.test.ts:
(pass) plugin.json schema > name field present and matches kebab-case [0.05ms]
(pass) plugin.json schema > name is exactly 'axhub' [0.03ms]
(pass) plugin.json schema > version is semver [0.02ms]
(pass) plugin.json schema > description present and non-empty [0.01ms]
(pass) plugin.json schema > author is object with name
(pass) plugin.json schema > author.url is HTTPS URL
(pass) plugin.json schema > homepage is HTTPS URL
(pass) plugin.json schema > repository is STRING (not object) — Phase 6 incident #1 [0.01ms]
(pass) plugin.json schema > repository ends in .git
(pass) plugin.json schema > license is recognized SPDX identifier [0.02ms]
(pass) plugin.json schema > keywords is array if present [0.04ms]
(pass) plugin.json schema > keywords contain 'axhub'
(pass) plugin.json schema > no unknown top-level keys [0.05ms]
(pass) plugin.json schema > version matches package.json version [0.01ms]
(pass) plugin.json schema > description mentions axhub [0.02ms]
(pass) marketplace.json schema > name field present [0.01ms]
(pass) marketplace.json schema > owner is object with name [0.01ms]
(pass) marketplace.json schema > owner.url is HTTPS URL
(pass) marketplace.json schema > plugins is non-empty array [0.01ms]
(pass) marketplace.json schema > each plugin has name [0.01ms]
(pass) marketplace.json schema > each plugin has source path [0.01ms]
(pass) marketplace.json schema > each plugin has description
(pass) marketplace.json schema > each plugin has semver version [0.01ms]
(pass) marketplace.json schema > plugin name in marketplace matches plugin.json name [0.09ms]
(pass) marketplace.json schema > plugin version in marketplace matches plugin.json version [0.04ms]
(pass) hooks.json structure > outer wrapper has 'hooks' key
(pass) hooks.json structure > description present and non-empty
(pass) hooks.json structure > contains SessionStart event
(pass) hooks.json structure > contains PreToolUse event
(pass) hooks.json structure > contains PostToolUse event
(pass) hooks.json structure > each event value is an array [0.02ms]
(pass) hooks.json structure > each hook group has hooks array [0.18ms]
(pass) hooks.json structure > each hook config has type 'command' [0.06ms]
(pass) hooks.json structure > each hook command references CLAUDE_PLUGIN_ROOT [0.02ms]
(pass) hooks.json structure > each hook command references axhub-helpers binary or session-start shim [0.03ms]
(pass) hooks.json structure > each hook timeout is positive integer if set [0.03ms]
(pass) hooks.json structure > PreToolUse + PostToolUse have Bash matcher [0.02ms]
(pass) hooks.json structure > SessionStart registers only the portable Unix shim in universal hooks.json
(pass) hooks.json structure > SessionStart entry [0] is bash (Unix) — preserved byte-identical from v0.1.6 [0.02ms]
(pass) hooks.json structure > universal SessionStart hook does not require PowerShell on non-Windows hosts [0.07ms]
(pass) hooks.json structure > PreToolUse + PostToolUse remain single-entry (no platform branching needed — direct binary call) [0.19ms]
(pass) Phase 11 deferred-doc artifacts > docs/pilot/windows-vm-smoke-checklist.md exists with 14 numbered steps [0.78ms]
(pass) Phase 11 deferred-doc artifacts > tests/smoke-windows-vm-checklist.ps1 exists with $env:AXHUB_VM_SMOKE guard [0.34ms]
(pass) Phase 11 deferred-doc artifacts > docs/pilot/authenticode-signing-runbook.md exists [0.39ms]
(pass) Phase 11 deferred-doc artifacts > .github/workflows/sign-windows.yml.template exists with workflow_dispatch + continue-on-error [0.24ms]
(pass) Phase 11 deferred-doc artifacts > .gitattributes contains *.yml.template linguist exemption [0.15ms]
(pass) hookSpecificOutput field validation in src/axhub-helpers/index.ts > source contains hookSpecificOutput emissions [0.02ms]
(pass) hookSpecificOutput field validation in src/axhub-helpers/index.ts > every hookSpecificOutput object literal includes hookEventName [0.08ms]
(pass) hookSpecificOutput field validation in src/axhub-helpers/index.ts > every hookEventName references a real Claude Code event [0.08ms]
(pass) hookSpecificOutput field validation in src/axhub-helpers/index.ts > permissionDecision values are valid (allow|deny|ask) [0.05ms]
(pass) hookSpecificOutput field validation in src/axhub-helpers/index.ts > at least one PreToolUse permissionDecision: deny path exists (gate works) [0.03ms]
(pass) hookSpecificOutput field validation in src/axhub-helpers/index.ts > at least one PreToolUse permissionDecision: allow path exists (escape valve) [0.02ms]
(pass) hookSpecificOutput field validation in src/axhub-helpers/index.ts > classify-exit emits systemMessage only on relevant exits [0.03ms]
(pass) hookSpecificOutput field validation in src/axhub-helpers/index.ts > source compiles to a valid TypeScript module (heuristic: has exports) [0.02ms]
(pass) commands/*.md frontmatter > exactly 10 command files exist, including the Korean deploy alias
(pass) commands/*.md frontmatter > each command file has YAML frontmatter (--- delimited) [0.11ms]
(pass) commands/*.md frontmatter > each command frontmatter has all required metadata fields [0.14ms]
(pass) commands/*.md frontmatter > each command description is non-empty string [0.13ms]
(pass) commands/*.md frontmatter > each command description ≤200 chars [0.08ms]
(pass) commands/*.md frontmatter > commands without name in frontmatter (auto-derived from filename) [0.05ms]
(pass) commands/*.md frontmatter > model field is present and valid Claude model [0.06ms]
(pass) commands/*.md frontmatter > argument-hint is present and non-empty [0.08ms]
(pass) commands/*.md frontmatter > body section exists after frontmatter [0.03ms]
(pass) commands/*.md frontmatter > help command remains least-privilege and tool-free [0.04ms]
(pass) commands/*.md frontmatter > Korean deploy alias delegates to deploy skill without forking deploy logic [0.05ms]
(pass) commands/*.md frontmatter > login command does not advertise unsupported token-file auth flags
(pass) commands/*.md frontmatter > help command exists [0.02ms]
(pass) commands/*.md frontmatter > deploy command exists
(pass) commands/*.md frontmatter > Korean deploy alias exists
(pass) commands/*.md frontmatter > login command exists (auth entrypoint)
(pass) commands/*.md frontmatter > commands allow axhub-helpers when their target skill invokes the helper binary [1.18ms]
(pass) skills/*/SKILL.md frontmatter > at least 11 skills exist [0.02ms]
(pass) skills/*/SKILL.md frontmatter > each skill dir has SKILL.md [0.06ms]
(pass) skills/*/SKILL.md frontmatter > each SKILL.md starts with --- frontmatter [0.02ms]
(pass) skills/*/SKILL.md frontmatter > each SKILL.md frontmatter has name field [0.09ms]
(pass) skills/*/SKILL.md frontmatter > each skill name matches its directory name [0.06ms]
(pass) skills/*/SKILL.md frontmatter > each SKILL.md frontmatter has description field [0.06ms]
(pass) skills/*/SKILL.md frontmatter > each description starts with 'This skill' or '이 스킬' (Korean equivalent — Phase 5 한국어 전환) [0.05ms]
(pass) skills/*/SKILL.md frontmatter > NO skill has allowed-tools in frontmatter — Phase 6 Q1 finding [0.05ms]
(pass) skills/*/SKILL.md frontmatter > NO skill has model field in frontmatter (skills are model-agnostic) [0.05ms]
(pass) skills/*/SKILL.md frontmatter > frontmatter contains ONLY allowed keys (Phase 18: + multi-step / needs-preflight) [0.09ms]
(pass) skills/*/SKILL.md frontmatter > description includes Korean trigger phrases (per skill convention) [0.05ms]
(pass) skills/*/SKILL.md frontmatter > body section exists after frontmatter [0.03ms]
(pass) skills/*/SKILL.md frontmatter > description length reasonable (≤2000 chars — skill activation dispatcher) [0.03ms]
(pass) skills/*/SKILL.md frontmatter > expected 11 specific skills present [0.02ms]
(pass) skills/*/SKILL.md frontmatter > deploy skill has body referencing axhub-helpers binary [0.01ms]
(pass) skills/*/SKILL.md frontmatter > auth skill has body referencing consent-mint (US-004 outcome)
(pass) skills/*/SKILL.md frontmatter > Phase 5 US-505: update skill does NOT force-set AXHUB_DISABLE_AUTOUPDATE=1 [0.02ms]
(pass) skills/*/SKILL.md frontmatter > skills use stdin JSON consent-mint instead of unsupported flags [0.08ms]
(pass) skills/*/SKILL.md frontmatter > auth headless token-paste docs use token-import and the plugin token path [0.06ms]
(pass) skills/*/SKILL.md frontmatter > auth logout path prompts with AskUserQuestion before running axhub auth logout [0.04ms]
(pass) skills/*/SKILL.md frontmatter > skills do not instruct unavailable deploy-list or helper-schedule commands [0.19ms]
(pass) skills/*/SKILL.md frontmatter > skill error-catalog cross references are resolvable relative paths [0.06ms]
(pass) cross-manifest consistency > headless auth references use implemented token-import command and plugin token path [1.17ms]
(pass) cross-manifest consistency > current user-facing auth docs do not advertise legacy token-file env or flags [0.46ms]
(pass) cross-manifest consistency > recover troubleshooting docs describe the shipped forward-fix skill [0.27ms]
(pass) cross-manifest consistency > plugin.json name matches package.json name suffix [0.02ms]
(pass) cross-manifest consistency > plugin.json version === package.json version [0.02ms]
(pass) cross-manifest consistency > plugin.json version === marketplace.json plugin version [0.03ms]
(pass) cross-manifest consistency > hooks.json command paths reference existing helper subcommands or shim [0.10ms]
(pass) cross-manifest consistency > README.md exists and references plugin name [0.44ms]
(pass) cross-manifest consistency > CLAUDE.md exists and is non-empty [0.39ms]
(pass) cross-manifest consistency > LICENSE file exists [0.03ms]
(pass) cross-manifest consistency > CHANGELOG.md exists [0.04ms]
(pass) cross-manifest consistency > package.json scripts include build, test, typecheck [0.02ms]
(pass) cross-manifest consistency > package.json scripts include build:all (cross-arch)
(pass) cross-manifest consistency > package.json scripts include smoke and smoke:full
(pass) cross-manifest consistency > package.json declares Bun engine
(pass) cross-manifest consistency > install.sh exists and is executable [0.48ms]

tests/telemetry.test.ts:
(pass) telemetry opt-in gate > AXHUB_TELEMETRY unset → no file written [1.33ms]
(pass) telemetry opt-in gate > AXHUB_TELEMETRY=0 → no file written [0.22ms]
(pass) telemetry opt-in gate > AXHUB_TELEMETRY=true (not '1') → no file written [0.43ms]
(pass) telemetry opt-in gate > AXHUB_TELEMETRY=1 → file created with one line [106.20ms]
(pass) telemetry envelope shape > envelope has all required meta fields [80.77ms]
(pass) telemetry envelope shape > ts is ISO 8601 UTC with Z suffix [75.96ms]
(pass) telemetry envelope shape > session_id pulled from CLAUDE_SESSION_ID [78.29ms]
(pass) telemetry envelope shape > session_id falls back to 'unknown' when env unset [227.31ms]
(pass) telemetry envelope shape > plugin_version matches package.json (codegen-synced — Phase 6 US-602 follow-up) [76.67ms]
(pass) telemetry envelope shape > custom payload fields preserved [82.19ms]
(pass) telemetry file behavior > multiple emits → multiple lines (append, not overwrite) [82.30ms]
(pass) telemetry file behavior > each line is valid JSON [82.73ms]
(pass) telemetry file behavior > file mode is 0600 (private) [81.10ms]
(pass) telemetry file behavior > dir created with mode 0700 [78.45ms]
(pass) telemetry error swallowing > does NOT throw when XDG path is unwritable (silent fail) [2.05ms]
(pass) telemetry error swallowing > does NOT throw when called without await (fire-and-forget) [0.23ms]

tests/list-deployments.test.ts:
(pass) token discovery (US-501) > returns null when no token source available [0.51ms]
(pass) token discovery (US-501) > AXHUB_TOKEN env var takes precedence [0.18ms]
(pass) token discovery (US-501) > falls back to ${XDG_CONFIG_HOME}/axhub-plugin/token file [0.60ms]
(pass) hub-api TLS pinning (PLAN row 60) > documents the current hub-api SPKI pin [0.17ms]
(pass) hub-api TLS pinning (PLAN row 60) > runs TLS pin checker before sending the bearer token [0.78ms]
(pass) hub-api TLS pinning (PLAN row 60) > fails closed on TLS pin mismatch before fetch runs [0.46ms]
(pass) hub-api TLS pinning (PLAN row 60) > AXHUB_ALLOW_PROXY bypasses the real pin checker for managed corporate proxy [0.08ms]
(pass) runListDeployments — auth gate (US-501) > missing token returns exit 65 + Korean message [0.36ms]
(pass) runListDeployments — auth gate (US-501) > invalid app id returns exit 67 [0.17ms]
(pass) runListDeployments — REST API (US-501) > successful list returns deployments with status name mapping [0.14ms]
(pass) runListDeployments — REST API (US-501) > 401 returns auth error with re-login Korean message [0.28ms]
(pass) runListDeployments — REST API (US-501) > 404 returns app-not-found error [0.24ms]
(pass) runListDeployments — REST API (US-501) > network error returns transport error [0.04ms]
(pass) runListDeployments — REST API (US-501) > AXHUB_ENDPOINT env var override is used [0.32ms]
(pass) runListDeployments — REST API (US-501) > default endpoint when AXHUB_ENDPOINT unset [0.08ms]
(pass) runListDeployments — REST API (US-501) > limit parameter propagates to per_page query [0.15ms]
(pass) runListDeployments — REST API (US-501) > Bearer token sent in Authorization header [0.30ms]

tests/install-ps1.test.ts:
(pass) bin/install.ps1 — Windows installer mirror > RELEASE_VERSION literal matches install.sh:48 (parity assertion) [0.47ms]
(pass) bin/install.ps1 — Windows installer mirror > AMD64 arch detection via PROCESSOR_ARCHITECTURE [0.09ms]
(pass) bin/install.ps1 — Windows installer mirror > Invoke-WebRequest with -TimeoutSec 600 (slow corp network pre-mortem #3) [0.04ms]
(pass) bin/install.ps1 — Windows installer mirror > Move-Item + Start-Sleep + Test-Path re-check (Defender post-Move pre-mortem #6) [0.06ms]
(pass) bin/install.ps1 — Windows installer mirror > NO Add-Type / NO Reflection.Assembly (EDR-clean — Phase 9 PInvoke not used in installer) [0.03ms]
(pass) bin/install.ps1 — Windows installer mirror > Explicit Test-Path / Remove-Item NOT install.sh:80 || operator (D4) [0.04ms]
(pass) bin/install.ps1 — Windows installer mirror > Every catch emits ConvertTo-Json @{ systemMessage = ... } envelope + exit 0 (F3) [0.05ms]

tests/release-config.test.ts:
(pass) release.yml workflow shape (US-204) > .github/workflows/release.yml exists [0.42ms]
(pass) release.yml workflow shape (US-204) > triggers on tag push (v*.*.*) [0.10ms]
(pass) release.yml workflow shape (US-204) > declares id-token: write for sigstore OIDC [0.03ms]
(pass) release.yml workflow shape (US-204) > declares contents: write for release upload [0.03ms]
(pass) release.yml workflow shape (US-204) > installs Bun via official curl install (avoids unzip dependency) [0.04ms]
(pass) release.yml workflow shape (US-204) > build-and-sign job runs on self-hosted Linux ARM64 runner [0.07ms]
(pass) release.yml workflow shape (US-204) > runs build:all to produce 5 cross-arch binaries [0.04ms]
(pass) release.yml workflow shape (US-204) > generates manifest.json via scripts/release/manifest.ts [0.05ms]
(pass) release.yml workflow shape (US-204) > installs cosign via sigstore/cosign-installer action [0.04ms]
(pass) release.yml workflow shape (US-204) > signs each binary with cosign sign-blob (keyless) [0.03ms]
(pass) release.yml workflow shape (US-204) > uploads release assets via gh CLI sequential loop (avoids race) [0.04ms]
(pass) release.yml workflow shape (US-204) > manual workflow_dispatch requires an explicit semver tag input [0.08ms]
(pass) manifest.ts generator (US-204) > scripts/release/manifest.ts exists [0.02ms]
(pass) manifest.ts generator (US-204) > exports a Manifest schema with required fields [0.23ms]
(pass) manifest.ts generator (US-204) > uses sha256 from node:crypto [0.08ms]
(pass) manifest.ts generator (US-204) > excludes cosign signature and certificate sidecars from binary manifest entries [0.04ms]
(pass) .versionrc.json release lifecycle > postbump stages all generated tracked version files before commit/tag [0.39ms]
(pass) .versionrc.json release lifecycle > posttag no longer asks maintainers to amend the already-created release tag [0.13ms]
(pass) verify-release.sh user-side script (US-204) > scripts/release/verify-release.sh exists and is executable [0.07ms]
(pass) verify-release.sh user-side script (US-204) > verifies manifest.json signature first (trust anchor) [0.28ms]
(pass) verify-release.sh user-side script (US-204) > uses certificate-identity-regexp + OIDC issuer [0.05ms]
(pass) verify-release.sh user-side script (US-204) > cross-checks sha256 against manifest entries [0.04ms]
(pass) docs/RELEASE.md (US-204) > exists and documents maintainer + user verification [0.18ms]

tests/classify-exit.test.ts:
(pass) classify() > exit 0 → 축하해요 emotion
(pass) classify() > exit 1 → 잠깐만요 + 연결 끊김 [0.22ms]
(pass) classify() > exit 64 base → 배포는 시작 안 했어요 [0.01ms]
(pass) classify() > exit 64 + validation.deployment_in_progress → 진행 중인 배포 entry [0.04ms]
(pass) classify() > exit 64 + validation.app_ambiguous → 같은 이름이 두 개 [0.02ms]
(pass) classify() > exit 64 + validation.app_list_truncated → 앱이 너무 많아서 [0.01ms]
(pass) classify() > exit 65 → 로그인이 만료됐을 뿐이에요 [0.03ms]
(pass) classify() > exit 65 with empty stdout → falls back to base 65 entry [0.01ms]
(pass) classify() > exit 66 base → 권한 문제 [0.01ms]
(pass) classify() > exit 66 + scope.downgrade_blocked → 안전장치가 작동했어요 [0.02ms]
(pass) classify() > exit 66 + update.cosign_verification_failed → 보안 검증에 실패 [0.02ms]
(pass) classify() > exit 67 → 그런 이름은 못 찾았어요 [0.01ms]
(pass) classify() > exit 68 → 너무 많이 요청해서 [0.01ms]
(pass) classify() > unknown exit code → generic default entry [0.02ms]
(pass) classify() > malformed stdout JSON → falls back to base exit code entry [0.01ms]

tests/run-corpus.test.ts:
(pass) tests/run-corpus.sh fixture replay runner > plugin mode writes committed 20-row plugin results instead of an empty placeholder [21.71ms]
(pass) tests/run-corpus.sh fixture replay runner > plugin mode can score the 100-row committed arm against the matching baseline [47.70ms]
(pass) tests/run-corpus.sh fixture replay runner > docs-only score is informational and does not fail the runner [37.86ms]
(pass) tests/run-corpus.sh fixture replay runner > full corpus without explicit fixture fails closed instead of fabricating results [11.53ms]

tests/codegen-version.test.ts:
(pass) codegen-install-version (US-602) > package.json version is valid semver [0.07ms]
(pass) codegen-install-version (US-602) > syncInstallVersion is idempotent (re-run produces no change) [0.64ms]
(pass) codegen-install-version (US-602) > bin/install.sh RELEASE_VERSION default matches package.json version [0.16ms]
(pass) codegen-install-version (US-602) > AXHUB_PLUGIN_RELEASE env override syntax preserved (codegen does not break override) [0.12ms]
(pass) codegen-install-version (US-602) > syncInstallVersion result reports before/after when no change [0.18ms]
(pass) codegen-install-version (US-602) > bin/install.ps1 $ReleaseVersion default matches package.json version (Phase 11 US-1101) [0.09ms]
(pass) codegen-install-version (US-602) > PowerShell single-quote literal preserved + pre-release tag regex round-trips [0.21ms]

tests/axhub-helpers.test.ts:
(pass) runPreflight() > cli too old (0.0.5) → in_range:false, cli_too_old:true, exit 64 [0.38ms]
(pass) runPreflight() > cli in range (0.1.0) + auth ok → in_range:true, exit 0 [0.06ms]
(pass) runPreflight() > cli too new (0.2.0) → cli_too_new:true, exit 64 [0.04ms]
(pass) runPreflight() > cli within range, intermediate (0.1.5) → in_range:true [0.14ms]
(pass) runPreflight() > cli missing (spawn returns empty stdout) → exit 64, no crash [0.05ms]
(pass) runPreflight() > cli missing (spawn throws ENOENT) → exit 64, no crash [0.11ms]
(pass) runPreflight() > cli ok but auth missing → exit 65, in_range stays true [0.01ms]
(pass) runResolve() > app resolved (slug=paydrop matches one app) → exit 0 [0.63ms]
(pass) runResolve() > app ambiguous (slug=app matches 2+ apps) → exit 64, error=app_ambiguous [0.25ms]
(pass) runResolve() > app not found (slug=zzz) → exit 67, error=app_not_found [0.04ms]
(pass) runResolve() > auth missing → exit 65
(pass) runResolve() > no candidate slug extractable → exit 67, error=no_candidate_slug
(pass) extractSlugCandidate() > paydrop 배포해줘 → paydrop
(pass) extractSlugCandidate() > 배포해줘 paydrop → paydrop (mid-sentence)
(pass) extractSlugCandidate() > ship paydrop now → paydrop
(pass) extractSlugCandidate() > Korean-only stop words → null
(pass) extractSlugCandidate() > empty utterance → null
(pass) extractSlugCandidate() > punctuation stripped: 'paydrop, ship!' → paydrop
(pass) filterAppsBySlug() > exact prefix match (paydrop) → both paydrop variants [0.21ms]
(pass) filterAppsBySlug() > substring fallback (rank) → ccrank
(pass) filterAppsBySlug() > no match → empty array

tests/ux-statusline.test.ts:
(pass) Phase 17 C7/US-1707 — statusline binary contract > bin/statusline.sh exists
(pass) Phase 17 C7/US-1707 — statusline binary contract > bin/statusline.sh is executable (mode +x) [0.07ms]
(pass) Phase 17 C7/US-1707 — statusline binary contract > bin/statusline.sh runs and exits 0 in <500ms cold [62.33ms]
(pass) Phase 17 C7/US-1707 — statusline binary contract > bin/statusline.sh output is ≤80 characters [5.31ms]
(pass) Phase 17 C7/US-1707 — statusline binary contract > bin/statusline.sh output uses 해요체 (no forbidden Toss tokens) [5.79ms]
(pass) Phase 17 C7/US-1707 — statusline binary contract > bin/statusline.sh output starts with 'axhub:' prefix (identity marker) [12.71ms]

tests/e2e-claude-cli-registry.test.ts:
(pass) Phase 22.0.4 — registry.json baseline (SB-2) > 13 top-level keys (2 메타 + 11 SKILL slug) [0.08ms]
(pass) Phase 22.0.4 — registry.json baseline (SB-2) > 9 actual safe_default rationale 엔트리 (auth ×2 / recover / apis / apps / clarify / doctor / update / upgrade) [0.16ms]
(pass) Phase 22.0.4 — registry.json baseline (SB-2) > 9 safe_default 값 (abort/stay/skip/later/show 카탈로그) [0.04ms]
(pass) Phase 22.0.4 — registry.json baseline (SB-2) > deploy / logs / status 는 safe_default 없이 _note + 메타 키만 (의도) [0.03ms]
(pass) Phase 22.0.4 — registry.json baseline (SB-2) > 모든 safe_default 엔트리에 rationale 첨부 (drift catch) [0.03ms]

tests/ux-skill-template-completeness.test.ts:
(pass) Phase 18 C3/US-1805 — skill:doctor --strict gate > bun run skill:doctor --strict exits 0 (all 11 SKILLs complete) [24.18ms]
(pass) Phase 18 C3/US-1805 — skill:doctor --strict gate > skill-doctor diagnostic format: machine-parseable line per finding [21.90ms]
(pass) Phase 18 C3/US-1805 — skill:doctor --strict gate > skill-doctor diagnostic format is exercised with a controlled missing-pattern fixture [23.42ms]

tests/lint-toss-tone.test.ts:
(pass) Phase 13 US-1306 — Toss tone conformance lint > FORBIDDEN tokens cover the 7 Toss rules + axhub deprecation [0.06ms]
(pass) Phase 13 US-1306 — Toss tone conformance lint > PHASE_13_FILES returns runtime + commands + install + hook scope [6.39ms]
(pass) Phase 13 US-1306 — Toss tone conformance lint > scan() returns Violation[] with proper shape (post-Phase 13: empty array OK) [6.62ms]
(pass) Phase 13 US-1306 — Toss tone conformance lint > scan() classifies errors vs warnings correctly [4.29ms]
(pass) Phase 13 US-1306 — Toss tone conformance lint > Rust include patterns can be scanned for Phase 0 tone lock [14.04ms]
(pass) Phase 13 US-1306 — Skill keyword preservation lint > snapshot() returns description_phrases + lexicon_phrases + timestamp [3.84ms]
(pass) Phase 13 US-1306 — Skill keyword preservation lint > baseline file exists at .omc/lint-baselines/skill-keywords.json [0.05ms]
(pass) Phase 13 US-1306 — Skill keyword preservation lint > baseline captured at least 10 SKILL.md files + 500 lexicon phrases [1.63ms]

tests/hook-latency.test.ts:
(pass) M4 hook no-op latency benchmark > package.json exposes bench:hooks [0.01ms]
(pass) M4 hook no-op latency benchmark > benchmark locks the 50ms p95 hot-path gate and both no-op hooks [0.34ms]
(pass) M4 hook no-op latency benchmark > PLAN and helper comments no longer promise an impossible 5ms compiled-binary gate [0.98ms]

tests/token-init.test.ts:
(pass) cmdTokenInit subcommand registration > dispatch switch includes 'token-init' case [0.36ms]
(pass) cmdTokenInit subcommand registration > USAGE documents token-init subcommand [0.16ms]
(pass) cmdTokenInit subcommand registration > cmdTokenInit function is defined [0.13ms]
(pass) cmdTokenInit subcommand registration > NO axhub --print-token call (CLI flag does not exist) [0.17ms]
(pass) cmdTokenInit subcommand registration > token storage path matches token-import (XDG_CONFIG_HOME-aware) [0.14ms]
(pass) cmdTokenInit subcommand registration > file mode 0600 + dir mode 0700 enforced (security parity with token-import) [0.06ms]
(pass) AXHUB_TOKEN env var precedence + keychain bridge > AXHUB_TOKEN env var path is checked before keychain [0.03ms]
(pass) AXHUB_TOKEN env var precedence + keychain bridge > readKeychainToken handles macOS via 'security find-generic-password' [0.04ms]
(pass) AXHUB_TOKEN env var precedence + keychain bridge > readKeychainToken handles Linux via secret-tool
(pass) AXHUB_TOKEN env var precedence + keychain bridge > Windows uses PowerShell + Add-Type PInvoke against advapi32!CredReadW

tests/ux-ask-fallback-defaults.test.ts:
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/upgrade/SKILL.md contains D1 sentinel [0.04ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/upgrade/SKILL.md D1 block references registry path [0.03ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/apis/SKILL.md contains D1 sentinel [0.05ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/apis/SKILL.md D1 block references registry path [0.01ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/update/SKILL.md contains D1 sentinel [0.04ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/update/SKILL.md D1 block references registry path
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/auth/SKILL.md contains D1 sentinel [0.05ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/auth/SKILL.md D1 block references registry path [0.02ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/deploy/SKILL.md contains D1 sentinel [0.02ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/deploy/SKILL.md D1 block references registry path [0.02ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/clarify/SKILL.md contains D1 sentinel
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/clarify/SKILL.md D1 block references registry path [0.05ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/doctor/SKILL.md contains D1 sentinel
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/doctor/SKILL.md D1 block references registry path
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/status/SKILL.md contains D1 sentinel [0.06ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/status/SKILL.md D1 block references registry path [0.02ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/logs/SKILL.md contains D1 sentinel [0.03ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/logs/SKILL.md D1 block references registry path
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/recover/SKILL.md contains D1 sentinel [0.03ms]
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/recover/SKILL.md D1 block references registry path
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/apps/SKILL.md contains D1 sentinel
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > skills/apps/SKILL.md D1 block references registry path
(pass) Phase 17 C1/US-1701 — D1 fallback sentinel in all 11 SKILLs > exactly 11 SKILLs have the sentinel (no drop) [0.25ms]

tests/session-start-ps1.test.ts:
(pass) hooks/session-start.ps1 — Windows SessionStart mirror > first line: $ErrorActionPreference = 'Stop' (US-1000 outcome A — silent assumed) [0.15ms]
(pass) hooks/session-start.ps1 — Windows SessionStart mirror > $Helper path uses CLAUDE_PLUGIN_ROOT + bin/axhub-helpers.exe
(pass) hooks/session-start.ps1 — Windows SessionStart mirror > install.ps1 spawn captures $LASTEXITCODE and surfaces non-zero (D3)
(pass) hooks/session-start.ps1 — Windows SessionStart mirror > `& $Helper session-start` — direct binary call (not PS-spawned) [0.15ms]
(pass) hooks/session-start.ps1 — Windows SessionStart mirror > `& $Helper token-init` — auto-trigger after auth status check
(pass) hooks/session-start.ps1 — Windows SessionStart mirror > MAX_PATH catch via [System.IO.PathTooLongException] (pre-mortem #5) [0.07ms]
(pass) hooks/session-start.ps1 — Windows SessionStart mirror > AMSI/EDR pattern detection in catch-all (pre-mortem #2)
(pass) hooks/session-start.ps1 — Windows SessionStart mirror > state dir uses XDG_STATE_HOME (NOT %LOCALAPPDATA%) — mirrors telemetry.ts:40-44 (F2) [0.13ms]
(pass) hooks/session-start.ps1 — Windows SessionStart mirror > token dir uses XDG_CONFIG_HOME — DISTINCT from state dir (mirrors index.ts cmdTokenInit) [0.09ms]

tests/consent.test.ts:
(pass) mintToken / verifyToken > roundtrip: matching binding verifies as valid [8.87ms]
(pass) mintToken / verifyToken > mint fails fast without CLAUDE_SESSION_ID instead of writing an unverifiable token [0.31ms]
(pass) mintToken / verifyToken > CLI consent-mint also fails fast without CLAUDE_SESSION_ID across process boundary [47.72ms]
(pass) mintToken / verifyToken > mint rejects a symlinked consent file instead of overwriting its target [3.81ms]
(pass) mintToken / verifyToken > expired token: ttl=1, sleep 2s, verify fails with expired reason [2105.19ms]
(pass) mintToken / verifyToken > zero leeway: exp = now - 1 is rejected [1.95ms]
(pass) mintToken / verifyToken > zero leeway: exp = now boundary is rejected [1.95ms]
(pass) mintToken / verifyToken > wrong app_id: minted with paydrop, verified with otherapp → invalid [1.54ms]
(pass) mintToken / verifyToken > wrong profile: minted with prod, verified with staging → invalid [1.81ms]
(pass) mintToken / verifyToken > missing token file: verify with no prior mint → invalid no_consent_token [0.27ms]
(pass) mintToken / verifyToken > wrong tool_call_id: rejects token from different call [1.43ms]
(pass) mintToken / verifyToken > wrong commit_sha: rejects after force-push [1.38ms]
(pass) parseAxhubCommand > axhub deploy create with --app/--branch/--commit extracts all flags [0.23ms]
(pass) parseAxhubCommand > axhub deploy create with --profile flag [0.20ms]
(pass) parseAxhubCommand > axhub update apply --force is destructive update_apply [0.19ms]
(pass) parseAxhubCommand > axhub auth login is destructive auth_login [0.16ms]
(pass) parseAxhubCommand > axhub deploy logs --follow --kill is destructive deploy_logs_kill [0.19ms]
(pass) parseAxhubCommand > ls -la is not destructive (not even axhub) [0.10ms]
(pass) parseAxhubCommand > axhub status is not destructive (read-only) [0.18ms]
(pass) parseAxhubCommand > axhub deploy logs without --kill is not destructive [0.15ms]
(pass) parseAxhubCommand > supports --flag=value form [0.16ms]
(pass) parseAxhubCommand — bypass hardening (T-ADV-PARSE-1..8) > T-ADV-PARSE-1: env-var prefix `AXHUB_TOKEN=foo axhub deploy create` is destructive [0.15ms]
(pass) parseAxhubCommand — bypass hardening (T-ADV-PARSE-1..8) > T-ADV-PARSE-2: bash -c sub-shell `bash -c "axhub deploy create ..."` is destructive [0.14ms]
(pass) parseAxhubCommand — bypass hardening (T-ADV-PARSE-1..8) > T-ADV-PARSE-3: compound `cd /tmp && axhub deploy create` is destructive [0.17ms]
(pass) parseAxhubCommand — bypass hardening (T-ADV-PARSE-1..8) > T-ADV-PARSE-4: leading `; axhub deploy create` is destructive [0.17ms]
(pass) parseAxhubCommand — bypass hardening (T-ADV-PARSE-1..8) > T-ADV-PARSE-5: eval-quoted `eval "axhub deploy create ..."` is destructive [0.19ms]
(pass) parseAxhubCommand — bypass hardening (T-ADV-PARSE-1..8) > T-ADV-PARSE-6: paren sub-shell `(axhub deploy create ...)` is destructive [0.13ms]
(pass) parseAxhubCommand — bypass hardening (T-ADV-PARSE-1..8) > T-ADV-PARSE-7: `echo axhub deploy create` is NOT destructive (axhub is an argument) [0.16ms]
(pass) parseAxhubCommand — bypass hardening (T-ADV-PARSE-1..8) > T-ADV-PARSE-8: `axhub apps list --json` (read-only) remains NOT destructive [0.12ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub apps list --json [0.15ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub apps list --json --per-page=10 [0.08ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub apis list --json --query auth [0.15ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub apis list --app-id 42 --json [0.10ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub auth status --json [0.21ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub auth login [0.13ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub auth logout [0.12ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub deploy create --app paydrop --branch main --commit abc [0.10ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub deploy create --app paydrop --branch main --commit abc [0.05ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub deploy status dep_42 --watch --json [0.12ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub deploy logs dep_42 --follow --source build --json [0.29ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub deploy logs dep_42 --source build [0.13ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub update check --json [0.05ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub update apply --yes [0.17ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub update apply --force --yes [0.14ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub --version [0.05ms]
(pass) parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0 > v0.1.0 command never produces deploy_logs_kill: axhub --help [0.18ms]
(pass) parseAxhubCommand — Gotcha #1: trailing close-delimiter on action token > (axhub auth login) — paren-wrapped detected as auth_login [0.08ms]
(pass) parseAxhubCommand — Gotcha #1: trailing close-delimiter on action token > `axhub auth login` — backtick-wrapped detected as auth_login [0.22ms]
(pass) parseAxhubCommand — Gotcha #1: trailing close-delimiter on action token > (axhub deploy create --app paydrop --branch main --commit abc) — paren-wrapped deploy detected with extracted flags [0.04ms]
(pass) parseAxhubCommand — Gotcha #1: trailing close-delimiter on action token > flag value with trailing close-delimiter is stripped (--commit abc) → abc) [0.13ms]
(pass) parseAxhubCommand — Gotcha #1: trailing close-delimiter on action token > $(axhub deploy create --app paydrop --branch main --commit abc) — sub-shell wrapper detected [0.12ms]
(pass) parseAxhubCommand — Gotcha #2: nested sub-shell inside eval/bash -c > eval "bash -c \"axhub deploy create --app paydrop --branch main --commit abc\"" — eval-of-bash detected [0.13ms]
(pass) parseAxhubCommand — Gotcha #2: nested sub-shell inside eval/bash -c > bash -c "(axhub auth login)" — bash-c with paren-wrapped inner detected [0.17ms]
(pass) parseAxhubCommand — Gotcha #2: nested sub-shell inside eval/bash -c > sh -c "$(axhub deploy create --app paydrop --branch main --commit abc)" — sh-c with $() inner detected [0.12ms]
(pass) parseAxhubCommand — Gotcha #2: nested sub-shell inside eval/bash -c > zsh -c '`axhub auth login`' — zsh-c with backtick inner detected [0.10ms]
(pass) parseAxhubCommand — Gotcha #2: nested sub-shell inside eval/bash -c > eval "(bash -c \"axhub update apply --yes\")" — triple-nested still detected [0.13ms]
(pass) parseAxhubCommand — read-only allowlist (Phase 5 US-504) > axhub deploy status dep_X --watch --json → not destructive [0.26ms]
(pass) parseAxhubCommand — read-only allowlist (Phase 5 US-504) > axhub deploy logs dep_X --follow --source build → not destructive [0.15ms]
(pass) parseAxhubCommand — read-only allowlist (Phase 5 US-504) > axhub deploy logs dep_X --source pod → not destructive (no --kill) [0.15ms]
(pass) parseAxhubCommand — read-only allowlist (Phase 5 US-504) > axhub apps list --json → not destructive [0.11ms]
(pass) parseAxhubCommand — read-only allowlist (Phase 5 US-504) > axhub apis list --json --query auth → not destructive [0.14ms]
(pass) parseAxhubCommand — read-only allowlist (Phase 5 US-504) > axhub auth status --json → not destructive (only auth login is) [0.12ms]
(pass) parseAxhubCommand — read-only allowlist (Phase 5 US-504) > axhub --version → not destructive [0.12ms]
(pass) parseAxhubCommand — read-only allowlist (Phase 5 US-504) > axhub --help → not destructive [0.15ms]
(pass) parseAxhubCommand — Gotcha #3: quoted subcommand tokens > axhub "deploy" "create" --app paydrop — double-quoted subcommands detected [0.13ms]
(pass) parseAxhubCommand — Gotcha #3: quoted subcommand tokens > axhub 'auth' 'login' — single-quoted subcommands detected [0.10ms]
(pass) parseAxhubCommand — Gotcha #3: quoted subcommand tokens > axhub "update" "apply" --yes — quoted update apply detected [0.18ms]
(pass) parseAxhubCommand — Gotcha #3: quoted subcommand tokens > axhub "deploy" create — mixed quoted/bare subcommands detected [0.13ms]
(pass) parseAxhubCommand — Gotcha #3: quoted subcommand tokens > read-only quoted subcommand stays NOT destructive (axhub 'apps' 'list') [0.10ms]

tests/ux-argument-hints.test.ts:
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/apis.md has argument-hint frontmatter [0.09ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/apps.md has argument-hint frontmatter [0.08ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/deploy.md has argument-hint frontmatter [0.05ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/doctor.md has argument-hint frontmatter [0.03ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/help.md has argument-hint frontmatter [0.03ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/login.md has argument-hint frontmatter [0.03ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/logs.md has argument-hint frontmatter [0.03ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/status.md has argument-hint frontmatter [0.03ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/update.md has argument-hint frontmatter [0.03ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > commands/배포.md has argument-hint frontmatter [0.04ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > exactly 10 command files exist, including the Korean deploy alias [0.04ms]
(pass) Phase 17 C4/US-1704 — argument-hint frontmatter in 10 commands > Korean deploy alias participates in command metadata checks [0.02ms]

tests/keychain-windows.test.ts:
(pass) readWindowsKeychain — mocked-runner pre-mortem coverage > case 1: success — AXHUB_OK:<base64> sentinel returns extracted token [0.25ms]
(pass) readWindowsKeychain — mocked-runner pre-mortem coverage > case 2: ExecutionPolicy block — exit 1 + stderr 'execution of scripts' → ExecutionPolicy 4-part Korean [0.06ms]
(pass) readWindowsKeychain — mocked-runner pre-mortem coverage > case 3: NOT_FOUND — stdout ERR:NOT_FOUND → 4-part Korean missing-credential [0.04ms]
(pass) readWindowsKeychain — mocked-runner pre-mortem coverage > case 4: PInvoke load failure — stdout ERR:LOAD_FAIL → 4-part Korean PInvoke error [0.02ms]
(pass) readWindowsKeychain — mocked-runner pre-mortem coverage > case 5: EDR signal kill — signalCode set OR exit ∈ {-1, 0xC0000409} → EDR honesty (no IT-allowlist) [0.02ms]
(pass) readWindowsKeychain — mocked-runner pre-mortem coverage > case 6: spawnSync throws — runner rejects → 4-part Korean spawn-failure [0.06ms]

tests/ux-ask-fallback-registry.test.ts:
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > registry file exists and parses
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/upgrade/SKILL.md questions all have registered safe_default [0.59ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/apis/SKILL.md questions all have registered safe_default [0.10ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/update/SKILL.md questions all have registered safe_default [0.03ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/auth/SKILL.md questions all have registered safe_default [0.02ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/deploy/SKILL.md questions all have registered safe_default [0.02ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/clarify/SKILL.md questions all have registered safe_default [0.03ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/doctor/SKILL.md questions all have registered safe_default [0.01ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/status/SKILL.md questions all have registered safe_default [0.02ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/logs/SKILL.md questions all have registered safe_default [0.02ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/recover/SKILL.md questions all have registered safe_default [0.03ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > skills/apps/SKILL.md questions all have registered safe_default [0.02ms]
(pass) Phase 17 C5/US-1705 — per-question fallback registry coverage > registry has no stale entries (every key matches a SKILL question) [0.31ms]

tests/runtime-fallback.test.ts:
(pass) AXHUB_HELPERS_RUNTIME rust delegation > runtime=rust delegates to configured Rust helper binary with stdin preserved [414.30ms]
(pass) AXHUB_HELPERS_RUNTIME rust delegation > runtime=auto delegates supported commands when Rust helper exists [392.97ms]
(pass) AXHUB_HELPERS_RUNTIME rust delegation > runtime=auto falls back to TypeScript when Rust helper is missing [43.06ms]
(pass) AXHUB_HELPERS_RUNTIME rust delegation > runtime=ts never delegates even when Rust helper exists [39.53ms]

tests/redact.test.ts:
(pass) redact() > NFKC normalize: Cyrillic а (U+0430) survives but is normalized [0.07ms]
(pass) redact() > NFKC normalize: fullwidth chars collapse to ASCII [0.75ms]
(pass) redact() > Zero-width joiner (U+200D) stripped: 'pay‍drop' → 'paydrop' [0.05ms]
(pass) redact() > Zero-width non-joiner (U+200C) stripped [0.02ms]
(pass) redact() > Zero-width space (U+200B) stripped [0.02ms]
(pass) redact() > Bidi LRE (U+202A) stripped [0.05ms]
(pass) redact() > Bidi RLE (U+202B) stripped [0.03ms]
(pass) redact() > Bearer token redacted: ≥20 char token [0.01ms]
(pass) redact() > Bearer token NOT redacted when shorter than 20 chars [0.03ms]
(pass) redact() > AXHUB_TOKEN redacted: ≥20 char token
(pass) redact() > AXHUB_TOKEN NOT redacted when shorter than 20 chars [0.01ms]
(pass) redact() > raw axhub_pat_* token redacted (Phase 11 v0.1.10 — caught by live smoke) [0.01ms]
(pass) redact() > axhub_pat_* shorter than 16 chars NOT redacted (regex floor)
(pass) redact() > ANSI escape sequences stripped: colour codes
(pass) redact() > ANSI bold stripped
(pass) redact() > Multiple redactions in one string
(pass) redact() > Plain text with no secrets passes through unchanged [0.02ms]
(pass) redact() > Empty string returns empty string

tests/ux-todowrite.test.ts:
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/upgrade/SKILL.md (multi-step: true) contains TodoWrite call [0.06ms]
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/upgrade/SKILL.md TodoWrite activeForm in 해요체 (no forbidden tokens) [0.08ms]
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/apis/SKILL.md (multi-step: false) — TodoWrite not required
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/update/SKILL.md (multi-step: true) contains TodoWrite call
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/update/SKILL.md TodoWrite activeForm in 해요체 (no forbidden tokens)
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/auth/SKILL.md (multi-step: false) — TodoWrite not required
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/deploy/SKILL.md (multi-step: true) contains TodoWrite call
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/deploy/SKILL.md TodoWrite activeForm in 해요체 (no forbidden tokens)
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/clarify/SKILL.md (multi-step: false) — TodoWrite not required
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/doctor/SKILL.md (multi-step: true) contains TodoWrite call
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/doctor/SKILL.md TodoWrite activeForm in 해요체 (no forbidden tokens)
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/status/SKILL.md (multi-step: false) — TodoWrite not required
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/logs/SKILL.md (multi-step: false) — TodoWrite not required
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/recover/SKILL.md (multi-step: true) contains TodoWrite call [0.04ms]
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/recover/SKILL.md TodoWrite activeForm in 해요체 (no forbidden tokens) [0.02ms]
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > skills/apps/SKILL.md (multi-step: false) — TodoWrite not required
(pass) Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter > at least 5 SKILLs are declared multi-step: true (Phase 18 baseline) [0.30ms]

tests/plan-consistency.test.ts:
(pass) Phase 1 PLAN reconciliation > cancellation decision remains explicit in the audit trail [0.31ms]
(pass) Phase 1 PLAN reconciliation > active milestones do not list canceled M7 implementation work [0.01ms]
(pass) Phase 1 PLAN reconciliation > active scope permanently excludes plugin server work [0.02ms]
(pass) Phase 1 PLAN reconciliation > repository layout documents absence of plugin server placeholder [0.03ms]
(pass) Phase 1 PLAN reconciliation > repository layout mirrors current Bun helper implementation, not stale Go scaffolding [0.02ms]
(pass) Phase 1 PLAN reconciliation > current architecture names CLI as the shared surface [0.06ms]
(pass) PLAN release artifact reconciliation > supply-chain section matches current release artifact names [0.03ms]
(pass) PLAN release artifact reconciliation > active supply-chain section names the current Bun helper, not a new Go rewrite [0.01ms]
(pass) PLAN plugin schema reconciliation > active schema snippet matches current release metadata decisions [0.01ms]
(pass) PLAN best-practices checklist reconciliation > best-practices section is a status ledger, not an unchecked open-work list [0.06ms]
(pass) PLAN best-practices checklist reconciliation > manual review-only rows are marked as evidence/replaced instead of active implementation gaps [0.02ms]
(pass) PLAN best-practices checklist reconciliation > skill bodies remain free of the explicit 'you should' anti-pattern [0.26ms]

tests/ux-skill-preflight-injection.test.ts:
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/upgrade/SKILL.md (needs-preflight: false) — preflight injection not required [0.04ms]
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/apis/SKILL.md (needs-preflight: true) contains !command preflight literal [0.02ms]
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/update/SKILL.md (needs-preflight: false) — preflight injection not required
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/auth/SKILL.md (needs-preflight: false) — preflight injection not required
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/deploy/SKILL.md (needs-preflight: true) contains !command preflight literal
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/clarify/SKILL.md (needs-preflight: false) — preflight injection not required
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/doctor/SKILL.md (needs-preflight: false) — preflight injection not required
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/status/SKILL.md (needs-preflight: false) — preflight injection not required
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/logs/SKILL.md (needs-preflight: false) — preflight injection not required
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/recover/SKILL.md (needs-preflight: true) contains !command preflight literal
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > skills/apps/SKILL.md (needs-preflight: true) contains !command preflight literal
(pass) Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter > at least 4 SKILLs are declared needs-preflight: true (Phase 18 baseline) [0.23ms]

tests/ux-askuserquestion-headers.test.ts:
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/upgrade/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.21ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/apis/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.04ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/update/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.02ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/auth/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.08ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/deploy/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.03ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/clarify/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.03ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/doctor/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.06ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/status/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.03ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/logs/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.03ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/recover/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.04ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > skills/apps/SKILL.md AskUserQuestion headers ≤12 Korean chars [0.02ms]
(pass) Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars > at least 8 SKILLs have a header field (Phase 17 C3 polish coverage) [0.15ms]

tests/keychain.test.ts:
(pass) parseKeyringValue (go-keyring-base64 decoder) > strips 'go-keyring-base64:' prefix + decodes JSON + extracts access_token [0.10ms]
(pass) parseKeyringValue (go-keyring-base64 decoder) > works without 'go-keyring-base64:' prefix (raw base64) [0.03ms]
(pass) parseKeyringValue (go-keyring-base64 decoder) > returns null on empty input
(pass) parseKeyringValue (go-keyring-base64 decoder) > returns null on invalid base64 [0.03ms]
(pass) parseKeyringValue (go-keyring-base64 decoder) > returns null when decoded JSON has no access_token field [0.02ms]
(pass) parseKeyringValue (go-keyring-base64 decoder) > returns null when access_token is too short (< 16 chars) [0.02ms]
(pass) parseKeyringValue (go-keyring-base64 decoder) > returns null when decoded payload is not valid JSON [0.01ms]
(pass) parseKeyringValue (go-keyring-base64 decoder) > returns null when decoded JSON is array (not object) [0.01ms]
(pass) keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1) > source contains exactly 7 4-part error blocks (1 macOS miss + 1 macOS parse + 1 macOS catch + 1 Linux miss + 1 Linux parse + 1 Linux catch + 1 platform fallback) [0.03ms]
(pass) keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1) > each error starts with emotion word (감정 prefix per error-empathy-catalog) [0.03ms]
(pass) keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1) > macOS keychain miss preserves 'axhub auth login' kernel keyword [0.02ms]
(pass) keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1) > macOS parse failure preserves '--force' kernel keyword + axhub CLI version mention [0.01ms]
(pass) keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1) > macOS catch preserves 'security' command + PATH kernel keyword [0.01ms]
(pass) keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1) > Linux miss preserves 'libsecret-tools' + 'secret-tool' kernel keywords [0.01ms]
(pass) keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1) > Linux parse failure preserves 'axhub auth login --force' kernel [0.01ms]
(pass) keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1) > Linux catch preserves 'D-Bus' + 'dbus-launch' kernel keywords (architect-flagged systemd-keyring concern) [0.02ms]
(pass) keychain.ts 4-part Korean error format-parity (Phase 11 US-1102 closes #1) > platform fallback preserves AXHUB_TOKEN env var escape kernel + interpolates platform name [0.01ms]

tests/skill-noninteractive-guard.test.ts:
(pass) Phase 12 v0.1.12 + v0.1.15 — non-interactive guard regression lock > skills/status/SKILL.md has TTY guard literal `[ -t 1 ]` [0.08ms]
(pass) Phase 12 v0.1.12 + v0.1.15 — non-interactive guard regression lock > skills/status/SKILL.md uses WATCH=--watch / WATCH= toggle [0.05ms]
(pass) Phase 12 v0.1.12 + v0.1.15 — non-interactive guard regression lock > skills/logs/SKILL.md has TTY guard literal `[ -t 1 ]` [0.05ms]
(pass) Phase 12 v0.1.12 + v0.1.15 — non-interactive guard regression lock > skills/logs/SKILL.md uses FOLLOW=--follow / FOLLOW= toggle [0.03ms]
(pass) Phase 12 v0.1.12 + v0.1.15 — non-interactive guard regression lock > All three skills check $CI env var for headless detection [0.08ms]
(pass) Phase 12 v0.1.12 + v0.1.15 — non-interactive guard regression lock > All three skills check $CLAUDE_NON_INTERACTIVE for explicit override [0.08ms]
(pass) Phase 12 v0.1.12 + v0.1.15 — non-interactive guard regression lock > skills/deploy/SKILL.md post-deploy chain has TTY guard for `--watch` [0.04ms]
(pass) Phase 12 v0.1.12 + v0.1.15 — non-interactive guard regression lock > skills/deploy/SKILL.md does NOT call `axhub deploy status` with raw `--watch` flag [0.04ms]

tests/session-start.test.ts:
(pass) session-start preflight diagnostics > reports CLI/auth/profile diagnostics and removes placeholder copy [388.74ms]
(pass) session-start preflight diagnostics > missing CLI still exits zero with actionable guidance [39.01ms]

tests/e2e/staging.test.ts:
Skipped: AXHUB_E2E_STAGING_TOKEN not set. See tests/e2e/README.md for how to enable.
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > (unnamed)
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > axhub auth status --json returns valid identity
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > axhub apps list --json returns array (may be empty)
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > parseAxhubCommand → action mapping is consistent with real CLI surface
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > classify-exit produces Korean 4-part template for real exit codes
(pass) ax-hub-cli staging E2E (skipped — no AXHUB_E2E_STAGING_TOKEN) > placeholder: set AXHUB_E2E_STAGING_TOKEN + AXHUB_E2E_STAGING_ENDPOINT to enable [0.02ms]

5 tests skipped:
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > (unnamed)
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > axhub auth status --json returns valid identity
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > axhub apps list --json returns array (may be empty)
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > parseAxhubCommand → action mapping is consistent with real CLI surface
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > classify-exit produces Korean 4-part template for real exit codes

 557 pass
 5 skip
 0 fail
 2888 expect() calls
Ran 562 tests across 34 files. [5.13s]
$ bun test tests/e2e/
bun test v1.3.10 (30e609e0)

tests/e2e/staging.test.ts:
Skipped: AXHUB_E2E_STAGING_TOKEN not set. See tests/e2e/README.md for how to enable.
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > (unnamed)
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > axhub auth status --json returns valid identity
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > axhub apps list --json returns array (may be empty)
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > parseAxhubCommand → action mapping is consistent with real CLI surface
(skip) ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN) > classify-exit produces Korean 4-part template for real exit codes
(pass) ax-hub-cli staging E2E (skipped — no AXHUB_E2E_STAGING_TOKEN) > placeholder: set AXHUB_E2E_STAGING_TOKEN + AXHUB_E2E_STAGING_ENDPOINT to enable [0.01ms]

 1 pass
 5 skip
 0 fail
 1 expect() calls
Ran 6 tests across 1 file. [41.00ms]
$ bun scripts/check-toss-tone-conformance.ts --strict --strict

0 error(s), 0 warning(s) across 30 file(s)
$ bun scripts/check-toss-tone-conformance.ts --include "crates/**/*.rs" --strict

0 error(s), 0 warning(s) across 50 file(s)
$ bun scripts/check-skill-keywords-preserved.ts --check --check
OK — keywords preserved (no diff vs baseline)
```

## Fuzz harness lint/fmt
```
cargo fmt --manifest-path fuzz/Cargo.toml -- --check => PASS
cargo clippy --manifest-path fuzz/Cargo.toml --bin parser -- -D warnings => PASS
```

## Additional external verification round
timestamp_utc=2026-04-29T06:38:04Z

### Tool availability
```
Docker version 29.4.0, build 9d7ad9f
29.4.0
/Users/wongil/.local/bin/claude
2.1.123 (Claude Code)
/opt/homebrew/bin/axhub
axhub 0.5.11 (commit 9fe5a3fe07c4d33ed1bfab977e8a5be289b3276b, built 2026-04-29T05:13:02Z, darwin/arm64)
axhub API v1.x compatible
```

## Linux Secret Service Docker live keychain smoke
Command: docker run rust:1.83-bookworm + libsecret/gnome-keyring, seed go-keyring envelope, run ignored live Rust test
```
exit=127
secret-tool recovered length=258
Unable to find image 'rust:1.83-bookworm' locally
1.83-bookworm: Pulling from library/rust
0a96bdb82805: Pulling fs layer
54c7be425079: Pulling fs layer
7aa8176e6d89: Pulling fs layer
1523f4b3f560: Pulling fs layer
38d98af84437: Pulling fs layer
1523f4b3f560: Waiting
38d98af84437: Waiting
0a96bdb82805: Download complete
54c7be425079: Verifying Checksum
54c7be425079: Download complete
7aa8176e6d89: Verifying Checksum
7aa8176e6d89: Download complete
0a96bdb82805: Pull complete
54c7be425079: Pull complete
38d98af84437: Verifying Checksum
38d98af84437: Download complete
7aa8176e6d89: Pull complete
1523f4b3f560: Verifying Checksum
1523f4b3f560: Download complete
1523f4b3f560: Pull complete
38d98af84437: Pull complete
Digest: sha256:a45bf1f5d9af0a23b26703b3500d70af1abff7f984a7abef5a104b42c02a292b
Status: Downloaded newer image for rust:1.83-bookworm
debconf: delaying package configuration, since apt-utils is not installed
bash: line 15: cargo: command not found
```

## Linux Secret Service Docker live keychain smoke retry
```
exit=101
cargo 1.83.0 (5ffbef321 2024-10-29)
rustc 1.83.0 (90b35a623 2024-11-26)
secret-tool recovered length=258
debconf: delaying package configuration, since apt-utils is not installed
    Updating crates.io index
 Downloading crates ...
  Downloaded bitflags v2.11.1
  Downloaded powerfmt v0.2.0
  Downloaded futures-io v0.3.32
  Downloaded zeroize v1.8.2
  Downloaded lru-slab v0.1.2
  Downloaded autocfg v1.5.0
  Downloaded socket2 v0.6.3
  Downloaded subtle v2.6.1
  Downloaded digest v0.10.7
  Downloaded anyhow v1.0.102
  Downloaded rand_core v0.9.5
  Downloaded zerofrom-derive v0.1.7
  Downloaded quinn-udp v0.5.14
  Downloaded serde_urlencoded v0.7.1
  Downloaded num-traits v0.2.19
  Downloaded der-parser v9.0.0
  Downloaded version_check v0.9.5
  Downloaded litemap v0.8.2
  Downloaded tower-service v0.3.3
  Downloaded proc-macro2 v1.0.106
  Downloaded yoke-derive v0.8.2
  Downloaded memchr v2.8.0
  Downloaded time-core v0.1.8
error: failed to parse manifest at `/tmp/cargo-home/registry/src/index.crates.io-6f17d22bba15001f/time-core-0.1.8/Cargo.toml`

Caused by:
  feature `edition2024` is required

  The package requires the Cargo feature called `edition2024`, but that feature is not stabilized in this version of Cargo (1.83.0 (5ffbef321 2024-10-29)).
  Consider trying a newer version of Cargo (this may require the nightly release).
  See https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#edition-2024 for more information about the status of this feature.
```

## Linux Secret Service Docker live keychain smoke latest-rust retry
```
exit=101
cargo 1.95.0 (f2d3ce0bd 2026-03-21)
rustc 1.95.0 (59807616e 2026-04-14)
secret-tool recovered length=258
debconf: unable to initialize frontend: Dialog
debconf: (TERM is not set, so the dialog frontend is not usable.)
debconf: falling back to frontend: Readline
debconf: unable to initialize frontend: Readline
debconf: (This frontend requires a controlling tty.)
debconf: falling back to frontend: Teletype
debconf: unable to initialize frontend: Teletype
debconf: (This frontend requires a controlling tty.)
debconf: falling back to frontend: Noninteractive
    Updating crates.io index
 Downloading crates ...
  Downloaded icu_properties v2.2.0
  Downloaded serde_derive v1.0.228
  Downloaded idna_adapter v1.2.2
  Downloaded itoa v1.0.18
  Downloaded subtle v2.6.1
  Downloaded untrusted v0.9.0
  Downloaded colorchoice v1.0.5
  Downloaded percent-encoding v2.3.2
  Downloaded time-core v0.1.8
  Downloaded yoke-derive v0.8.2
  Downloaded powerfmt v0.2.0
  Downloaded pin-project-lite v0.2.17
  Downloaded num-integer v0.1.46
  Downloaded rand_chacha v0.9.0
  Downloaded anstyle v1.0.14
  Downloaded displaydoc v0.2.5
  Downloaded utf8parse v0.2.2
  Downloaded bitflags v2.11.1
  Downloaded serde_urlencoded v0.7.1
  Downloaded utf8_iter v1.0.4
  Downloaded clap_derive v4.6.1
  Downloaded tower-service v0.3.3
  Downloaded anstyle-parse v1.0.0
  Downloaded zerofrom v0.1.7
  Downloaded getrandom v0.3.4
  Downloaded want v0.3.1
  Downloaded icu_normalizer v2.2.0
  Downloaded bytes v1.11.1
  Downloaded quinn v0.11.9
  Downloaded thiserror-impl v1.0.69
  Downloaded zerofrom-derive v0.1.7
  Downloaded simple_asn1 v0.6.4
  Downloaded mio v1.2.0
  Downloaded zmij v1.0.21
  Downloaded iri-string v0.7.12
  Downloaded zeroize v1.8.2
  Downloaded icu_properties_data v2.2.0
  Downloaded yoke v0.8.2
  Downloaded tempfile v3.27.0
  Downloaded thiserror-impl v2.0.18
  Downloaded chrono v0.4.44
  Downloaded zerovec-derive v0.11.3
  Downloaded tokio-rustls v0.26.4
  Downloaded time-macros v0.2.27
  Downloaded smallvec v1.15.1
  Downloaded semver v1.0.28
  Downloaded quinn-proto v0.11.14
  Downloaded aho-corasick v1.1.4
  Downloaded tinyvec v1.11.0
  Downloaded serde_core v1.0.228
  Downloaded rustls-pki-types v1.14.1
  Downloaded tinystr v0.8.3
  Downloaded socket2 v0.6.3
  Downloaded slab v0.4.12
  Downloaded idna v1.1.0
  Downloaded hyper v1.9.0
  Downloaded uuid v1.23.1
  Downloaded x509-parser v0.16.0
  Downloaded zerotrie v0.2.4
  Downloaded typenum v1.20.0
  Downloaded tower v0.5.3
  Downloaded unicode-normalization v0.1.25
  Downloaded zerovec v0.11.6
  Downloaded reqwest v0.12.28
  Downloaded tower-http v0.6.8
  Downloaded webpki-roots v1.0.7
  Downloaded serde_json v1.0.149
  Downloaded time v0.3.47
  Downloaded url v2.5.8
  Downloaded libc v0.2.186
  Downloaded serde v1.0.228
  Downloaded zerocopy v0.8.48
  Downloaded rustls v0.23.40
  Downloaded regex-syntax v0.8.10
  Downloaded syn v2.0.117
  Downloaded rustls-webpki v0.103.13
  Downloaded unicode-ident v1.0.24
  Downloaded tracing-core v0.1.36
  Downloaded rustix v1.1.4
  Downloaded futures-util v0.3.32
  Downloaded tracing v0.1.44
  Downloaded clap_builder v4.6.0
  Downloaded writeable v0.6.3
  Downloaded thiserror v2.0.18
  Downloaded thiserror v1.0.69
  Downloaded synstructure v0.13.2
  Downloaded regex v1.12.3
  Downloaded http v1.4.0
  Downloaded regex-automata v0.4.14
  Downloaded der-parser v9.0.0
  Downloaded shlex v1.3.0
  Downloaded rand v0.9.4
  Downloaded num-bigint v0.4.6
  Downloaded nom v7.1.3
  Downloaded memchr v2.8.0
  Downloaded icu_normalizer_data v2.2.0
  Downloaded hyper-util v0.1.20
  Downloaded cc v1.2.61
  Downloaded asn1-rs v0.6.2
  Downloaded minimal-lexical v0.2.1
  Downloaded icu_collections v2.2.0
  Downloaded tokio v1.52.1
  Downloaded base64 v0.22.1
  Downloaded sha2 v0.10.9
  Downloaded icu_locale_core v2.2.0
  Downloaded clap v4.6.1
  Downloaded version_check v0.9.5
  Downloaded sync_wrapper v1.0.2
  Downloaded proc-macro2 v1.0.106
  Downloaded once_cell v1.21.4
  Downloaded num-traits v0.2.19
  Downloaded log v0.4.29
  Downloaded icu_provider v2.2.0
  Downloaded getrandom v0.4.2
  Downloaded anyhow v1.0.102
  Downloaded strsim v0.11.1
  Downloaded rusticata-macros v4.1.0
  Downloaded quinn-udp v0.5.14
  Downloaded jsonwebtoken v9.3.1
  Downloaded httparse v1.10.1
  Downloaded getrandom v0.2.17
  Downloaded iana-time-zone v0.1.65
  Downloaded ppv-lite86 v0.2.21
  Downloaded autocfg v1.5.0
  Downloaded try-lock v0.2.5
  Downloaded tinyvec_macros v0.1.1
  Downloaded rustc-hash v2.1.2
  Downloaded quote v1.0.45
  Downloaded litemap v0.8.2
  Downloaded ipnet v2.12.0
  Downloaded futures-channel v0.3.32
  Downloaded data-encoding v2.11.0
  Downloaded anstream v1.0.0
  Downloaded hyper-rustls v0.27.9
  Downloaded find-msvc-tools v0.1.9
  Downloaded fastrand v2.4.1
  Downloaded http-body-util v0.1.3
  Downloaded ring v0.17.14
  Downloaded digest v0.10.7
  Downloaded rand_core v0.9.5
  Downloaded pem v3.0.6
  Downloaded deranged v0.5.8
  Downloaded http-body v1.0.1
  Downloaded cpufeatures v0.2.17
  Downloaded atomic-waker v1.1.2
  Downloaded anstyle-query v1.1.5
  Downloaded tower-layer v0.3.3
  Downloaded stable_deref_trait v1.2.1
  Downloaded lazy_static v1.5.0
  Downloaded is_terminal_polyfill v1.70.2
  Downloaded futures-task v0.3.32
  Downloaded futures-io v0.3.32
  Downloaded cfg_aliases v0.2.1
  Downloaded block-buffer v0.10.4
  Downloaded asn1-rs-derive v0.5.1
  Downloaded errno v0.3.14
  Downloaded cfg-if v1.0.4
  Downloaded asn1-rs-impl v0.2.0
  Downloaded ryu v1.0.23
  Downloaded potential_utf v0.1.5
  Downloaded oid-registry v0.7.1
  Downloaded num-conv v0.2.1
  Downloaded lru-slab v0.1.2
  Downloaded hmac v0.12.1
  Downloaded heck v0.5.0
  Downloaded generic-array v0.14.7
  Downloaded futures-sink v0.3.32
  Downloaded futures-core v0.3.32
  Downloaded form_urlencoded v1.2.2
  Downloaded crypto-common v0.1.7
  Downloaded clap_lex v1.1.0
  Downloaded linux-raw-sys v0.12.1
   Compiling proc-macro2 v1.0.106
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.45
   Compiling itoa v1.0.18
   Compiling libc v0.2.186
   Compiling memchr v2.8.0
   Compiling stable_deref_trait v1.2.1
   Compiling serde_core v1.0.228
   Compiling autocfg v1.5.0
   Compiling cfg-if v1.0.4
   Compiling bytes v1.11.1
   Compiling shlex v1.3.0
   Compiling pin-project-lite v0.2.17
   Compiling find-msvc-tools v0.1.9
   Compiling futures-core v0.3.32
   Compiling zmij v1.0.21
   Compiling serde v1.0.228
   Compiling num-traits v0.2.19
   Compiling litemap v0.8.2
   Compiling cc v1.2.61
   Compiling version_check v0.9.5
   Compiling powerfmt v0.2.0
   Compiling futures-sink v0.3.32
   Compiling once_cell v1.21.4
   Compiling socket2 v0.6.3
   Compiling mio v1.2.0
   Compiling writeable v0.6.3
   Compiling time-core v0.1.8
   Compiling num-conv v0.2.1
   Compiling smallvec v1.15.1
   Compiling serde_json v1.0.149
   Compiling time-macros v0.2.27
   Compiling generic-array v0.14.7
   Compiling tokio v1.52.1
   Compiling deranged v0.5.8
   Compiling syn v2.0.117
   Compiling ring v0.17.14
   Compiling getrandom v0.2.17
   Compiling http v1.4.0
   Compiling subtle v2.6.1
   Compiling zeroize v1.8.2
   Compiling utf8_iter v1.0.4
   Compiling untrusted v0.9.0
   Compiling icu_properties_data v2.2.0
   Compiling icu_normalizer_data v2.2.0
   Compiling rustls-pki-types v1.14.1
   Compiling time v0.3.47
   Compiling http-body v1.0.1
   Compiling minimal-lexical v0.2.1
   Compiling thiserror v1.0.69
   Compiling percent-encoding v2.3.2
   Compiling futures-io v0.3.32
   Compiling typenum v1.20.0
   Compiling slab v0.4.12
   Compiling futures-task v0.3.32
   Compiling httparse v1.10.1
   Compiling nom v7.1.3
   Compiling futures-util v0.3.32
   Compiling num-integer v0.1.46
   Compiling synstructure v0.13.2
   Compiling rustls v0.23.40
   Compiling tower-service v0.3.3
   Compiling try-lock v0.2.5
   Compiling base64 v0.22.1
   Compiling anyhow v1.0.102
   Compiling num-bigint v0.4.6
   Compiling displaydoc v0.2.5
   Compiling zerovec-derive v0.11.3
   Compiling zerofrom-derive v0.1.7
   Compiling yoke-derive v0.8.2
   Compiling serde_derive v1.0.228
   Compiling thiserror-impl v1.0.69
   Compiling asn1-rs-derive v0.5.1
   Compiling asn1-rs-impl v0.2.0
   Compiling want v0.3.1
   Compiling zerofrom v0.1.7
   Compiling rusticata-macros v4.1.0
   Compiling tracing-core v0.1.36
   Compiling yoke v0.8.2
   Compiling futures-channel v0.3.32
   Compiling thiserror v2.0.18
   Compiling atomic-waker v1.1.2
   Compiling bitflags v2.11.1
   Compiling zerovec v0.11.6
   Compiling zerotrie v0.2.4
   Compiling getrandom v0.4.2
   Compiling utf8parse v0.2.2
   Compiling tracing v0.1.44
   Compiling anstyle-parse v1.0.0
   Compiling hyper v1.9.0
   Compiling asn1-rs v0.6.2
   Compiling rustls-webpki v0.103.13
   Compiling tinystr v0.8.3
   Compiling potential_utf v0.1.5
   Compiling thiserror-impl v2.0.18
   Compiling crypto-common v0.1.7
   Compiling block-buffer v0.10.4
   Compiling icu_locale_core v2.2.0
   Compiling icu_collections v2.2.0
   Compiling form_urlencoded v1.2.2
   Compiling sync_wrapper v1.0.2
   Compiling is_terminal_polyfill v1.70.2
   Compiling anstyle-query v1.1.5
   Compiling oid-registry v0.7.1
   Compiling tower-layer v0.3.3
   Compiling anstyle v1.0.14
   Compiling ipnet v2.12.0
   Compiling colorchoice v1.0.5
   Compiling axhub-codegen v0.1.23 (/work/crates/axhub-codegen)
   Compiling icu_provider v2.2.0
   Compiling tower v0.5.3
   Compiling digest v0.10.7
   Compiling anstream v1.0.0
   Compiling hyper-util v0.1.20
   Compiling webpki-roots v1.0.7
   Compiling aho-corasick v1.1.4
   Compiling icu_normalizer v2.2.0
   Compiling icu_properties v2.2.0
   Compiling ryu v1.0.23
   Compiling strsim v0.11.1
   Compiling regex-syntax v0.8.10
   Compiling iri-string v0.7.12
   Compiling tokio-rustls v0.26.4
   Compiling tinyvec_macros v0.1.1
   Compiling heck v0.5.0
   Compiling clap_lex v1.1.0
   Compiling idna_adapter v1.2.2
   Compiling hyper-rustls v0.27.9
   Compiling tinyvec v1.11.0
   Compiling serde_urlencoded v0.7.1
   Compiling clap_derive v4.6.1
   Compiling idna v1.1.0
   Compiling clap_builder v4.6.0
   Compiling regex-automata v0.4.14
   Compiling tower-http v0.6.8
   Compiling axhub-helpers v0.1.23 (/work/crates/axhub-helpers)
   Compiling simple_asn1 v0.6.4
   Compiling der-parser v9.0.0
   Compiling url v2.5.8
   Compiling pem v3.0.6
   Compiling http-body-util v0.1.3
   Compiling rustix v1.1.4
   Compiling lazy_static v1.5.0
   Compiling data-encoding v2.11.0
   Compiling iana-time-zone v0.1.65
   Compiling log v0.4.29
   Compiling cpufeatures v0.2.17
   Compiling jsonwebtoken v9.3.1
   Compiling sha2 v0.10.9
   Compiling unicode-normalization v0.1.25
error: failed to run custom build command for `axhub-helpers v0.1.23 (/work/crates/axhub-helpers)`

Caused by:
  process didn't exit successfully: `/tmp/axhub-target/debug/build/axhub-helpers-f17266d839117d0a/build-script-build` (exit status: 1)
  --- stdout
  cargo:rustc-check-cfg=cfg(coverage)
  cargo:rerun-if-changed=../../src/axhub-helpers/catalog.ts

  --- stderr
  Error: catalog parser failed (expected string quote, got '이'); bun fallback failed: 
warning: build failed, waiting for other jobs to finish...
```

## Linux Secret Service Docker live keychain smoke after codegen fix
```
exit=0
cargo 1.95.0 (f2d3ce0bd 2026-03-21)
rustc 1.95.0 (59807616e 2026-04-14)
secret-tool recovered length=258

running 1 test
test linux_secret_service_reads_go_keyring_envelope ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

debconf: unable to initialize frontend: Dialog
debconf: (TERM is not set, so the dialog frontend is not usable.)
debconf: falling back to frontend: Readline
debconf: unable to initialize frontend: Readline
debconf: (This frontend requires a controlling tty.)
debconf: falling back to frontend: Teletype
debconf: unable to initialize frontend: Teletype
debconf: (This frontend requires a controlling tty.)
debconf: falling back to frontend: Noninteractive
    Updating crates.io index
 Downloading crates ...
  Downloaded atomic-waker v1.1.2
  Downloaded errno v0.3.14
  Downloaded num-conv v0.2.1
  Downloaded powerfmt v0.2.0
  Downloaded lazy_static v1.5.0
  Downloaded is_terminal_polyfill v1.70.2
  Downloaded lru-slab v0.1.2
  Downloaded http-body v1.0.1
  Downloaded idna_adapter v1.2.2
  Downloaded heck v0.5.0
  Downloaded cpufeatures v0.2.17
  Downloaded percent-encoding v2.3.2
  Downloaded rand_core v0.9.5
  Downloaded oid-registry v0.7.1
  Downloaded potential_utf v0.1.5
  Downloaded pem v3.0.6
  Downloaded ppv-lite86 v0.2.21
  Downloaded pin-project-lite v0.2.17
  Downloaded untrusted v0.9.0
  Downloaded quinn-udp v0.5.14
  Downloaded tower-service v0.3.3
  Downloaded once_cell v1.21.4
  Downloaded iana-time-zone v0.1.65
  Downloaded rand_chacha v0.9.0
  Downloaded getrandom v0.3.4
  Downloaded num-traits v0.2.19
  Downloaded tinyvec_macros v0.1.1
  Downloaded proc-macro2 v1.0.106
  Downloaded subtle v2.6.1
  Downloaded icu_properties v2.2.0
  Downloaded sync_wrapper v1.0.2
  Downloaded icu_locale_core v2.2.0
  Downloaded want v0.3.1
  Downloaded icu_normalizer v2.2.0
  Downloaded version_check v0.9.5
  Downloaded nom v7.1.3
  Downloaded mio v1.2.0
  Downloaded zerofrom-derive v0.1.7
  Downloaded utf8parse v0.2.2
  Downloaded reqwest v0.12.28
  Downloaded rustc-hash v2.1.2
  Downloaded futures-util v0.3.32
  Downloaded zerofrom v0.1.7
  Downloaded quinn-proto v0.11.14
  Downloaded try-lock v0.2.5
  Downloaded stable_deref_trait v1.2.1
  Downloaded clap_builder v4.6.0
  Downloaded tinystr v0.8.3
  Downloaded slab v0.4.12
  Downloaded yoke-derive v0.8.2
  Downloaded regex-syntax v0.8.10
  Downloaded time-macros v0.2.27
  Downloaded shlex v1.3.0
  Downloaded zeroize v1.8.2
  Downloaded rusticata-macros v4.1.0
  Downloaded thiserror-impl v2.0.18
  Downloaded thiserror-impl v1.0.69
  Downloaded synstructure v0.13.2
  Downloaded chrono v0.4.44
  Downloaded zerovec-derive v0.11.3
  Downloaded thiserror v2.0.18
  Downloaded thiserror v1.0.69
  Downloaded yoke v0.8.2
  Downloaded zmij v1.0.21
  Downloaded writeable v0.6.3
  Downloaded smallvec v1.15.1
  Downloaded tokio-rustls v0.26.4
  Downloaded tempfile v3.27.0
  Downloaded semver v1.0.28
  Downloaded socket2 v0.6.3
  Downloaded tinyvec v1.11.0
  Downloaded serde_core v1.0.228
  Downloaded unicode-ident v1.0.24
  Downloaded simple_asn1 v0.6.4
  Downloaded sha2 v0.10.9
  Downloaded uuid v1.23.1
  Downloaded libc v0.2.186
  Downloaded ryu v1.0.23
  Downloaded zerotrie v0.2.4
  Downloaded zerovec v0.11.6
  Downloaded unicode-normalization v0.1.25
  Downloaded tower v0.5.3
  Downloaded serde_json v1.0.149
  Downloaded serde v1.0.228
  Downloaded webpki-roots v1.0.7
  Downloaded x509-parser v0.16.0
  Downloaded time v0.3.47
  Downloaded url v2.5.8
  Downloaded typenum v1.20.0
  Downloaded zerocopy v0.8.48
  Downloaded syn v2.0.117
  Downloaded tracing-core v0.1.36
  Downloaded tower-http v0.6.8
  Downloaded serde_derive v1.0.228
  Downloaded rustls v0.23.40
  Downloaded rustls-webpki v0.103.13
  Downloaded rustls-pki-types v1.14.1
  Downloaded rustix v1.1.4
  Downloaded tracing v0.1.44
  Downloaded regex-automata v0.4.14
  Downloaded icu_properties_data v2.2.0
  Downloaded hyper v1.9.0
  Downloaded aho-corasick v1.1.4
  Downloaded tower-layer v0.3.3
  Downloaded regex v1.12.3
  Downloaded iri-string v0.7.12
  Downloaded idna v1.1.0
  Downloaded hyper-util v0.1.20
  Downloaded http v1.4.0
  Downloaded rand v0.9.4
  Downloaded num-bigint v0.4.6
  Downloaded minimal-lexical v0.2.1
  Downloaded memchr v2.8.0
  Downloaded cc v1.2.61
  Downloaded bytes v1.11.1
  Downloaded utf8_iter v1.0.4
  Downloaded time-core v0.1.8
  Downloaded serde_urlencoded v0.7.1
  Downloaded quinn v0.11.9
  Downloaded icu_collections v2.2.0
  Downloaded tokio v1.52.1
  Downloaded asn1-rs v0.6.2
  Downloaded getrandom v0.4.2
  Downloaded strsim v0.11.1
  Downloaded log v0.4.29
  Downloaded jsonwebtoken v9.3.1
  Downloaded icu_normalizer_data v2.2.0
  Downloaded futures-channel v0.3.32
  Downloaded der-parser v9.0.0
  Downloaded icu_provider v2.2.0
  Downloaded clap v4.6.1
  Downloaded base64 v0.22.1
  Downloaded clap_derive v4.6.1
  Downloaded anyhow v1.0.102
  Downloaded litemap v0.8.2
  Downloaded httparse v1.10.1
  Downloaded hmac v0.12.1
  Downloaded getrandom v0.2.17
  Downloaded displaydoc v0.2.5
  Downloaded digest v0.10.7
  Downloaded bitflags v2.11.1
  Downloaded quote v1.0.45
  Downloaded ipnet v2.12.0
  Downloaded find-msvc-tools v0.1.9
  Downloaded deranged v0.5.8
  Downloaded autocfg v1.5.0
  Downloaded data-encoding v2.11.0
  Downloaded http-body-util v0.1.3
  Downloaded hyper-rustls v0.27.9
  Downloaded crypto-common v0.1.7
  Downloaded num-integer v0.1.46
  Downloaded fastrand v2.4.1
  Downloaded anstyle-parse v1.0.0
  Downloaded anstyle v1.0.14
  Downloaded futures-core v0.3.32
  Downloaded ring v0.17.14
  Downloaded form_urlencoded v1.2.2
  Downloaded clap_lex v1.1.0
  Downloaded itoa v1.0.18
  Downloaded generic-array v0.14.7
  Downloaded futures-task v0.3.32
  Downloaded futures-sink v0.3.32
  Downloaded futures-io v0.3.32
  Downloaded cfg_aliases v0.2.1
  Downloaded block-buffer v0.10.4
  Downloaded asn1-rs-impl v0.2.0
  Downloaded anstyle-query v1.1.5
  Downloaded anstream v1.0.0
  Downloaded colorchoice v1.0.5
  Downloaded cfg-if v1.0.4
  Downloaded asn1-rs-derive v0.5.1
  Downloaded linux-raw-sys v0.12.1
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling unicode-ident v1.0.24
   Compiling itoa v1.0.18
   Compiling libc v0.2.186
   Compiling memchr v2.8.0
   Compiling stable_deref_trait v1.2.1
   Compiling serde_core v1.0.228
   Compiling autocfg v1.5.0
   Compiling cfg-if v1.0.4
   Compiling pin-project-lite v0.2.17
   Compiling bytes v1.11.1
   Compiling shlex v1.3.0
   Compiling find-msvc-tools v0.1.9
   Compiling futures-core v0.3.32
   Compiling zmij v1.0.21
   Compiling num-traits v0.2.19
   Compiling cc v1.2.61
   Compiling serde v1.0.228
   Compiling once_cell v1.21.4
   Compiling smallvec v1.15.1
   Compiling version_check v0.9.5
   Compiling litemap v0.8.2
   Compiling mio v1.2.0
   Compiling socket2 v0.6.3
   Compiling powerfmt v0.2.0
   Compiling writeable v0.6.3
   Compiling time-core v0.1.8
   Compiling num-conv v0.2.1
   Compiling serde_json v1.0.149
   Compiling futures-sink v0.3.32
   Compiling ring v0.17.14
   Compiling tokio v1.52.1
   Compiling deranged v0.5.8
   Compiling time-macros v0.2.27
   Compiling generic-array v0.14.7
   Compiling getrandom v0.2.17
   Compiling http v1.4.0
   Compiling syn v2.0.117
   Compiling icu_properties_data v2.2.0
   Compiling untrusted v0.9.0
   Compiling subtle v2.6.1
   Compiling utf8_iter v1.0.4
   Compiling zeroize v1.8.2
   Compiling icu_normalizer_data v2.2.0
   Compiling thiserror v1.0.69
   Compiling slab v0.4.12
   Compiling time v0.3.47
   Compiling rustls-pki-types v1.14.1
   Compiling http-body v1.0.1
   Compiling futures-io v0.3.32
   Compiling typenum v1.20.0
   Compiling httparse v1.10.1
   Compiling percent-encoding v2.3.2
   Compiling futures-task v0.3.32
   Compiling minimal-lexical v0.2.1
   Compiling futures-util v0.3.32
   Compiling nom v7.1.3
   Compiling num-integer v0.1.46
   Compiling try-lock v0.2.5
   Compiling synstructure v0.13.2
   Compiling tower-service v0.3.3
   Compiling base64 v0.22.1
   Compiling rustls v0.23.40
   Compiling anyhow v1.0.102
   Compiling num-bigint v0.4.6
   Compiling want v0.3.1
   Compiling displaydoc v0.2.5
   Compiling zerofrom-derive v0.1.7
   Compiling yoke-derive v0.8.2
   Compiling zerovec-derive v0.11.3
   Compiling serde_derive v1.0.228
   Compiling thiserror-impl v1.0.69
   Compiling rusticata-macros v4.1.0
   Compiling asn1-rs-derive v0.5.1
   Compiling asn1-rs-impl v0.2.0
   Compiling futures-channel v0.3.32
   Compiling zerofrom v0.1.7
   Compiling tracing-core v0.1.36
   Compiling getrandom v0.4.2
   Compiling yoke v0.8.2
   Compiling bitflags v2.11.1
   Compiling utf8parse v0.2.2
   Compiling thiserror v2.0.18
   Compiling atomic-waker v1.1.2
   Compiling zerovec v0.11.6
   Compiling zerotrie v0.2.4
   Compiling tracing v0.1.44
   Compiling anstyle-parse v1.0.0
   Compiling hyper v1.9.0
   Compiling asn1-rs v0.6.2
   Compiling rustls-webpki v0.103.13
   Compiling tinystr v0.8.3
   Compiling potential_utf v0.1.5
   Compiling thiserror-impl v2.0.18
   Compiling block-buffer v0.10.4
   Compiling icu_locale_core v2.2.0
   Compiling icu_collections v2.2.0
   Compiling crypto-common v0.1.7
   Compiling form_urlencoded v1.2.2
   Compiling sync_wrapper v1.0.2
   Compiling ipnet v2.12.0
   Compiling tower-layer v0.3.3
   Compiling is_terminal_polyfill v1.70.2
   Compiling oid-registry v0.7.1
   Compiling anstyle-query v1.1.5
   Compiling anstyle v1.0.14
   Compiling colorchoice v1.0.5
   Compiling icu_provider v2.2.0
   Compiling tower v0.5.3
   Compiling hyper-util v0.1.20
   Compiling digest v0.10.7
   Compiling axhub-codegen v0.1.23 (/work/crates/axhub-codegen)
   Compiling webpki-roots v1.0.7
   Compiling anstream v1.0.0
   Compiling aho-corasick v1.1.4
   Compiling icu_properties v2.2.0
   Compiling icu_normalizer v2.2.0
   Compiling ryu v1.0.23
   Compiling iri-string v0.7.12
   Compiling strsim v0.11.1
   Compiling heck v0.5.0
   Compiling regex-syntax v0.8.10
   Compiling tokio-rustls v0.26.4
   Compiling clap_lex v1.1.0
   Compiling tinyvec_macros v0.1.1
   Compiling clap_derive v4.6.1
   Compiling idna_adapter v1.2.2
   Compiling tinyvec v1.11.0
   Compiling serde_urlencoded v0.7.1
   Compiling idna v1.1.0
   Compiling hyper-rustls v0.27.9
   Compiling tower-http v0.6.8
   Compiling clap_builder v4.6.0
   Compiling axhub-helpers v0.1.23 (/work/crates/axhub-helpers)
   Compiling simple_asn1 v0.6.4
   Compiling url v2.5.8
   Compiling regex-automata v0.4.14
   Compiling der-parser v9.0.0
   Compiling pem v3.0.6
   Compiling http-body-util v0.1.3
   Compiling lazy_static v1.5.0
   Compiling data-encoding v2.11.0
   Compiling rustix v1.1.4
   Compiling log v0.4.29
   Compiling iana-time-zone v0.1.65
   Compiling cpufeatures v0.2.17
   Compiling jsonwebtoken v9.3.1
   Compiling sha2 v0.10.9
   Compiling x509-parser v0.16.0
   Compiling reqwest v0.12.28
   Compiling chrono v0.4.44
   Compiling clap v4.6.1
   Compiling unicode-normalization v0.1.25
   Compiling uuid v1.23.1
   Compiling hmac v0.12.1
   Compiling regex v1.12.3
   Compiling linux-raw-sys v0.12.1
   Compiling semver v1.0.28
   Compiling fastrand v2.4.1
   Compiling tempfile v3.27.0
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 37s
     Running tests/linux_keychain_live.rs (/tmp/axhub-target/debug/deps/linux_keychain_live-a1273ebe276f5cf9)
```

## macOS Keychain read-only probe
Command: security find-generic-password -s axhub -w (token content redacted; only exit and length recorded)
```
exit=0
token_length=642
stderr=
```

## macOS Keychain Rust live smoke
Command: cargo test -p axhub-helpers --test macos_keychain_live -- --ignored --nocapture
```
exit=0

running 1 test
test macos_keychain_reads_existing_axhub_item ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Compiling axhub-helpers v0.1.23 (/Users/wongil/Desktop/work/jocoding/axhub/crates/axhub-helpers)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.51s
     Running tests/macos_keychain_live.rs (target/debug/deps/macos_keychain_live-be7f0c3ab00443f7)
```

## Plugin helper T2 e2e matrix
Command: bun run build && bun run test:plugin-e2e:t2
```
exit=0
  [25ms]  bundle  90 modules
  [93ms] compile  bin/axhub-helpers
[run-matrix] tier=t2 selected 11 case(s): 20 21 23 25 26 27 28 29 30 31 34
[case 20] OK
[case 21] OK
[case 23] exit=0
[case 23] OK
[case 25] OK
[case 26] OK
[case 27] OK
[case 28] OK
[case 29] OK
[case 30] OK
[case 31] OK
[mock-hub] ready at http://127.0.0.1:18080 (pid 70202)
[case 34] exit=65
[case 34] OK

[run-matrix] summary: /Users/wongil/Desktop/work/jocoding/axhub/tests/e2e/claude-cli/output/summary.tsv
case_id  state  exit  wall_s
20       PASS   0     1
21       PASS   0     0
23       PASS   0     0
25       PASS   0     0
26       PASS   0     0
27       PASS   0     0
28       PASS   0     0
29       PASS   0     0
30       FAIL   64    0
31       PASS   0     0
34       FAIL   65    0

[run-matrix] OK — 11 / 11 case(s) passed
$ bun build src/axhub-helpers/index.ts --compile --outfile bin/axhub-helpers --target=bun
$ bash tests/e2e/claude-cli/run-matrix.sh --tier t2
```

## Plugin Claude T1 e2e matrix
Command: bun run test:plugin-e2e:t1
```
exit=1
[run-matrix] tier=t1 selected 8 case(s): 01 02 03 04 13 16 19 22
[case 01] state=PASS exit=0
[case 01] OK
[case 02] state=PASS exit=0
[case 02] OK
[mock-hub] ready at http://127.0.0.1:18080 (pid 82848)
[case 03] state=PASS exit=0
[case 03] OK
[mock-hub] ready at http://127.0.0.1:18080 (pid 18788)
[case 04] state=PASS exit=0
[case 04] OK
[case 13] state=PASS exit=0
[case 13] OK
[case 16] state=PASS exit=0
[case 16] OK
[case 19] state=PASS exit=0
[case 19] OK
[case 22] state=PASS exit=0

[run-matrix] summary: /Users/wongil/Desktop/work/jocoding/axhub/tests/e2e/claude-cli/output/summary.tsv
case_id  state  exit  wall_s
01       PASS   0     10
02       PASS   0     59
03       PASS   0     24
04       PASS   0     34
13       PASS   0     11
16       PASS   0     55
19       PASS   0     14
22       PASS   0     17

[run-matrix] FAIL — 1 / 8 case(s) failed
$ bash tests/e2e/claude-cli/run-matrix.sh --tier t1
  FAIL: no Korean cli_too_old phrase (오래된|업그레이드|버전|확인|axhub) in output
error: script "test:plugin-e2e:t1" exited with code 1
```

## Plugin Claude T1 case 22 rerun
Command: bash tests/e2e/claude-cli/run-matrix.sh --only 22
```
exit=1
[run-matrix] tier=pr selected 1 case(s): 22
[case 22] state=PASS exit=0

[run-matrix] summary: /Users/wongil/Desktop/work/jocoding/axhub/tests/e2e/claude-cli/output/summary.tsv
case_id  state  exit  wall_s
22       PASS   0     14

[run-matrix] FAIL — 1 / 1 case(s) failed
  FAIL: no Korean cli_too_old phrase (오래된|업그레이드|버전|확인|axhub) in output
```

## Plugin Claude T1 case 22 after doctor keyword fix
Command: bash tests/e2e/claude-cli/run-matrix.sh --only 22
```
exit=0
[run-matrix] tier=pr selected 1 case(s): 22
[case 22] state=PASS exit=0
[case 22] OK

[run-matrix] summary: /Users/wongil/Desktop/work/jocoding/axhub/tests/e2e/claude-cli/output/summary.tsv
case_id  state  exit  wall_s
22       PASS   0     13

[run-matrix] OK — 1 / 1 case(s) passed
```

## Plugin Claude T1 e2e matrix after doctor keyword fix
Command: bun run test:plugin-e2e:t1
```
exit=1
[run-matrix] tier=t1 selected 8 case(s): 01 02 03 04 13 16 19 22
[case 01] state=PASS exit=0
[case 01] OK
[case 02] state=PASS exit=0
[case 02] OK
[mock-hub] ready at http://127.0.0.1:18080 (pid 44795)
[case 03] state=PASS exit=0
[case 03] OK
[mock-hub] ready at http://127.0.0.1:18080 (pid 62526)
[case 04] state=PASS exit=0
[case 04] OK
[case 13] state=PASS exit=0
[case 13] OK
[case 16] state=PASS exit=0
[case 16] OK
[case 19] state=PASS exit=0
[case 19] OK
[case 22] state=PASS exit=0

[run-matrix] summary: /Users/wongil/Desktop/work/jocoding/axhub/tests/e2e/claude-cli/output/summary.tsv
case_id  state  exit  wall_s
01       PASS   0     12
02       PASS   0     80
03       PASS   0     13
04       PASS   0     19
13       PASS   0     40
16       PASS   0     41
19       PASS   0     58
22       PASS   0     20

[run-matrix] FAIL — 1 / 8 case(s) failed
$ bash tests/e2e/claude-cli/run-matrix.sh --tier t1
  FAIL: no Korean cli_too_old phrase (오래된|업그레이드|버전|확인|axhub) in output
error: script "test:plugin-e2e:t1" exited with code 1
```

## Plugin Claude T1 case 22 after utterance alignment
Command: bash tests/e2e/claude-cli/run-matrix.sh --only 22
```
exit=0
[run-matrix] tier=pr selected 1 case(s): 22
[case 22] state=PASS exit=0
[case 22] OK

[run-matrix] summary: /Users/wongil/Desktop/work/jocoding/axhub/tests/e2e/claude-cli/output/summary.tsv
case_id  state  exit  wall_s
22       PASS   0     21

[run-matrix] OK — 1 / 1 case(s) passed
```

## Plugin Claude T1 e2e matrix after utterance alignment
Command: bun run test:plugin-e2e:t1
```
exit=1
[run-matrix] tier=t1 selected 8 case(s): 01 02 03 04 13 16 19 22
[case 01] state=PASS exit=0
[case 01] OK
[case 02] state=PASS exit=0
[case 02] OK
[mock-hub] ready at http://127.0.0.1:18080 (pid 97103)
[case 03] state=PASS exit=0
[case 03] OK
[mock-hub] ready at http://127.0.0.1:18080 (pid 41834)
[case 04] state=PASS exit=0
[case 04] OK
[case 13] state=PASS exit=0
[case 13] OK
[case 16] state=PASS exit=0
[case 16] OK
[case 19] state=PASS exit=0
[case 19] OK
[case 22] state=PASS exit=0

[run-matrix] summary: /Users/wongil/Desktop/work/jocoding/axhub/tests/e2e/claude-cli/output/summary.tsv
case_id  state  exit  wall_s
01       PASS   0     8
02       PASS   0     64
03       PASS   0     30
04       PASS   0     24
13       PASS   0     20
16       PASS   0     40
19       PASS   0     22
22       PASS   0     16

[run-matrix] FAIL — 1 / 8 case(s) failed
$ bash tests/e2e/claude-cli/run-matrix.sh --tier t1
  FAIL: no Korean cli_too_old phrase (오래된|업그레이드|버전|확인|axhub) in output
error: script "test:plugin-e2e:t1" exited with code 1
```


## Final external verification addendum — prompt-route and plan sync

- Plugin Claude T1 after `UserPromptSubmit` prompt-route hook: **PASS** (`.omc/evidence/plugin-e2e-t1-after-prompt-route.stdout`, `OK — 8 / 8 case(s) passed`).
- Rust final gate after prompt-route fix: **PASS through `cargo audit`**, including `cargo llvm-cov --workspace --fail-under-lines 90` with **90.69% line coverage** (`.omc/evidence/final-regression-after-prompt-route.log`).
- Bun/Plugin final sync gate: **PASS** (`.omc/evidence/final-regression-after-plan-sync.log`): `bun test` 562 pass / 5 skip / 0 fail, `bun run test:e2e` 1 pass / 5 skip / 0 fail, `bun run test:plugin-e2e:t2` 11 / 11 case scripts passed, `bunx tsc --noEmit`, `lint:tone`, `lint:tone:rust`, `lint:keywords`, and `git diff --check`.
- GitNexus `detect_changes(scope=all)` reported **CRITICAL** blast radius because this Ralph branch changes the shared TS dispatcher/version-sync files plus docs; this is expected for the version-sync + hook-routing work and is covered by the final gates above.
- Remaining credential/environment-gated checks: real staging token run (`AXHUB_E2E_STAGING_TOKEN` / endpoint) and real Windows V3/AhnLab cohort were not executable from this macOS session.


## Ralph deslop pass — changed-file scope

Scope: `.omc/evidence/plugin-e2e-t1-after-prompt-route.stdout`, `.omc/evidence/ralph-external-verification-20260429.md`, `.plan/10-source-mapping.md`, `PLAN.md`, `bin/install.ps1`, `bin/install.sh`, `src/axhub-helpers/index.ts`, `src/axhub-helpers/telemetry.ts`.

Behavior lock: final gates in `.omc/evidence/final-regression-after-prompt-route.log` and `.omc/evidence/final-regression-after-plan-sync.log`, plus fresh `bun test tests/plan-consistency.test.ts` and `git diff --check` after the plan/source-map sync.

Cleanup plan: keep this as a no-op cleanup because remaining diffs are generated version-sync files, evidence logs, and plan/source-map reconciliation; deleting or reshaping them would reduce traceability instead of reducing slop. No new abstractions, dependencies, or behavior edits were introduced in the cleanup pass.

Quality gate addendum: LSP diagnostics on `src/axhub-helpers/index.ts`, `src/axhub-helpers/prompt-route.ts`, and `src/axhub-helpers/telemetry.ts` reported 0 errors.
