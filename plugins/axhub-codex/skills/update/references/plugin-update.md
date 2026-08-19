# 플러그인 업데이트 reference (codex)

`codex plugin list --json` 이 성공했을 때만 읽어요. 플러그인 업데이트를 직접 끝내는 세부 계약이에요.

## 설치 판정

`codex plugin list --json` 출력의 `installed` 배열에서 `pluginId` 가 `axhub-codex@axhub` 인 항목을 찾아요. `version` 을 `<PLUGIN_VERSION>` 으로, `marketplaceSource.sourceType`(`git` 또는 `local`)을 `<SOURCE_TYPE>` 으로 둬요. `available` 배열은 판정에 쓰지 않아요 — 빈 배열로 나와요. 항목이 없으면 이 단계를 생략하고 CLI 결과만으로 결과 카드를 써요.

현재 `<PLUGIN_VERSION>` 이 CLI 응답의 플러그인 최신 버전 이상이면 실행하지 않고 `axhub 플러그인은 이미 최신이에요 (v<PLUGIN_VERSION>).` 로 닫아요.

## Direct update

- CLI 응답에 plugin block 이 없으면 구 CLI이므로 이 단계를 생략해요.
- `AXHUB_NO_AUTO_UPDATE` 면 적용하지 않고 `axhub 플러그인 새 버전(v<latest>)이 있어요. AXHUB_NO_AUTO_UPDATE 설정이라 자동 적용은 안 해요 — 안내된 명령으로 직접 받거나 플래그를 끄면 돼요.`라고 안내해요.
- 업데이트가 필요하고 적용 가능하면 아래 순서로 처리해요.

1. `axhub 플러그인 새 버전(v<current> → v<latest>)이 나왔어요. 지금 받을게요…`라고 말해요.
2. `<SOURCE_TYPE>` 으로 적용 명령을 갈라요 (둘 다 셸 도구로 직접 실행하는 비대화형 명령이에요):
   - **`git`** → 정확히 `codex plugin marketplace upgrade axhub` 를 실행해요. marketplace snapshot 이 갱신되면서 설치된 플러그인이 함께 최신으로 재설치돼요.
   - **`local`** → 정확히 `codex plugin add axhub-codex@axhub` 를 다시 실행해요. 로컬 marketplace 는 `upgrade` 대상이 아니라서 재`add` 가 갱신 경로예요 (in-place 갱신, 멱등).
3. 성공하면 `codex plugin list --json` 을 한 번 더 실행해 `installed` 의 `axhub-codex@axhub` 버전을 확정해요. 확인이 안 되면 CLI 응답의 latest 를 써요.
4. 정확히 `받았어요. Codex 를 재시작하면 새 버전이 적용돼요.` 라고 말해요.
5. 실패하면 raw 에러를 숨기고 안내해요 — `git` 이면 `플러그인 자동 업데이트가 안 됐어요. codex plugin marketplace upgrade axhub 를 직접 실행해 주세요.`, `local` 이면 `플러그인 자동 업데이트가 안 됐어요. codex plugin add axhub-codex@axhub 를 직접 실행해 주세요.`

`codex plugin list --json` 성공 뒤 `대화형이라 직접 실행할 수 없다`고 답하거나 사용자에게 명령 실행을 떠넘기는 것은 명령 실행 전 dead-end 이므로 실패예요.
