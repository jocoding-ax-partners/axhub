# Phase 1 — Foundation (Week 1~3)

**기간:** 2~3주
**목표:** 가장 단순한 3 모듈 (redact, catalog, telemetry) Rust 포팅. Cargo workspace + CI 구축.
**위험:** 낮음 — 의존성 0, 보안 surface 없음
**선행 조건:** Phase 0 (`.plan/01-phase-0-prerequisite.md`) 완료

---

## 1. 산출물

### 1.1 Cargo workspace 구조

```
axhub/
├── src/axhub-helpers/        # TS (Phase 4 까지 유지)
├── crates/
│   ├── axhub-helpers/        # main binary crate
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs       # placeholder (Phase 4 에 채움)
│   │   │   ├── lib.rs
│   │   │   ├── messages.rs   # Phase 0 에서 작성
│   │   │   ├── spawn.rs      # Phase 0 shim
│   │   │   ├── redact.rs     # ← 본 phase
│   │   │   ├── catalog.rs    # ← 본 phase
│   │   │   └── telemetry.rs  # ← 본 phase
│   │   └── tests/
│   │       └── integration_redact.rs
│   └── axhub-codegen/        # build.rs helpers
│       ├── Cargo.toml
│       └── src/lib.rs
├── Cargo.toml                # workspace
├── tests/                    # 기존 TS test (점진 삭제)
└── .tool-versions            # Phase 0
```

### 1.2 workspace `Cargo.toml`

```toml
[workspace]
members = ["crates/axhub-helpers", "crates/axhub-codegen"]
resolver = "2"

[workspace.package]
version = "0.1.23"
edition = "2021"
rust-version = "1.83"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1.10"
anyhow = "1"
thiserror = "1"
tokio = { version = "1", features = ["fs", "macros", "rt"] }
chrono = { version = "0.4", features = ["serde"] }
```

---

## 2. 모듈별 작업

### 2.1 `redact.rs` (48 LOC TS → ~80 LOC Rust)

**TS source:** `src/axhub-helpers/redact.ts`
**역할:** PII (이메일, token, 개인정보) 마스킹

**작업:**

1. `crates/axhub-helpers/src/redact.rs` 생성

```rust
use regex::Regex;
use std::sync::LazyLock;

static EMAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap()
});

static TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(eyJ[A-Za-z0-9_-]+\.){2}[A-Za-z0-9_-]+").unwrap()
});

pub fn redact(input: &str) -> String {
    let s = EMAIL_RE.replace_all(input, "<email>");
    let s = TOKEN_RE.replace_all(&s, "<jwt>");
    s.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_masked() {
        assert_eq!(redact("contact: user@example.com"), "contact: <email>");
    }

    #[test]
    fn jwt_masked() {
        let jwt = "eyJhbGciOi.eyJzdWIi.signature123";
        assert!(redact(jwt).contains("<jwt>"));
    }

    #[test]
    fn idempotent() {
        let once = redact("a@b.c");
        assert_eq!(redact(&once), once);
    }
}
```

2. TS test (`tests/redact.test.ts`) 의 모든 케이스를 Rust `#[test]` 로 이전
3. `cargo test --package axhub-helpers redact` PASS 확인
4. TS test 는 일단 유지 (Phase 4 에 일괄 삭제)

**Exit criteria:**
- [ ] `cargo test redact` 모든 case PASS
- [ ] TS `redact.test.ts` 와 동등 case 매핑 표 작성 (`crates/axhub-helpers/tests/redact_parity.md`)
- [ ] benchmarking: input 10KB redact 시간 < 1ms

---

### 2.2 `catalog.rs` (188 LOC TS → ~250 LOC Rust)

**TS source:** `src/axhub-helpers/catalog.ts`
**역할:** NL command classifier. corpus.jsonl 에서 분류 lookup table 생성 (codegen target)

**작업:**

1. `crates/axhub-codegen/src/lib.rs` 생성 — `build.rs` 가 호출하는 codegen helper:

```rust
// crates/axhub-codegen/src/lib.rs
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct CorpusEntry {
    input: String,
    intent: String,
}

pub fn generate_catalog(corpus_path: &Path, out_path: &Path) -> anyhow::Result<()> {
    let content = fs::read_to_string(corpus_path)?;
    let entries: Vec<CorpusEntry> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    
    let code = generate_lookup_code(&entries);
    fs::write(out_path, code)?;
    Ok(())
}

fn generate_lookup_code(entries: &[CorpusEntry]) -> String {
    // ... regex set 생성 ...
}
```

2. `crates/axhub-helpers/build.rs`:

```rust
fn main() {
    let corpus = Path::new("../../tests/corpus.jsonl");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out = Path::new(&out_dir).join("catalog_generated.rs");
    axhub_codegen::generate_catalog(corpus, &out).unwrap();
    println!("cargo:rerun-if-changed=../../tests/corpus.jsonl");
}
```

3. `crates/axhub-helpers/src/catalog.rs`:

```rust
include!(concat!(env!("OUT_DIR"), "/catalog_generated.rs"));

pub fn classify(input: &str) -> Option<&'static str> {
    // generated lookup
    LOOKUP_TABLE.iter()
        .find(|(re, _)| re.is_match(input))
        .map(|(_, intent)| *intent)
}
```

4. corpus.100.jsonl 로 분류 결과가 TS 와 동일한지 검증:

```rust
#[test]
fn classify_parity_100() {
    let corpus = include_str!("../../../tests/corpus.100.jsonl");
    let mut total = 0;
    let mut match_count = 0;
    for line in corpus.lines().filter(|l| !l.trim().is_empty()) {
        let entry: CorpusEntry = serde_json::from_str(line).unwrap();
        total += 1;
        let actual = classify(&entry.input);
        if actual == Some(entry.intent.as_str()) {
            match_count += 1;
        }
    }
    assert_eq!(match_count, total, "classify parity failed: {}/{}", match_count, total);
}
```

**Exit criteria:**
- [ ] `cargo test classify_parity_100` PASS (100/100 match)
- [ ] `corpus.jsonl` (전체) parity test PASS
- [ ] codegen 결과 `cargo build` 시 자동 재생성 (rerun-if-changed)
- [ ] TS `codegen:catalog` script 와 dual-emit (양쪽 동일 결과)

---

### 2.3 `telemetry.rs` (87 LOC TS → ~150 LOC Rust)

**TS source:** `src/axhub-helpers/telemetry.ts`
**역할:** `~/.axhub/telemetry/*.jsonl` 에 envelope append

**작업:**

```rust
// crates/axhub-helpers/src/telemetry.rs
use chrono::Utc;
use serde::Serialize;
use std::path::PathBuf;
use tokio::fs::{create_dir_all, OpenOptions};
use tokio::io::AsyncWriteExt;

#[derive(Serialize)]
pub struct MetaEnvelope {
    pub ts: String,
    pub event: String,
    pub repo: Option<String>,
    pub session: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

pub async fn emit_meta_envelope(env: MetaEnvelope) -> anyhow::Result<()> {
    let dir = telemetry_dir()?;
    create_dir_all(&dir).await?;
    let path = dir.join(format!("{}.jsonl", Utc::now().format("%Y-%m-%d")));
    let mut f = OpenOptions::new().create(true).append(true).open(&path).await?;
    let line = serde_json::to_string(&env)?;
    f.write_all(line.as_bytes()).await?;
    f.write_all(b"\n").await?;
    Ok(())
}

fn telemetry_dir() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    Ok(home.join(".axhub").join("telemetry"))
}
```

**Tests:**

```rust
#[tokio::test]
async fn append_envelope() {
    let tmp = tempdir::TempDir::new("axhub-tel").unwrap();
    std::env::set_var("HOME", tmp.path());
    
    let env = MetaEnvelope {
        ts: "2026-04-29T00:00:00Z".into(),
        event: "test".into(),
        repo: Some("axhub".into()),
        session: None,
        extra: Default::default(),
    };
    emit_meta_envelope(env).await.unwrap();
    
    let path = tmp.path().join(".axhub/telemetry/2026-04-29.jsonl");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("\"event\":\"test\""));
}
```

**Exit criteria:**
- [ ] `cargo test telemetry` PASS
- [ ] TS test 의 envelope schema 와 byte-equal (직접 비교 test 작성)
- [ ] 동시 write race condition 검증 (10 concurrent emit)

---

## 3. CI 통합

`.github/workflows/rust-ci.yml` 에 phase 1 specific:

```yaml
- name: foundation modules test
  run: |
    cargo test --package axhub-helpers redact
    cargo test --package axhub-helpers catalog
    cargo test --package axhub-helpers telemetry
- name: corpus parity (100)
  run: cargo test classify_parity_100
- name: corpus parity (full)
  run: cargo test classify_parity_full
  continue-on-error: false
```

---

## 4. Phase 1 Exit Criteria

- [ ] Cargo workspace + 첫 PR merge
- [ ] redact / catalog / telemetry Rust 모듈 작성
- [ ] 각 모듈 cargo test PASS (TS test parity 매핑 표 첨부)
- [ ] codegen:catalog dual-emit (TS + Rust 동일 결과)
- [ ] CI rust-ci workflow green (Linux + macOS + Win MSVC)
- [ ] release.yml 에 Rust build 단계 추가 (산출물 unused, 검증만)
- [ ] release-check.rs 작성 (release-check.ts 와 동등 동작)
- [ ] 한글 메시지 catalog (messages.rs) 0개 변경 (foundation 모듈은 user-facing 메시지 없음)

---

## 5. 위험 + 완화

| 위험 | 완화 |
|------|------|
| Cargo workspace cache 가 cross-compile 시 문제 | `cargo-chef` + sccache 도입 |
| corpus.jsonl 파싱 차이 (TS vs Rust JSON) | parity test 100% match 강제 |
| build.rs 가 매 PR 마다 재실행되어 CI 느림 | rerun-if-changed 정확히 명시 |
| TS test parity 매핑 누락 | PR 템플릿에 매핑 표 첨부 강제 |

---

## 6. 다음 Phase

Phase 1 완료 시:
- 첫 Rust binary 가 production 에 들어가지는 않음 (검증 단계)
- 다음: `03-phase-2-stateless.md` (preflight, resolve, list-deployments)
- list-deployments 가 TLS pin 포함 — Phase 2 가 첫 보안 critical 모듈
