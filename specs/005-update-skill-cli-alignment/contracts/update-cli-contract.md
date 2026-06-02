# Contract: `axhub update` (v0.17.2) — 스킬 정합 대상

> 이 문서는 `skills/update/SKILL.md` 가 **반드시 일치해야 하는** 외부 계약이에요. 출처: live `axhub update --help`, `axhub/src/commands/update.rs`, `crates/axhub-core/src/{exit_code,error}.rs`, `docs/cli-exit-codes.md` (ax-hub-cli v0.17.2). 스킬이 이 계약을 벗어나면(없는 flag/env/exit/subcode) 정합 위반이에요.

## 1. 명령 surface

```
axhub update            # check/apply 디스패치
axhub update check      # 새 릴리스 있는지 확인
axhub update apply      # 다운로드 + 설치(원자적 swap)
```

### `update check` 플래그
전역 플래그만 (`--json`, `--profile`, `--tenant`, `--timeout`, `--non-interactive`, ...). update 전용 플래그 없음.

### `update apply` 플래그
| 플래그 | 의미 | 계약 |
|---|---|---|
| `--dry-run` | preview only. **기본 true** | destructive 아님 |
| `--execute` | download+verify+swap. `--dry-run` 과 conflict | destructive — 명시 필수 |
| `-y, --yes` | execute 의 post-confirm prompt skip. dry-run 에선 무효 | |
| `--force` | downgrade 게이트만 우회. **cosign 검증은 절대 우회 안 함** | 서명 실패는 force 와 무관하게 swap 차단 |
| `--json` | JSON 출력 | |

**금지(존재 안 함)**: `AXHUB_REQUIRE_COSIGN`, `AXHUB_ALLOW_UNSIGNED`, `AXHUB_DISABLE_AUTOUPDATE` 환경변수. 어떤 cosign 강제용 env 접두도 쓰지 않음 — cosign 은 기본 Enforce.

## 2. JSON 봉투

### `update check --json`
```json
{ "current": "v0.17.2", "latest": "v0.18.0", "has_update": true }
```
- `has_update: false` → 이미 최신.

### `update apply --dry-run --json` (preview)
```json
{
  "applied": false,
  "preview": true,
  "current": "v0.17.2",
  "latest": "v0.18.0",
  "has_update": true,
  "is_downgrade": false,
  "feed_base": "https://<cdn>/...",
  "next_step": "Pass --execute to download, verify, and swap the binary."
}
```
- `is_downgrade: true` → execute 시 `--force` 필요.

### `update apply --execute --json` (성공)
```json
{
  "applied": true,
  "install_kind": "self_replace",
  "current": "v0.17.2",
  "latest": "v0.18.0",
  "binary": "/Users/<u>/.axhub/bin/axhub"
}
```
- human 모드: `Updated axhub from <current> to <latest>. Run 'axhub --version' to verify.`

### 오류 봉투
`error.subcode` 를 hint 텍스트보다 우선해서 분기. (예: `{"error": {"subcode": "update.cosign_enforce_failed", ...}}`)

## 3. 종료 코드 ↔ subcode 계약 (update 관련)

| exit | Error variant | subcode | 의미 | 스킬 요구 행동 |
|---|---|---|---|---|
| 0 | — | — | 성공 | 완료 안내 |
| 1 | Io/Serde/Other | — | Generic | 일반 오류 |
| 4 | Unauthenticated | `token_missing`/`token_invalid` | 미인증 | `axhub auth login` |
| 10 | Timeout | — | 전송 타임아웃 | 안내. **apply 전송 실패는 자동 재시도 금지** |
| 14 | VerifyDigestMismatch{expected,actual} | — | 다운로드 산출물 SHA256 ≠ 매니페스트 핀 | **변조 신호 → 즉시 중단, `--force` 금지, IT/보안 통보** |
| 15 | SwapFailed{detail} | — | self_replace 원자 교체 실패 | **자동 재시도 금지(부분 교체 가능), `~/.axhub/bin/axhub.<old>.bak` 롤백 안내** |
| 64 | Usage | — | clap/사용 오류 | 인자 교정 |
| 66 | DowngradeBlocked{subcode} | `update.downgrade_blocked` | 다운그레이드 차단 | `--force`(cosign 안전) 안내 |
| 66 | CosignEnforceBlocked | `update.cosign_enforce_failed` | cosign enforce 실패 | **하드 스톱, 우회 없음, IT/보안 통보** |

> **exit 2 는 update 계약에 없음** — clap usage 예약("Do NOT remap"). 스킬은 exit 2 를 autoupdate 정책으로 해석하면 안 됨.

## 4. cosign 규칙 (보안 critical)

1. cosign 검증은 **기본 Enforce** — 스킬은 env 로 켜지 않음, 그냥 apply 호출.
2. `--force` 는 downgrade 게이트만 우회, **서명 검증은 절대 우회 안 함**.
3. exit 14(digest)·exit 66+`update.cosign_enforce_failed` 는 **하드 스톱** — 어떤 bypass 도 제시 금지.
4. v0.14.0+ : 자산별 `.sha256` + cosign keyless `.sig`/`.pem` 사이드카 검증.

## 5. 정합 검증 (이 계약 대비)

- **CMD/FLAG**: 스킬이 부르는 모든 `axhub update ...` 명령이 §1 surface 의 부분집합 → live `axhub update apply --dry-run`(+잘못된 인자 reject) 으로 확인.
- **NO-FAKE-ENV**: 스킬 본문 grep `AXHUB_REQUIRE_COSIGN|AXHUB_ALLOW_UNSIGNED|AXHUB_DISABLE_AUTOUPDATE` = 0.
- **EXIT/SUBCODE**: §3 의 각 행이 스킬에 정확히 1개 대응 행동으로 존재, subcode 문자열 정확.
- **NO-BREW**: 스킬 본문 grep `package_manager|brew|scoop` = 0.
