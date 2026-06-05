#!/usr/bin/env bun
import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const files = [
  "crates/axhub-helpers/src/main.rs",
  "crates/axhub-helpers/src/tdd_inject.rs",
  "hooks/post-tool-verify-deploy-artifacts.ts",
];
const required = ["Observed:", "Suggested:", "Skip: AXHUB_DISABLE_HOOK="];
const routeControlRequired = ["Skip: AXHUB_DISABLE_HOOK="];
let failures = 0;
for (const file of files) {
  const raw = readFileSync(join(root, file), "utf8");
  const tags = [...raw.matchAll(/<axhub-[^>]+>[\s\S]*?<\/axhub-[^>]+>/g)].map((m) => m[0]);
  if (tags.length === 0) {
    process.stderr.write(`[hook-inject] ${file}: no axhub additionalContext tag found\n`);
    failures += 1;
    continue;
  }
  for (const tag of tags) {
    const isRouteControlHint =
      tag.includes("Control only; do not summarize this block to the user.") ||
      tag.includes("First visible sentence") ||
      tag.includes("첫 문장:");
    const requiredTokens = isRouteControlHint ? routeControlRequired : required;
    for (const token of requiredTokens) {
      const satisfied =
        tag.includes(token) || (token === "Observed:" && tag.includes("{observed_block}"));
      if (!satisfied) {
        process.stderr.write(`[hook-inject] ${file}: tag missing ${token}\n${tag}\n`);
        failures += 1;
      }
    }
    const approxTokens = Math.ceil(tag.length / 3);
    const limit = isRouteControlHint
      ? 1_200
      : tag.includes("deploy-verify")
        ? 200
        : tag.includes("preflight")
          ? 120
          : 120;
    if (approxTokens > limit) {
      process.stderr.write(`[hook-inject] ${file}: tag budget ${approxTokens} > ${limit}\n`);
      failures += 1;
    }
  }
}
if (failures > 0) process.exit(1);
process.stdout.write("[hook-inject] OK\n");
