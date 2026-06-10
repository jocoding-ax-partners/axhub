import axios from "axios";
import { Pool } from "pg";

const pool = new Pool({ connectionString: "x" });

export async function load() {
  const res = await axios.get("https://api.axhub.dev/v1/posts");
  const r2 = await fetch("https://backend.example.com/data");
  const rows = await pool.query("select 1");
  return [res, r2, rows];
}
