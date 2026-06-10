'use client';
import { db } from "@ax-hub/sdk";

// 클라이언트 컴포넌트에서 server-only SDK import — boundary 위반(block).
// 'use client' 단따옴표 — 룰이 따옴표 종류 무관하게 잡아야 해요 (F1 re-vendor).
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
