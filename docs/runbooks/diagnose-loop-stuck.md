# Runbook: diagnose loop stuck

## 증상

- 사용자가 `axhub-diagnose` SKILL 호출했는데 응답이 안 옴
- `~/.axhub/loops/<loop_id>/` 디렉토리 남아있음
- `DiagnoseSession` 가 IDLE 로 복귀 안 함

## 원인

1. **HITL session 타임아웃 (300s) 후 사용자 응답 대기** — 정상 동작이지만 vibe coder 가 자리 비웠을 가능성
2. **flock 경합 deadlock** — 다른 process 가 audit ledger lock 점유 중
3. **state machine deadlock** — 동일 process 내 multi-thread 동시 `apply()` 호출 (가능성 낮음, Mutex 가 serialize)

## 즉시 조치

```bash
# 1. 활성 loop 확인
ls ~/.axhub/loops/

# 2. 가장 오래된 loop 의 audit ledger 확인
cat ~/.axhub/audit-ledger/ledger.jsonl | grep <loop_id> | tail -10

# 3. 강제 cleanup (audit 남기고 cwd-shadow 만 제거)
rm -rf ~/.axhub/loops/<loop_id>/cwd-shadow

# 4. 다음 진단 시 새 loop_id 사용 (UUID v4 자동 생성)
```

## 근본 fix

- HITL session timeout 단축 (`HITL_SESSION_TIMEOUT` env, default 300s)
- per-prompt timeout 조정 (`HITL_TIMEOUT` env, default 60s)
- audit ledger lock timeout 추가 — Phase 0b 의 fslock 은 무한 대기. v0.8.1 에서 60s timeout 추가 검토.

## 관련 코드

- `crates/axhub-helpers/src/diagnose/hitl.rs:HITL_SESSION_TIMEOUT_SECS`
- `crates/axhub-helpers/src/diagnose/state.rs:DiagnoseSession::apply`
- `crates/axhub-helpers/src/audit_ledger.rs:append_entry_to`
