# TODOS

리뷰에서 의도적으로 유보(Defer)한 작업 목록이에요. 각 항목은 착수자가 3개월 뒤에 읽어도 맥락이 복원되도록 적어요.

## X6 — 설치 후 `claude plugin validate` 자동 실행 (P3, S)

- **What:** plugins skill §3 Host install 이 `--host claude` 설치에 성공(`status=installed`)하면, 재시작 안내 전에 설치된 플러그인에 `claude plugin validate` 를 실행해 결과를 보여 줘요.
- **Why:** 지금 설치 성공은 CLI 검증(size·SHA-256·manifest mismatch 등)까지만 보장해요. host 가 manifest·SKILL frontmatter 를 읽지 못하면 재시작 뒤에야 플러그인이 안 보이는 것으로 드러나요. 이 레포도 v0.2.1 의 SKILL frontmatter YAML 오류를 `claude plugin validate` 로 잡은 이력이 있어요(CHANGELOG).
- **Cons:** `claude` CLI 가 PATH 에 없거나 낮은 버전이면 건너뛰는 분기가 필요하고, Codex host 의 같은 검증 수단은 따로 확인해야 해요. 검증 실패를 설치 실패(되돌리기)로 볼지 경고로 볼지 정해야 해요.
- **연관 파일:** `skills/plugins/SKILL.md`(§3 Host install), `tests/plugins-skill-contract.test.ts`. CLI 가 맡는다면 `clients/cli/axhub/src/commands/plugin/install.rs`
- **Depends on:** 없음 (2026-10-23 플러그인 마켓플레이스 결함 수정 계획 `docs/plugin-marketplace/plan.md` 에서 범위 밖 개선으로 Defer)
