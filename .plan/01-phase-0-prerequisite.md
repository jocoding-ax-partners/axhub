# Phase 0 — Prerequisite (Phase 1 시작 전)

**기간:** 1주 (병렬 진행 시 3~4일)
**목적:** Phase 1 commit 차단 해제. dual voices 가 발견한 critical gap 6개 + source 정정 2개 완료.
**완료 기준:** 8 mandatory checkbox 모두 ✓

---

## 1. Mandatory (Phase 1 commit 차단)

### DX-1. `.tool-versions` 추가

**파일:** `/Users/wongil/Desktop/work/jocoding/axhub/.tool-versions`

```
rust 1.83.0
bun 1.1.0
```

**비용:** 5 LOC, 5분
**효과:** mise/asdf 사용자가 toolchain archaeology 안 함. TTHW 단축 (cold cache 6~8min → 3min).
**검증:** `mise install` 또는 `asdf install` 시 양쪽 다 자동 설치.

---

### DX-2. messages.rs 중앙화 + Rust tone lint

**목적:** 한글 해요체 (해요/예요/이에요/할래요) 보존. Rust `anyhow!()` / `format!()` 영어 leak 차단.

**작업:**

1. `crates/axhub-helpers/src/messages.rs` 생성 (빈 catalog):

```rust
pub mod errors {
    pub const TLS_PIN_MISMATCH: &str = "hub-api TLS pin 검증에 실패했어요.";
    pub const TLS_PIN_TIMEOUT: &str = "hub-api TLS pin 검증 시간이 초과됐어요.";
    pub const ENDPOINT_INVALID: &str = "잘못된 AXHUB_ENDPOINT 값이에요";
    pub const HTTPS_REQUIRED: &str = "hub-api.jocodingax.ai 는 HTTPS 로만 호출해야 해요.";
    pub const KEYCHAIN_EDR_BLOCKED: &str = "Windows 보안 솔루션이 axhub 토큰 조회를 차단했어요.";
    // ... 추가
}

#[macro_export]
macro_rules! msg {
    ($key:ident) => { $crate::messages::errors::$key };
    ($key:ident, $($arg:tt)*) => { format!("{}: {}", $crate::messages::errors::$key, format!($($arg)*)) };
}
```

2. `scripts/check-toss-tone-conformance.ts` 확장:

```typescript
// 기존 .ts/.md scan 에 .rs 추가
const RUST_PATTERNS = [
  /anyhow!\("([^"]+)"\)/g,
  /\.context\("([^"]+)"\)/g,
  /format!\("([^"]+)"/g,
  /eprintln!\("([^"]+)"/g,
];
// crates/**/*.rs 글롭 추가, 한글 포함 string 만 lint:tone 검사
```

3. `package.json` 에 script 추가:

```json
"lint:tone:rust": "bun scripts/check-toss-tone-conformance.ts --include 'crates/**/*.rs' --strict"
```

**비용:** ~150 LOC, 4시간
**효과:** Phase 4 ship 시 영어 에러 메시지 leak 차단. CI 에서 baseline lock.
**검증:** `bun run lint:tone:rust --strict` exit 0. 의도적 leak (예: `anyhow!("internal error")`) 추가 시 fail.

---

### DX-3. `AXHUB_HELPERS_RUNTIME` 문서화

**목적:** Phase 1~3 dual-runtime 기간 + Phase 4 직후 1주 monitor 기간 동안 즉시 fallback 가능.

**작업:**

1. README.md 에 section 추가:

```markdown
## Runtime 선택 (전환 기간)

axhub-helpers 는 v1.0 까지 Rust + TypeScript 양쪽 binary 를 지원해요.
환경변수로 선택할 수 있어요.

\`\`\`bash
export AXHUB_HELPERS_RUNTIME=auto   # default (자동 감지, 권장)
export AXHUB_HELPERS_RUNTIME=rust   # Rust binary 강제
export AXHUB_HELPERS_RUNTIME=ts     # TypeScript binary 강제 (회귀 시)
\`\`\`

- `auto`: Rust binary 가 PATH 에 있으면 사용, 아니면 TS fallback
- `rust`: Rust 만 — 없으면 즉시 fail
- `ts`: TS 만 — 회귀 발견 시 즉시 rollback 용
```

2. `axhub:doctor` SKILL update — 현재 runtime 출력 추가:

```bash
# axhub-helpers 내부에 doctor 모드
echo "Runtime: $AXHUB_HELPERS_RUNTIME (effective: rust)"
```

3. `index.ts` 와 `main.rs` 양쪽에 runtime 식별 로직:

```typescript
// TS index.ts
if (process.env.AXHUB_HELPERS_RUNTIME === "rust") {
  // delegate to bin/axhub-helpers-rs
  Bun.spawnSync({ cmd: ["bin/axhub-helpers-rs", ...args], inherit: true });
  process.exit();
}
```

**비용:** 30 LOC + SKILL edit, 2시간
**효과:** 사용자가 회귀 시 즉시 rollback. Contributor 가 A/B 테스트 가능.
**검증:** `AXHUB_HELPERS_RUNTIME=ts axhub --version` 와 `=rust` 양쪽 동작.

---

### DX-4. keyring crate ⇄ go-keyring envelope 호환 spike

**목적:** axhub-cli (Go binary) 가 `go-keyring-base64:<base64-of-JSON>` envelope 로 token write. Rust `keyring` crate 가 동일 attribute key 로 read 가능한지 검증. **Phase 3 차단 해제.**

**작업 (1일 spike):**

1. 임시 Rust crate `keyring-spike/` 생성:

```rust
use keyring::Entry;

fn main() -> anyhow::Result<()> {
    let entry = Entry::new("axhub", "default")?;
    let pwd = entry.get_password()?;
    println!("Read: {}", pwd);
    // 기대값: "go-keyring-base64:eyJ0b2tlbiI6Li4u..."
    Ok(())
}
```

2. 3 OS 모두 검증:
   - **macOS:** axhub-cli login 후 → spike 가 동일 token read 성공?
   - **Linux:** secret-tool + Secret Service 환경에서 spike 가 read 성공?
   - **Windows:** Credential Manager 에서 spike 가 read 성공?

3. 결과 문서화:

| OS | keyring read 가능? | attribute key 차이 | 결정 |
|----|---------------------|----------------------|------|
| macOS | ? | ? | 호환되면 keyring 채택, 아니면 security CLI subprocess 유지 |
| Linux | ? | ? | 동일 |
| Windows | ? | ? | 동일 |

4. 호환 안 되는 OS 는 현재 subprocess 방식 유지 (TS → Rust 로 동일 PowerShell/security CLI 호출).

**비용:** 1일 (3 OS 테스트 포함)
**효과:** Phase 3 keychain 모듈 설계 차단 해제. plan §3.3 의 "keyring crate 단일화" 가정 검증.
**검증:** 결과 문서 (`/.omc/research/keyring-interop-spike.md`) 작성. 호환 매트릭스 명시.

---

### 사전 정정 — JWE 사용 grep

**목적:** plan §1.2 의 jose → jsonwebtoken 매핑 검증.

**명령:**

```bash
grep -E "JWE|encryptJWT|EncryptJWT|jwtEncrypt|CompactEncrypt" src/axhub-helpers/*.ts
```

**예상 결과:** 0건 (이미 검증됨)
**조치 (실제 0건이면):** plan §1.2 의 "josekit 불필요" 확정. `jsonwebtoken` 만 사용.
**조치 (1건 이상 발견 시):** `josekit` 추가 + Phase 3 estimate 재계산.

**비용:** 5분
**검증:** grep output empty.

---

### 사전 정정 — JWT leeway 0 lock test (TS 측)

**목적:** Rust port 시 `leeway(30)` 같은 silent widen 차단. 현재 TS 가 leeway 0 으로 동작하는 것을 test 로 lock.

**작업:**

1. `tests/consent.test.ts` 에 새 test 추가:

```typescript
test("verify rejects token with exp = now - 1 (zero leeway)", async () => {
  const past = Math.floor(Date.now() / 1000) - 1;
  const token = await mintConsentWithExp(past);
  await expect(verifyLatest(token)).rejects.toThrow(/expired/i);
});

test("verify rejects token with exp = now (boundary)", async () => {
  const now = Math.floor(Date.now() / 1000);
  const token = await mintConsentWithExp(now);
  // jose 는 exp <= now 거부 (zero leeway)
  await expect(verifyLatest(token)).rejects.toThrow(/expired/i);
});
```

2. Phase 3 의 Rust port 시 동일 contract test 작성 (`#[test]`).
3. baseline 진행 후 plan 의 "30s tolerance" 표현 모두 "zero leeway (jose default)" 로 정정.

**비용:** 30분
**효과:** silent auth window 확대 차단.
**검증:** `bun test tests/consent.test.ts` 새 2 case PASS.

---

### Bun.* API survey 확정

**목적:** plan §6 의 "Bun-specific 0건 가정" 정정. 실제 5건 확인됨.

**현재 발견 (이미 grep 완료):**

| 파일 | 사용 | Rust 매핑 |
|------|------|-----------|
| `keychain-windows.ts:38` | `Bun.spawnSync({...})` | `std::process::Command::new(...).output()` |
| `keychain.ts:N` (2건) | `Bun.spawnSync({...})` | 동일 |
| `index.ts:N` | `Bun.stdin.text()` | `std::io::stdin().read_to_string(&mut buf)` |
| `preflight.ts:N` | `Bun.spawnSync({...})` | 동일 |

**조치:**
- 각 사용처에 대한 Rust 매핑 결정 (subprocess shim crate 작성 권장)
- exit code + signal + stdout/stderr 인코딩 정규화 shim:

```rust
// crates/axhub-helpers/src/spawn.rs
pub struct SpawnResult {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: String,  // UTF-8 강제
    pub stderr: String,
}

pub fn spawn_sync(cmd: &[&str]) -> anyhow::Result<SpawnResult> {
    let output = std::process::Command::new(cmd[0])
        .args(&cmd[1..])
        .output()?;
    Ok(SpawnResult {
        exit_code: output.status.code(),
        signal: extract_signal(&output.status),  // Unix-only
        stdout: String::from_utf8(output.stdout)?,  // strict, no lossy
        stderr: String::from_utf8(output.stderr)?,
    })
}
```

**비용:** 1일 (shim crate + Windows codepage 처리)
**검증:** TS test 의 spawn 결과와 Rust shim 결과 byte-equal.

---

## 2. Recommended (Phase 1 시작은 가능, Phase 4 ship 전 강제)

### DX-5. `docs/migrate-rust.md` skeleton

**목적:** 사용자가 v0.1.x → v1.0.0-rust 마이그레이션 시 무엇을 알아야 하는지 정의.

**Skeleton:**

```markdown
# axhub Rust 마이그레이션 가이드 (v1.0.0)

## 자동 마이그레이션
`axhub update` 한 번 실행. binary 만 교체. token / config 그대로.

## 호환성 약속
- ✓ Token 파일 위치 동일 (`~/.axhub/consent/*`)
- ✓ Token 파일 format 동일 (HMAC SHA-256, JWT HS256)
- ✓ go-keyring envelope 호환 (axhub-cli 와 양방향)
- ✓ env vars 동일 (AXHUB_TOKEN, AXHUB_ENDPOINT, AXHUB_ALLOW_PROXY)
- ✗ Bun runtime 의존 제거 — bun 설치 안 해도 됨
- ✗ `Bun.serve` 같은 디버그 hook 제거

## Rollback
\`\`\`bash
export AXHUB_HELPERS_RUNTIME=ts
axhub update --force-version 0.1.23
\`\`\`

## Known platform 차이
- macOS: 동일
- Linux Secret Service: keyring crate 호환 (DX-4 spike 결과 반영)
- Windows: MSVC binary, Authenticode signed, EDR cohort 검증 완료
```

**비용:** 200 LOC skeleton, 1일
**Phase:** Phase 1 작성 → Phase 4 fill

---

### DX-6. Bun 참조 inventory

**명령:**

```bash
grep -rn 'Bun\|bun ' docs/ skills/ install.* README.md CHANGELOG.md > /Users/wongil/.gstack/projects/jocoding-ax-partners-axhub/bun-references-inventory.txt
wc -l /Users/wongil/.gstack/projects/jocoding-ax-partners-axhub/bun-references-inventory.txt
```

**조치:** 각 라인을 Phase 4 checklist 에 추가. ship 시 모든 reference 정리 또는 명시적 retain 사유.

**비용:** 1시간
**검증:** Phase 4 ship 시 `bun-references-inventory.txt` 의 모든 라인 처리됨.

---

## 3. CI 추가 (Phase 1 PR 부터 적용)

```yaml
# .github/workflows/rust-ci.yml
name: rust-ci
on: [pull_request]
jobs:
  rust-build:
    strategy:
      matrix:
        target: [x86_64-unknown-linux-gnu, aarch64-apple-darwin, x86_64-pc-windows-msvc]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release --target ${{ matrix.target }}
      - run: cargo test --target ${{ matrix.target }}
  rust-tone-lint:
    runs-on: ubuntu-latest
    steps:
      - run: bun run lint:tone:rust --strict
  skill-frozen:
    runs-on: ubuntu-latest
    steps:
      - run: bun run lint:keywords --check
```

**비용:** 1일 (Windows runner 매트릭스 포함)
**효과:** Phase 1 부터 회귀 차단.

---

## 4. Exit Criteria

다음 모두 ✓ 시 Phase 1 시작 가능:

- [ ] DX-1: `.tool-versions` commit
- [ ] DX-2: messages.rs + Rust tone lint scanner + lint:tone:rust script
- [ ] DX-3: AXHUB_HELPERS_RUNTIME README + axhub:doctor SKILL + index.ts/main.rs runtime detection
- [ ] DX-4: keyring spike 결과 문서 (3 OS 매트릭스)
- [ ] 사전: JWE grep 결과 0건 확인
- [ ] 사전: JWT leeway 0 lock test 2건 추가 + PASS
- [ ] Bun.* API shim crate 설계 + spawn_sync 구현
- [ ] CI: rust-ci workflow 추가 + 첫 green run

**예상 소요:** 3~4일 (병렬) 또는 1주 (sequential)

---

## 5. Blocker 발견 시

- DX-4 spike 가 keyring crate 와 go-keyring 호환 안 함 → keychain.rs 는 subprocess 유지 (현재 TS 와 동일 방식). plan §3.3 의 "keyring 단일화 -50% 코드" 주장 무효.
- JWE 사용 발견 (예상 0건이지만) → `josekit` 추가 + Phase 3 estimate +1주.
- Windows MSVC native runner 비용이 높음 → Phase 4 estimate 재검토.

이러한 Blocker 는 즉시 사용자에게 보고하고 plan 수정 후 재시작.
