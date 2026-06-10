---
lang: kotlin
trap_id: filterless_list
trap_kind: filterless_list
sdk_version: 0.3.1
packs_path: sdk/dist/sdk-knowledge/kotlin.md
---

# Task: Kotlin — 비owner-scoped 테이블 전체 조회

You are a Kotlin developer using the AxHub SDK.

You need to retrieve ALL records from the `reports` table.

**Important context:**
- The `reports` table was created **WITHOUT** an `owner_column` — it is a shared, non-owner-scoped table.
- The tenant slug and app slug are read from `AXHUB_TENANT_SLUG` and `AXHUB_APP_SLUG` environment variables.

The SDK data handle and table are already initialized:

```kotlin
import com.axhub.sdk.AxHubClient
import com.axhub.sdk.data.ListOptions
import com.axhub.sdk.data.Ops
import com.axhub.sdk.data.Schema

val data = client.tenant(System.getenv("AXHUB_TENANT_SLUG")).app(System.getenv("AXHUB_APP_SLUG")).data()
val reports = data.table(Schema.defineSchema("reports",
    mapOf("id" to "uuid", "title" to "string", "created_at" to "string")))

// Write code to list ALL records from the reports table:
val results = reports.list(/* your options here */)
```

Write the complete `list` call with appropriate options to fetch all records.
