# deploy 보조 명령 커버리지 (v0.2.0)

deploy SKILL 핵심 흐름(배포·verify) 밖의 읽기/취소 보조 명령이에요.

## deploy list

Read-only deployment browsing:

```bash
axhub deploy list --app "$APP_ID" --json
```

pagination 이 보이면 첫 페이지만 보여주고 follow-up 을 제안해요 (긴 목록 dump 금지).

## deploy cancel

Cancel is a mutation. Preview the in-progress deployment first (app id/slug, deployment id, branch/commit, current status, expected effect), confirm approval, then run:

```bash
axhub deploy cancel "$DEPLOYMENT_ID" --app "$APP_ID" --execute --json
```

After cancellation, run a read-only status check and summarize the terminal state.
