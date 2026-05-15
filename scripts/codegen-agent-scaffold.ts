#!/usr/bin/env bun
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const slug = process.argv[2];
const modelArgIndex = process.argv.indexOf("--model");
const model = modelArgIndex >= 0 ? process.argv[modelArgIndex + 1] : "sonnet";
if (!slug || !/^[a-z][a-z0-9-]*$/.test(slug)) {
  process.stderr.write("usage: bun run agent:new <slug> [--model haiku|sonnet|opus]\n");
  process.exit(1);
}
if (!["haiku", "sonnet", "opus"].includes(model)) {
  process.stderr.write("error: --model must be haiku, sonnet, or opus\n");
  process.exit(1);
}
const template = readFileSync(join(root, "agents/_template/AGENT.md.tmpl"), "utf8");
const target = join(root, "agents", `${slug}.md`);
if (existsSync(target)) {
  process.stderr.write(`error: agents/${slug}.md already exists\n`);
  process.exit(1);
}
mkdirSync(join(root, "agents"), { recursive: true });
const description = `${slug} specialist. Korean 해요체 output with evidence.`;
writeFileSync(target, template.replaceAll("{{SLUG}}", slug).replaceAll("{{MODEL}}", model).replaceAll("{{DESCRIPTION}}", description));
process.stdout.write(`✓ Created agents/${slug}.md\n`);
