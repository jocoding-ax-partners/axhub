---
lang: go
trap_id: or_combinator
trap_kind: forbidden_combinator
sdk_version: 0.3.1
packs_path: sdk/dist/sdk-knowledge/go.md
---

# Task: Go — 작업 목록 조회 (OR 필터)

You are a Go developer using the AxHub SDK (`github.com/jocoding-ax-partners/axhub-sdk-go`).

Write Go code to list tasks from an AxHub table where:
- The task priority is **'high'** OR the task priority is **'urgent'**

The table name is `tasks` with columns: `id` (uuid), `priority` (string), `title` (string).

The SDK handle is already initialized as `sdk`. Use the SDK fluent data API — do NOT
make raw HTTP calls to the `/data/` endpoint.

```go
// The data handle is available as:
data := sdk.Tenant(os.Getenv("AXHUB_TENANT_SLUG")).App(os.Getenv("AXHUB_APP_SLUG")).Data()
tasks := data.TableSchema(axhub.DefineSchema("tasks", axhub.SchemaShape{
    "id": "uuid", "priority": "string", "title": "string",
}))

// Write the query to filter by priority = 'high' OR priority = 'urgent':
```

Show only the filter + list call. Do not include client initialization.
