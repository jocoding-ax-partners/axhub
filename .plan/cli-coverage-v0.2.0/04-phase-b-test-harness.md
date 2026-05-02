# Phase B-test — E2E Harness 확장 + Lifecycle E2E

> codex ENG finding E6 mitigation. 현재 E2E harness 는 single-step 만 지원. lifecycle 6-step chain (init → apps create → github connect → env set → deploy → open) 검증 불가. 확장 필요.

---

## 현재 harness 한계 (codex evidence)

- `tests/e2e/claude-cli/run-matrix.sh:94-127` = case 별 독립 실행
- `tests/e2e/claude-cli/lib/spawn.sh:68-69` = sandbox delete + recreate per invocation
- `spawn.sh:153-164` = `--no-session-persistence` 강제

→ 6-step chain 의 state carryover 안 됨. 각 step 가 fresh sandbox.

## 확장 작업

### BT-1. `run-matrix.sh` multi-step opt-in 플래그

- 파일: `tests/e2e/claude-cli/run-matrix.sh`
- 변경: `--persist-session` flag 추가 시 sandbox delete 건너뜀
- 변경: case 파일 안에 `MULTI_STEP=1` env 선언 시 자동 persist
- 기존 single-step case = 영향 0 (default behavior 유지)

### BT-2. `spawn.sh` session-persistence opt-in

- 파일: `tests/e2e/claude-cli/lib/spawn.sh:68-69, 153-164`
- 변경:
  ```bash
  if [ "$PERSIST_SESSION" != "1" ]; then
    rm -rf "$SANDBOX_DIR"
    mkdir -p "$SANDBOX_DIR"
  fi
  
  if [ "$PERSIST_SESSION" = "1" ]; then
    EXTRA_ARGS=()  # session-persistence 활성
  else
    EXTRA_ARGS=("--no-session-persistence")
  fi
  ```

### BT-3. lifecycle E2E case 작성

- 파일: `tests/e2e/claude-cli/cases/t2-lifecycle.sh`
- 시나리오:
  ```
  Step 1: 빈 dir 에서 NL "Next.js 결제 앱 만들어줘"
          → init SKILL → mock-hub stack 선택 자동 (T2 fixture) → tarball mock → npm install mock
          → axhub.yaml 생성 검증
  Step 2: NL "axhub apps create 해줘"
          → apps create → mock-hub apps_create 200 응답 → app id 받음
  Step 3: NL "GitHub repo 연결해 / 내 paydrop-frontend repo"
          → github connect → mock-hub github_connect 200
  Step 4: NL "DATABASE_URL 환경변수 추가해 / postgres://..."
          → env set --from-stdin → mock-hub env_set 200 → value 가 argv 안 들어가는지 검증
  Step 5: NL "배포해"
          → deploy create → consent-mint → deploy 진행 → mock-hub deploy_create 200
  Step 6: NL "결과 봐"
          → open → axhub.yaml read → URL 출력 → 브라우저 호출 (mock)
  ```
- 각 step 후 sandbox 의 axhub.yaml / .git / node_modules / 기타 state 검증
- T2 tier (mutate) 안에 lifecycle 단일 case

### BT-4. Free-form preview policy ADR

- 파일: `docs/adr/0009-free-form-preview-policy.md`
- 내용 (codex ENG finding E7 응답):
  - registry test 의 `tests/ux-ask-fallback-registry.test.ts:37-40` 가 structured `"question"` JSON 만 추출
  - deploy SKILL 의 free-form text preview (`skills/deploy/SKILL.md:59-70`) = 명시적 미커버
  - 신규 11 destructive SKILL 모두 free-form preview 허용 (vibe coder UX 우선) + 개별 SKILL 책임 (registry 가 보장 X)
  - registry test = baseline lock 만, 실제 fallback 보장 = SKILL 작성자 책임
  - 향후 v0.3.0 에서 structured preview 의 standardization 검토

### BT-5. Cross-platform bootstrap smoke test (Phase A0-bootstrap 흡수)

Phase A0-bootstrap 의 cross-platform 테스트가 여기서 합류:
- fresh Mac VM smoke (manual, screenshot 첨부)
- fresh Ubuntu LTS smoke (CI runner)
- fresh Windows 11 smoke (CI runner)
- nvm/asdf detect 시 volta 충돌 처리 검증
- corporate proxy mock 환경 (HTTPS_PROXY=http://proxy:8080) 시 fallback

## Test count 추가 estimates

- BT-1 + BT-2 (harness 확장): test 자체 추가 X, infra 만
- BT-3 (lifecycle case): +1 E2E t2 case
- BT-4 (ADR): test X (docs only)
- BT-5 (cross-platform smoke): +15 cases (5 binary × 3 OS)

총 신규 test = 16 (lifecycle 1 + cross-platform 15)

## Validation gate before Phase C

- [ ] lifecycle E2E case PASS (mock-hub fixture, single 직렬 6 step)
- [ ] cross-platform smoke 15 PASS (CI matrix)
- [ ] free-form preview ADR commit
- [ ] 기존 single-step E2E 모두 영향 없음 (regression test)

## Effort

- BT-1 + BT-2: ~1시간 (harness 변경, 정형 패턴)
- BT-3: ~1.5시간 (mock-hub fixture 설계 + state 검증)
- BT-4: ~30분 (ADR write only)
- BT-5: ~2시간 (CI matrix + smoke script)
- **Total: ~5시간**
