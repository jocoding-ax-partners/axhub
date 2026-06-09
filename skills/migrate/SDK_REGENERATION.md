# Regenerating the SDK knowledge packs

The packs (`sdk-knowledge/<lang>.md`), the pin (`sdk-knowledge/PINNED_SDK.lock.json`
/ `PINNED_SDK.json`), and the `agents/axhub-sdk-<lang>-expert.md` agents are
**generated** by a distiller in the separate SDK repo. Do not hand-edit them —
regenerate. This runbook is the process for when an AxHub SDK release bumps.

## What lives where

- **Distiller (source of truth):** `<sdk-repo>/scripts/gen-sdk-distill.py`
- **Deliberate pin (human-bumped):** `<sdk-repo>/PINNED_SDK.lock.json` (source) and
  the vendored copy `skills/migrate/sdk-knowledge/PINNED_SDK.lock.json`
- **Generated, vendored into this plugin:** `skills/migrate/sdk-knowledge/<lang>.md`
  (6), `skills/migrate/sdk-knowledge/PINNED_SDK.json`, `agents/axhub-sdk-<lang>-expert.md` (6)
- SDK repo has **no git remote** — it is a dev-machine workspace, so regeneration is
  not reproducible in CI. The pack-integrity test (below) is what CI enforces.

## Steps

1. **Check out the target SDK release.** In the SDK workspace, update each
   `axhub-sdk-<lang>` sub-repo to the new release commit.

2. **Bump the deliberate pin.** Edit `PINNED_SDK.lock.json` in BOTH the SDK repo and
   the vendored copy: set each language's `source_sha` (the sub-repo HEAD) and
   `sdk_version`. The distiller refuses to generate against an unpinned checkout, so
   this is a conscious step, not automatic.

3. **Regenerate.** In the SDK repo: `python3 scripts/gen-sdk-distill.py`
   - It asserts each `git_sha(axhub-sdk-<lang>) == lock[lang].source_sha` and fails
     loud on a mismatch ("bump the lock deliberately, or checkout the pinned ref").
   - It stamps the lock sha into the packs + `PINNED_SDK.json`, emits §1 (client
     wrapper) / §6 (data ops) / agents into `dist/`, and is deterministic.

4. **Vendor.** Copy into this plugin:
   - `dist/sdk-knowledge/*.md` → `skills/migrate/sdk-knowledge/`
   - `dist/sdk-knowledge/PINNED_SDK.json` → `skills/migrate/sdk-knowledge/`
   - `dist/agents/*.md` → `agents/`

5. **Verify (the gates CI runs).**
   - `cargo test -p axhub-helpers` — `pack_client_init_equals_render_wrapper_preview_full_body`
     (pack §1 == the helper's `render_wrapper_preview`) and `migrate_sdk_installed_cli`.
   - `bun test tests/sdk-knowledge-pack.test.ts` — pack-integrity (pack `source_sha` ==
     the committed lock; §6 surface present) and `tests/migrate-skill-contract.test.ts`
     (expert dispatch + data mode + auth advisory).
   - `bun run skill:doctor --strict`, `lint:tone --strict`, `lint:keywords --check`.

## What the gates catch (and don't)

- **Catch:** a missing / hand-edited / sha-tampered pack, a pack diverging from the
  lock, a §1 client wrapper that drifted from the helper, a missing §6 surface.
- **Don't catch (known limitation, item 1B):** a regeneration skipped *entirely* —
  the lock and the packs then stay stale together. Detecting that needs a remote + CI
  on the SDK repo so the live SDK HEAD can be compared. Deferred.
