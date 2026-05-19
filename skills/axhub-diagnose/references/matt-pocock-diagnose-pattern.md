# Matt Pocock diagnose pattern → axhub 5-Phase loop mapping

Source: https://github.com/mattpocock/skills/blob/main/skills/engineering/diagnose/SKILL.md

## 핵심 원칙

> "If you have a fast, deterministic, agent-runnable pass/fail signal for the bug, you will find the cause."

성공의 80% 는 가설 세우기 전 **deterministic feedback loop** 구축. 카탈로그는 부수적.

## 원본 6 Phase → axhub 5 Phase 매핑

| 원본 | axhub | 목적 |
|---|---|---|
| 1. Build feedback loop | **Phase 1L** | event_log + recovery_scan + HITL fallback 으로 fail/pass boolean 생성 |
| 2. Reproduce | (Phase 2 gate 내부) | symptom confirmed 후 가설 단계 진입 |
| 3. Hypothesize | **Phase 2R** | 3-5 ranked falsifiable hypothesis + If-X-then-Y |
| 4. Instrument | **Phase 3I** | Probe trait 로 single-variable change + boundary guard |
| 5. Fix + regression test | **Phase 4F** | LOOP_VERIFY 가 자동 regression test |
| 6. Cleanup + post-mortem | **Phase 5P** | probe manifest 기반 cleanup + learning emit + recurrence detect |

axhub 는 5 단계로 압축 — Matt 의 5+6 (fix + cleanup) 을 LOOP_VERIFY 가 묶어요.

## HITL fallback

원본: `hitl-loop.template.sh` bash script.
axhub: `axhub-helpers diagnose hitl` Rust subcommand (단일 codepath, byte-identical cross-platform, base64 escape 우회 불필요).

## v0.8.0 차이점

- v0.8.0 = deploy + test 2 tool 만 ship (npm v0.8.1, 나머지 v0.9)
- LLM-augmented hypothesis v0.8.1 (비용 정책 결정 후)
- CodeInjectionProbe v0.8.1 (security path validation 보강 후)
