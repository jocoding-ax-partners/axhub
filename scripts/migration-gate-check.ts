#!/usr/bin/env bun
/**
 * Phase 6 — Migration Gate aggregator.
 *
 * Runs the 6 gates from docs/migration-gate.md and emits a Markdown evidence
 * block suitable for pasting into a PR description. Gate names + commands are
 * kept in lockstep with the docs table.
 *
 * Exit code: 1 if any non-manual gate fails, 0 otherwise.
 *   bun run scripts/migration-gate-check.ts > /tmp/gate-evidence.md
 */

import { spawnSync } from "node:child_process";

export interface GateResult {
  name: string;
  passed: boolean;
  evidence: string;
  duration_ms: number;
  manual?: boolean;
}

interface GateOptions {
  manual?: boolean;
  check?: (stdout: string, stderr: string, exitCode: number | null) => { passed: boolean; evidence: string };
  cwd?: string;
}

export function runGate(name: string, cmd: string[], opts: GateOptions = {}): GateResult {
  if (opts.manual) {
    return {
      name,
      passed: true,
      evidence: `manual: ${cmd.join(" ")}`,
      duration_ms: 0,
      manual: true,
    };
  }
  const t0 = Date.now();
  const head = cmd[0];
  if (!head) {
    return { name, passed: false, evidence: "empty command", duration_ms: 0 };
  }
  const result = spawnSync(head, cmd.slice(1), {
    encoding: "utf8",
    timeout: 600_000,
    cwd: opts.cwd,
  });
  const duration_ms = Date.now() - t0;
  const stdout = result.stdout ?? "";
  const stderr = result.stderr ?? "";
  const exitCode = result.status;

  if (opts.check) {
    const verdict = opts.check(stdout, stderr, exitCode);
    return { name, passed: verdict.passed, evidence: verdict.evidence, duration_ms };
  }

  const passed = exitCode === 0;
  const tail = (passed ? stdout : stderr).split("\n").filter((l) => l.trim()).pop()?.slice(0, 160) ?? "";
  const evidence = passed ? `exit 0 — ${tail}` : `exit ${exitCode} — ${tail}`;
  return { name, passed, evidence, duration_ms };
}

export function formatMarkdown(gates: GateResult[]): string {
  const rows = gates.map((g) => {
    const icon = g.manual ? "⚠️" : g.passed ? "✅" : "❌";
    const evidence = g.evidence.replace(/\|/g, "\\|");
    return `| ${icon} | ${g.name} | ${evidence} | ${g.duration_ms}ms |`;
  });
  return [
    "## Migration Gate Evidence (Approach E)",
    "",
    "| Status | Gate | Evidence | Duration |",
    "|--------|------|----------|----------|",
    ...rows,
  ].join("\n");
}

export function shouldExitWithError(gates: GateResult[]): boolean {
  return gates.some((g) => !g.passed && !g.manual);
}

export function parseLatencyP95(text: string): number | null {
  const values = [...text.matchAll(/\bp95\s*[=:]\s*([\d.]+)\s*ms\b/gi)]
    .map((match) => Number(match[1]))
    .filter((value) => Number.isFinite(value));
  if (values.length === 0) return null;
  return Math.max(...values);
}

export function buildAllGates(): GateResult[] {
  return [
    runGate("Targeted Rust e2e", ["cargo", "test", "-p", "axhub-helpers", "cli_prompt_route", "--test", "cli_e2e"]),
    runGate("Workspace test", ["cargo", "test", "--workspace"]),
    runGate("TypeScript test", ["bun", "test"]),
    runGate("Hook latency benchmark", ["bun", "run", "scripts/benchmark-hooks.ts"], {
      check: (stdout, stderr, exitCode) => {
        const p95 = parseLatencyP95(stdout) ?? parseLatencyP95(stderr);
        if (p95 === null) {
          return { passed: false, evidence: `cannot parse p95 ms (exit ${exitCode})` };
        }
        return { passed: p95 < 50, evidence: `p95=${p95}ms (threshold 50ms)` };
      },
    }),
    runGate("Routing-score 100-row", [
      "bash", "tests/run-corpus.sh", "--mode", "plugin", "--corpus", "tests/corpus.100.jsonl", "--vs", "claude-native", "--score",
    ]),
    runGate("High-risk live canary", ["manual run: 배포해줘 / 이 앱 삭제해 / DB_URL=foo 설정 / 로그인"], { manual: true }),
  ];
}

if (import.meta.main) {
  const gates = buildAllGates();
  process.stdout.write(formatMarkdown(gates) + "\n");
  process.exit(shouldExitWithError(gates) ? 1 : 0);
}
