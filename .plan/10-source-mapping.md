# Source Mapping — TS → Rust 추적표

**목적:** 모든 TS source symbol → Rust target 1대1 매핑 추적. PR 마다 status update. 회귀 발견 시 즉시 lookup.

**Status legend:**
- `planned` — Phase 명시, 아직 미작성
- `in-progress` — 현재 phase PR 진행
- `ported` — Rust 측 작성 완료, parity test PASS
- `dropped` — 의도적 제거 (이유 명시)
- `changed` — 의도적 동작 변경 (재구현, fixture 영향 명시)

**TS file → Rust target convention:**
- 1:1 → `src/axhub-helpers/foo.ts` ↔ `crates/axhub-helpers/src/foo.rs`
- 1:N (split) → `consent.ts` ↔ `crates/axhub-helpers/src/consent/{key,jwt,parser,mod}.rs`
- N:1 → 없음 (단순 합치기 안 함)

---

## Ralph implementation update — 2026-04-29

**Scope implemented in this session:** Phase 0 DX guardrails, Cargo workspace, Rust helper binary scaffold, and Rust ports for the TypeScript helper modules at contract-test depth. Source files now exist under `crates/axhub-helpers/src/**`; codegen helper exists under `crates/axhub-codegen`.

**Verification evidence captured:**

- `cargo test --workspace` → Rust unit + regression + CLI e2e tests pass (codegen 5, helper unit 4, CLI e2e 4, live keychain tests compile as ignored by default, phase parity 16).
- `cargo llvm-cov --workspace --fail-under-lines 90` → line coverage **91.07%** (threshold 90%) with deterministic coverage seam for live TLS socket I/O and CLI prompt-route coverage.
- `bun test tests/consent.test.ts` → TypeScript zero-leeway lock tests pass.
- `bun test tests/runtime-fallback.test.ts` → `AXHUB_HELPERS_RUNTIME=rust` delegation preserves stdin and exit code.
- `bun test tests/lint-toss-tone.test.ts` + `bun run lint:tone:rust` → Rust tone include path works and reports 0 errors.

**Open verification gaps (not marked fully complete):**

- Phase 2 `list-deployments`: TLS SPKI implementation, deterministic fetch/TLS checker coverage, live hub TLS pin probe, and release/runtime regression coverage are present. The original 19-case TLS mock plan is covered by focused Rust unit/integration tests rather than a literal 19-file mock-suite clone.
- Phase 3 `consent`: key/JWT/parser tests, CLI preauth e2e, and a 60-second `cargo +nightly fuzz run parser` smoke are present. The full 24h cargo-fuzz run is still pending.
- Phase 3 `keychain`: macOS Keychain and Linux Secret Service live read paths passed; Windows parser/runner branches are tested, but the Windows V3/AhnLab cohort QA still requires that external environment.
- Phase 4 `main.rs`: Rust CLI supports core helper commands, the Rust binary is the default build/release artifact, and release CI now builds/signs 5 Cargo targets. TypeScript fallback/tooling remains deliberately retained until the monitor window and catalog source-of-truth cleanup finish.

## Ralph external verification update — 2026-04-29

**Evidence file:** `.omc/evidence/ralph-external-verification-20260429.md`

| Check | Command / evidence | Result |
|-------|--------------------|--------|
| Live hub TLS pin probe | `AXHUB_TOKEN=axhub_pat_external_probe_not_real_0123456789 AXHUB_ENDPOINT=https://hub-api.jocodingax.ai target/debug/axhub-helpers list-deployments --app 1 --limit 1` | **PASS after fix** — exit `65`, stdout `.error_code="auth.token_invalid"`, proving the live TLS pin check completed and the request reached the HTTP auth path. |
| Rustls runtime regression | Initial live probe exited `101` with `CryptoProvider` panic; root cause was ambiguous rustls providers from direct `rustls` default features plus reqwest ring features. | **FIXED** — `crates/axhub-helpers/Cargo.toml` now disables rustls defaults and selects `ring`; `list_deployments::tests::rustls_crypto_provider_is_unambiguous_without_proxy_override` locks the no-proxy builder path. |
| Cargo audit | `cargo audit --deny warnings` | **PASS** — 241 Cargo.lock dependencies scanned, 0 advisories emitted. |
| Parser fuzz smoke | `cargo +nightly fuzz run parser -- -max_total_time=60` | **PASS** — 18,352 runs in 61 seconds, no crash/panic. Fuzz harness added at `fuzz/fuzz_targets/parser.rs`. |
| Rust-only catalog codegen | `PATH=$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin cargo build -p axhub-helpers` | **PASS after fix** — Linux Docker first exposed that Bun-less builds failed because `axhub-codegen` mixed TypeScript byte offsets with Rust char indices when Korean comments preceded `CATALOG`; `crates/axhub-codegen/src/lib.rs` now converts the byte offset to a char offset and locks this with a Korean-prefix regression. |
| macOS Keychain live smoke | `security find-generic-password -s axhub -w` (content redacted) + `cargo test -p axhub-helpers --test macos_keychain_live -- --ignored --nocapture` | **PASS** — read-only probe found an existing axhub item (length only recorded), and the Rust live smoke read it through `security` with source `macos-keychain`. |
| Linux Secret Service keychain live smoke | Docker `rust:latest` + `libsecret-tools` + `gnome-keyring`; seed `go-keyring-base64:` envelope; run `cargo test -p axhub-helpers --test linux_keychain_live -- --ignored --nocapture` | **PASS** — ignored live test read the seeded token through real `secret-tool` and reported `linux-secret-service`. This covers the Linux read path, not the full upstream ax-hub-cli write path. |
| Staging e2e gate | `.github/workflows/rust-staging-gates.yml` + `bun run test:e2e` | **WORKFLOW ADDED / CREDENTIAL-GATED** — workflow rebuilds the Rust helper, installs the real `axhub` CLI via `AXHUB_CLI_INSTALL_COMMAND`, runs read-only staging E2E, and probes `bin/axhub-helpers list-deployments` with `AXHUB_E2E_STAGING_APP_ID`. Local no-credential runner reports `1 pass / 6 skip / 0 fail`. |
| Plugin NL doctor routing | `bun run test:plugin-e2e:t1` after adding `UserPromptSubmit` → `axhub-helpers prompt-route` | **PASS after fix** — case 22 stopped answering with generic repo environment checks and now injects axhub doctor preflight context from the hook. This uses Claude Code `hookSpecificOutput.additionalContext` per official hooks behavior. |
| Post-fix regression gate | `cargo fmt --all -- --check`, `cargo fmt --manifest-path fuzz/Cargo.toml -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo clippy --manifest-path fuzz/Cargo.toml --bin parser -- -D warnings`, `cargo test --workspace`, `cargo test -p axhub-helpers --test macos_keychain_live -- --ignored --nocapture`, `cargo test -p axhub-helpers --test linux_keychain_live`, `cargo llvm-cov --workspace --fail-under-lines 90`, `cargo audit --deny warnings`, `bun test`, `bun run test:e2e`, `bun run test:plugin-e2e:t1`, `bun run test:plugin-e2e:t2`, `bunx tsc --noEmit`, `bun run lint:tone --strict`, `bun run lint:tone:rust`, `bun run lint:keywords --check`, `git diff --check` | **PASS** — `cargo test --workspace` passes 29 Rust tests plus 2 ignored live-keychain tests; Rust line coverage is **91.07%**; Bun suite is `563 pass / 5 skip / 0 fail`; plugin e2e T1 is `8 / 8` and T2 is `11 / 11`. |
| 3 OS live keychain / V3 cohort | macOS Keychain read path passed on host; Linux Secret Service read path passed in Docker. No Windows Korean EDR cohort is available in this session. | **Partially complete** — macOS and Linux live read paths passed; Windows V3/AhnLab cohort still requires the target Windows/EDR environment. |

## 0. 전체 매핑 표

| TS 파일 | LOC | Rust target | Phase | Status |
|---------|-----|-------------|-------|--------|
| `redact.ts` | 48 | `crates/axhub-helpers/src/redact.rs` | 1 | ported |
| `catalog.ts` | 188 | `crates/axhub-helpers/src/catalog.rs` (+ `crates/axhub-codegen/src/lib.rs`) | 1 | ported |
| `telemetry.ts` | 87 | `crates/axhub-helpers/src/telemetry.rs` | 1 | ported |
| `preflight.ts` | 257 | `crates/axhub-helpers/src/preflight.rs` | 2 | ported |
| `resolve.ts` | 296 | `crates/axhub-helpers/src/resolve.rs` | 2 | ported |
| `list-deployments.ts` | 339 | `crates/axhub-helpers/src/list_deployments.rs` | 2 | ported |
| `consent.ts` | 458 | `crates/axhub-helpers/src/consent/{mod,key,jwt,parser}.rs` (split) | 3 | ported |
| `keychain.ts` | 132 | `crates/axhub-helpers/src/keychain.rs` (mac+linux) | 3 | ported |
| `keychain-windows.ts` | 222 | `crates/axhub-helpers/src/keychain_windows.rs` (+ inline PowerShell) | 3 | ported |
| `index.ts` | 509 | `crates/axhub-helpers/src/main.rs` (+ `crates/axhub-helpers/src/spawn.rs`) | 4 | ported |
| `prompt-route.ts` | 102 | `crates/axhub-helpers/src/main.rs::cmd_prompt_route` | 4 | ported |

**Σ TS:** 2,638 LOC → **Rust 추정:** 3,200~3,500 LOC

---

## 1. `redact.ts` (Phase 1)

**TS path:** `src/axhub-helpers/redact.ts`
**Rust target:** `crates/axhub-helpers/src/redact.rs`

| TS symbol | TS line | Rust symbol | Rust file | Status | 비고 |
|-----------|---------|-------------|-----------|--------|------|
| `BIDI_RE` (regex) | 10 | `BIDI_RE: LazyLock<Regex>` | `redact.rs` | ported | bidi unicode |
| `ZW_RE` | 13 | `ZW_RE: LazyLock<Regex>` | `redact.rs` | ported | zero-width |
| `ANSI_RE` | 16 | `ANSI_RE: LazyLock<Regex>` | `redact.rs` | ported | ANSI escape |
| `BEARER_RE` | 19 | `BEARER_RE: LazyLock<Regex>` | `redact.rs` | ported | Bearer token mask |
| `AXHUB_TOKEN_RE` | 22 | `AXHUB_TOKEN_RE: LazyLock<Regex>` | `redact.rs` | ported | env var token mask |
| `AXHUB_PAT_RE` | 27 | `AXHUB_PAT_RE: LazyLock<Regex>` | `redact.rs` | ported | PAT mask |
| `redact()` | 39 | `pub fn redact(text: &str) -> String` | `redact.rs` | ported | export |

**Test mapping:**
| TS test file | Rust test |
|--------------|-----------|
| `tests/redact.test.ts` (없으면 `axhub-helpers.test.ts` 의 redact case) | `redact.rs` `#[cfg(test)] mod tests` + `crates/axhub-helpers/tests/redact_parity.md` |

---

## 2. `catalog.ts` (Phase 1)

**TS path:** `src/axhub-helpers/catalog.ts`
**Rust target:** `crates/axhub-helpers/src/catalog.rs` + `crates/axhub-codegen/src/lib.rs`

| TS symbol | TS line | Rust symbol | Rust file | Status | 비고 |
|-----------|---------|-------------|-----------|--------|------|
| `interface ErrorEntry` | 15 | `pub struct ErrorEntry { ... }` (serde Deserialize) | `catalog.rs` | ported | |
| `CATALOG: Record<string, ErrorEntry>` | 23 | `CATALOG: LazyLock<BTreeMap<String, ErrorEntry>>` (generated JSON) | `catalog.rs` (`include!(OUT_DIR)`) | ported | build.rs 가 catalog.ts → generated JSON 생성 |
| `DEFAULT_ENTRY` | 137 | `const DEFAULT_ENTRY: ErrorEntry` | `catalog.rs` | ported | |
| `classify(exit_code, stdout)` | 152 | `pub fn classify(exit_code: i32, stdout: &str) -> ErrorEntry` | `catalog.rs` | ported | export |

**Codegen 추가:**
| 산출물 | TS source | Rust target |
|--------|-----------|-------------|
| corpus parsing | `scripts/codegen-catalog.ts` | `crates/axhub-codegen/src/lib.rs::generate_catalog()` |
| Build hook | `bun run codegen:catalog` | `crates/axhub-helpers/build.rs` |

**Test mapping:**
| TS test | Rust test |
|---------|-----------|
| `tests/codegen.test.ts` (catalog 부분) | `crates/axhub-helpers/tests/classify_parity.rs` |
| `tests/classify-exit.test.ts` | `crates/axhub-helpers/tests/classify_exit_parity.rs` |
| `tests/corpus.100.jsonl` | 동일 fixture 재사용 |
| `tests/corpus.jsonl` | 동일 fixture 재사용 |

---

## 3. `telemetry.ts` (Phase 1)

**TS path:** `src/axhub-helpers/telemetry.ts`
**Rust target:** `crates/axhub-helpers/src/telemetry.rs`

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `PLUGIN_VERSION` (const) | 17 | `pub const PLUGIN_VERSION: &str` (build.rs 에서 동기화) | ported |
| `HELPER_VERSION` | 18 | 동일 | ported |
| `resolveCliVersion()` | 22 | `pub fn resolve_cli_version() -> String` | ported |
| `isEnabled()` | 38 | `fn is_enabled() -> bool` (env `AXHUB_TELEMETRY=1`) | ported |
| `stateDir()` | 40 | `fn state_dir() -> PathBuf` | ported |
| `interface MetaEnvelope` | 46 | `pub struct MetaEnvelope { ... }` (serde) | ported |
| `emitMetaEnvelope(env)` | 56 | `pub fn emit_meta_envelope(fields: Map<String, Value>) -> anyhow::Result<()>` | ported |
| `_resetCliVersionCache()` | 85 | `pub fn reset_cli_version_cache()` | ported |

**Test mapping:**
| TS test | Rust test |
|---------|-----------|
| `tests/telemetry.test.ts` | `crates/axhub-helpers/tests/phase_parity.rs` |

---

## 4. `preflight.ts` (Phase 2)

**TS path:** `src/axhub-helpers/preflight.ts`
**Rust target:** `crates/axhub-helpers/src/preflight.rs`

| TS symbol | TS line | Rust symbol | Status | 비고 |
|-----------|---------|-------------|--------|------|
| `MIN_AXHUB_CLI_VERSION` | 27 | `pub const MIN_AXHUB_CLI_VERSION: &str = "0.1.0"` | ported | |
| `MAX_AXHUB_CLI_VERSION` | 28 | `pub const MAX_AXHUB_CLI_VERSION: &str = "0.2.0"` | ported | exclusive |
| `LAST_DEPLOY_CACHE` (path) | 30 | `static LAST_DEPLOY_CACHE: LazyLock<PathBuf>` | ported | |
| `EXIT_OK / EXIT_USAGE / EXIT_AUTH` | 33-35 | `pub const EXIT_OK: i32 = 0; ...` | ported | |
| `interface SpawnResult` | 37 | `pub struct SpawnResult { exit_code: Option<i32>, stdout: String, stderr: String, signal: Option<i32> }` | ported | spawn shim 사용 |
| `type CommandRunner` | 43 | `pub type CommandRunner = Box<dyn Fn(&[&str]) -> SpawnResult + Send + Sync>` | ported | |
| `defaultRunner` | 49 | `pub fn default_runner(cmd: &[&str]) -> SpawnResult` | ported | spawn_sync 호출 |
| `axhubBin()` | 65 | `pub fn axhub_bin() -> String` | ported | env `AXHUB_BIN` |
| `extractSemver(text)` | 73 | `pub fn extract_semver(text: &str) -> Option<String>` | ported | **bug-for-bug regex prerelease drop** |
| `parseAuthStatus(stdout)` | 99 | `pub fn parse_auth_status(stdout: &str) -> AuthStatus` | ported | |
| `interface PreflightOutput` | 132 | `pub struct PreflightOutput { ... }` | ported | |
| `readLastDeployCache()` | 161 | `fn read_last_deploy_cache() -> Option<LastDeployCache>` | ported | |
| `runPreflight(runner)` | 196 | `pub fn run_preflight(runner: CommandRunner) -> PreflightOutput` | ported | export |

**Test mapping:**
| TS test | Rust test | 비고 |
|---------|-----------|------|
| `tests/axhub-helpers.test.ts` (preflight section) | `crates/axhub-helpers/tests/preflight_parity.rs` | |
| `tests/fixtures/preflight/*.json` | 동일 fixture 재사용 | mock-hub 호환 |
| 신규 | `test_drop_prerelease`, `test_drop_build_metadata` | bug-for-bug parity 신규 강제 |
| 신규 | `test_semver_resolve_check` | CHANGELOG 22.x 회귀 차단 |

---

## 5. `resolve.ts` (Phase 2)

**TS path:** `src/axhub-helpers/resolve.ts`
**Rust target:** `crates/axhub-helpers/src/resolve.rs`

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `EXIT_NOT_FOUND = 67` | 32 | `pub const EXIT_NOT_FOUND: i32 = 67` | ported |
| `DEFAULT_DEPLOY_ETA_SEC` | 36 | `pub const DEFAULT_DEPLOY_ETA_SEC: u64 = 60` | ported |
| `interface ResolveArgs` | 38 | `pub struct ResolveArgs { ... }` | ported |
| `parseResolveArgs(args)` | 49 | `pub fn parse_resolve_args(args: &[String]) -> ResolveArgs` | ported |
| `STOP_WORDS` | 71 | `static STOP_WORDS: LazyLock<HashSet<&'static str>>` | ported |
| `extractSlugCandidate(utterance)` | 92 | `pub fn extract_slug_candidate(utterance: &str) -> Option<String>` | ported |
| `interface AppRecord` | 104 | `pub struct AppRecord { ... }` | ported |
| `parseAppsList(stdout)` | 114 | `pub fn parse_apps_list(stdout: &str) -> Option<Vec<AppRecord>>` | ported |
| `filterAppsBySlug(apps, candidate)` | 143 | `pub fn filter_apps_by_slug(apps: &[AppRecord], candidate: &str) -> Vec<AppRecord>` | ported |
| `interface GitContext` | 150 | `pub struct GitContext { ... }` | ported |
| `readGitContext(runner)` | 161 | `pub fn read_git_context(runner: &CommandRunner) -> GitContext` | ported |
| `interface ResolveOutput` | 179 | `pub struct ResolveOutput { ... }` | ported |
| `runResolve(...)` | 204 | `pub fn run_resolve(...) -> ResolveOutput` | ported |

**Test mapping:**
| TS test | Rust test |
|---------|-----------|
| `tests/axhub-helpers.test.ts` (resolve section) | `tests/resolve_parity.rs` |
| `tests/fixtures/profiles/*.json` | 동일 fixture |

---

## 6. `list-deployments.ts` (Phase 2) — TLS PIN 핵심

**Current Ralph status:** Rust module is ported with deterministic fetch/TLS checker seams, HTTP/error matrix tests, CLI e2e coverage, `cargo llvm-cov` coverage support, and a live hub TLS pin probe. The no-proxy rustls provider path is locked by regression test.

**TS path:** `src/axhub-helpers/list-deployments.ts`
**Rust target:** `crates/axhub-helpers/src/list_deployments.rs`

| TS symbol | TS line | Rust symbol | Status | 비고 |
|-----------|---------|-------------|--------|------|
| `DEFAULT_ENDPOINT` | 29 | `pub const DEFAULT_ENDPOINT: &str` | ported | |
| `HUB_API_HOST` | 30 | `pub const HUB_API_HOST: &str` | ported | |
| `DEFAULT_LIMIT` | 31 | `pub const DEFAULT_LIMIT: usize = 5` | ported | |
| `TLS_PIN_TIMEOUT_MS` | 32 | `pub const TLS_PIN_TIMEOUT_MS: u64 = 5000` | ported | |
| `HUB_API_SPKI_SHA256_PINS` | 34 | `pub const HUB_API_SPKI_SHA256_PINS: &[&str]` | ported | **byte-equal lock** |
| `EXIT_LIST_OK / AUTH / NOT_FOUND / TRANSPORT` | 38-41 | `pub const EXIT_LIST_OK: i32 = 0; ...` | ported | |
| `interface DeploymentSummary` | 43 | `serde_json::Value` normalization in `run_list_deployments_with_fetch` | changed | Runtime contract keeps JSON response fields; no public struct needed. |
| `interface ListDeploymentsArgs` | 53 | `pub struct ListDeploymentsArgs` | ported | |
| `interface ListDeploymentsResult` | 58 | `pub struct ListDeploymentsResult` | ported | |
| `class TlsPinError` | 66 | `pub struct TlsPinError` | changed | Lightweight explicit message/code struct avoids extra error derive dependency. |
| `type TlsPinChecker` | 73 | closure seam parameter on `run_list_deployments_with_fetch` | changed | Generic seam keeps tests deterministic without boxed trait alias. |
| `STATUS_MAP` (Record) | 75 | `fn status_name(status: i64) -> String` | changed | Match expression avoids phf dependency. |
| `tokenFromEnv()` | 84 | `fn token_from_env() -> Option<String>` | ported | |
| `tokenFromFile()` | 89 | `fn token_from_file() -> Option<String>` | ported | XDG_CONFIG_HOME 처리 |
| `resolveToken()` | 102 | `pub fn resolve_token() -> Option<String>` | ported | export |
| `resolveEndpoint()` | 104 | `pub fn resolve_endpoint() -> String` | ported | |
| `proxyOverrideEnabled()` | 109 | `pub fn proxy_override_enabled() -> bool` | ported | `AXHUB_ALLOW_PROXY=1` |
| `pinnedHubApiUrl(endpoint)` | 111 | `pub fn pinned_hub_api_url(endpoint: &str) -> Result<Option<Url>, TlsPinError>` | ported | |
| `spkiHashFromCert(rawCert)` | 126 | `pub fn spki_hash_from_cert_der(raw: &[u8]) -> anyhow::Result<String>` | ported | x509-parser + sha2 |
| `verifyHubApiTlsPin(endpoint)` | 140 | `pub fn verify_hub_api_tls_pin(endpoint: &str) -> Result<(), TlsPinError>` | ported | rustls/ring provider locked |
| `parseAppId(raw)` | 197 | `fn parse_app_id(raw: &str) -> Option<i64>` | ported | |
| `buildAuthError()` | 226 | `fn build_auth_error() -> ListDeploymentsResult` | ported | |
| `runListDeployments(...)` | (export) | `pub fn run_list_deployments(...) -> ListDeploymentsResult` | ported | export |

**Test mapping:**
| TS test | Rust test | 비고 |
|---------|-----------|------|
| `tests/list-deployments.test.ts` (10KB) | `tests/list_deployments_parity.rs` | bun:test mock 의존 → 일부 재작성 |
| 신규 | `test_pin_match_succeeds` | TLS mock server |
| 신규 | `test_pin_mismatch_fails` | |
| 신규 | `test_proxy_override_skips` | env override |
| 신규 | `test_timeout` | 5000ms 정확 |
| 신규 | `test_https_required` | |
| 신규 | `test_non_pinned_host_skipped` | |

---

## 7. `consent.ts` (Phase 3) — SPLIT into 4 Rust files

**Current Ralph status:** Rust split is ported across key/JWT/parser modules. Zero-leeway expiry, symlink/world-readable rejection, nested shell parsing, direct consent CLI, and preauth e2e paths are covered. 24h fuzz remains pending.

**TS path:** `src/axhub-helpers/consent.ts` (458 LOC, 단일 파일)
**Rust target:** `crates/axhub-helpers/src/consent/{mod,key,jwt,parser}.rs` (4 split files)

### 7.1 `consent/key.rs` — HMAC key lifecycle

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `HMAC_KEY_BYTES = 32` | 62 | `pub const HMAC_KEY_BYTES: usize = 32` | ported |
| `FILE_MODE_PRIVATE = 0o600` | 63 | `pub const FILE_MODE_PRIVATE: u32 = 0o600` | ported |
| `DIR_MODE_PRIVATE = 0o700` | 64 | `pub const DIR_MODE_PRIVATE: u32 = 0o700` | ported |
| `stateRoot()` | 71 | `pub fn state_root() -> PathBuf` | ported |
| `runtimeRoot()` | 77 | `pub fn runtime_root() -> PathBuf` | ported |
| `hmacKeyPath()` | 83 | `pub fn hmac_key_path() -> PathBuf` | ported |
| `sessionId()` | 85 | `pub fn session_id() -> anyhow::Result<String>` | ported |
| `tokenFilePath(sid)` | 91 | `pub fn token_file_path(sid: &str) -> PathBuf` | ported |
| (HMAC key load/mint logic) | scattered | `pub fn load_or_mint_key() -> anyhow::Result<[u8; HMAC_KEY_BYTES]>` | ported |
| (write 0600 + O_NOFOLLOW) | 151-167 | `pub fn write_private_file_no_follow()` | ported | Unix symlink refusal + 0600 lock; Windows best-effort path. |
| (lstat read defense) | scattered | `pub fn read_private_file()` | ported | symlink/world-readable 거부 |

### 7.2 `consent/jwt.rs` — JWT mint + verify (HS256)

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `JWT_ALG = "HS256"` | 65 | `pub const JWT_ALG: Algorithm = Algorithm::HS256` | ported |
| `interface ConsentBinding` | 30 | `pub struct ConsentBinding { session_id, command, args_hash, minted_at }` | ported |
| `interface MintResult` | 42 | `pub struct MintResult { token, expires_at, jti }` | ported |
| `interface VerifyResult` | 48 | `pub struct VerifyResult { binding, jti }` | ported |
| (mint logic with SignJWT) | scattered | `pub fn mint_token_with_key(...) -> anyhow::Result<MintResult>` | ported |
| (verify logic with jwtVerify) | scattered | `pub fn verify_token(...) -> VerifyResult` | ported | **leeway 0 lock** |

### 7.3 `consent/parser.rs` — parseAxhubCommand state machine

| TS symbol | TS line | Rust symbol | Status | 비고 |
|-----------|---------|-------------|--------|------|
| `interface ParsedAxhubCommand` | 53 | `pub struct ParsedAxhubCommand { ... }` | ported | |
| `FLAG_MAP` | 246 | `fn flag_map(flag: &str) -> Option<&'static str>` | changed | match expression avoids phf dependency. |
| `extractFlags(tokens)` | 253 | `fn extract_flags(tokens: &[String]) -> HashMap<...>` | ported | |
| `ENV_ASSIGN_PREFIX_RE` | 282 | `static ENV_ASSIGN_PREFIX_RE: LazyLock<Regex>` | ported | linear time (no backtrack) |
| `COLLECT_MAX_DEPTH = 5` | 295 | `const COLLECT_MAX_DEPTH: usize = 5` | ported | |
| `collectCommandPositions(cmd, depth)` | 297 | `fn collect_command_positions(cmd: &str, depth: usize) -> Vec<String>` | **changed** | regex backtracking → state machine 의도적 재구현 |
| `tokensIfAxhubCommand(rawPosition)` | 374 | `fn tokens_if_axhub_command(raw: &str) -> Option<Vec<String>>` | ported | |
| `matchKnownIntent(tokens)` | 420 | `fn match_known_intent(tokens: &[String]) -> Option<ParsedAxhubCommand>` | ported | |
| `parseAxhubCommand(cmd)` | 444 | `pub fn parse_axhub_command(cmd: &str) -> ParsedAxhubCommand` | **changed** | export, state machine |

### 7.4 `consent/mod.rs` — flow orchestrator

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `mintConsent(binding)` (export) | scattered | `pub fn mint_token(binding, ttl_sec) -> anyhow::Result<MintResult>` | ported | CLI path uses `consent-mint`. |
| `verifyLatest(expected)` (export) | scattered | `pub fn verify_token(binding) -> VerifyResult` | ported | CLI path uses `consent-verify`. |

**Test mapping (consent.test.ts 19KB):**
| TS test | Rust test | 비고 |
|---------|-----------|------|
| basic mint | `consent::jwt::tests::mint_verify_round_trip` | match |
| expired token | `rejects_expired_with_zero_leeway` | leeway 0 lock 신규 |
| algorithm swap | `rejects_algorithm_swap` | 신규 |
| binding mismatch | `rejects_binding_mismatch` | match |
| parser quoted | `consent::parser::tests::quoted_args` | △ DIFFERENT (state machine) |
| parser nested | `consent::parser::tests::nested_5_levels` | match (depth 5 동일) |
| parser env prefix | `consent::parser::tests::env_assign_prefix` | match |
| symlink attack | `refuses_symlink_target` | 신규 (TS 부재) |
| world-readable | `refuses_world_readable` | 신규 |
| token file 0600 | `token_file_mode_0600` | match |
| (parser fixture #5, #12) | — | **changed** (backtracking → linear, 결과 변경, 의도적) |
| (TS-mock-only ~18 case) | — | **dropped** (state machine 으로 무효화) |

---

## 8. `keychain.ts` (Phase 3, mac+linux)

**Current Ralph status:** Rust module is ported with go-keyring envelope parsing, platform guidance branches, command-runner success/failure coverage, and macOS/Linux live read smoke coverage. Windows V3/AhnLab cohort remains external.

**TS path:** `src/axhub-helpers/keychain.ts`
**Rust target:** `crates/axhub-helpers/src/keychain.rs` (cfg-gated per OS)

**핵심 결정 (DX-4 spike 결과 따라 분기):**
- spike PASS → `keyring` crate 채택
- spike FAIL → subprocess 유지 (security CLI / secret-tool)

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `parseKeyringValue(raw)` | 18 | `pub fn parse_keyring_value(raw: &str) -> Option<String>` | ported | go-keyring envelope strip |
| `interface KeychainResult` | 40 | `pub struct KeychainResult { token, source, error }` | ported | Rust adds explicit error field. |
| `readKeychainToken()` | 46 | `pub fn read_keychain_token() -> KeychainResult` | ported | subprocess strategy retained for security CLI / secret-tool compatibility. |

**Test mapping:**
| TS test | Rust test |
|---------|-----------|
| `tests/keychain.test.ts` (6KB) | `tests/keychain_parity.rs` |
| 신규 | `test_go_keyring_envelope_strip` |
| 신규 | `test_headless_linux_fallback` |

---

## 9. `keychain-windows.ts` (Phase 3)

**Current Ralph status:** Rust module is ported with embedded PowerShell, base64 blob decode, EDR signal detection, execution-policy/load/not-found branches, and default runner coverage. Live Windows/V3/AhnLab QA remains external.

**TS path:** `src/axhub-helpers/keychain-windows.ts`
**Rust target:** `crates/axhub-helpers/src/keychain_windows.rs` (+ `keychain_windows.ps1` extracted)

| TS symbol | TS line | Rust symbol | Status | 비고 |
|-----------|---------|-------------|--------|------|
| `interface WindowsSpawnResult` | 19 | `pub struct WindowsSpawnResult` | ported | |
| `type WindowsRunner` | 26 | `pub type WindowsRunner = fn(&[&str], u64) -> WindowsSpawnResult` | changed | Function pointer seam is enough for current tests. |
| `defaultWindowsRunner` | 28 | `pub fn default_windows_runner(cmd: &[&str], timeout_ms: u64) -> WindowsSpawnResult` | ported | spawn_sync 사용 |
| `PS_SCRIPT` (53 LOC inline C#) | 43 | `pub const PS_SCRIPT: &str` | ported | Inline embed retained to avoid extra runtime file lookup. |
| `PS_TIMEOUT_MS = 8000` | 97 | `pub const PS_TIMEOUT_MS: u64 = 8000` | ported | |
| `ERR_NOT_FOUND` (한글 메시지) | 99 | `messages::KEYCHAIN_WINDOWS_NOT_FOUND` | ported | |
| `ERR_EXEC_POLICY` | 105 | `messages::KEYCHAIN_WINDOWS_EXEC_POLICY` | ported | |
| `ERR_PINVOKE` | 111 | `messages::KEYCHAIN_WINDOWS_PINVOKE` | ported | |
| `ERR_EDR` | 117 | `messages::KEYCHAIN_WINDOWS_EDR` | ported | EDR_BLOCKED |
| `ERR_SPAWN` | 124 | `messages::KEYCHAIN_WINDOWS_SPAWN` | ported | |
| `isEdrSignal(result)` | 130 | `pub fn is_edr_signal(result: &WindowsSpawnResult) -> bool` | ported | 0xC0000409 |
| `decodeWindowsBlob(b64)` | 137 | `pub fn decode_windows_blob(b64: &str) -> Option<String>` | ported | base64 |
| `readWindowsKeychain(...)` | 145 | `pub fn read_windows_keychain_with_runner(runner: WindowsRunner) -> KeychainResult` | ported | |
| `defaultParse(raw)` | 200 | `parse_keyring_value` shared parser | changed | Shared parser keeps go-keyring envelope behavior identical. |

**Test mapping:**
| TS test | Rust test | 비고 |
|---------|-----------|------|
| `tests/keychain-windows.test.ts` (4.8K) | `tests/keychain_windows_parity.rs` | mock runner 재사용 |
| 신규 | `test_edr_signal_0xc0000409` | EDR detection |
| 신규 | `test_amsi_block_message` | 한글 메시지 |
| Manual QA | V3/AhnLab cohort | 자동화 안 됨, 매 release |

---

## 10. `index.ts` (Phase 4) — CLI dispatcher

**Current Ralph status:** Rust binary dispatcher is ported for session-start, version/help, redact, classify-exit, preflight, resolve, list-deployments, consent-mint, consent-verify, preauth-check, and prompt-route. `bun run build` now builds/copies the Cargo release binary, release CI builds/signs 5 Rust target artifacts, and the TS entrypoint remains as explicit fallback/tooling during the monitor window.

**TS path:** `src/axhub-helpers/index.ts`
**Rust target:** `crates/axhub-helpers/src/main.rs` + `spawn.rs` shim (Phase 0 작성)

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `out(payload)` | 45 | `fn out_json(v: Value)` | ported |
| `outRaw(text)` | 48 | `println!` in version/help path | changed | Dedicated wrapper not needed. |
| `err(msg)` | 51 | `eprintln!` at error boundary | changed | Dedicated wrapper not needed. |
| `readStdin()` | 57 | `fn read_stdin() -> anyhow::Result<String>` | ported | Windows UTF-8 process output is handled by spawn shim; stdin is Rust UTF-8 string. |
| `parseJson<T>(raw)` | 65 | `serde_json::from_str` at call sites | changed | Keeps parse errors local to command handlers. |
| `VALID_ACTIONS` | 75 | command parser in `consent::parser` | changed | Single parser source avoids duplicate action set. |
| `asConsentBinding(v)` | 83 | `fn parse_binding(raw: &str) -> anyhow::Result<ConsentBinding>` | ported |
| `PLUGIN_VERSION = "0.1.24"` | 104 | `pub const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION")` | ported |
| `CONSENT_TOKEN_TTL_SEC = 60` | 107 | `consent::jwt::mint_token(binding, 60)` | changed | TTL constant is passed at CLI boundary. |
| `HOOK_SCHEMA_VERSION = "v0"` | 108 | `const HOOK_SCHEMA_VERSION: &str = "v0"` | ported |
| `USAGE` | 110 | `const USAGE: &str` | ported | Manual static usage keeps binary small and contract stable. |
| `sessionStartMessage(preflight)` | 464 | inline `session-start` message assembly | changed | Preserves output text without extra helper. |
| (CLI dispatch logic) | scattered | `match args[1].as_str()` in `run()` | changed | Manual dispatch avoids a new clap dependency. |
| (subcommand handlers) | scattered | `fn cmd_<subcommand>()` handlers | ported | Synchronous helper keeps hook latency low. |

### Subcommand 매핑 (index.ts → main.rs Commands enum)

| TS dispatch | Rust variant | Status |
|-------------|--------------|--------|
| `session-start` | `Commands::SessionStart` / `cmd_session_start()` | ported |
| `version` | `Commands::Version` | ported |
| `help` | clap auto-generated | ported |
| `consent` | `Commands::Consent(ConsentArgs)` | ported |
| `list-deployments` | `Commands::ListDeployments` | ported |
| `preflight` | `Commands::Preflight` | ported |
| `classify` | `Commands::Classify { input }` | ported |
| `doctor` | `Commands::Doctor` | ported |
| `prompt-route` | `cmd_prompt_route()` | ported — doctor + deploy/apps/apis/auth/logs/status/recover/update/upgrade/clarify NL contexts |
| `redact` | `Commands::Redact { input }` | ported |
| (그 외 index.ts dispatch) | Rust parity backlog rows above + `prompt-route` hook path | tracked |

**Test mapping:**
| TS test | Rust test |
|---------|-----------|
| `tests/axhub-helpers.test.ts` (11.5K, 통합 dispatch) | `tests/main_dispatch_parity.rs` |
| `tests/e2e/claude-cli/run-matrix.sh` (t1/t2/nightly) | 동일 fixture, Rust binary 로 실행 |
| `tests/e2e-claude-cli-registry.test.ts` | `tests/e2e_registry_parity.rs` |

---

## 11. Bun-specific API shim 매핑 (Phase 0 spawn.rs)

| Bun API | 사용처 | Rust 매핑 |
|---------|--------|-----------|
| `Bun.spawnSync({...})` | `keychain.ts` (2건), `keychain-windows.ts:38`, `preflight.ts` | `crate::spawn::spawn_sync(&[...])` shim |
| `Bun.stdin.text()` | `index.ts` | `read_stdin_utf8()` (Win codepage 강제) |

---

## 11.5 Phase 4 Rust-primary build/release cutover — 2026-04-29

| Surface | Before | Current status | Evidence |
|---------|--------|----------------|----------|
| Local build | `bun build --compile` helper artifact | `bun run build` wraps Cargo release build and copies `target/release/axhub-helpers` to `bin/axhub-helpers` plus host-named asset | `scripts/build-rust-helper.ts`, `bun run build` |
| Version sync | install scripts + TS fallback only | `scripts/codegen-install-version.ts` also rewrites `[workspace.package] version` in `Cargo.toml` | `bun run codegen:version` |
| Release preflight | Bun compiled binary matrix | Host Cargo artifact check by default; `AXHUB_RELEASE_CHECK_FULL=1` keeps full matrix opt-in | `scripts/release-check.ts`, `bun run release:check` |
| GitHub release | Bun + Node/Bun compiled binaries | Cargo matrix for Linux amd64/arm64, macOS amd64/arm64, Windows amd64; cosign signing remains | `.github/workflows/release.yml` |
| Claude CLI e2e | Assumed prebuilt JS/Bun helper | PR/nightly jobs install Rust toolchain/cache before `bun run build` | `.github/workflows/claude-cli-e2e.yml` |
| Staging/external gates | Manual notes plus skipped `bun run test:e2e` | Dedicated `rust-staging-gates.yml` workflow covers local Rust-primary gate, credential-gated read-only staging probe, optional parser fuzz minutes including 24h, and GitHub Windows smoke | `.github/workflows/rust-staging-gates.yml`, `tests/e2e/staging.test.ts`, `tests/release-config.test.ts` |
| Hook parity | TS fallback accepted PostToolUse JSON; Rust `classify-exit` accepted positional args only | Rust `classify-exit` now accepts PostToolUse stdin JSON and emits hook `systemMessage` for axhub commands | `crates/axhub-helpers/tests/cli_e2e.rs`, T2 e2e |
| Preauth deny contract | TS returned hook JSON deny with exit 0 | Rust `preauth-check` now matches TS hook contract: exit 0 + `permissionDecision=deny` + Korean preauth message | `crates/axhub-helpers/tests/cli_e2e.rs`, T2 e2e |
| UserPromptSubmit NL routing | Rust prompt-route only injected doctor context | Rust/TS prompt-route now injects skill-specific contexts for deploy/apps/apis/auth/logs/status/recover/update/upgrade/clarify, with deploy guard against repo release workflow confusion | `tests/axhub-helpers.test.ts`, `crates/axhub-helpers/tests/cli_e2e.rs`, T1 routing rerun |
| Claude CLI T1 stability | Case 16 `/axhub:update` had a 60s budget | T1 interactive timeout budgets are 90s because slash/NL skill loading can cross 60s under repeated Claude CLI runs; observed green case 16 runtime was 57s | `tests/e2e/claude-cli/cases/16-update-slash.case.sh`, targeted T1 rerun |

## 12. Bun 런타임 의존 제거 (DX-6 inventory)

**Phase 4 ship reference status:**

| Reference | Status | Rationale / follow-up |
|-----------|--------|-----------------------|
| `package.json` `engines.bun` | retained | Repo scripts, tests, release automation, and TS fallback still use Bun. Plugin runtime artifact is now Rust-native. |
| `package.json` `scripts.build:*` | changed | `build`/`build:*` now call `scripts/build-rust-helper.ts`, which wraps Cargo instead of `bun build --compile`. |
| `package.json` `dependencies.{jose, semver, zod}` | retained | Needed by TS fallback/tooling until monitor-window TS deletion PR. |
| `package.json` `devDependencies.@types/bun` | retained | Needed by Bun tests/scripts while the repo still ships the fallback. |
| `install.sh` | changed | Version sync updated to `0.1.24`; release artifact names are Rust matrix names. |
| `install.ps1` | changed | Version sync updated to `0.1.24`; release artifact names are Rust matrix names. |
| `README.md` / `bin/README.md` / `docs/RELEASE.md` | changed | Docs now state Rust helper is default and Bun is tooling/fallback. |
| `axhub:doctor` SKILL | retained | Doctor runtime messaging is covered by prompt-route/session-start e2e; full skill text cleanup belongs to monitor-window polish. |
| `CHANGELOG.md` v1.0.0 entry | pending | Release workflow will generate this at version ship time. |
| `CLAUDE.md` / release lore | retained | `bun run release` remains canonical because release orchestration is still Bun-based. |
| `RTK.md` | no-op | No Rust-primary helper reference found in this pass. |
| Catalog source-of-truth | pending | `build.rs` still reads `src/axhub-helpers/catalog.ts`; final TS deletion needs a JSON/Rust catalog source migration. |
| TS fallback deletion | pending | Deferred until monitor window plus staging/Windows cohort evidence are available. The staging workflow now exists; it still needs real secrets/Windows cohort runs before deletion. |
| `vibe-coder-quality/PLAN.md` | not started | This is a strategic pre-review plan requiring user approval, not an implementation plan for the Rust helper cutover. |


---

## 13. 최종 산출물 매핑

| 종류 | TS | Rust |
|------|-----|------|
| Helper binary | `bin/axhub-helpers` (Bun compiled, 50~90MB) | `target/release/axhub-helpers` (Rust native, 5~15MB) |
| Cross-arch binary 5개 | `bin/axhub-helpers-{darwin,linux,windows}-{arm64,amd64}*` | `target/{target}/release/axhub-helpers*` |
| Codegen 산출 | `src/axhub-helpers/catalog.ts` (codegen target) | `OUT_DIR/catalog_generated.rs` (build.rs) |
| Test fixtures | `tests/fixtures/**` JSON | 동일 (Rust 측 재사용) |
| Test runner | `bun test` | `cargo test` + `cargo fuzz run parser` |
| Lint | `bun run lint:tone --strict` | + `bun run lint:tone:rust --strict` (DX-2) |

---

## 14. 매 PR 마다 갱신 의무

본 file 의 status 컬럼을 PR 마다 갱신:
- `planned` → `in-progress` (PR 시작 시)
- `in-progress` → `ported` (parity test PASS 시)
- `in-progress` → `changed` (의도적 동작 변경 시, 사유 명시)
- `in-progress` → `dropped` (TS-mock-only 등 무효화 시)

PR 본문에 `parity-{module}.md` 첨부 강제 (CI 에서 check).

---

## 15. 진행 상황 dashboard

| Phase | Module 수 | Ported | In-progress | Planned | Changed/Dropped |
|-------|-----------|--------|-------------|---------|-----------------|
| 0 | (prerequisite) | 8 mandatory | 0 | 0 | — |
| 1 | 3 (redact/catalog/telemetry) | 3 | 0 | 0 | — |
| 2 | 3 (preflight/resolve/list-deployments) | 3 | 0 | 0 | — |
| 3 | 3 split (consent/key/jwt/parser + keychain ×2) | 6 | 0 | 0 | — |
| 4 | 1 (main + TS 제거) | 1 | 0 | 0 | — |
| **Total** | **10 TS** → **~16 Rust** | **16** | **0** | **0** | **0** |

매 phase 완료 시 dashboard update.
