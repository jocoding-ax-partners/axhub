# Rust helper 전환 가이드

axhub-helpers 는 Rust helper 를 단일 사용자 binary 로 사용해요. Bun 은 repo 스크립트와 release 검증 runner 로 남아 있고, 사용자 release artifact 는 Rust native binary 예요.

## 자동 마이그레이션

```bash
axhub update
```

업데이트는 helper binary 만 교체해요. 토큰, profile, app 설정은 그대로 유지해요.

## Runtime 상태

```bash
bin/axhub-helpers version
bun run build
```

- 현재 release 는 Rust helper 만 사용자 경로로 제공해요.
- 예전 runtime 선택 환경변수는 현재 사용자 rollback 경로가 아니에요.
- 회귀가 보이면 서명된 이전 release helper 로 되돌려요.

## 호환성 약속

### 그대로 유지해요

- Token/env 계약: `AXHUB_TOKEN`, `AXHUB_ENDPOINT`, `AXHUB_ALLOW_PROXY`.
- Consent token: HS256, zero leeway, 60초 TTL, session/tool-call binding.
- keychain read path: macOS Keychain, Linux Secret Service, Windows Credential Manager guidance.
- Hub API fallback: bearer token 전송 전 TLS SPKI pin 확인.
- Hook JSON schema: `hookSpecificOutput` / `systemMessage` 구조 유지.
- 한국어 user-facing 메시지: 해요체 톤 유지.

### 바뀌어요

- helper release artifact 는 Bun-compiled binary 가 아니라 Rust native binary 예요.
- release workflow 는 Bun cross compile 대신 Rust target matrix 로 5개 binary 를 만들어요.
- `bun run build` 는 Cargo wrapper 로 동작해요. Bun 은 helper compile 이 아니라 repo script runner 로만 남아요.

## Rollback

회귀가 보이면 이전 서명 release 로 되돌려요.

```bash
axhub update --force-version 0.1.23
```

## Platform notes

- macOS: Keychain live read smoke 통과했어요.
- Linux: Docker 안에서 Secret Service live read smoke 통과했어요. headless 환경은 `AXHUB_TOKEN` fallback 을 유지해요.
- Windows: Credential Manager parser/runner branch 는 테스트돼요. V3/AhnLab cohort 는 실제 Windows/EDR 환경에서 매 release 수동 확인이 필요해요.

## 검증 baseline

- `cargo test --workspace`.
- `cargo llvm-cov --workspace --fail-under-lines 90`.
- `bun test` / `bun run test:plugin-e2e:t1` / `bun run test:plugin-e2e:t2`.
- `bun run release:check` 로 host Rust artifact 와 release matrix wiring 을 확인해요.

## sh/ps1 wrapper 흡수 (v0.8.x → v0.9.0)

### 사용자 영향 요약

sh + ps1 wrapper 페어 5쌍 (~1100 LOC) 의 OS-conditional 로직이 `axhub-helpers` Rust subcommand 로 응집됐어요. 사용자 가시 동작은 동일하지만 Windows parity gap 두 개가 해소돼서 Windows 사용자가 처음으로 deploy SKILL 의 token-freshness gate + auth-refresh-bg chain 을 정상적으로 사용할 수 있어요.

| 항목 | 이전 | v0.8.x → v0.9.0 |
|---|---|---|
| `hooks/token-freshness-gate.sh` | bash shim 가 polling + auth probe | `axhub-helpers token-gate` Rust subcommand 가 단일 구현. shim 자체는 v0.9.0 에서 삭제 (test 의존 정리 후). |
| `hooks/session-start.ps1` auth-refresh trigger | 누락 (Windows parity gap #2) | `Start-Process` detach 추가 — `AXHUB_AUTH_BG_REFRESH` 가 sh 와 동일하게 동작 |
| `hooks/session-start-autowire.{sh,ps1}` | 130 + 158 줄 dispatcher 본체 | thin wrapper (~40/55 줄). helper `--scope auto` 가 scope 감지 + disclosure marker + mtime guard + orphan-stub install + merge 통합 |
| `bin/statusline.ps1` 폴백 | P/Invoke `CredReadW` 직접 호출 | `keychain_windows.rs` 단일 출처. P/Invoke 블록 제거, token file + env 검사 보존 |
| `bin/install.{sh,ps1}` 후처리 | wrapper 가 `.gitignore` + post-commit + disclosure 직접 작성 | `axhub-helpers post-install` subcommand 위임. CLI contract: `--target-name --bin-dir --link-path [--repo-root]` |
| `_AXHUB_DISCLOSURE_VER` drift | `v0.5.13` 하드코딩 (release v0.8.0 까지 stale) | `scripts/codegen-install-version.ts` 가 release version 과 자동 sync |

### 호환성 보장

- env 컨트랙트 동일: `AXHUB_GATE_*`, `AXHUB_DISABLE_HOOKS`, `AXHUB_DISABLE_HOOK`, `AXHUB_DISABLE_STATUSLINE_AUTOWIRE`, `AXHUB_AUTH_BG_REFRESH`, `AXHUB_NO_DISCLOSURE`, `AXHUB_SKIP_AUTODOWNLOAD`, `AXHUB_POSTCOMMIT_INSTALL`.
- exit code 동일: token-gate UNAUTHORIZED → 65, kill switch → 0.
- shell wrapper 가 자체적으로 helper 부재 케이스 처리 (broken install 시 silent exit 0).
- `DISABLE_AXHUB=1` legacy alias 는 v0.8.x 까지 유지 (deprecation warning 출력). v0.9.0 에서 제거 예정 — 사전에 `AXHUB_DISABLE_HOOKS=1` 또는 `AXHUB_DISABLE_HOOK=<csv>` 로 마이그레이션 권장.

### 신규 subcommand

| subcommand | 용도 |
|---|---|
| `axhub-helpers token-gate` | deploy Step 3.5 polling consumer. SKILL 이 직접 호출. |
| `axhub-helpers post-install` | install.sh/ps1 후처리 (`.gitignore`, post-commit, disclosure marker) |
| `axhub-helpers autowire-statusline --scope auto` | SessionStart hook 가 scope 감지를 helper 에 위임 |

### Rollback

- v0.8.x 사용자: 자동 마이그레이션 (`axhub update`) 이외 추가 작업 없음.
- 만약 회귀 발견 시 이전 서명된 release 로 rollback (`axhub update --force-version 0.8.0`).
- shim `hooks/token-freshness-gate.sh` 가 v0.9.0 에서 제거되므로 v0.9.0 이상 버전을 직접 다운로드한 사용자는 SKILL 갱신 없이 자동 helper 호출 — 호환성 영향 없음.
