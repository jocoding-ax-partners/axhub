# TODOS

이 파일은 향후 작업 후보예요. 각 항목은 별도 PR 에서 다뤄요.

## P2 — STOP_WORDS layer 일관성 + Korean 형태소 분석

**Why:** `crates/axhub-helpers/src/resolve.rs:47-101` 의 `STOP_WORDS` HashSet 가 prompt-route layer 와 같은 단어 (`"배포"`, `"deploy"`) 를 정반대 의미로 다룸 (signal vs noise). 또한 한국어 조사 처리가 50개 손으로 고른 list 에 의존해서 `"checkout-v2를"` 같은 입력은 SLUG_RE (`^[a-z0-9][a-z0-9-]*$`) 매칭 fail 회귀.

**What:**
- `STOP_WORDS` HashSet 폐기
- `lindera-ko-dic` crate (~5MB Rust port) 도입해서 한국어 형태소 분석 → 조사 자동 제거
- 영문/외국어 stop word 처리는 명시적으로 분리 (관사/전치사 mini-list, language-tagged)
- slug NER 두 경로 (a) 따옴표 감싼 명시적 ID detection (b) `axhub apps` catalog fuzzy match (offline 캐시 + 5초 hook budget 안 짧은 fuzzy)

**Pros:**
- 같은 단어가 layer 마다 정반대 의미 갖는 drift 제거
- 일본어/중국어 사용자 silent fail (현재 SLUG_RE 미매칭 → None 반환) 부분 완화
- 한국어 조사 자동 처리 → 발화 변형 robust

**Cons:**
- `lindera-ko-dic` 추가 dependency. binary +5MB.
- `axhub apps` 카탈로그 fuzzy match 가 network 또는 캐시 필요 → offline-first 보장 약간 약화. 캐시 TTL 결정 필요.

**Context:** routing 큰 vision (Approach A — Hybrid embedding) 은 측정 phase 가 분기 결정. 이 항목은 그것과 *별개* 가치 — STOP_WORDS layer drift 자체로 즉시 가치.

**Effort:** human ~3d / CC+gstack ~1h
**Priority:** P2
**Depends on:** 없음 (이번 Skeptic fix PR 와 독립)
**Blocks:** Approach A (만약 채택 시 형태소 분석 prerequisite)
