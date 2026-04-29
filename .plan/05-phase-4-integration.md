# Phase 4 — Entry + 통합 (Week 11~14)

**기간:** 3~4주 (현실 추정, plan 의 2~3주 보다 +1주, MSVC + EDR cohort 반영)
**목표:** main.rs CLI dispatcher 작성 → TS binary 제거 → v1.0.0-rust ship
**위험:** 높음 — 사용자 마이그레이션 단계
**선행 조건:** Phase 3 완료 + DX-5 (migrate-rust.md) skeleton + DX-6 (Bun 참조 inventory)

---

## 1. main.rs CLI dispatcher

**TS source:** `src/axhub-helpers/index.ts` (509 LOC)

**의존:**

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
anyhow = { workspace = true }
tokio = { workspace = true }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**구조:**

```rust
// crates/axhub-helpers/src/main.rs
use clap::{Parser, Subcommand};
use axhub_helpers::{consent, list_deployments, preflight, resolve, telemetry, redact};

#[derive(Parser)]
#[command(name = "axhub-helpers", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// CLI session start hook
    SessionStart,
    /// Get version (smoke test)
    Version,
    /// Help
    Help,
    /// Mint consent JWT
    Consent(ConsentArgs),
    /// List deployments
    ListDeployments,
    /// Preflight check
    Preflight,
    /// Classify NL command
    Classify { input: String },
    /// Doctor (env diagnostic)
    Doctor,
    // ... 나머지 ...
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    // AXHUB_HELPERS_RUNTIME 체크 (DX-3)
    log_runtime();
    
    let cli = Cli::parse();
    match cli.command {
        Commands::SessionStart => run_session_start().await,
        Commands::Version => { println!("{}", env!("CARGO_PKG_VERSION")); Ok(()) },
        Commands::Help => { print_help(); Ok(()) },
        Commands::Consent(args) => run_consent(args).await,
        Commands::ListDeployments => run_list_deployments().await,
        Commands::Preflight => run_preflight().await,
        Commands::Classify { input } => {
            match catalog::classify(&input) {
                Some(intent) => println!("{}", intent),
                None => std::process::exit(1),
            }
            Ok(())
        },
        Commands::Doctor => run_doctor().await,
    }
}

fn log_runtime() {
    let runtime = std::env::var("AXHUB_HELPERS_RUNTIME").unwrap_or_else(|_| "auto".into());
    tracing::debug!("axhub-helpers runtime: {} (effective: rust)", runtime);
}

async fn run_session_start() -> anyhow::Result<()> {
    // index.ts:N 의 session-start 동작 복제
    // - check $HOME .axhub directory
    // - emit telemetry envelope
    // - print 한글 시작 메시지 (해요체)
    Ok(())
}
```

**Stdin 처리 (Bun.stdin.text → std::io::stdin):**

```rust
fn read_stdin_utf8() -> anyhow::Result<String> {
    use std::io::Read;
    let mut buf = String::new();
    
    // Windows: codepage 강제 UTF-8
    #[cfg(windows)]
    {
        use windows::Win32::System::Console::SetConsoleCP;
        unsafe { SetConsoleCP(65001).ok(); }  // CP_UTF8
    }
    
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}
```

**Subprocess shim 사용 (Phase 0 spawn.rs):**

```rust
// 모든 child_process / Bun.spawnSync 사용처는 spawn_sync() 호출
let result = crate::spawn::spawn_sync(&["axhub-cli", "--version"])?;
```

**Exit criteria (main.rs):**
- [ ] clap derive 로 모든 subcommand 정의
- [ ] index.ts 의 모든 명령 동등 동작 (session-start, version, help, consent, list-deployments, preflight, classify, doctor 등)
- [ ] AXHUB_HELPERS_RUNTIME 로깅
- [ ] Windows codepage UTF-8 강제
- [ ] e2e matrix t1/t2/nightly 100% PASS (Rust binary 단독)

---

## 2. release.yml 수정

```yaml
# .github/workflows/release.yml
name: release
on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            runner: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            runner: ubuntu-latest
            cross: true
          - target: x86_64-apple-darwin
            runner: macos-latest
          - target: aarch64-apple-darwin
            runner: macos-latest
          - target: x86_64-pc-windows-msvc
            runner: windows-latest
    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - if: matrix.cross == true
        run: cargo install cross
      - name: build
        run: |
          if [ "${{ matrix.cross }}" = "true" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi
        shell: bash
      - name: strip
        if: matrix.runner != 'windows-latest'
        run: strip target/${{ matrix.target }}/release/axhub-helpers
      - name: rename
        run: |
          # macOS arm64 → axhub-helpers-darwin-arm64 등
          # 기존 naming convention 유지
      - uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/axhub-helpers*

  sign:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
      - name: cosign sign
        run: |
          for binary in binaries-*/*; do
            cosign sign-blob --yes "$binary" --output-signature "$binary.sig"
          done
      - name: gh release
        run: |
          gh release create ${{ github.ref_name }} \
            binaries-*/* \
            --generate-notes
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  windows-authenticode:
    needs: build
    runs-on: windows-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: binaries-x86_64-pc-windows-msvc
      - name: signtool
        run: |
          # sign-windows.yml.template 의 동일 워크플로
          signtool.exe sign /td sha256 /fd sha256 /a axhub-helpers-windows-amd64.exe
```

---

## 3. TS 제거 PR

**대상:**
- `src/axhub-helpers/*.ts` 모두 삭제
- `package.json` 에서:
  - `scripts.build:*` 제거 (Bun compile)
  - `dependencies.{jose,semver,zod}` 제거
  - `engines.bun` 제거 (또는 plugin runtime 만 유지)
- `tests/*.test.ts` 의 Rust 측 동등 test 가 있는 항목 삭제
- `bin/axhub-helpers*` (TS 산출물) 삭제

**유지 (scripts/ 는 Bun 으로 계속):**
- `scripts/codegen-*.ts`
- `scripts/skill-new.ts`
- `scripts/skill-doctor.ts`
- `scripts/check-toss-tone-conformance.ts` (Rust scanner 확장 후에도 Bun 으로 실행)

**검증:**

```bash
# 모든 Bun 참조가 inventory (DX-6) 에 있는지
diff <(grep -rn 'Bun\|bun ' --include='*.md' --include='*.sh' --include='*.ps1') \
     /Users/wongil/.gstack/projects/jocoding-ax-partners-axhub/bun-references-inventory.txt
```

---

## 4. Migration 가이드 fill (DX-5)

`docs/migrate-rust.md` 완성:

```markdown
# axhub Rust 마이그레이션 가이드 (v1.0.0)

## 자동 마이그레이션
\`\`\`bash
axhub update
\`\`\`
binary 만 교체. token / config 그대로.

## 호환성 약속

### 호환 (변경 없음)
- ✓ Token 파일: `~/.local/state/axhub/hmac-key`, mode 0600
- ✓ JWT format: HS256, exp 5분 TTL, leeway 0
- ✓ go-keyring envelope 호환 (axhub-cli 와 양방향)
- ✓ env vars: AXHUB_TOKEN, AXHUB_ENDPOINT, AXHUB_ALLOW_PROXY, AXHUB_HELPERS_RUNTIME
- ✓ Hub API endpoint + TLS SPKI pin
- ✓ 한글 메시지 해요체

### 변경 (호환 깨짐 — 새 동작)
- ✗ Bun runtime 의존 제거 — bun 설치 안 해도 됨
- ✗ Binary 사이즈 ~80% 감소 (cosign 다운로드 빨라짐)
- ✗ Cold start 향상 (~50ms → ~10ms)

### Rollback (회귀 시)
\`\`\`bash
export AXHUB_HELPERS_RUNTIME=ts
axhub update --force-version 0.1.23
\`\`\`
v0.1.23 binary 가 cosign 서명된 채로 GH release 에 보존됨.

## Known platform 차이
- macOS: 변경 없음
- Linux Secret Service: keyring crate 사용 (DX-4 spike 결과: 호환됨/안 됨 — Phase 0 결과 적용)
- Windows: MSVC binary, Authenticode 서명, V3/AhnLab cohort QA 통과
```

---

## 5. CHANGELOG entry

```markdown
## [1.0.0] - 2026-XX-XX

axhub-helpers 를 TypeScript (Bun) 에서 Rust 로 완전 포팅했어요. binary 크기가 ~80% 줄고 cold start 가 5배 빨라졌어요.

### Changed (Breaking)
- Bun runtime 의존성 제거. bun 설치 안 해도 axhub 사용 가능
- Binary 사이즈: ~50MB → ~10MB
- Cold start: ~50ms → ~10ms

### Added
- AXHUB_HELPERS_RUNTIME 환경변수 (ts|rust|auto, transition 기간)
- Rust messages.rs 중앙 한글 catalog
- cargo-fuzz parser 24h 무결함 검증

### Migration
- 자동: `axhub update` 한 번. token/config 그대로
- Rollback: `AXHUB_HELPERS_RUNTIME=ts axhub update --force-version 0.1.23`
- 자세한 가이드: `docs/migrate-rust.md`

### Test baseline
- 498 → 480 test (TS 측 mock 의존 18 case 가 Rust state machine 구현으로 무효화 — 의도적)
- cargo test 380 + cargo-fuzz parser 24h 무결함
- V3/AhnLab live cohort QA 통과

### Honest tradeoff
양 모델 (Codex + Claude subagent) 의 STOP 권장에도 사용자가 user sovereignty 행사하여 진행 결정. 결과적으로 binary 크기 + cold start 개선 달성, 다만 (1) consent 의 parser state machine 재구현으로 일부 backtracking-의존 fixture 의 동작 변경, (2) 8~12주 → 10~14주 실제 소요, (3) 한국 EDR (V3/AhnLab) cohort QA 가 자동화 안 되어 매 release 마다 manual 확인 필요.
```

---

## 6. Monitor 기간 (Phase 4 ship 직후 1주)

### Mandatory monitor

- [ ] GH issue 자동 알림 (Rust 관련 keyword: "consent", "keychain", "TLS pin", "EDR")
- [ ] axhub:doctor 출력에 runtime 명시 (사용자가 자기 환경 확인 가능)
- [ ] 사용자 감지된 회귀 시 즉시 hotfix release (`bun run release -- --release-as patch`)
- [ ] AXHUB_HELPERS_RUNTIME=ts fallback 안내

### Rollback trigger

다음 중 1건 발생 시 즉시 rollback PR:

- consent JWT verify 실패율 > 0.1%
- keychain read 실패율 > 5% (특정 OS)
- TLS pin 검증 실패 (사용자 한 명이라도)
- 한글 메시지 영어 leak 발견

---

## 7. Phase 4 Exit Criteria

- [ ] main.rs CLI dispatcher 모든 명령 동작
- [ ] e2e matrix t1/t2/nightly 100% PASS (Rust 단독)
- [ ] release.yml 5 binary build + cosign + Authenticode 통과
- [ ] TS 제거 PR merge
- [ ] `package.json` 정리 (deps + scripts)
- [ ] `docs/migrate-rust.md` 완성
- [ ] CHANGELOG v1.0.0 entry (한글 narrative + Test baseline + Honest tradeoff)
- [ ] DX-6 Bun 참조 inventory 100% 처리 (삭제 또는 명시적 retain)
- [ ] V3/AhnLab live cohort QA 통과
- [ ] 1주 monitor 기간 무회귀 → TS 코드 완전 삭제 PR (다음 minor)

---

## 8. 위험 + 완화

| 위험 | 영향 | 완화 |
|------|------|------|
| 사용자가 axhub update 후 token 잃음 | **치명** | go-keyring envelope 호환 검증 (Phase 0 spike) + token-file format hard contract test |
| Windows MSVC native runner 비용 | 중 | GH Actions windows-latest matrix 추가 (+1주 CI 설정) |
| Authenticode 서명 워크플로 깨짐 | 높음 | sign-windows.yml.template 그대로 유지, MSVC PE32+ 산물에 적용 |
| 한국 EDR cohort 회귀 | 매우 높음 | manual QA + AXHUB_HELPERS_RUNTIME=ts fallback |
| TS test 와 Rust test 의 미묘한 동작 차이 | 중 | parity 매핑 표 PR 마다 첨부, 의도적 변경은 명시 |

---

## 9. 완료 후 작업

Phase 4 ship 1개월 후:
- TS 코드 완전 삭제 PR (남은 `src/axhub-helpers/` + `tests/` Rust-equivalent 가진 항목)
- AXHUB_HELPERS_RUNTIME=ts 옵션 deprecation 경고 (다음 minor)
- 별도 plan: plugin runtime 의 Bun 의존성 제거 (12-month ideal)
