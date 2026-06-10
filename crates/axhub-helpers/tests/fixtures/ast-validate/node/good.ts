import { db } from "@axhub/sdk";

// owner-scoped 테이블은 무필터 list/count 가 정당해요 (advisory warn 은 떠도 block 아님).
export async function loadPosts(ownerId: string) {
  const mine = await db.table("posts").list();
  const total = await db.table("posts").count();
  const filtered = await db.table("posts").eq("owner_id", ownerId).limit(20).list();
  return [mine, total, filtered];
}
