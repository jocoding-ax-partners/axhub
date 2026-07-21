# Plugin Update Reference

`claude plugin list` 가 성공했을 때만 읽어요. Desktop 에서 플러그인 업데이트를 직접 끝내는 세부 계약이에요.

## 중복 설치 판정 알고리즘

`claude plugin list` 에 `axhub@axhub` 가 여러 번 나오면 모든 block 을 끝까지 읽고 `Status: ✔ enabled` 만 모아요. enabled 항목 중 **가장 높은 semver** 를 `<PLUGIN_VERSION>` 으로 삼아요. 같은 최고 버전이 여러 scope 에 있으면 현재 작업공간에 가까운 `local` → `project` → `user` 순서로 `<SCOPE>` 를 골라요. **그 최고 버전을 가진 block 들 안에서만** scope 를 선택해요.

낮은 버전 block 이 남아 있어도 그것은 cleanup 대상이 아니며 최신성 판정에서 무시해요. 현재 확인한 최고 enabled 버전이 CLI 응답의 플러그인 최신 버전 이상이면 업데이트 필요처럼 보여도 실행하지 않고 `axhub 플러그인은 이미 최신이에요 (v<PLUGIN_VERSION>).` 로 닫아요. `local 1.8.2` 와 `user 1.8.0` 이 함께 있으면 현재 버전은 `1.8.2` 이며 `user 1.8.0 → 1.8.2` 같은 정리성 업데이트나 결과 카드를 만들지 않아요. 낮은 버전이 함께 남아 있어도 사용자에게 중복 설치·scope 원문을 설명하지 않고, 최종 카드에는 선택된 최고 버전만 써요.

## Direct update

- plugin block 이 없으면 구 CLI이므로 이 단계를 생략해요.
- 플러그인 업데이트가 필요 없거나 최고 enabled 버전이 latest 이상이면 이때 낮은 중복 scope 가 있어도 `claude plugin update` 를 실행하지 않아요.
- `AXHUB_NO_AUTO_UPDATE` 면 적용하지 않고 `axhub 플러그인 새 버전(v<latest>)이 있어요. AXHUB_NO_AUTO_UPDATE 설정이라 자동 적용은 안 해요 — claude plugin update axhub@axhub 로 직접 받거나 플래그를 끄면 돼요.`라고 안내해요.
- 업데이트가 필요하고 적용 가능하면 아래 순서로 처리해요.

1. 사용자에게 `플러그인 설치 위치를 확인할게요.`라고 말하되 `Scope:` 원문은 보여주지 않아요. scope 를 못 찾으면 `user`로 둬요.
2. `axhub 플러그인 새 버전(v<current> → v<latest>)이 나왔어요. 지금 받을게요…`라고 말해요.
3. Desktop Bash tool 로 정확히 `claude plugin update axhub@axhub --scope <SCOPE>` 를 직접 실행해요. slash command 나 대화형 패널이 아니에요.
4. 성공하면 `claude plugin list` 를 한 번 더 실행해 enabled 최고 semver 를 확정해요. 확인된 받은 버전이 CLI 응답의 플러그인 최신 버전보다 높아도 최종 카드에는 확인된 받은 버전만 한국어 결과 줄로 써요. 확인이 안 되면 CLI 응답의 latest 를 써요.
5. 정확히 `받았어요. Claude Code 를 재시작하면 새 버전이 적용돼요.` 라고 말해요.
6. 실패하면 raw 에러를 숨기고 `플러그인 자동 업데이트가 안 됐어요. claude plugin update axhub@axhub --scope <SCOPE> 를 직접 실행해 주세요.`라고 안내해요.

`claude plugin list` 성공 뒤 `대화형 패널이라 직접 실행할 수 없다`고 답하거나 사용자를 `/plugin update`로 보내는 것은 명령 실행 전 dead-end 이므로 실패예요. `성공하면 claude plugin list 를 한 번 더 실행해` 받은 버전을 확인하고, 낮은 중복 항목을 나열하지 않아요.
