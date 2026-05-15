# Manual post-commit setup

install.sh 가 기존 `.git/hooks/post-commit` 을 발견했고 자동 append 를 건너뛰었다면 아래 줄을 기존 hook 마지막에 추가해요.

```bash
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$HOME/.claude/plugins/axhub}"
"$PLUGIN_ROOT/bin/axhub-helpers" state-update --post-commit-promote 2>/dev/null || true
```

끄려면:

```bash
export AXHUB_DISABLE_POSTCOMMIT=1
```

`.axhub-state/` 는 local-only state 예요. repo 에 commit 하지 않도록 `.gitignore` 에 추가해요.
