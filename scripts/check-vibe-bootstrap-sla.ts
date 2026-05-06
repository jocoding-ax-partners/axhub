#!/usr/bin/env bun
import { readFileSync } from "node:fs";
import type { GateMode, MeasurementSummary } from "./measure-vibe-bootstrap.ts";

export type SlaCheckOptions = {
  p95Seconds: number;
  minSamples: number;
  mode: GateMode;
};

export type SlaCheckResult = {
  pass: boolean;
  errors: string[];
  warnings: string[];
  p95_seconds: number;
  sample_size: number;
  mode: GateMode;
};

export function evaluateSla(summary: MeasurementSummary, options: SlaCheckOptions): SlaCheckResult {
  const errors: string[] = [];
  const warnings: string[] = [];
  if (summary.sample_size < options.minSamples) {
    const message = `sample_size ${summary.sample_size} < min_samples ${options.minSamples}`;
    if (options.mode === "blocking") errors.push(message);
    else warnings.push(message);
  }
  if (summary.p95_seconds > options.p95Seconds) {
    const message = `p95_seconds ${summary.p95_seconds} > threshold ${options.p95Seconds}`;
    if (options.mode === "blocking") errors.push(message);
    else warnings.push(message);
  }
  if (!summary.live_url_present) {
    const message = "live_url_present is false";
    if (options.mode === "blocking") errors.push(message);
    else warnings.push(message);
  }
  if (summary.consent_block_count !== 0) {
    const message = `consent_block_count ${summary.consent_block_count} != 0`;
    if (options.mode === "blocking") errors.push(message);
    else warnings.push(message);
  }
  return {
    pass: errors.length === 0,
    errors,
    warnings,
    p95_seconds: summary.p95_seconds,
    sample_size: summary.sample_size,
    mode: options.mode,
  };
}

const readArg = (name: string, fallback?: string): string | undefined => {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : fallback;
};

const usage = (): never => {
  process.stderr.write("Usage: bun scripts/check-vibe-bootstrap-sla.ts --summary <file> [--p95-seconds 480] [--min-samples 20] [--mode advisory|blocking]\n");
  process.exit(64);
};

if (import.meta.main) {
  if (process.argv.includes("--help")) usage();
  const summaryPath = readArg("--summary");
  if (!summaryPath) usage();
  const checkedSummaryPath = summaryPath as string;
  const rawMode = readArg("--mode", "advisory");
  if (rawMode !== "advisory" && rawMode !== "blocking") usage();
  const mode = rawMode as GateMode;
  const p95Seconds = Number(readArg("--p95-seconds", "480"));
  const minSamples = Number(readArg("--min-samples", "20"));
  if (!Number.isFinite(p95Seconds) || p95Seconds <= 0 || !Number.isInteger(minSamples) || minSamples <= 0) usage();
  const summary = JSON.parse(readFileSync(checkedSummaryPath, "utf8")) as MeasurementSummary;
  const result = evaluateSla(summary, { p95Seconds, minSamples, mode });
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  process.exit(result.pass ? 0 : 1);
}
