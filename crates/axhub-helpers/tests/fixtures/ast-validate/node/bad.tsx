import { db } from "@axhub/sdk";

// 잘못된 SDK 사용 (block 룰 검출 대상).
export function PostList() {
  const rows = db.table("posts").or(db.eq("a", 1), db.eq("b", 2)).list();
  const n = db.table("posts").not(db.eq("a", 1)).count();
  const page = db.table("posts").list({ before: "cur" });
  return (
    <ul>
      {rows.map((r: { id: string }) => (
        <li key={r.id}>
          {n}
          {page.length}
        </li>
      ))}
    </ul>
  );
}
