# Implementation Plan: 플러그인 로그인 consent deny 수정 (TMPDIR 핸드오프)

**Branch**: `003-fix-plugin-login-deny` | **Date**: 2026-06-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/003-fix-plugin-login-deny/spec.md`

## Summary

플러그인 로그인이 macOS Claude Code 에서 100% deny 되는 버그를 고쳐요. 근본 원인은 `consent/key.rs::runtime_root()` 가 `XDG_RUNTIME_DIR` 부재 시 `std::env::temp_dir()`(=`$TMPDIR`)로 폴백하는데, Claude Code 가 Bash tool 프로세스와 hook subprocess 에 **서로 다른 `$TMPDIR`** 를 부여해서 mint 가 쓴 consent 파일을 hook 이 못 찾는 거예요.

**기술 접근**: `runtime_root()` 의 폴백을 `$TMPDIR` 에서 **HOME-anchored 안정 경로**(`state_root().join("runtime")` = `~/.local/state/axhub/runtime`)로 바꿔요. 이 경로는 이미 HMAC 키가 쓰는 `state_root()` 와 같은 뿌리라, **두 프로세스가 동일하게 해석된다는 게 실증돼 있어요**(키 검증이 양쪽에서 성공 중). XDG_RUNTIME_DIR 가 있는 경로(Linux)는 손대지 않아 회귀가 없어요. 곁들여 mint/preauth 시 만료된 `consent-*.json` 을 opportunistic 스윕하고(FR-007), deny 출력에 `permissionDecisionReason` 을 더해 사유가 표면화되게 해요(P2, additive).

## Technical Context

**Language/Version**: Rust (2024 edition workspace), 도구/테스트 하니스는 bun(TypeScript)

**Primary Dependencies**: `jsonwebtoken`(HS256), `chrono`, `uuid`, `serde_json`, `anyhow`, `clap`, `libc`(O_NOFOLLOW). **새 런타임 의존성 추가 없음.**

**Storage**: 파일시스템 — consent 토큰은 `0600` JSON, runtime 디렉터리(`0700`) 아래. HMAC 키는 `state_root()`(`~/.local/state/axhub`) 아래. 본 작업은 consent 토큰의 **저장 위치만** 바꿔요.

**Testing**: `cargo test` / `cargo nextest`(Rust 단위·`tests/cli_e2e.rs` 통합), `bun test`(TS 하니스), `bunx tsc --noEmit`

**Target Platform**: macOS(주 재현 환경), Linux, Windows — Claude Code hook 이 호출하는 크로스플랫폼 CLI

**Project Type**: CLI / 플러그인 헬퍼 바이너리(`axhub-helpers`)

**Performance Goals**: hook 지연 무시 가능 — 스윕은 작은 `read_dir` 1회(pending 파일 소수)

**Constraints**: hook fail-open(어떤 실패에서도 exit 0), `0600`/`0700` 권한 보존, consent TTL 60초·pending single-use·HMAC 계약 유지, `XDG_RUNTIME_DIR` 경로 무회귀

**Scale/Scope**: 로그인 1회당 consent 파일 1개, 동시 pending 소수. 코드 변경 ~5줄 핵심 + 만료 consent 스윕 헬퍼 + deny 필드 + 회귀 테스트.

## Constitution Check

*GATE: Phase 0 전 통과 필수. Phase 1 후 재확인.*

`.specify/memory/constitution.md` 는 **미작성 템플릿**(placeholder 만 존재) — 비준된 프로젝트 헌법이 없어요. 따라서 프로젝트별 정식 gate 는 없고, 저장소 자체 엔지니어링 표준(`CLAUDE.md` 의 강제 규칙)을 gate 로 적용해요:

| Gate | 적용 | 상태 |
|---|---|---|
| hook fail-open (exit 0, panic 금지) | preauth-check 출력 경로 유지, `unwrap`/`panic` 미도입 | ✅ PASS |
| 권한 보존 (`0600`/`0700`) | 저장 위치만 변경, 기존 `write_private_file_no_follow` + `set_private_dir_mode` 재사용 | ✅ PASS |
| consent 보안 계약 (TTL·pending single-use·HMAC) | pending claim 소비·TTL·서명 로직 보존, 위치만 이동 | ✅ PASS |
| surgical change (`CLAUDE.md` §3) | `runtime_root()` 폴백 한 줄 + 스윕 헬퍼 + deny 필드 추가, 무관 코드 미수정 | ✅ PASS |
| 무회귀 (`cargo test`/`tsc` green) | 기존 테스트는 XDG_RUNTIME_DIR 분기 고정 → 폴백 변경과 직교 | ✅ PASS (Phase 1 재확인) |

**위반 없음** → Complexity Tracking 비움.

## Project Structure

### Documentation (this feature)

```text
specs/003-fix-plugin-login-deny/
├── plan.md              # 이 파일
├── spec.md              # 기능 명세 (specify + clarify 완료)
├── research.md          # Phase 0 — 저장 경로·스윕·P2 필드·Windows 경계 결정
├── data-model.md        # Phase 1 — Consent Token(불변) + runtime_root 해석표
├── quickstart.md        # Phase 1 — fail-before/pass-after 재현·검증 절차
├── contracts/
│   └── preauth-check-output.md   # PreToolUse hook 출력 JSON 계약(allow/deny, P2 필드)
├── checklists/
│   └── requirements.md  # specify 단계 품질 체크리스트 (16/16)
└── tasks.md             # /speckit-tasks 산출물 (이 명령이 만들지 않음)
```

### Source Code (repository root)

```text
crates/axhub-helpers/
├── src/
│   ├── consent/
│   │   ├── key.rs       # ★ runtime_root() 폴백 수정 (핵심) — token_file_path / pending_token_file_path
│   │   ├── jwt.rs       # mint_token_to_path(create_dir+write), claim_pending_token(read_dir+sweep) — 만료 consent 스윕 헬퍼 추가
│   │   └── parser.rs    # format_preauth_deny_hint (P2 사유 텍스트, 변경 없음)
│   └── main.rs          # cmd_preauth_check(deny 출력 → P2 permissionDecisionReason 추가), cmd_consent_mint
└── tests/
    └── cli_e2e.rs       # ★ 신규 회귀 테스트: XDG_RUNTIME_DIR unset + mint/claim TMPDIR 상이 → allow
```

**Structure Decision**: 단일 Rust 크레이트(`axhub-helpers`) 내부 변경이에요. 새 모듈·새 파일 없이 기존 `consent/` 모듈 3파일 + `main.rs` 1지점 + 통합 테스트 1파일만 손대요. `runtime_root()` 의 **블라스트 반경은 정확히 4개 call site**(`key.rs:38,41` 경로 빌더, `jwt.rs:137-138` mint, `jwt.rs:209` claim)로 한정 — 다른 모듈은 `runtime_root` 를 안 써요(검증: `grep -rn runtime_root`).

## Complexity Tracking

> Constitution Check 위반 없음 — 비움.

## Phase 출력

- **Phase 0** → [research.md](./research.md): 저장 경로 결정, FR-007 스윕 범위, P2 출력 필드(연구 필요), Windows HOME 경계, gitnexus 도구명 메모.
- **Phase 1** → [data-model.md](./data-model.md), [contracts/preauth-check-output.md](./contracts/preauth-check-output.md), [quickstart.md](./quickstart.md), CLAUDE.md SPECKIT 마커 갱신.
- **Phase 2** (다음 명령): `/speckit-tasks` 가 tasks.md 생성. P1(runtime_root 수정)이 **단독 ship 가능**하도록 정렬 — P2 는 분리 task.

## 구현 단계 메모 (tasks/implement 용)

1. **`gitnexus_impact` 강제 준수**: `CLAUDE.md` 가 symbol 편집 전 `gitnexus_impact` 를 요구해요. 도구는 `mcp__gitnexus__impact` 로 노출돼요(`gitnexus_impact` 아님 — ToolSearch 주의). 본 plan 의 수동 4-call-site grep 은 대체 근거지만, implement 단계에서 `mcp__gitnexus__impact({target:"runtime_root", direction:"upstream"})` 로 재확인해요.
2. **P1 standalone**: `runtime_root()` 폴백 수정 + 회귀 테스트만으로 로그인이 통과해야 해요. P2·스윕은 같은 PR 에 묶되 독립 커밋/태스크.
3. **경로 형태 주의**: 현재 구현은 마지막에 `.join("axhub")` 를 붙이는 구조라, 단순 치환하면 `state_root()/runtime/axhub` 가 될 수 있어요. 반드시 `XDG_RUNTIME_DIR` 분기에서만 `.join("axhub")` 하고, 폴백은 정확히 `state_root().join("runtime")` 를 반환하게 구현해요.
4. **권한·계약 불변**: 저장 위치만 이동. `mint_token_to_path` 의 `create_dir_all`+`set_private_dir_mode` 가 새 경로에도 `0700` 보장하는지 확인(state_root 은 load_or_mint_key 가 이미 0700 생성). `/axhub:auth` pending consent 는 1회 claim 후 삭제되어야 하고, 기존 session/always decision token semantics 는 본 작업에서 새로 바꾸지 않아요.
5. **문서 갱신**: README/README.html 의 consent 경로 설명(`$XDG_RUNTIME_DIR/axhub`, `${XDG_RUNTIME_DIR:-/tmp}/axhub`)이 새 fallback 계약과 어긋나지 않게 별도 task 에서 갱신해요.
6. **인덱스 갱신**: 커밋 후 `npx gitnexus analyze`(PostToolUse hook 자동) + `bun run release` 흐름은 별도.
