# Quickstart: `infer-tables-env`

## 사용자 흐름 (end-to-end)

1. **분석 요청 또는 자동 넛지**
   - 명시: "내 코드 분석해서 필요한 테이블이랑 환경변수 추천해줘"
   - 자동(US3): `init`/`deploy` 흐름 끝에 "필요한 테이블·환경변수 추천해드릴까요?" 경량 넛지 → 수락 시 분석.
2. **분석(read-only)** — preflight 로 인증/대상 앱 확인 → declarative 아티팩트(schema.prisma, 마이그레이션, .env.example) 우선 분석 → `axhub tables list`/`axhub env list` cross-check.
3. **추천 표 표시** — 테이블(컬럼·타입·제약·근거·상태) + 환경변수(키·시크릿?·기본값·근거·상태) + 커버리지 한 줄. 시크릿 값 비노출.
4. **승인 분기(AskUserQuestion)** — `추천만 볼게요` / `적용할게요`. (비대화 subprocess 면 D1 guard 로 안전 기본값 `추천만`.)
5. **적용(승인 시)** — 최종 미리보기(마스킹) + 명시 확인 → 테이블 자동 생성 + env 키 등록(값은 stdin 입력/건너뜀) → 항목별 결과 보고. 멱등.

## 빌드·생성 (개발자 — 이 SKILL 만들기)

```bash
# 1) 스캐폴드 (직접 작성 금지 — Phase 17/18 패턴 자동 삽입)
bun run skill:new infer-tables-env --model sonnet
#   frontmatter: multi-step: true, needs-preflight: true, model: sonnet

# 2) SKILL.md 채우기
#    - description: 분석형 트리거 어구만 (CRUD 어구 금지 — tables/env 소유)
#    - Workflow: preflight → 분석 → 추천 표 → AskUserQuestion 분기 → 적용 위임
#    - NEVER: 시크릿 값/리터럴 비노출, 승인 없이 mutate 금지

# 3) AskUserQuestion safe_default 등록
#    tests/fixtures/ask-defaults/registry.json 에 분석/적용 분기 채널 추가 (기본 '추천만')
```

## 검증 (Self-Check)

```bash
bun run skill:doctor --strict     # D1/TodoWrite/in-body preflight/step-collision
bun run lint:tone --strict        # 해요체 0 err
bun run lint:keywords --check      # nl-lexicon (새 SKILL → 베이스라인 재캡처 필요)
bun test                          # ux-* 패턴 회귀 + ask-fallback-registry
bunx tsc --noEmit                 # clean
```

## 수용 시나리오 → fixture 매핑 (SC-001/002 평가)

| Fixture | 기대 |
|---|---|
| `tests/fixtures/infer/nextjs-prisma/` | schema.prisma → 테이블·컬럼·제약 추론, `.env.example` → env 키. recall ≥90% |
| `tests/fixtures/infer/fastapi-sqlmodel/` | 마이그레이션/Settings → 추론. 코드-only 모델은 "검토 필요" |
| (시크릿 하드코딩 샘플) | 보안 발견 플래그, 값 비노출(SC-005) |
| (이미 설정된 앱) | 재적용 시 0 변경(멱등, SC-007) |

> recall/precision 은 fixture 골든 비교로 평가(결정론 유닛테스트 아님 — research D1 트레이드오프).

## 경계 (안 하는 것)

- env *값* 추론·생성 안 함(FR-016).
- 추천 결과 파일 저장 안 함(휘발성, FR-015).
- 승인 없이 mutate 안 함(FR-006).
- 외부 커넥터 데이터 소스를 새 테이블로 추천 안 함(스펙 Edge).
- 새 Rust 바이너리·새 mutation CLI 경로 안 만듦(기존 tables/env 위임).
