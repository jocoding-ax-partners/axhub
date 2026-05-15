# axhub v1.0 Product Contract

## "발화 없이 자동" 의 정확한 의미

axhub v1.0 quality auto-mode 는 **best-effort next-turn reminder** 예요.

- user 가 "리뷰해줘" 같은 발화를 안 쳐도 review SKILL 호출 가능해요.
- Edit / Write 같은 코드 행위가 state 를 누적하고 다음 응답에 reminder 로 들어가요.
- commit / push 는 hard gate 로 물어봐요.
- 코드 행위 직후 즉시 invoke 가 아니라 다음 user 응답 처리 시점이에요.
- model 이 megaskill directive 를 따라야 하므로 100% 보장 장치는 아니에요.
- v1.0 baseline 은 obedience rate 60% 이상을 목표로 해요.

## Hard Gate vs Soft Trigger

| 상황 | Mechanism | 보장 수준 |
| --- | --- | --- |
| commit review missing | PreToolUse permissionDecision ask | hard gate |
| push review missing | PreToolUse permissionDecision ask | hard gate |
| 50+ lines edit 후 다음 turn | megaskill directive → axhub-review 권장 | best-effort |
| test fail 후 다음 turn | megaskill directive → axhub-debug 권장 | best-effort |
| test ratio low | megaskill directive → axhub-tdd 권장 | best-effort |
| arch change | megaskill directive → axhub-plan 권장 | best-effort |

## v1.1 Roadmap

best-effort 에서 hard gate 강화로 이동할지 production telemetry 기반으로 결정해요.
