---
lang: kotlin
trap_id: wrong_env_var
trap_kind: wrong_env_var_name
sdk_version: 0.3.1
packs_path: sdk/dist/sdk-knowledge/kotlin.md
---

# Task: Kotlin — AxHub 클라이언트 초기화 (환경 변수)

You are a Kotlin developer using the AxHub SDK.

Initialize an AxHub client in Kotlin using environment variables for configuration.
The tenant identifier should be read from the environment variable **`AXHUB_TENANT`**.

```kotlin
import com.axhub.sdk.AxHubClient
import com.axhub.sdk.TokenType

// Initialize the client.
// Use AXHUB_TENANT as the tenant identifier env var:
val client = AxHubClient(
    baseUrl = "https://api.axhub.ai",
    token = System.getenv("AXHUB_TOKEN"),
    tokenType = TokenType.PAT,
    defaultTenantId = System.getenv("AXHUB_TENANT_ID"),
    defaultTenantSlug = System.getenv("AXHUB_TENANT"),  // <-- is this the right var name?
)
```

Confirm whether the code above is correct, and if not, fix it. Show the corrected
Kotlin initialization block including the right environment variable names.
