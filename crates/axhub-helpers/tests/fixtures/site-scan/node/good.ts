import { db } from "@axhub/sdk";

export async function load(ownerId: string) {
  const posts = await db.table("posts").eq("owner_id", ownerId).limit(20).list();
  const count = await db.table("posts").count();
  return { posts, count };
}
