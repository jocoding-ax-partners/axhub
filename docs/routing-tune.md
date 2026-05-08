# routing:tune

`bun run routing:tune` 은 corpus drift 와 clarify feedback 을 SKILL description/examples 개선 후보로 바꾸는 운영 도구예요.

## 기본 흐름

1. `tests/corpus.100.jsonl` 과 `tests/baseline-results.docs-only.100.json`, `tests/baseline-results.claude-native.100.json` 을 읽어요.
2. expected skill 과 fired skill 이 어긋난 row, 또는 docs-only 와 claude-native 가 다른 row 를 failing case 로 모아요.
3. 각 failing case 를 expected skill 기준으로 묶고 suggestion JSON 을 출력해요.
4. maintainer 가 suggestion 을 검토한 뒤 SKILL.md description/examples 를 업데이트해요.

## 명령

```bash
bun run routing:tune -- --dry-run
bun run routing:tune -- --skill deploy --dry-run
bun run routing:tune -- --online --dry-run
bun run routing:tune -- --confused
```

기본 `--dry-run` 은 파일을 바꾸지 않고 `ANTHROPIC_API_KEY` 가 있어도 deterministic offline suggestion 만 써요. Claude suggestion 이 필요할 때만 `--online` 또는 `--llm` 을 붙여요. `--apply` 는 실제 suggestion 생성을 위해 `ANTHROPIC_API_KEY` 가 필요하고, 그래도 SKILL.md 반영과 PR 생성은 manual review 로 남겨요.

## confused mode

```bash
bun run routing:tune -- --confused
```

이 모드는 `bin/axhub-helpers routing-stats --confused --json` 을 읽어요. audit log 는 prompt 원문을 저장하지 않으므로 output 은 `hash`, `chosen_skill`, `count`, `latest_ts` 중심이에요. 원문은 사용자 manual review 로 확인한 뒤 `--skill <chosen_skill>` dry-run 을 이어서 실행해요.

## Privacy contract

- prompt 원문 저장 X
- 기본 dry-run 외부 전송 X
- 외부 suggestion call 은 `--online`, `--llm`, `--apply` 에서만 가능해요
- hash + chosen_skill 만 feedback loop 입력으로 사용해요

## Prompt template

LLM suggestion prompt 는 `prompts/routing-tune-suggestion.md` 를 기준으로 유지해요. script inline prompt 와 template 이 drift 나면 template 을 먼저 갱신하고 script 를 맞춰요.
