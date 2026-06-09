## Handoff: team-plan(consensus) → team-exec

- **Decided**: 공유 tenant-picker 2계층(L1 canonical inline bash + L2 AskUserQuestion prose). **Decision 2C** — 인라인 bash 상수 + bun-test fake-axhub PATH 스텁 골든테스트, **Rust axhub-helpers 무변경**. 캐시 `.axhub/state/tenant.json` `{tenant,source,ts}` + TTL(8h, `AXHUB_TENANT_CACHE_TTL_SECS`), **session_id 없음(VERIFIED 부재)**. 매 블록 캐시 re-read(env 펜스간 휘발). Smart trigger(멤버십≥2 & 미선택 & TTY). fallback 경고는 **L1 bash echo**(non-TTY는 L2 미실행). migrate **포함**(option b: contract test:50을 --tenant 기대로 갱신).
- **Rejected**: 2A(Rust 헬퍼 서브명령) — 5-binary 릴리스 결합 + version-skew fail-wrong, "skill만" 의도 충돌. 1B byte-identical 전체 강제 — 동적 후보 불가. session_id 키 캐시 — env 부재로 구현 불가.
- **Risks**: AC2 per-call `--tenant` threading은 contract test가 라인단위 regex로 강제(toContain 금지). migrate 잠금 라인(Windows parity/CLI boundary) 무회귀 필수. L2 sentinel collision lint 회피.
- **Files**: `.omc/plans/tenant-picker-consensus.md`(consensus plan, Critic APPROVED), `.omc/specs/deep-interview-tenant-picker.md`(spec).
- **Remaining(team-exec)**: Phase A = A1(block 상수) → A2(bun fixture) / A3(scaffold) / A4(doctor+manifest) → A5(init+deploy) / A8(migrate) → A6(registry+gitignore) / A7(contract test). 의존성은 task blockedBy에 인코딩. 공통 게이트: skill:doctor --strict, lint:tone --strict(해요체), lint:keywords --check, tsc --noEmit, bun test(≥498).
