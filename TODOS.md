# TODOS

리뷰에서 의도적으로 유보(Defer)한 작업 목록이에요. 각 항목은 착수자가 3개월 뒤에 읽어도 맥락이 복원되도록 적어요.

## E1 — AP-14 SessionStart 폴백 다이어트 (P2, M)

- **What:** hooks.json 4번째 SessionStart entry(update-first Code-mode 폴백, ~3.5KB 영어 계약 블록)를 2–3문장 포인터로 축소해요.
- **Why:** 이 폴백은 매 세션·매 `/clear`·매 compaction 마다 무조건 주입되고, UserPromptSubmit match 가 뜨는 턴엔 같은 내용이 2벌 들어가요 — 모든 사용자 세션의 고정 토큰 비용이에요. 계약 본문은 `skills/update/SKILL.md` 가 이미 소유하고 있어 훅은 라우팅 포인터만으로 충분할 가능성이 커요.
- **선행 조건 (이것부터):** Claude Desktop Code 모드에서 ① UserPromptSubmit 훅이 실제로 발화하는지, ② SessionStart 훅 발화 여부와 auto-update prompt 의 mkdir/touch/marker 명령이 권한 카드로 노출되는지 재검증해요. 이 폴백 텍스트는 Desktop eval 실패를 막으려고 자란 이력(1.10.21~1.10.27 패치 사이클)이 있어서, 검증 없이 줄이면 라우팅 회귀 위험이 있어요.
- **부분 대안:** SessionStart 입력의 `source` 필드(startup/resume/clear/compact)로 게이트해 `/clear`·compaction 재주입만 먼저 줄일 수 있어요 (텍스트 무변경).
- **Cons:** 축소 후 Desktop 라우팅 지표가 나빠지면 되돌려야 해요 — 축소 전후 대표 여정 수동 확인 필수.
- **연관 파일:** `hooks/hooks.json`(SessionStart 4번째 entry), `skills/update/SKILL.md`, `tests/smooth-behavior.test.ts:798-812`, `tests/update-desktop-ux-contract.test.ts`
- **Depends on:** 없음 (독립 PR 권장 — 2026-07-13 auto-update hardening 리뷰에서 Defer 확정)

## E6 — 라우터 디버그 스위치 `AXHUB_HOOK_DEBUG` (P3, S)

- **What:** `AXHUB_HOOK_DEBUG=1` 일 때 각 UserPromptSubmit 라우터가 "어느 게이트에서 멈췄는지/어느 규칙에 매치됐는지" 한 줄을 stderr 로 남겨요.
- **Why:** 훅은 suppressOutput 이라 사용자 오탐 제보("엉뚱한 말에 update 가 떠요")를 재현 없이 진단할 수단이 없어요. 스위치가 있으면 제보자에게 env 하나 켜고 재현해 달라는 질문 한 번으로 끝나요.
- **현재 대안:** `tests/hook-execution.test.ts` 의 합성 payload 재현 + 배포 전 tee 캡처 절차 (플랜 T8) — 실제 제보가 발생하기 전까지는 이걸로 충분하다고 판단했어요 (2026-07-13 eng review D8).
- **Cons:** 버그픽스 PR 범위 밖 기능 추가 + stderr 출력 규약(훅 출력 비노출 원칙)과의 정합 검토 필요.
- **연관 파일:** `hooks/update-router.sh` (clarity/import/status-resume 라우터는 diet 복귀로 제거됨 — frontmatter 라우팅이 담당)
- **Depends on:** 실제 오탐 제보 발생 (그 전에는 착수하지 않아요)
