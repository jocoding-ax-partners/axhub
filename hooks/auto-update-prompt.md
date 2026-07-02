# axhub 자동 업데이트 (내부 — 훅 트리거)

SessionStart 훅이 24시간에 한 번 이 지침을 부르면, axhub CLI 와 플러그인의 새 버전을 확인하고 적용해요. 사용자가 직접 부르는 skill 이 아니에요.

**핵심 원칙:** best-effort·비차단이에요. 실패·구 CLI·네트워크 오류면 조용히 건너뛰고, 사용자의 실제 요청을 절대 막지 않아요. 업데이트 안내는 짧게 한 줄로 끝내고 원래 작업을 이어가요.

---

## 0. 사전 점검 (네트워크 0)

1. `command -v axhub` 가 실패하면 즉시 멈춰요 — CLI 가 없는 건 onboarding 소관이에요.
2. `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 의 `version` 을 읽어 `<PLUGIN_VERSION>` 으로 둬요 (못 읽으면 plugin 확인은 생략하고 CLI 만 봐요).
3. 환경변수 `AXHUB_NO_AUTO_UPDATE` 가 설정돼 있으면 **안내만** 모드예요 — 아래 자동 적용(2단계 apply)을 건너뛰고, 새 버전이 있을 때 한 줄 안내만 해요.

---

## 1. 버전 확인 (네트워크 1회)

1. 실행해요:

   ```bash
   axhub update check --plugin-version <PLUGIN_VERSION> --json
   ```

2. 결과와 무관하게 재확인 주기 캐시를 바로 갱신해요 (24h throttle 의 기준점):

   ```bash
   mkdir -p "$HOME/.axhub/cache" && : > "$HOME/.axhub/cache/.plugin-update-check"
   ```

3. 출력 JSON 을 읽어요:
   - CLI: `{ current, latest, has_update, disabled }`
   - (있으면) 플러그인: `plugin: { current, latest, has_update }`
4. 호출이 실패하거나 JSON 이 비면 (구 CLI·네트워크 실패) 조용히 멈춰요 — 작업을 막지 않아요.

---

## 2. CLI 업데이트

분기로 처리해요:

- **`disabled == true`** (패키지 매니저가 관리하는 설치) **또는 `AXHUB_NO_AUTO_UPDATE` 설정** → 자동 적용하지 않아요. `has_update` 면 한 줄만 안내해요:
  > `axhub 새 버전(<latest>)이 있어요. axhub update apply 로 받을 수 있어요.`
- **`has_update == false`** → 아무것도 보여주지 않고 조용히 통과해요.
- **`has_update == true` 이고 적용 가능** → 사용자에게 해요체로 알리고 바로 적용해요 (auto):
  1. 안내 한 줄: `axhub 새 버전(<current> → <latest>)이 나왔어요. 지금 업데이트할게요…`
  2. 실행: `axhub update apply --execute --yes`
  3. 끝나면 `axhub --version` 으로 재확인하고 한 줄: `axhub <새 버전> 으로 업데이트됐어요.`
  4. 적용이 실패하면 (권한·네트워크 등) raw 에러는 숨기고 한 줄만 안내한 뒤 비차단으로 계속해요:
     > `자동 업데이트가 안 됐어요. axhub update apply 를 직접 한 번 실행해 주세요.`

---

## 3. 플러그인 업데이트 (`claude plugin update` — 자동 적용, 재시작 후 반영)

- `plugin` 블록이 없거나 **`plugin.has_update == false`** → 생략해요.
- **`command -v claude` 실패** (Claude Code CLI 없음) → 한 줄 안내만: `axhub 플러그인 새 버전(<plugin.latest>)이 있어요. Claude Code 에서 업데이트해 주세요.`
- **`AXHUB_NO_AUTO_UPDATE` 설정** → 적용하지 않고 한 줄 안내만: `axhub 플러그인 새 버전(<plugin.latest>)이 있어요. claude plugin update axhub@axhub 로 받을 수 있어요.`
- **`plugin.has_update == true` 이고 적용 가능** → 자동 적용해요:
  1. 설치 scope 를 먼저 확인해요 — `claude plugin list` 출력에서 `axhub@axhub` 항목의 `Scope:` 값(user/project/local/managed)을 읽어 `<SCOPE>` 로 둬요. 못 찾으면 `user` 로 둬요.
  2. 안내 한 줄: `axhub 플러그인 새 버전(<plugin.current> → <plugin.latest>)이 나왔어요. 지금 받을게요…`
  3. 실행: `claude plugin update axhub@axhub --scope <SCOPE>`
  4. **재시작 안내(필수 — plugin 업데이트는 재시작해야 적용돼요):** `받았어요. Claude Code 를 재시작하면 새 버전이 적용돼요.`
  5. 실패하면 raw 에러는 숨기고 한 줄만 안내한 뒤 비차단으로 계속해요: `플러그인 자동 업데이트가 안 됐어요. claude plugin update axhub@axhub --scope <SCOPE> 를 직접 실행해 주세요.`

---

## 가시성·안전 규칙

- raw JSON·명령 출력·내부 값은 chat 에 echo 하지 않고, 위의 한국어 한 줄들만 보여줘요.
- 전 과정 비차단이에요 — 업데이트가 사용자의 실제 요청보다 우선하지 않아요. 사용자가 이미 다른 일을 시키는 중이면 안내만 한 줄 남기고 원래 작업을 이어가요.
- 이 흐름은 SessionStart 당 최대 1회(24시간 throttle)만 돌아요. 완전히 끄려면 `AXHUB_NO_AUTO_UPDATE=1` 을 설정하면 돼요.
