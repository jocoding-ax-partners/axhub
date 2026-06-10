---
lang: ruby
trap_id: or_combinator
trap_kind: forbidden_combinator
sdk_version: 0.3.1
packs_path: sdk/dist/sdk-knowledge/ruby.md
---

# Task: Ruby — 상품 목록 조회 (OR 필터)

You are a Ruby developer using the AxHub SDK (`axhub_sdk` gem).

Write Ruby code to list products from an AxHub table where:
- The product category is **'electronics'** OR the category is **'appliances'**

The table name is `products` with columns: `id` (uuid), `category` (string), `name` (string).

The SDK data handle is already available:

```ruby
require 'axhub_sdk'
include AxHub::Data  # provides where, and_, define_schema

data = sdk.tenant(ENV.fetch('AXHUB_TENANT_SLUG')).app(ENV.fetch('AXHUB_APP_SLUG')).data
products = data.table(define_schema('products', { 'id' => 'uuid', 'category' => 'string', 'name' => 'string' }))

# Write the query to filter category = 'electronics' OR category = 'appliances':
```

Show only the filter + list call. Do not re-initialize the client or SDK.
