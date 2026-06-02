# Contract: verify 가 구동하는 CLI/helper 표면 (v0.17.2) — 스킬 정합 대상

> `skills/verify/SKILL.md` 가 **반드시 일치해야 하는** 외부 계약. 출처: live `--help`, `verify_helper.rs`, `cli/args/mod.rs`, ax-hub-cli `deploy/*`. 벗어나면(없는 flag/값, 틀린 verdict/필드) 정합 위반.

## 1. helper verify (`axhub-helpers verify`)

```
axhub-helpers verify --app-id <APP> [--json]      # primary
axhub-helpers verify --app <APP> [--json]         # --app 은 visible_alias
```

### 출력 (`--json`) = VerifyResult
```json
{
  "verdict": "live",                    // ∈ {"live","suspect","not_live"} (snake_case)
  "state": "active",                    // nullable; helper LIVE_STATES 로 판정
  "last_deploy_id": "dep_abc",          // nullable
  "last_deploy_age_secs": 120,          // nullable
  "errors": [],                         // ERROR/FATAL 로그 라인
  "reasons": ["..."]                    // 한국어 사유 — verdict 아래 그대로 출력
}
```
- **금지**: `verdict:"passed"` (존재 안 함).
- verdict 매핑: `live`→✅ 라이브 / `suspect`→⚠️ 의심 / `not_live`→❌ 안 됨.
- LIVE_STATES = `live`/`running`/`deployed`/`active`/`ok`/`succeeded`. FRESH window = 600s.
- no-recent-deploy sentinel: `{"state":"unknown","last_deploy_id":null}` → not_live.

## 2. deploy status

```
axhub deploy status [DEPLOYMENT_ID] --app <APP> --json        # 5s timeout
```
- `.status` = **백엔드 free string** (닫힌 enum 아님). live 판정은 §1 LIVE_STATES 미러. 그 외(failed/stopped/pending/building 류)는 미라이브 휴리스틱.
- `.current_stage` 로 단계 안내. `--watch`/`--watch-interval` 은 verify 에서 미사용.

## 3. deploy list

```
axhub deploy list --app <APP> --json
```
- `--limit` **없음**. 최신 배포는 출력에서 client-side 선택.

## 4. deploy logs (app-level)

```
axhub deploy logs --app <APP> --json [--source <label>] [--follow]
```
- **app-level** 로그 (`list_app_logs`). `[DEPLOYMENT_ID]` positional 은 **legacy**(스코핑 안 함).
- `--source` = free passthrough 문자열 — `pod`/`runtime`/`build` 같은 **고정 enum 없음**. verify 는 source 없이(또는 단순 label) app-level 로그를 받아 client-side 마지막 ~50줄 trim 후 `ERROR`/`FATAL` grep.
- `--tail` 류 N-라인 flag 없음. `--follow` 는 비-TTY/agent 면 단일 스냅샷 degrade — verify 는 단발만.

## 5. helper list-deployments

```
axhub-helpers list-deployments --app-id <APP> [--app <APP>] [--limit <N>] [--json]
```
- primary `--app-id`, alias `--app`, `--limit` 지원.

## 6. error_code 라우팅 (참조 정본)

`skills/recover/SKILL.md` §canonical 표 cross-link (정정 불요):
| error_code | 안내 |
|---|---|
| `auth.token_invalid` (65) | `/axhub:auth` 재인증 |
| `resource.app_not_found` (67) | did-you-mean + `apps` |
| `transport.timeout` (1) | 재시도 1회 + 네트워크/버전 확인 |
| `transport.cli_missing` (1) | `axhub --version` → `/axhub:install-cli`/`/axhub:setup` |
| `response.invalid_json` (1) | `/axhub:update`/`/axhub:doctor` |
| `response.error_envelope_unknown_shape` (1) | `/axhub:upgrade` |

preflight `auth_error_code`: `cli_not_found`/`cli_unavailable`→install-cli, `cli_config_corrupted`→auth, `cli_too_old`→upgrade.

## 7. 정합 검증 (이 계약 대비)

- **VERDICT**: 스킬 본문 grep `verdict.*passed` = 0; verdict 3값 매핑 존재.
- **CMD/FLAG**: 스킬이 부르는 `axhub deploy {status,list,logs}` + `axhub-helpers {verify,list-deployments}` 가 §1-5 surface 부분집합 → live `--help` 로 확인.
- **STATUS**: live 판정이 LIVE_STATES(`ok` 포함)와 일치; status 를 닫힌 enum 으로 안 다룸.
- **LOGS**: `deploy logs --app` (app-level); `pod` 고정 source 가정 없음.
- **APP-ARG**: `--app-id`(primary)/`--app`(alias) 설명 정확.
