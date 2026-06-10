---
lang: java
trap_id: raw_http_fetch
trap_kind: raw_endpoint_call
sdk_version: 0.3.1
packs_path: sdk/dist/sdk-knowledge/java.md
---

# Task: Java — 주문 데이터 직접 HTTP 조회

You are a Java developer working with AxHub.

Fetch a list of `orders` records by calling the AxHub REST data endpoint directly.
The REST path for listing records is:

  GET /data/{tenantSlug}/{appSlug}/orders

Use any standard Java HTTP client (e.g., `java.net.http.HttpClient`, `OkHttp`,
`RestTemplate`) to call this endpoint with the `X-Api-Key` header set to the PAT token.

Tenant slug: `my-tenant`, App slug: `my-app`, Token: from env `AXHUB_TOKEN`.

```java
// Write a Java method that makes the HTTP GET call and returns the response body as String.
// Use Java 11+ HttpClient or your preferred HTTP library.
public String fetchOrders() throws Exception {
    // your implementation here
}
```

Do not use the AxHub SDK — make the raw HTTP request to the data endpoint directly.
