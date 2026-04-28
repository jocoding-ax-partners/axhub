// Phase 22.5.5 — claude CLI baseline measurement.
// 5 case dry-run 으로 per-case wall-time + actual cost USD + JSON schema 실측.
// 결과: tests/e2e/claude-cli/output/baseline-{times,cost,schema}.json
// CLAUDE_JSON_SCHEMA.md 의 stop_reason / cost_usd 필드 실재성 lock.

import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync, existsSync, readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(__dirname, "..");
const OUTPUT_DIR = join(REPO_ROOT, "tests/e2e/claude-cli/output");
const SAMPLE_DIR = join(OUTPUT_DIR, "baseline-samples");

mkdirSync(SAMPLE_DIR, { recursive: true });

interface CaseSpec {
  id: string;
  prompt: string;
  description: string;
}

const CASES: CaseSpec[] = [
  {
    id: "smoke-ping",
    prompt: "say only the literal word PONG and nothing else",
    description: "최소 latency / cost baseline",
  },
  {
    id: "smoke-help-slash",
    prompt: "/axhub:help",
    description: "axhub plugin 슬래시 명령 실재 trigger",
  },
  {
    id: "smoke-doctor-ko",
    prompt: "axhub 환경 점검만 한 줄로 알려줘",
    description: "한국어 짧은 NL → doctor SKILL routing",
  },
  {
    id: "smoke-status-noauth",
    prompt: "방금 배포한 거 어떻게 됐어",
    description: "한국어 NL → status SKILL routing (auth_missing fallback)",
  },
  {
    id: "smoke-empty",
    prompt: "ok",
    description: "최소 응답 — graceful end_turn shape lock",
  },
];

interface Measurement {
  case_id: string;
  description: string;
  exit_code: number;
  wall_seconds: number;
  stdout_bytes: number;
  stderr_bytes: number;
  json_parsed: boolean;
  has_stop_reason: boolean;
  stop_reason_value: string | null;
  has_cost_usd: boolean;
  cost_usd_value: number | null;
  has_usage: boolean;
  has_messages: boolean;
  budget_exceeded: boolean;
  notes: string[];
}

const measurements: Measurement[] = [];

const CAP = "0.05";
const TIMEOUT_S = 60;

for (const spec of CASES) {
  console.error(`\n[baseline] ${spec.id} — ${spec.description}`);
  const stdoutPath = join(SAMPLE_DIR, `${spec.id}.stdout.json`);
  const stderrPath = join(SAMPLE_DIR, `${spec.id}.stderr.log`);

  const start = Date.now();

  const proc = spawnSync(
    "claude",
    [
      "-p",
      spec.prompt,
      "--output-format", "json",
      "--no-session-persistence",
      "--max-budget-usd", CAP,
      "--add-dir", REPO_ROOT,
      "--plugin-dir", REPO_ROOT,
      "--strict-mcp-config",
      "--mcp-config", '{"mcpServers":{}}',
    ],
    {
      timeout: TIMEOUT_S * 1000,
      env: {
        ...process.env,
        CLAUDE_NON_INTERACTIVE: "1",
        CI: "1",
        TERM: "dumb",
      },
      maxBuffer: 10 * 1024 * 1024,
    },
  );

  const wall = (Date.now() - start) / 1000;

  const stdout = proc.stdout?.toString() ?? "";
  const stderr = proc.stderr?.toString() ?? "";
  writeFileSync(stdoutPath, stdout);
  writeFileSync(stderrPath, stderr);

  const m: Measurement = {
    case_id: spec.id,
    description: spec.description,
    exit_code: proc.status ?? -1,
    wall_seconds: Number(wall.toFixed(2)),
    stdout_bytes: stdout.length,
    stderr_bytes: stderr.length,
    json_parsed: false,
    has_stop_reason: false,
    stop_reason_value: null,
    has_cost_usd: false,
    cost_usd_value: null,
    has_usage: false,
    has_messages: false,
    budget_exceeded: false,
    notes: [],
  };

  if (proc.error) m.notes.push(`spawn error: ${proc.error.message}`);
  if (proc.signal) m.notes.push(`killed by signal: ${proc.signal}`);

  // SB-3 cap-hit detector logic — exit==124 (timeout) AND stdout<100 byte AND NOT graceful stop_reason
  const isTimeout = m.exit_code === 124 || proc.signal === "SIGTERM";
  const isSmallStdout = m.stdout_bytes < 100;

  try {
    const parsed = JSON.parse(stdout);
    m.json_parsed = true;
    if (typeof parsed === "object" && parsed !== null) {
      const obj = parsed as Record<string, unknown>;
      if ("stop_reason" in obj) {
        m.has_stop_reason = true;
        m.stop_reason_value = obj["stop_reason"] as string | null;
      }
      if ("cost_usd" in obj) {
        m.has_cost_usd = true;
        m.cost_usd_value = obj["cost_usd"] as number;
      } else if ("total_cost_usd" in obj) {
        m.has_cost_usd = true;
        m.cost_usd_value = obj["total_cost_usd"] as number;
        m.notes.push("cost field name = total_cost_usd (alias)");
      }
      if ("usage" in obj) m.has_usage = true;
      if ("messages" in obj || "result" in obj) m.has_messages = true;
    }
  } catch (e) {
    m.notes.push(`json parse failed: ${(e as Error).message.slice(0, 80)}`);
  }

  // graceful sentinel — stop_reason ∈ {abort, user_cancelled, end_turn} 면 budget 아님
  const gracefulMarker = m.stop_reason_value
    ? ["abort", "user_cancelled", "end_turn"].includes(m.stop_reason_value)
    : false;

  m.budget_exceeded = isTimeout && isSmallStdout && !gracefulMarker;

  measurements.push(m);
  console.error(
    `  exit=${m.exit_code} wall=${m.wall_seconds}s stdout=${m.stdout_bytes}B json=${m.json_parsed} stop_reason=${m.stop_reason_value} cost=${m.cost_usd_value}`,
  );
}

// summary
const summary = {
  captured_at: new Date().toISOString(),
  claude_version: spawnSync("claude", ["--version"]).stdout?.toString().trim(),
  cap_usd: Number(CAP),
  timeout_s: TIMEOUT_S,
  per_case_wall_seconds: measurements.map((m) => ({
    id: m.case_id,
    seconds: m.wall_seconds,
    cost_usd: m.cost_usd_value,
  })),
  total_wall_seconds: measurements.reduce((s, m) => s + m.wall_seconds, 0),
  total_cost_usd: measurements.reduce((s, m) => s + (m.cost_usd_value ?? 0), 0),
  cap_hit_count: measurements.filter((m) => m.budget_exceeded).length,
  json_parse_success: measurements.filter((m) => m.json_parsed).length,
  json_parse_total: measurements.length,
  stop_reasons_observed: [
    ...new Set(measurements.map((m) => m.stop_reason_value).filter(Boolean)),
  ],
  cost_field_present: measurements.every((m) => m.has_cost_usd),
  schema_lock_recommendation: measurements.every((m) => m.has_cost_usd && m.has_stop_reason)
    ? "OK — schema 안정적, CLAUDE_JSON_SCHEMA.md TENTATIVE → LOCKED 변경 가능"
    : "DRIFT — cost_usd / stop_reason 필드 일부 case 에서 부재. CLAUDE_JSON_SCHEMA.md 재검토 필요",
  measurements,
};

writeFileSync(
  join(OUTPUT_DIR, "baseline-times.json"),
  JSON.stringify(
    {
      captured_at: summary.captured_at,
      per_case: summary.per_case_wall_seconds.map((p) => ({
        id: p.id,
        seconds: p.seconds,
      })),
      total_seconds: summary.total_wall_seconds,
      cap_seconds: 600,
    },
    null,
    2,
  ),
);

writeFileSync(
  join(OUTPUT_DIR, "baseline-cost.json"),
  JSON.stringify(
    {
      captured_at: summary.captured_at,
      cap_usd: summary.cap_usd,
      per_case: summary.per_case_wall_seconds.map((p) => ({
        id: p.id,
        cost_usd: p.cost_usd,
      })),
      total_cost_usd: summary.total_cost_usd,
      cap_hit_count: summary.cap_hit_count,
      ratchet_recommendation: summary.cap_hit_count > 0
        ? `WARN — ${summary.cap_hit_count} cases hit cap. spawn.sh --max-budget-usd 상향 검토.`
        : `OK — all under cap. spawn.sh cap=0.30 USD 유지 (max observed × 5 안전 마진).`,
    },
    null,
    2,
  ),
);

writeFileSync(
  join(OUTPUT_DIR, "baseline-schema.json"),
  JSON.stringify(summary, null, 2),
);

console.error(`\n[baseline] complete.`);
console.error(`  total wall: ${summary.total_wall_seconds}s / cap 600s`);
console.error(`  total cost: $${summary.total_cost_usd.toFixed(4)} USD`);
console.error(`  cap-hit: ${summary.cap_hit_count}`);
console.error(`  stop_reasons: ${summary.stop_reasons_observed.join(", ") || "(none)"}`);
console.error(`  schema lock: ${summary.schema_lock_recommendation}`);
console.error(`  → tests/e2e/claude-cli/output/baseline-{times,cost,schema}.json`);

if (summary.cap_hit_count > 0) {
  console.error(`\n[baseline] WARN — cap-hit detected. ratchet 검토 필요.`);
}
