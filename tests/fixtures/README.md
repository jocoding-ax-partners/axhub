# Fixtures — frozen parseAxhubCommand contract

40 hand-curated `parseAxhubCommand` cases, one .json file each. Loaded by `tests/fixtures.test.ts` to detect any unintentional change in parser semantics.

## Curation methodology

Each fixture pins the EXPECTED parser output for one specific input. The fixtures collectively cover the 6 axes the parser must handle correctly:

| Category | Count | What it pins |
|---|---|---|
| `destructive-*` | 10 | All destructive command shapes (deploy_create with various flag styles, update_apply, auth_login) |
| `ro-*` | 10 | Read-only commands that MUST NOT trigger consent gate (apps/apis list, deploy status/logs, auth status) |
| `adv-*` | 8 | Adversarial wrappers that MUST still be detected (env-prefix, $(), eval, &&, ;, |, bash -c) |
| `uni-*` | 4 | Unicode edge cases (Cyrillic homoglyph, ZWJ, full-width digit, NBSP) |
| `prf-*` | 4 | Profile/headless boundary (AXHUB_PROFILE env, --version, --help, token-paste) |
| `neg-*` | 4 | False-positive checks (echo strings, comments, similarly-named tools) |

## Schema

```json
{
  "description": "What this fixture pins",
  "input": { "command": "axhub deploy create --app paydrop ..." },
  "expected": {
    "is_destructive": true,
    "action": "deploy_create",
    "app_id": "paydrop",
    "branch": "main",
    "commit_sha": "abc123"
  }
}
```

`expected.action`, `app_id`, `branch`, `commit_sha`, `profile` are optional. For adversarial and unicode wrappers, only `is_destructive` + `action` are pinned (the parser may correctly skip per-field extraction inside wrappers — that's defense-in-depth, not a regression).

## Update protocol (frozen-by-design)

These files are **frozen**. Changing one means we are intentionally changing parser semantics. The protocol:

1. **Discovered a real bug?** Edit the .json + run `bun test tests/fixtures.test.ts` → see new failure → fix parser → tests pass again. Commit both.
2. **Intentional refactor?** Update `_curated.ts` source-of-truth + run `bun tests/fixtures/_curated.ts` to regenerate .json files + commit both.
3. **NEVER** edit a .json without updating `_curated.ts` (drift). NEVER regenerate without inspecting the diff.

## Adding a fixture

1. Append to `_curated.ts` `FIXTURES` array (keep category counts balanced — update the totals header if shifting).
2. Run `bun tests/fixtures/_curated.ts` to generate the new .json.
3. Run `bun test tests/fixtures.test.ts` to confirm the new fixture passes.
4. Update the count assertion in `tests/fixtures.test.ts` (and the count in this README).
5. Commit `_curated.ts` + new `.json` + the test update together.

## Why .json files instead of inline literals

- **Diff visibility**: a single fixture change is one .json file diff, not a hunk inside a 1000-line ts.
- **Frozen intent signal**: editing a .json is a louder signal in PR review than editing a literal in a TS file.
- **Reproducible from source**: `_curated.ts` is the regenerable index; .json files are the contract.
- **External tooling**: future fixture validators (e.g., a parser-fuzzer that uses these as ground truth) can consume .json directly without a TS toolchain.
