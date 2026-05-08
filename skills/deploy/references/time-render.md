# Time Rendering Rules (시각 표시 단순화)

`auth status --json` / preflight 가 emit 하는 raw `expires_at` (ISO-8601 RFC3339) 을
helper 가 사전 포맷한 `expires_human` 필드를 SKILL 이 그대로 echo 하는 규칙이에요.

## Placeholder

SKILL prompt 본문은 `<EXPIRES_HUMAN>` 한 개만 사용해요. 이 placeholder 는
`auth status --json` 의 `expires_human` 필드 (helper 사전 포맷, deterministic) 를
그대로 인용해요. LLM 변환 책임 zero.

## Helper 출력 형식 (Format X 7 case)

| 잔여 시간 | 출력 예시 |
|---|---|
| 365일 이상 | `약 73년 남았어요` |
| 7일 ~ 365일 미만 | `30일 5시간 남았어요` |
| 24시간 ~ 7일 미만 | `21시간 36분 남았어요` |
| 1시간 ~ 24시간 미만 | `3시간 12분 남았어요` |
| 5분 ~ 1시간 미만 | `42분 남았어요` |
| 0초 ~ 5분 미만 | `곧 만료돼요 (5분 미만)` |
| 음수 (이미 만료) | `이미 만료됐어요` |

`expires_at` null/missing 시 `expires_human` 도 None 이라 SKILL 은 "만료 정보 없음"
literal 출력해요.

## Anti-patterns (NEVER)

- NEVER raw ISO-8601 timestamp 를 SKILL 출력에 echo. v3 이전 동작
  (`tests/e2e/claude-cli/output/13/stdout.json` 의 `"만료: 2099-01-01 (남은 시간: 약 73년)"`)
  은 anti-pattern 이에요. v3 는 helper 가 `"약 73년 남았어요"` 만 emit, SKILL 은
  그것만 echo.
- NEVER `<expires_at>`, `<EXPIRES_AT>`, `(남은 시간:`, `<DELTA>` placeholder 사용.
  `<EXPIRES_HUMAN>` 한 슬롯만 사용해요.
- NEVER timezone 명시 (`KST`, `UTC+9`) — helper 가 `tz` 인자로 받아 처리해요.
- NEVER LLM 측에서 시간 계산 — helper 출력 byte-identical echo 만.

## Used by

- `skills/auth/SKILL.md` (logged-in identity card, row 33)
- `skills/doctor/SKILL.md` (diagnostic card, row 125)
