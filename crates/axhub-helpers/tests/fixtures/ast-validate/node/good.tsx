import { db } from "@axhub/sdk";

// owner-scoped 무필터 list 는 정당해요 (advisory warn 만, block 아님).
export function PostList({ ownerId }: { ownerId: string }) {
  const rows = db.table("posts").list();
  const filtered = db.table("posts").eq("owner_id", ownerId).limit(20).list();
  return (
    <ul>
      {filtered.map((r: { id: string; title: string }) => (
        <li key={r.id}>
          {r.title}
          {rows.length}
        </li>
      ))}
    </ul>
  );
}
