# Audit Privacy Contract — Approach E

원본 plan: `.plan/ceo-review-nl-routing/2026-05-07-nl-routing-redesign.md`
Phase 0 sub-task 0.5 deliverable. Phase 3 (audit module) + Phase 7 (docs/routing.md) implementation contract.

---

## What we collect

매 prompt 마다 audit log JSONL line 한 줄을 로컬 디스크에 append 해요. 다음 field 만 기록해요:

| Field | Type | Description |
|-------|------|-------------|
| `ts` | string | ISO 8601 UTC 타임스탬프 |
| `prompt_hash` | string | sha256 hex (32-byte hash, prompt 원문 X) |
| `prompt_len` | number | prompt 길이 (chars) |
| `cli_version` | string\|null | preflight 결과 (`axhub --version`) |
| `auth_ok` | bool | preflight 결과 (`axhub auth status`) |
| `is_axhub_related` | bool | `prompt.contains("axhub")` substring boolean (단순 measurement) |

이게 전부예요. prompt content 원문은 절대 저장 안 해요.

---

## What we don't collect

- prompt content (원문)
- 사용자 발화 의도 / 결정된 skill / fired_skill
- 외부 telemetry endpoint 전송 (모두 로컬 디스크)
- 식별 정보 (user id / hostname / IP / git remote)
- 다른 process / 다른 plugin 의 데이터

---

## Storage

- **Path**: `runtime_paths::state_dir()` 결과 (예: `~/.local/share/axhub-plugin/`) 안 `routing-audit-{YYYY-MM-DD}.jsonl`
- **Permissions** (Unix): dir `0700` (owner rwx only), file `0600` (owner rw only)
- **Permissions** (Windows): NTFS ACL — current user only (Phase 3 implementation 책임)
- **Rotation**: 매 `routing-stats` CLI 호출 시 자동. 7일 이상 된 파일 삭제. today + yesterday 보존.

---

## Hash 익명화 한계 (중요)

짧은 prompt 의 sha256 hash 는 reverse 가능해요:

- `"deploy"` (6 bytes) → 미리 계산된 hash dictionary 로 reverse 가능
- `"배포"` (6 bytes UTF-8) → 동일
- `"axhub deploy"` (12 bytes) → 흔한 phrase 라 reverse 가능

긴 prompt (100+ chars) 의 hash 는 사실상 reverse 불가능. 하지만 *이론적 익명화 보장* 은 안 해요.

따라서 사용자에게 명시적으로 안내해야 해요:

> "짧은 prompt 의 hash 는 익명화 보장 안 돼요. privacy 우려 시 AXHUB_NO_AUDIT=1 으로 끄거나 cleanup-audit --all 로 전체 삭제해요."

---

## Opt-out

`AXHUB_NO_AUDIT=1` 환경 변수 set 시 audit::append 가 즉시 no-op return. 디스크 write 0.

```bash
# 한 세션만
AXHUB_NO_AUDIT=1 claude

# 영구 (zsh)
echo 'export AXHUB_NO_AUDIT=1' >> ~/.zshrc
```

---

## Deletion

```bash
# 7일 이상 된 파일만 삭제 (rotation)
axhub-helpers cleanup-audit

# 전체 삭제 (--yes 로 confirm 우회)
axhub-helpers cleanup-audit --all --yes
```

cleanup-audit subcommand 의 구현 책임은 Phase 3 (audit module) 에 있어요.

---

## Disclosure 노출 위치 (Phase 7 implementation 책임)

이 Privacy Contract 는 다음 4 곳에 사용자 disclosure 로 노출돼요:

### 1. SessionStart 첫 v0.4.0 알림 (한 번만)

`crates/axhub-helpers/src/bootstrap.rs` 의 SessionStart systemMessage v0.4.0 첫 세션 marker 안:

> "axhub 가 prompt 통계 (sha256 hash 만, 외부 전송 X) 를 7일간 로컬에 보관해요. 끄려면 AXHUB_NO_AUDIT=1 환경 변수 설정해요. 짧은 prompt 의 hash 는 익명화 보장 안 돼요."

### 2. README.md 의 라우팅 섹션

> "audit log 로컬 7일 보관 (외부 전송 X). 끄려면 AXHUB_NO_AUDIT=1. 상세: docs/routing.md"

### 3. docs/routing.md 의 Privacy 섹션

이 문서의 What we collect / don't collect / Hash 한계 / Opt-out / Deletion 섹션을 그대로 복사해요.

### 4. routing-stats CLI 의 footer

`axhub-helpers routing-stats` 출력 마지막 3 줄:

> "audit log 위치: `<dir>`"
> "끄려면: AXHUB_NO_AUDIT=1"
> "삭제: axhub-helpers cleanup-audit --all"

---

## Phase 3 / Phase 7 Implementation Reference

Phase 3 (audit module) 가 다음을 보장해요:

- AuditRecord schema = 위 6 field 만
- prompt content field 추가 금지 (struct 자체에 field 없음)
- redact() defense-in-depth pass on JSONL line
- panic::catch_unwind on redact (hook crash 방지)
- AXHUB_NO_AUDIT=1 → 0 disk I/O
- Unix file 권한 0700/0600 enforced (Windows = ACL equivalent)
- cleanup-audit subcommand 구현 + 한국어 confirm UX
- 7-day auto-rotation

Phase 7 (docs/routing.md + SessionStart) 가 다음을 보장해요:

- 4 곳 disclosure 모두 이 contract 의 한국어 해요체 문구 사용
- v0.4.0 첫 세션 marker (`.v0.4.0-welcome-shown`)
- README 라우팅 섹션 + docs/routing.md 의 Privacy 섹션
- routing-stats CLI footer 3 줄

---

## Compliance posture

- 외부 telemetry endpoint X — GDPR / CCPA "data sale" 분류 해당 X
- prompt 원문 저장 X — PII 직접 포함 risk minimal
- hash 만 — derived data (GDPR 4(1) 의 "personal data" 분류 회색지대 but 단독으로는 식별 X)
- 사용자 disclosure + opt-out + deletion 제공 — best-effort compliance

이 plan 은 *legal compliance 가이드 아님*. 실제 제품 launch 전 sosec / privacy 팀 review 권장.
