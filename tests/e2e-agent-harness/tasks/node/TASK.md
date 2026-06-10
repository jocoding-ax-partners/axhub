---
lang: node
trap_id: or_combinator
trap_kind: forbidden_combinator
sdk_version: 2.1.2
packs_path: sdk/dist/sdk-knowledge/node.md
---

# Task: TypeScript — 주문 목록 조회 (OR 필터)

You are a TypeScript developer using the AxHub SDK (`@ax-hub/sdk`).

Write TypeScript code to fetch orders from an AxHub table where:
- The order status is **'paid'** OR the order status is **'pending'**

The table name is `orders` with columns: `id` (uuid), `status` (string), `total` (number).

Assume the SDK client is already initialized as `axhub` and the tenant/app handles are available.

Use the AxHub SDK `data` API (not raw HTTP). Show the full query code including the filter.

```ts
// Expected output structure:
const orders = axhub.tenant(process.env.AX_HUB_TENANT_SLUG!).app(process.env.AX_HUB_APP_SLUG!).data.table(Orders);
const result = await orders.list({ where: /* your filter here */ });
```

Write only the data query portion. Do not include client initialization.
