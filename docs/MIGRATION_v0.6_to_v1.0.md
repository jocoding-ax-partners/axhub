# axhub v0.6 → v1.0 Migration Guide

## 변경 사항

### 신규

- 5 quality SKILL: axhub-review, axhub-debug, axhub-ship, axhub-tdd, axhub-plan
- 8 agent md: 3 quality (`axhub-reviewer`, `axhub-debugger`, `axhub-shipper`) + 5 migrate planning (`axhub-migrate-discoverer`, `axhub-migrate-planner`, `axhub-migrate-architect`, `axhub-migrate-critic`, `axhub-migrate-reviewer`)
- using-axhub-quality megaskill
- karpathy-guidelines UserPromptSubmit reminder
- `.axhub-state/quality.json` per-repo state
- post-commit hook promotion
- commit / push review gate
- `AXHUB_DISABLE_*` opt-out 환경변수

### 변경 없음

- 기존 deployment SKILL 은 그대로예요.
- Korean NL routing 은 그대로예요.
- classify-exit Korean 해요체 매핑은 그대로예요.
- cosign release pipeline 은 그대로예요.
- hook safety fail-open contract 도 그대로예요.

## 첫 SessionStart 영향

1. post-commit hook 감지 시 선택지를 보여줘요.
2. `.gitignore` 에 `.axhub-state/` 를 한 번 추가해요.
3. 첫 SessionStart 에 quality auto-mode consent 를 물어봐요.
4. 켜면 megaskill 과 karpathy reminder 가 시작돼요.

## Opt-out

```bash
export AXHUB_DISABLE_TRIGGERS=1
export AXHUB_DISABLE_MEGASKILL=1
export AXHUB_DISABLE_KARPATHY=1
export AXHUB_DISABLE_POSTCOMMIT=1
```

## Breaking Changes

없어요. v1.0 은 additive only 예요.
