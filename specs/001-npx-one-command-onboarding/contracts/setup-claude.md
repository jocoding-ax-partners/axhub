# Contract: `axhub setup claude`

**소유**: ax-hub-cli repo (WS-A) · **참조**: 스펙 FR-001~013·FR-015, research R-4~R-6·R-10·R-11

## Invocation

```
axhub setup claude [--yes] [--dry-run] [--json]
```

- `--yes` : 비대화형 강제 (현재 프롬프트가 없어도 계약으로 고정 — 미래 프롬프트 추가 시에도 유효)
- `--dry-run` : 고지 목록만 출력하고 아무것도 바꾸지 않아요
- `--json` : 사람용 출력 대신 기계용 결과 (SetupStep·InstallationFootprint·HandoffCard 직렬화)

## 실행 순서 (data-model SetupStep 순서 고정)

1. self-install — npx 임시 캐시 실행이면 `~/.axhub/bin` 영구 복사 + PATH 등록(`repair-path` 재사용)
2. `claude` CLI 감지 — 부재 시 설치 안내 출력 후 종료 (이후 단계 미실행, FR-004)
3. 실행 전 고지 — InstallationFootprint 를 install/skip 표시와 함께 출력 (확인 프롬프트 없음, FR-013)
4. `claude plugin marketplace add jocoding-ax-partners/axhub`
5. `claude plugin install axhub@axhub`
6. `claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp`
7. HandoffCard 출력 (next_phrase·restart_note·rerun_note 필수)

## Exit codes

| code | 의미 |
|---|---|
| 0 | 전 단계 done/skipped (dry-run 완료 포함) |
| 1 | 단계 실패 — 실패 단계·원인·다음 행동을 한국어 1-2문장으로 출력(FR-011) |
| 2 | 사전 조건 미충족 — claude 부재(설치 안내 출력) 또는 환경 `block` 등급(FR-015) |

정확한 코드 값은 CLI 관례에 맞춰 구현 시 확정하되, "성공 / 실패 / 사전 조건" 3분류 구분은 계약이에요.

## 불변식

- **멱등**: 같은 머신에서 재실행하면 완료 단계는 `skipped` 표시 후 통과, 두 번째 연속 실행은 무변경이에요(FR-003, SC-003).
- **비대화형**: 입력 불가·CI·`--yes` 에서 입력 대기 0회·브라우저 열기 0회 — 입력 대기로 멈추면 계약 위반이에요(FR-012).
- **auth 무접촉**: 로그인·토큰·GitHub App 을 다루지 않아요(FR-006). Windows Git Bash 계약(AP-13)과의 접점이 없어야 해요.
- **user-scope 전용**: admin/시스템 권한 요구 금지(FR-005).
- **출력**: 사용자 노출 한글은 전부 해요체(DP-2), raw stderr·내부 id 비노출(FR-011), URL 은 평문 절대 URL.
- **고지-문서 일치**: 3단계 고지 목록은 설계 §4.3(사용자 문서 공개본)과 항목이 일치해야 해요(FR-007·FR-013) — parity 테스트 대상.

## 계약 테스트 (WS-A)

- claude 유/무 × 단계별 기존재/신규 × 재실행 — 상태 전이표(data-model) 일치
- `--dry-run` 후 파일시스템·claude 설정 무변경
- stdin 닫힘 + `CI=1` 환경에서 입력 대기 없이 종료 (timeout 가드)
- `--json` 스키마 안정성
