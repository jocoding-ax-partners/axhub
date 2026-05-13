# ADR-0010: Stderr filter graceful degradation

## Status

Accepted (2026-05-13)

## Context

PR #80 의 SKILL `skills/deploy/SKILL.md` Step 4 selective stderr filter 가 외부 `axhub` CLI binary 의 stderr 가 다음 format 으로 emit 한다고 가정해요:

```
axhub-error-sub-key: 64:validation.deployment_in_progress
```

그러나 monorepo 내에서 이 prefix 를 emit 하는 코드는 단 2 곳뿐이에요:

- `crates/axhub-helpers/src/main.rs:1845`
- `crates/axhub-helpers/src/quality_gate.rs:15` (`SUB_KEY` const)

두 곳 모두 `quality_gate_failed` 만 emit. **`validation.deployment_in_progress` 를 이 prefix 로 emit 하는 monorepo 내 코드는 0건**이에요. `axhub deploy create` 자체는 외부 binary (별 repo) 이고 stderr emission contract 가 lock 안 됐어요.

만약 외부 axhub binary 가 다른 prefix (예: `Error:`, JSON-only, 또는 다른 schema) 로 emit 하면:

```bash
if [ $AXHUB_EXIT -eq 64 ] && grep -qE '^axhub-error-sub-key:.*64:validation\.deployment_in_progress' "$AXHUB_STDERR_TMP" 2>/dev/null; then
  # silent swallow path
```

위 grep anchor 가 fail → `else` 분기로 raw stderr 가 chat 으로 흘러요. PR #80 description 의 "raw Error: Exit code 64 chat 노출 0회" claim 이 깨질 수 있어요.

## Decision

Step 4 grep anchor `^axhub-error-sub-key:.*64:validation\.deployment_in_progress` **유지**해요. 대신 PR #80 의 약속을 **graceful degradation** 로 정확화해요:

> "raw Error chat 노출 0회" → **"best-effort selective filter + Step 6 reactive empathy template fallback"**

흐름:

1. 외부 axhub binary 가 verified format (`axhub-error-sub-key: 64:validation.deployment_in_progress`) 로 emit → grep anchor match → silent swallow + Step 5 watch 라우팅. PR #80 의 의도된 happy path.
2. 외부 binary 가 다른 prefix 로 emit (drift) → grep anchor fail → `else` 분기 → raw stderr 가 chat 노출. **단**, Step 6 (`references/error-empathy-catalog.md` 의 exit 64 + `validation.deployment_in_progress` 4-part Korean empathy template) 가 user-facing wording 으로 wrap → "raw Error: Exit code 64" 텍스트 자체는 user 가 봤어도 한국어 empathy template 가 즉시 surface → vibe coder UX 손상 최소.

### Pattern relaxation 비채택

대안으로 grep pattern 을 **substring** 형태 (`grep -qE 'validation\.deployment_in_progress'`) 로 완화하는 옵션도 검토했어요. 폐기 사유:

- **False-positive risk**: catalog 의 다른 stderr lines (예: 다른 SKILL 의 verbose logs, `validation.deployment_in_progress` 가 metadata 로 등장하는 documentation dump) 에 우연히 substring 매칭 → wrong silent swallow.
- **Root cause 회피**: substring 완화도 외부 binary contract 가 stable 하다는 가정. drift 가 substring 자체를 바꾸면 동일 break.
- **Anchor `^axhub-error-sub-key:.*` 가 더 안전**: false-positive 가 ranges of stderr 안에서 발생할 가능성 극히 낮음.

## Consequences

### + 긍정

- PR #80 claim 정직성 회복 — 0 leak 약속 → best-effort + fallback 명시.
- grep anchor 유지 — false-positive 위험 회피.
- Step 6 empathy template 가 fallback 으로 명시 — drift 시 vibe coder UX 가 catastrophic 하지 않음.

### − 부정

- 외부 axhub CLI binary stderr format drift 시 client side 가 detect 불가 — 별 contract test 또는 nightly cron 필요.
- "raw Error: Exit code 64" 가 0.X% 케이스 (drift) 에서 user 에게 잠깐 노출될 수 있음 — Step 6 empathy template 가 즉시 follow up 하지만 정신적 부담 미미.

## Follow-ups

- **Phase 2 (별 RFC)**: cross-repo contract test — `axhub` CLI repo 와 stderr emission golden file SLA 협의. Nightly drift cron (issue #88 추적).
- PR #80 의 PR description / CHANGELOG 명시적 정정 안 함 (canonical surface = 본 ADR-0010).

## 관련

- PR #80: `/axhub:deploy` push 후 중복 deploy_create race 차단
- PR #84: `--refresh-in-flight` selective refresh (issue #81 M3)
- Issue #86: axhub CLI stderr format contract gap (본 ADR 의 발견 issue)
- Issue #88: nightly staging drift cron
- `skills/deploy/SKILL.md:537` POSIX grep anchor
- `skills/deploy/SKILL.md:572` PowerShell Select-String anchor
- `crates/axhub-helpers/src/main.rs:1845` axhub-error-sub-key emit (quality_gate_failed only)
- `crates/axhub-helpers/src/quality_gate.rs:15` SUB_KEY const
- `skills/deploy/references/error-empathy-catalog.md` exit 64 4-part Korean empathy template (Step 6 fallback)
