# Phase 1 Data Model: update 스킬이 분기하는 계약 엔티티

이 feature 는 데이터 저장이 없어요. 여기서 "엔티티"는 **스킬이 파싱·분기하는 CLI 계약 표면** 이에요 — 스킬 rewrite 가 정확히 이 모양을 읽고 처리해야 정합이에요.

## 엔티티

### UpdateCheckResult (`update check --json`)
| 필드 | 타입 | 비고 |
|---|---|---|
| current | string | 현재 버전 태그 (예 `v0.17.2`) |
| latest | string | 최신 태그 |
| has_update | bool | true → 업그레이드 가능. false → 최신 |

### ApplyPreview (`update apply --dry-run --json`)
| 필드 | 타입 | 비고 |
|---|---|---|
| applied | bool | 항상 false (preview) |
| preview | bool | 항상 true |
| current / latest | string | |
| has_update | bool | |
| is_downgrade | bool | true → execute 시 `--force` 필요 |
| feed_base | string | CDN feed URL (안내용) |
| next_step | string | "Pass --execute ..." |

### ApplyResult (`update apply --execute --json`, 성공)
| 필드 | 타입 | 비고 |
|---|---|---|
| applied | bool | true |
| install_kind | string | `"self_replace"` |
| current / latest | string | |
| binary | string | 교체된 바이너리 경로 (`~/.axhub/bin/axhub`) |

### UpdateError (오류 봉투 / 종료 코드)
| 필드 | 타입 | 비고 |
|---|---|---|
| exit_code | u8 | 0/1/4/10/14/15/64/66 (contract §3) |
| subcode | string? | `update.downgrade_blocked` / `update.cosign_enforce_failed` / `token_*` 등. hint 보다 우선 |
| detail/expected/actual | string? | variant별 (SwapFailed.detail, VerifyDigestMismatch.expected/actual) |

규칙:
- exit 14 → 변조, 하드 스톱, force 금지.
- exit 15 → 재시도 금지(부분 교체).
- exit 66 → subcode 분기: downgrade(force 우회 가능) vs cosign(하드 스톱).
- exit 2 → 계약에 없음(clap 예약). 스킬이 정책으로 해석 금지.

## 상태 전이 (스킬 워크플로)

```
[start]
  └─ check --json
       ├─ has_update=false ───────────────→ [최신 안내] (종료)
       └─ has_update=true ─→ 업그레이드 카드 + AskUserQuestion(업그레이드?)
            ├─ skip ────────────────────────→ [현재 유지] (종료)
            ├─ notes ─→ 릴리즈 노트 → (다시 카드)
            └─ apply ─→ apply --dry-run --json (preview: 버전/install/feed/is_downgrade)
                 └─ AskUserQuestion(적용?) [비대화형이면 기본 skip]
                      ├─ 취소 ───────────────→ (종료)
                      └─ 적용 ─→ apply --execute --yes --json
                           ├─ exit 0 (applied=true) ─→ [완료 안내]
                           ├─ exit 14 ─→ [변조 하드 스톱]
                           ├─ exit 15 ─→ [재시도 금지 복구 안내]
                           ├─ exit 66 + downgrade_blocked ─→ [--force 안내]
                           ├─ exit 66 + cosign_enforce_failed ─→ [cosign 하드 스톱]
                           └─ exit 1/4/10/64 ─→ [error-empathy-catalog 라우팅]
```

## 검증 규칙 (스킬↔엔티티)

- 스킬이 읽는 모든 JSON 필드 ∈ 위 엔티티 (없는 필드 가정 금지).
- 스킬이 분기하는 모든 exit/subcode ∈ contract §3 (없는 exit 2/허구 subcode 금지).
- non-interactive 기본값 = apply skip (registry `update.apply_consent` 와 일치).

## 비-엔티티 (이 feature 가 만들지 않음)

- 영속 데이터·DB·config 파일 신규 생성 없음.
- drift-guard fixture/snapshot 없음 (범위 밖, spec Clarifications).
