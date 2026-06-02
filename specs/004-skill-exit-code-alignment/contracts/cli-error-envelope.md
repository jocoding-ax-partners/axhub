# Contract — ax-hub-cli `--json` 실패 envelope (정합 목표)

이 문서는 복구 라우팅이 따라야 할 **CLI 측 계약**이에요. ax-hub-cli 0.17.2 가 실패 시 `--json` 모드로 emit 하는 구조이고, 정합 작업의 진실의 출처예요. (관찰 + authoritative Read 로 확인.)

## Envelope 모양 (관찰)

```json
{
  "schema_version": "1",
  "status": "error",
  "error": {
    "code": "<flat-slug>",
    "category": "client_error | server_error | ...",
    "hint": "<human message>",
    "subcode": "<optional-namespaced-subcode>"
  }
}
```

- live 관찰 1: `axhub deploy list --app probe-nonexistent-xyz --json` → `{"status":"error","error":{"code":"not_found","category":"client_error","hint":"not found: app `probe-nonexistent-xyz`"}}` (process exit `5`).
- live 관찰 2: `axhub deploy status --json` → `{"status":"error","error":{"code":"usage",...}}` (exit `64`).
- 성공: `{"status":"ok","data":{...}}`; dry-run: `{"status":"dry_run",...}` (axhub-output/lib.rs).

## 계약 규칙 (라우팅이 의존해도 되는 것)

1. **1차 키 = `error.code` (flat slug)**. version-agnostic. 라우팅은 이걸 우선 키로 써요.
2. **2차 = process exit code** (`exit_code.rs ExitCode`, `docs/cli-exit-codes.md` SLA): `4`=unauth · `5`=notfound · `6`=ratelimit · `7`=api · `8`=tenant · `9`=conflict · `10`=timeout · `11`=dryrun · `12`=domain · `13`=invite-expired · `14`=digest · `15`=swap · `64`=usage · `66`=enforce. `0`/`1`/`2`(clap)/`3`(shell) 관례.
3. **subcode** (`error.subcode`): 같은 code 의 분기 (예: `66` enforce → `scope.downgrade_blocked` vs `update.cosign_verification_failed`; `7` api → `backend_unimplemented`). 라우팅이 같은 base 안에서 분기할 때 써요.
4. **미지 비-0 코드** 는 helper 가 `1`(Generic)로 collapse — 라우팅은 그걸 catch-all 로 처리.
5. 옛 `65`/`67`/`68`/`70` 은 이 CLI 가 **emit 하지 않아요** (git pickaxe 무이력). 라우팅 키로 절대 쓰면 안 돼요 (FR-005).

## 확정 필요 (plan T0)

- `unauthenticated`(4) · `rate_limited`(6) · `tenant_*`(8) 등 미관찰 slug 의 정확한 문자열 → deauth/401 live 실행 + `error.rs Error::code()` 전수.
- `apis.call_consent_required` 의 실제 출처 (ax-hub-cli grep 0건 — plugin-side 또는 stale 가능성).

## drift-guard (FR-012)

이 계약의 `{code, exit_code, subcode}` 집합을 `crates/axhub-helpers/data/cli-exit-contract.json` 으로 pin 하고, parity 테스트가 catalog 키 ↔ pin 일치를 CI 에서 강제해요. CLI 가 계약을 바꾸면 테스트 fail → 사람이 snapshot 재동기화.
