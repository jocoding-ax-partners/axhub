# Contract: `axhub setup doctor`

**소유**: ax-hub-cli repo (WS-A) · **참조**: 스펙 FR-014·FR-015, research R-7·R-8

## Invocation

```
axhub setup doctor [--json]
```

## 동작

- **읽기 전용** — 어떤 상태도 바꾸지 않아요. 자동 수정 없음(v1), 네트워크 필수 아님(로컬 감지 우선).
- 검사 항목은 DoctorReport(data-model) 의 5종이에요: `cli_install`(버전 포함) · `marketplace` · `plugin` · `mcp` · `runtime`(EnvironmentCheck 등급).
- 판정 로직은 `plugin-support onboarding-detect` 를 내부 재사용해요 — 별도 감지 로직을 이중 구현하지 않아요.
- 각 problem 항목에 해결 명령 또는 다음 행동을 한 줄로 제시해요 (예: "axhub setup claude 를 다시 실행하면 이 단계만 채워져요").

## Exit codes

| code | 의미 |
|---|---|
| 0 | 전 항목 ok |
| 1 | 하나 이상 problem — 각 항목의 fix 안내 출력 |

## 불변식

- 출력 한글은 해요체(DP-2), raw JSON/stderr 비노출(사람용 모드), `--json` 은 DoctorReport 직렬화.
- 공개 명령이에요 — hidden `plugin-support` 표면을 사용자에게 안내하지 않아요(DP-6·AP-9 정합).
- 기존 `diagnosis` 스킬(배포 실패 전용, AP-4)과 역할이 겹치지 않아요 — deploy 관련 검사를 포함하지 않아요.

## 계약 테스트 (WS-A)

- 5개 검사 항목의 ok/problem 조합별 판정·fix 문구
- 읽기 전용 보장 — 실행 전후 파일시스템·claude 설정 diff 0
- exit code 0/1 매핑
