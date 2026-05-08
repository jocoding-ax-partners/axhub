/**
 * Phase 0 MEASUREMENTS.md publisher.
 *
 * Spec: .plan/deploy-time-reduction/phase-0-measurement-first.md §8 + §10
 *
 * Reads test-results.json and rewrites the matching OS subsection inside
 * .plan/deploy-time-reduction/MEASUREMENTS.md. Idempotent — re-running on
 * the same OS replaces that section's table rather than appending duplicates.
 *
 * Usage: bun run scripts/perf-publish-measurements.ts --os macos-latest
 */

import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import process from "node:process";
import type { ResultsFile, ScenarioRecord } from "./perf-parse-results";

const MEASUREMENTS_PATH = join(
  ".plan",
  "deploy-time-reduction",
  "MEASUREMENTS.md",
);

const SCENARIO_LABELS: Record<string, string> = {
  "scenario-1-warm-redeploy": "Scenario 1 — warm redeploy",
  "scenario-2-cold-first-deploy": "Scenario 2 — cold first deploy",
  "scenario-3-fresh-home-no-token":
    "Scenario 3 — fresh HOME (no token)",
};

function parseArgs(argv: string[]): { os: string; resultsFile: string } {
  let os: string | undefined;
  let resultsFile = process.env.RESULTS_FILE ?? "test-results.json";
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--os") {
      os = argv[++i];
    } else if (arg === "--results") {
      resultsFile = argv[++i];
    }
  }
  if (!os) {
    console.error("perf-publish: --os <name> required");
    process.exit(1);
  }
  return { os, resultsFile };
}

export function buildOsBlock(os: string, results: ResultsFile, timestamp: string): string {
  const rows = Object.keys(SCENARIO_LABELS).map((scenarioId) => {
    const record: ScenarioRecord | undefined = results[scenarioId];
    const label = SCENARIO_LABELS[scenarioId];
    if (!record) {
      return `| ${label} | _missing_ | _missing_ | _missing_ |`;
    }
    return `| ${label} | ${record.avg}ms | ${record.p95}ms | ${record.runs.length} |`;
  });
  return [
    `### ${os}`,
    "",
    `_last updated: ${timestamp}_`,
    "",
    "| Scenario | avg | p95 | runs |",
    "|----------|-----|-----|------|",
    ...rows,
    "",
  ].join("\n");
}

export function spliceOsBlock(
  doc: string,
  os: string,
  block: string,
): string {
  const heading = `### ${os}`;
  const lines = doc.split("\n");
  const startIdx = lines.findIndex((line) => line.trim() === heading);
  if (startIdx === -1) {
    // append a fresh section before the first H2 of "Future phases" or end of file
    if (!doc.endsWith("\n")) doc += "\n";
    return `${doc}\n${block}\n`;
  }
  let endIdx = lines.length;
  for (let i = startIdx + 1; i < lines.length; i++) {
    const line = lines[i];
    if (line.startsWith("### ") || line.startsWith("## ") || line.startsWith("# ")) {
      endIdx = i;
      break;
    }
  }
  const before = lines.slice(0, startIdx).join("\n");
  const after = lines.slice(endIdx).join("\n");
  const middle = block.replace(/\n+$/, "") + "\n\n";
  const beforePart = before.length > 0 ? before + "\n" : "";
  return `${beforePart}${middle}${after}`;
}

function scaffold(): string {
  return `---
status: PHASE_0_PENDING_CI
phase: 0
generated_by: scripts/perf-publish-measurements.ts
---

# Deploy Walltime Measurements

> CI matrix runs \`tests/perf/deploy_walltime.test.ts\` and republishes each
> OS subsection idempotently via \`scripts/perf-publish-measurements.ts --os <runner>\`.
> The canonical doc lives at \`.plan/deploy-time-reduction/MEASUREMENTS.md\`
> (\`.plan/\` is gitignored — this CI scaffold is regenerated per run when
> the canonical doc is absent).

## Phase 0 Baseline

`;
}

async function ensureMeasurementsDoc(): Promise<string> {
  try {
    return await readFile(MEASUREMENTS_PATH, "utf8");
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
    await mkdir(dirname(MEASUREMENTS_PATH), { recursive: true });
    const initial = scaffold();
    await writeFile(MEASUREMENTS_PATH, initial, "utf8");
    return initial;
  }
}

async function main(): Promise<void> {
  const { os, resultsFile } = parseArgs(process.argv.slice(2));
  const raw = await readFile(resultsFile, "utf8");
  const results = JSON.parse(raw) as ResultsFile;
  const doc = await ensureMeasurementsDoc();
  const block = buildOsBlock(os, results, new Date().toISOString());
  const next = spliceOsBlock(doc, os, block);
  await writeFile(MEASUREMENTS_PATH, next, "utf8");
  process.stdout.write(`perf-publish: updated ${MEASUREMENTS_PATH} for ${os}\n`);
}

if (import.meta.main) {
  await main();
}
