# 세션 맥락 핸드오프 계약 (session carry-over)

`init` 과 `deploy` 가 **같은 대화** 안의 조회·온보딩 맥락을 이어받을 때 따르는 공유 계약이에요. 두 스킬이 같은 규칙을 쓰도록 여기 한 곳에만 적어요. 별도 state 파일·CLI 호출·마커 없이, LLM 이 자기 대화 컨텍스트만 봐요.

## 감지 휴리스틱 — "이 대화에서 했나"

다음 **구체 근거**가 지금 대화 컨텍스트에 실제로 보일 때만 "했다" 로 판정해요:

- **리소스 조회:** `connector_query` / `connector_resources` / `row_list` / `table_list` 같은 도구의 실제 결과가 이 대화에 있음.
- **온보딩 완료:** 이 대화에 온보딩 Ready card(`VIBE_READY`) 출력이 있음.

근거가 안 보이면 "안 했다" 로 보고 콜드(평소) 동작으로 가요. 짐작으로 판정하지 않아요.

## Confabulation 가드 (핵심)

조회한 리소스를 이어받는 건 위 구체 근거가 있을 때만이에요. 근거가 없으면:

- 리소스·테이블·connector 이름을 지어내지 않아요.
- 데이터 추천은 코드 기준으로만 하고, "방금 본 데이터" 같은 carry-over 주장을 하지 않아요.

즉 **조회한 적 없으면 carry-over 침묵**이 기본이에요. 본 것만 이어받아요.

## 마찰 억제 범위

같은 대화에서 온보딩이 이미 끝났으면 중복 안내를 줄여요. 단 줄이는 건 **재설명·재안내뿐**이에요:

- ✅ 줄여도 됨: 이 대화에서 이미 보여준 install-link 재안내, 셋업 다시 설명.
- ❌ 절대 우회 금지: auth 판정(`preflight`), GitHub 설치 판정(`accounts list`), owner-pick(2+ 설치 시 어느 계정에 repo 만들지), 0-install gate. 이건 맥락과 무관하게 항상 그대로 실행해요.

마커는 마찰만 줄여요. correctness gate 를 대신 통과시키지 않아요.

## D1 헤드리스 가드

조건부 ack·마찰 억제는 대화형에서만 해요. subprocess(`claude -p` / CI / `$CLAUDE_NON_INTERACTIVE` / no TTY)면 평소 동작으로 가고, ack 도 생략해요.

## compaction 한계

대화가 길어 컨텍스트가 줄거나(compaction) 새 대화로 넘어오면 근거가 안 보여요. 그땐 콜드 동작이 정상이에요 — 대화를 넘는 끊김 없는 핸드오프는 Phase 2(durable state)에서 다뤄요.
