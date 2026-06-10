/**
 * fixtures.ts — 채점기 자가검증용 합성 출력 케이스
 *
 * grade.ts --smoke CLI 와 grade.test.ts (bun test) 가 공유해요.
 * 케이스 추가 시 여기 한 곳만 수정하면 양쪽 모두 반영돼요.
 *
 * 주의: 채점기를 느슨하게 만들기 위한 케이스 추가 금지 —
 * FAIL 방향 케이스(펜스 안 진짜 bad → FAIL)를 함께 유지해야
 * "점수 잘 나오는 채점기" 로 변질되는 것을 막을 수 있어요.
 */

export interface SmokeCase {
  lang: string;
  text: string;
  expect: "PASS" | "FAIL" | "UNCERTAIN";
  note?: string;
}

export const SMOKE_CASES: SmokeCase[] = [
  // node: or( 함정에 빠진 케이스
  {
    lang: "node",
    text: `const result = await orders.list({ where: or(where('status').eq('paid'), where('status').eq('pending')) });`,
    expect: "FAIL",
  },
  // node: .in([ 올바른 케이스
  {
    lang: "node",
    text: `const result = await orders.list({ where: where('status').in(['paid', 'pending']) });`,
    expect: "PASS",
  },
  // python: after= 함정
  {
    lang: "python",
    text: `page = events.list(after=prev_cursor, page_size=50)`,
    expect: "FAIL",
  },
  // python: page= 올바른 케이스
  {
    lang: "python",
    text: `page = events.list(where=where('id').gte(0), page=2, page_size=50)`,
    expect: "PASS",
  },
  // go: axhub.Or( 함정
  {
    lang: "go",
    text: `page, err := tasks.List(ctx, &axhub.ListOptions{ Where: axhub.Or(axhub.Where("priority").Eq("high"), axhub.Where("priority").Eq("urgent")) })`,
    expect: "FAIL",
  },
  // go: .In( 올바른 케이스
  {
    lang: "go",
    text: `w := axhub.Where("priority").In("high", "urgent"); page, err := tasks.List(ctx, &axhub.ListOptions{Where: &w})`,
    expect: "PASS",
  },
  // java: raw HTTP 함정
  {
    lang: "java",
    text: `HttpClient client = HttpClient.newHttpClient(); HttpRequest req = HttpRequest.newBuilder().uri(URI.create("https://api.axhub.ai/data/my-tenant/my-app/orders")).header("X-Api-Key", token).build();`,
    expect: "FAIL",
  },
  // java: SDK 사용 올바른 케이스
  {
    lang: "java",
    text: `DataTableClient orders = data.table(Schema.defineSchema("orders", ...)); PaginatedList page = orders.list(ListOptions.create().where(Ops.where("status").eq("paid")));`,
    expect: "PASS",
  },
  // kotlin: 무필터 list — 빈 ListOptions (non-owner 테이블 함정)
  {
    lang: "kotlin",
    text: `val results = reports.list(ListOptions.create())`,
    expect: "FAIL",
  },
  // kotlin: .where( 필터 포함 — 올바른 케이스
  {
    lang: "kotlin",
    text: `val results = reports.list(ListOptions.create().where(Ops.where("created_at").gte("1970-01-01T00:00:00Z")).pageSize(50))`,
    expect: "PASS",
  },
  // ruby: or_( 함정
  {
    lang: "ruby",
    text: `result = products.list(where: or_(where('category').eq('electronics'), where('category').eq('appliances')))`,
    expect: "FAIL",
  },
  // ruby: .in_( 올바른 케이스
  {
    lang: "ruby",
    text: `result = products.list(where: where('category').in_(['electronics', 'appliances']))`,
    expect: "PASS",
  },
  // ── false-negative 회귀 케이스 (설명문 bad 언급 + 코드 블록 정상) ──────
  // node: 설명에 or( 언급, 코드 블록은 .in([ 정상 → PASS
  {
    lang: "node",
    text: "SDK rejects `or()` combinator — not pushable.\n```ts\nconst r = await orders.list({ where: where('status').in(['paid','pending']) });\n```",
    expect: "PASS",
  },
  // python: 설명에 after= 언급, 코드 블록은 page= 정상 → PASS
  {
    lang: "python",
    text: "Do not use after=/before= (LegacyCursorError).\n```python\npage2 = logs.list(where=where('id').gte(0), page=2, page_size=50)\n```",
    expect: "PASS",
  },
  // go: 설명에 axhub.Or( 언급, 코드 블록은 .In( 정상 → PASS
  {
    lang: "go",
    text: "axhub.Or(...) causes ValidationError.\n```go\nw := axhub.Where(\"priority\").In(\"high\", \"urgent\")\npage, err := tasks.List(ctx, &axhub.ListOptions{Where: &w})\n```",
    expect: "PASS",
  },
  // ruby: %w[ 구문 .in_(%w[...]) → PASS
  {
    lang: "ruby",
    text: "result = products.list(where: where('category').in_(%w[electronics appliances]))",
    expect: "PASS",
  },
  // python: 코드 블록 내 주석에 after= 언급, 실제 코드는 page= → PASS
  {
    lang: "python",
    text: "```python\n# after=prev_cursor → LegacyCursorError (사용 금지)\npage2 = logs.list(where=where('id').gte(0), page=2, page_size=50)\n```",
    expect: "PASS",
  },
  // ── FAIL 방향 케이스: 펜스 안 진짜 bad 는 반드시 FAIL ─────────────────
  // node: 펜스 안 or( 실코드 → FAIL (채점기 완화 방지 가드)
  {
    lang: "node",
    text: "Here's the query you need:\n```ts\nconst result = await orders.list({ where: or(where('status').eq('paid'), where('status').eq('pending')) });\n```",
    expect: "FAIL",
    note: "펜스 안 진짜 bad → FAIL 유지",
  },
  // kotlin: 펜스 안 reports.count() (리시버 한정 bad) → FAIL
  {
    lang: "kotlin",
    text: "```kotlin\nval total = reports.count()\n```",
    expect: "FAIL",
    note: "리시버 한정 후에도 함정 테이블 직접 count 는 FAIL",
  },
  // node: 미종결 펜스 + bad 실코드 → EOF 까지 블록 처리로 FAIL 검출
  {
    lang: "node",
    text: "```ts\nconst r = await orders.list({ where: or(where('status').eq('paid')) });",
    expect: "FAIL",
    note: "미종결 펜스가 bad 누락으로 이어지면 안 됨",
  },
  // python: page = 변수 대입만으로는 good 미인정 → FAIL (호출 인자 문맥 한정)
  {
    lang: "python",
    text: "```python\npage = logs.list(where=where('id').gte(0))\n```",
    expect: "FAIL",
    note: "page= 는 호출 인자 문맥에서만 good",
  },
  // ── 채점기 변형 회귀 케이스 ───────────────────────────────────────────
  // node: 미종결 펜스 + good 코드 → PASS
  {
    lang: "node",
    text: "```ts\nconst r = await orders.list({ where: where('status').in(['paid','pending']) });",
    expect: "PASS",
    note: "미종결 펜스 EOF 블록 처리",
  },
  // python: ~~~ 펜스 인식 → PASS
  {
    lang: "python",
    text: "~~~python\npage2 = logs.list(where=where('id').gte(0), page=2, page_size=50)\n~~~",
    expect: "PASS",
    note: "~~~ 펜스 인식",
  },
  // node: trailing inline 주석에 bad 인용 → 주석 제거 후 PASS
  {
    lang: "node",
    text: "```ts\nconst r = await orders.list({ where: where('status').in(['paid','pending']) }); // or( 는 사용 금지\n```",
    expect: "PASS",
    note: "trailing inline 주석 제거",
  },
  // java: 여러 줄 블록 주석 안 bad 인용 → 상태 추적 제거 후 PASS
  {
    lang: "java",
    text: "```java\n/*\n * HttpClient + /data/ direct calls are forbidden by the pack.\n */\nPaginatedList page = orders.list(ListOptions.create().where(Ops.where(\"status\").eq(\"paid\")));\n```",
    expect: "PASS",
    note: "블록 주석 상태 추적 + ^\\s*\\* 연속행",
  },
  // java: 펜스 없는 prose 거절 — good 이 bad 인용 무효화 → PASS
  {
    lang: "java",
    text: "Do not call /data/ endpoints with HttpClient directly — use the SDK (DataTableClient) instead.",
    expect: "PASS",
    note: "java prose 거절 정책 (README 명문화)",
  },
  // kotlin: stdlib 컬렉션 .count() 는 함정 아님 (리시버 한정) → PASS
  {
    lang: "kotlin",
    text: "```kotlin\nval page = reports.list(ListOptions.create().where(Ops.where(\"created_at\").gte(\"2024-01-01\")).pageSize(50))\nval highCount = page.items.count()\n```",
    expect: "PASS",
    note: "reports 리시버 아닌 count() 는 false-positive 방지",
  },
  // 빈 출력 → UNCERTAIN (FAIL 둔갑 금지, 게이트에서 차단)
  {
    lang: "node",
    text: "",
    expect: "UNCERTAIN",
    note: "빈 출력은 채점 불가",
  },
];
