import { db } from "@axhub/sdk";

// 잘못된 SDK 사용 — block 룰 검출 대상이에요.
export async function loadPosts() {
  // or() / not() 는 pushable 하지 않은 필터 조합이에요.
  const a = await db.table("posts").or(db.eq("x", 1), db.eq("y", 2)).list();
  const b = await db.table("posts").not(db.eq("x", 1)).count();
  // keyset 커서(after:)는 지원 안 해요.
  const c = await db.table("posts").list({ after: "cursor123" });
  return [a, b, c];
}
