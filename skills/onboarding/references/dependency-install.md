# Dependency Install Safety

Load this only for `first_gap=deps_missing`.

Onboarding may install project dependencies because frontmatter allows dependency execution, but only inside this safety box:

- manifest is present;
- lockfile is present;
- user explicitly confirms in an interactive session;
- selected package manager is derived from the lockfile, not guessed from preference;
- every install command includes `--ignore-scripts`;
- no install/update/auth/init/deps mutation runs in subprocess/headless mode.

## Prompt

```json
{
  "questions": [{
    "question": "의존성을 설치할까요?",
    "header": "의존성",
    "multiSelect": false,
    "options": [
      {"label": "설치", "description": "lockfile 기준으로 --ignore-scripts 를 붙여 설치해요"},
      {"label": "나중에", "description": "postinstall 자동 실행 없이 READY_WITH_USER_ACTION 으로 안내해요"}
    ]
  }]
}
```

If the user chooses later, stop with `READY_WITH_USER_ACTION`. If headless mode is active, do not ask and do not install.

## Lockfile Map

Use the project lockfile to pick one command:

- `bun.lock` or `bun.lockb`: `bun install --ignore-scripts`
- `pnpm-lock.yaml`: `pnpm install --ignore-scripts`
- `package-lock.json` or `npm-shrinkwrap.json`: `npm install --ignore-scripts`
- `yarn.lock`: `yarn install --ignore-scripts`

If more than one lockfile exists, prefer the package manager already recorded by detect if present; otherwise ask the user to choose. If no lockfile exists, do not ask which package manager to use. Say lockfile is required and leave a user-action card.

## Result Handling

Exit 0 means re-detect. Non-zero means do not claim `VIBE_READY`; summarize the package manager and safe next phrase. If `--ignore-scripts` prevents native module build, keep the install safety decision and downgrade to `READY_WITH_USER_ACTION` rather than rerunning scripts automatically.

Never run `npm rebuild`, postinstall hooks, project scripts, or arbitrary package-manager repair commands from onboarding.
