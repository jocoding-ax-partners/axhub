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

**Scope implemented in this session:** Phase 0 DX guardrails, Cargo workspace, Rust helper binary scaffold, and Rust ports for the 10 TypeScript helper modules at contract-test depth. Source files now exist under `crates/axhub-helpers/src/**`; codegen helper exists under `crates/axhub-codegen`.

**Verification evidence captured:**

- `cargo test --workspace` → Rust unit + regression + CLI e2e tests pass (codegen 5, helper unit 3, CLI e2e 3, phase parity 16).
- `cargo llvm-cov --workspace --fail-under-lines 90` → line coverage **90.23%** (threshold 90%) with deterministic coverage seam for live TLS socket I/O.
- `bun test tests/consent.test.ts` → TypeScript zero-leeway lock tests pass.
- `bun test tests/runtime-fallback.test.ts` → `AXHUB_HELPERS_RUNTIME=rust` delegation preserves stdin and exit code.
- `bun test tests/lint-toss-tone.test.ts` + `bun run lint:tone:rust` → Rust tone include path works and reports 0 errors.

**Open verification gaps (not marked fully complete):**

- Phase 2 `list-deployments`: TLS SPKI implementation, deterministic error/proxy coverage, and a live hub TLS pin probe are present. The planned 19-case TLS mock suite is still pending.
- Phase 3 `consent`: key/JWT/parser tests, CLI preauth e2e, and a 60-second `cargo +nightly fuzz run parser` smoke are present. The full 24h cargo-fuzz run is still pending.
- Phase 3 `keychain`: mac/linux/windows parser and runner branches are tested, but 3 OS live keychain interop and V3/AhnLab cohort QA are pending.
- Phase 4 `main.rs`: Rust CLI supports core helper commands and runtime fallback, but TypeScript removal/release pipeline cutover is pending.

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
| Staging e2e gate | `bun run test:e2e` | **BLOCKED/SKIPPED by missing credentials** — `AXHUB_E2E_STAGING_TOKEN` and endpoint are unset; runner reports `1 pass / 5 skip / 0 fail`. |
| Post-fix regression gate | `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `cargo llvm-cov --workspace --fail-under-lines 90`, `cargo audit --deny warnings`, `bun test`, `bun run test:e2e`, `bunx tsc --noEmit`, `bun run lint:tone --strict`, `bun run lint:tone:rust`, `bun run lint:keywords --check`, `git diff --check` | **PASS** — Rust tests now include 28 tests total (`cargo test --workspace`), Rust line coverage remains **90.23%**, Bun suite remains `557 pass / 5 skip / 0 fail`. |
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

**Σ TS:** 2,536 LOC → **Rust 추정:** 3,200~3,500 LOC

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

**Current Ralph status:** Rust module is ported with deterministic fetch/TLS checker seams, HTTP/error matrix tests, CLI e2e coverage, and `cargo llvm-cov` coverage support. Live hub TLS pin QA remains an external verification gap.

**TS path:** `src/axhub-helpers/list-deployments.ts`
**Rust target:** `crates/axhub-helpers/src/list_deployments.rs`

| TS symbol | TS line | Rust symbol | Status | 비고 |
|-----------|---------|-------------|--------|------|
| `DEFAULT_ENDPOINT` | 29 | `pub const DEFAULT_ENDPOINT: &str` | planned | |
| `HUB_API_HOST` | 30 | `pub const HUB_API_HOST: &str` | planned | |
| `DEFAULT_LIMIT` | 31 | `pub const DEFAULT_LIMIT: usize = 5` | planned | |
| `TLS_PIN_TIMEOUT_MS` | 32 | `pub const TLS_PIN_TIMEOUT_MS: u64 = 5000` | planned | |
| `HUB_API_SPKI_SHA256_PINS` | 34 | `pub const HUB_API_SPKI_SHA256_PINS: &[&str]` | planned | **byte-equal lock** |
| `EXIT_LIST_OK / AUTH / NOT_FOUND / TRANSPORT` | 38-41 | `pub const EXIT_LIST_OK: i32 = 0; ...` | planned | |
| `interface DeploymentSummary` | 43 | `pub struct DeploymentSummary` (serde) | planned | |
| `interface ListDeploymentsArgs` | 53 | `pub struct ListDeploymentsArgs` | planned | |
| `interface ListDeploymentsResult` | 58 | `pub struct ListDeploymentsResult` | planned | |
| `class TlsPinError` | 66 | `#[derive(thiserror::Error)] pub enum TlsPinError` | planned | |
| `type TlsPinChecker` | 73 | `pub type TlsPinChecker = ...` | planned | |
| `STATUS_MAP` (Record) | 75 | `static STATUS_MAP: phf::Map<i32, &'static str>` | planned | |
| `tokenFromEnv()` | 84 | `fn token_from_env() -> Option<String>` | planned | |
| `tokenFromFile()` | 89 | `fn token_from_file() -> Option<String>` | planned | XDG_CONFIG_HOME 처리 |
| `resolveToken()` | 102 | `pub fn resolve_token() -> Option<String>` | planned | export |
| `resolveEndpoint()` | 104 | `fn resolve_endpoint() -> String` | planned | |
| `proxyOverrideEnabled()` | 109 | `fn proxy_override_enabled() -> bool` | planned | `AXHUB_ALLOW_PROXY=1` |
| `pinnedHubApiUrl(endpoint)` | 111 | `fn pinned_hub_api_url(endpoint: &str) -> Result<Option<Url>, TlsPinError>` | planned | |
| `spkiHashFromCert(rawCert)` | 126 | `fn spki_hash_from_cert(raw: &[u8]) -> Result<String, TlsPinError>` | planned | x509-parser + sha2 |
| `verifyHubApiTlsPin(endpoint)` | 140 | `pub async fn verify_hub_api_tls_pin(endpoint: &str) -> Result<(), TlsPinError>` | planned | rustls |
| `parseAppId(raw)` | 197 | `fn parse_app_id(raw: &str) -> Option<u64>` | planned | |
| `buildAuthError()` | 226 | `fn build_auth_error() -> ListDeploymentsResult` | planned | |
| `runListDeployments(...)` | (export) | `pub async fn run_list_deployments(...) -> ListDeploymentsResult` | planned | export |

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
| `HMAC_KEY_BYTES = 32` | 62 | `const HMAC_KEY_BYTES: usize = 32` | planned |
| `FILE_MODE_PRIVATE = 0o600` | 63 | `const FILE_MODE_PRIVATE: u32 = 0o600` | planned |
| `DIR_MODE_PRIVATE = 0o700` | 64 | `const DIR_MODE_PRIVATE: u32 = 0o700` | planned |
| `stateRoot()` | 71 | `fn state_root() -> PathBuf` | planned |
| `runtimeRoot()` | 77 | `fn runtime_root() -> PathBuf` | planned |
| `hmacKeyPath()` | 83 | `fn hmac_key_path() -> PathBuf` | planned |
| `sessionId()` | 85 | `fn session_id() -> String` | planned |
| `tokenFilePath(sid)` | 91 | `fn token_file_path(sid: &str) -> PathBuf` | planned |
| (HMAC key load/mint logic) | scattered | `pub fn load_or_mint_key() -> anyhow::Result<[u8; 32]>` | planned |
| (write 0600 + O_NOFOLLOW) | 151-167 | `fn write_with_o_nofollow_0600()` | planned | nix(Unix) + ACL(Win) |
| (lstat read defense) | scattered | `fn read_with_lstat_check()` | planned | symlink 거부 |

### 7.2 `consent/jwt.rs` — JWT mint + verify (HS256)

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `JWT_ALG = "HS256"` | 65 | `const JWT_ALG: Algorithm = Algorithm::HS256` | planned |
| `interface ConsentBinding` | 30 | `pub struct ConsentBinding { session_id, command, args_hash, minted_at }` | planned |
| `interface MintResult` | 42 | `pub struct MintResult { token, expires_at, jti }` | planned |
| `interface VerifyResult` | 48 | `pub struct VerifyResult { binding, jti }` | planned |
| (mint logic with SignJWT) | scattered | `pub fn mint(binding: ConsentBinding, key: &[u8; 32]) -> Result<MintResult>` | planned |
| (verify logic with jwtVerify) | scattered | `pub fn verify(token: &str, key: &[u8; 32], expected: &ConsentBinding) -> Result<VerifyResult>` | planned | **leeway 0 lock** |

### 7.3 `consent/parser.rs` — parseAxhubCommand state machine

| TS symbol | TS line | Rust symbol | Status | 비고 |
|-----------|---------|-------------|--------|------|
| `interface ParsedAxhubCommand` | 53 | `pub struct ParsedAxhubCommand { ... }` | planned | |
| `FLAG_MAP` | 246 | `static FLAG_MAP: phf::Map<&str, &str>` | planned | |
| `extractFlags(tokens)` | 253 | `fn extract_flags(tokens: &[&str]) -> HashMap<...>` | planned | |
| `ENV_ASSIGN_PREFIX_RE` | 282 | `static ENV_ASSIGN_PREFIX_RE: LazyLock<Regex>` | planned | linear time (no backtrack) |
| `COLLECT_MAX_DEPTH = 5` | 295 | `const COLLECT_MAX_DEPTH: usize = 5` | planned | |
| `collectCommandPositions(cmd, depth)` | 297 | `fn collect_command_positions(cmd: &str, depth: usize) -> Vec<String>` | **changed** | regex backtracking → state machine 의도적 재구현 |
| `tokensIfAxhubCommand(rawPosition)` | 374 | `fn tokens_if_axhub_command(raw: &str) -> Option<Vec<String>>` | planned | |
| `matchKnownIntent(tokens)` | 420 | `fn match_known_intent(tokens: &[&str]) -> Option<ParsedAxhubCommand>` | planned | |
| `parseAxhubCommand(cmd)` | 444 | `pub fn parse_axhub_command(cmd: &str) -> ParsedAxhubCommand` | **changed** | export, state machine |

### 7.4 `consent/mod.rs` — flow orchestrator

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `mintConsent(binding)` (export) | scattered | `pub fn mint_consent(binding: ConsentBinding) -> Result<String>` | planned |
| `verifyLatest(expected)` (export) | scattered | `pub fn verify_latest(expected: &ConsentBinding) -> Result<()>` | planned |

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

**Current Ralph status:** Rust module is ported with go-keyring envelope parsing, platform guidance branches, and command-runner success/failure coverage. Live OS keychain QA remains pending.

**TS path:** `src/axhub-helpers/keychain.ts`
**Rust target:** `crates/axhub-helpers/src/keychain.rs` (cfg-gated per OS)

**핵심 결정 (DX-4 spike 결과 따라 분기):**
- spike PASS → `keyring` crate 채택
- spike FAIL → subprocess 유지 (security CLI / secret-tool)

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `parseKeyringValue(raw)` | 18 | `pub fn parse_keyring_value(raw: &str) -> Option<String>` | planned (go-keyring envelope strip) |
| `interface KeychainResult` | 40 | `pub struct KeychainResult { token, source }` | planned |
| `readKeychainToken()` | 46 | `pub fn read_keychain_token() -> KeychainResult` | planned (Phase 0 spike 결과 따라 keyring crate 또는 spawn_sync) |

**Test mapping:**
| TS test | Rust test |
|---------|-----------|
| `tests/keychain.test.ts` (6KB) | `tests/keychain_parity.rs` |
| 신규 | `test_go_keyring_envelope_strip` |
| 신규 | `test_headless_linux_fallback` |

---

## 9. `keychain-windows.ts` (Phase 3)

**Current Ralph status:** Rust module is ported with embedded PowerShell, base64 blob decode, EDR signal detection, execution-policy/load/not-found branches, and default runner coverage. Live Windows/V3/AhnLab QA remains pending.

**TS path:** `src/axhub-helpers/keychain-windows.ts`
**Rust target:** `crates/axhub-helpers/src/keychain_windows.rs` (+ `keychain_windows.ps1` extracted)

| TS symbol | TS line | Rust symbol | Status | 비고 |
|-----------|---------|-------------|--------|------|
| `interface WindowsSpawnResult` | 19 | `pub struct WindowsSpawnResult` | planned | |
| `type WindowsRunner` | 26 | `pub type WindowsRunner = Box<dyn Fn(...) -> WindowsSpawnResult>` | planned | |
| `defaultWindowsRunner` | 28 | `pub fn default_windows_runner(cmd: &[&str], timeout_ms: u64) -> WindowsSpawnResult` | planned | spawn_sync 사용 |
| `PS_SCRIPT` (53 LOC inline C#) | 43 | **moved** to `crates/axhub-helpers/src/keychain_windows.ps1` (file include) | planned | `include_str!()` 로 컴파일 시 embed |
| `PS_TIMEOUT_MS = 8000` | 97 | `const PS_TIMEOUT_MS: u64 = 8000` | planned | |
| `ERR_NOT_FOUND` (한글 메시지) | 99 | `const ERR_NOT_FOUND: &str` (messages.rs catalog) | planned | |
| `ERR_EXEC_POLICY` | 105 | 동일 (catalog) | planned | |
| `ERR_PINVOKE` | 111 | 동일 (catalog) | planned | |
| `ERR_EDR` | 117 | 동일 (catalog) | planned | EDR_BLOCKED |
| `ERR_SPAWN` | 124 | 동일 (catalog) | planned | |
| `isEdrSignal(result)` | 130 | `fn is_edr_signal(result: &WindowsSpawnResult) -> bool` | planned | 0xC0000409 |
| `decodeWindowsBlob(b64)` | 137 | `fn decode_windows_blob(b64: &str) -> Option<String>` | planned | base64 |
| `readWindowsKeychain(...)` | 145 | `pub fn read_windows_keychain(runner: WindowsRunner, parser: Option<...>) -> KeychainResult` | planned | |
| `defaultParse(raw)` | 200 | `fn default_parse(raw: &str) -> Option<String>` | planned | |

**Test mapping:**
| TS test | Rust test | 비고 |
|---------|-----------|------|
| `tests/keychain-windows.test.ts` (4.8K) | `tests/keychain_windows_parity.rs` | mock runner 재사용 |
| 신규 | `test_edr_signal_0xc0000409` | EDR detection |
| 신규 | `test_amsi_block_message` | 한글 메시지 |
| Manual QA | V3/AhnLab cohort | 자동화 안 됨, 매 release |

---

## 10. `index.ts` (Phase 4) — CLI dispatcher

**Current Ralph status:** Rust binary dispatcher is ported for session-start, version/help, redact, classify-exit, preflight, resolve, list-deployments, consent-mint, consent-verify, and preauth-check. The TS entrypoint now supports `AXHUB_HELPERS_RUNTIME=rust` fallback; full TS removal remains pending.

**TS path:** `src/axhub-helpers/index.ts`
**Rust target:** `crates/axhub-helpers/src/main.rs` + `spawn.rs` shim (Phase 0 작성)

| TS symbol | TS line | Rust symbol | Status |
|-----------|---------|-------------|--------|
| `out(payload)` | 45 | `fn out<T: Serialize>(payload: &T)` | planned |
| `outRaw(text)` | 48 | `fn out_raw(text: &str)` | planned |
| `err(msg)` | 51 | `fn err(msg: &str)` | planned |
| `readStdin()` | 57 | `fn read_stdin_utf8() -> anyhow::Result<String>` | planned (Windows codepage UTF-8 강제) |
| `parseJson<T>(raw)` | 65 | `fn parse_json<T: DeserializeOwned>(raw: &str) -> Option<T>` | planned |
| `VALID_ACTIONS` | 75 | `static VALID_ACTIONS: LazyLock<HashSet<&'static str>>` | planned |
| `asConsentBinding(v)` | 83 | `fn as_consent_binding(v: &Value) -> Option<ConsentBinding>` | planned |
| `PLUGIN_VERSION = "0.1.23"` | 104 | `pub const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION")` | planned |
| `CONSENT_TOKEN_TTL_SEC = 60` | 107 | `pub const CONSENT_TOKEN_TTL_SEC: i64 = 60` | planned |
| `HOOK_SCHEMA_VERSION = "v0"` | 108 | `pub const HOOK_SCHEMA_VERSION: &str = "v0"` | planned |
| `USAGE` | 110 | `const USAGE: &str` (clap 자동 생성 + 한글 텍스트 보강) | planned |
| `sessionStartMessage(preflight)` | 464 | `fn session_start_message(preflight: &PreflightOutput) -> String` | planned |
| (CLI dispatch logic) | scattered | `clap::Parser` derive on `Cli`, `Commands` enum | planned |
| (subcommand handlers) | scattered | `async fn run_<subcommand>()` per command | planned |

### Subcommand 매핑 (index.ts → main.rs Commands enum)

| TS dispatch | Rust variant | Status |
|-------------|--------------|--------|
| `session-start` | `Commands::SessionStart` | planned |
| `version` | `Commands::Version` | planned |
| `help` | clap auto-generated | planned |
| `consent` | `Commands::Consent(ConsentArgs)` | planned |
| `list-deployments` | `Commands::ListDeployments` | planned |
| `preflight` | `Commands::Preflight` | planned |
| `classify` | `Commands::Classify { input }` | planned |
| `doctor` | `Commands::Doctor` | planned |
| `redact` | `Commands::Redact { input }` | planned |
| (그 외 index.ts dispatch) | (확인 후 추가) | **needs grep** |

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

## 12. Bun 런타임 의존 제거 (DX-6 inventory)

**Phase 4 ship 시 처리할 reference:**
- [ ] `package.json` `engines.bun` 제거 (또는 plugin runtime 만 유지)
- [ ] `package.json` `scripts.build:*` 제거 (bun build --compile)
- [ ] `package.json` `dependencies.{jose, semver, zod}` 제거
- [ ] `package.json` `devDependencies.@types/bun` 제거 (helper 측), `commit-and-tag-version` 유지
- [ ] `install.sh` (binary 다운로드 path 변경)
- [ ] `install.ps1` (Windows binary path)
- [ ] `README.md` (Bun 설치 안내 제거)
- [ ] `axhub:doctor` SKILL (Bun version 체크 → rustc 체크 또는 binary type 체크)
- [ ] `CHANGELOG.md` v1.0.0 entry (마이그레이션 안내)
- [ ] `CLAUDE.md` (Phase 19 Release Workflow 의 bun run release 부분 — 유지)
- [ ] `RTK.md` (영향 없음)

(Phase 0 DX-6 grep 결과를 여기에 채워넣음)

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
