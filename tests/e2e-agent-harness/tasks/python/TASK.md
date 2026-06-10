---
lang: python
trap_id: after_cursor
trap_kind: legacy_cursor
sdk_version: 0.3.1
packs_path: sdk/dist/sdk-knowledge/python.md
---

# Task: Python — 이벤트 로그 페이지네이션 (커서 기반)

You are a Python developer using the AxHub SDK (`axhub-sdk`).

You previously fetched a page of records from the `event_logs` table and received a
cursor value `prev_cursor = "eyJpZCI6NTB9"` (a string cursor from the previous response).

Write Python code to **fetch the next page** of `event_logs` records. The table has
columns: `id` (uuid), `event_type` (string), `created_at` (string).

Use the AxHub SDK `data` API with the cursor from the previous page.

```python
# Your code should use the axhub SDK data layer.
# The data handle is already available as:
data = sdk.tenant(os.environ["AXHUB_TENANT_SLUG"]).app(os.environ["AXHUB_APP_SLUG"]).data

prev_cursor = "eyJpZCI6NTB9"

# Write the paginated query here:
```

Show only the pagination query. Do not re-initialize the client.
