#!/usr/bin/env bun
/** Cross-platform audit for SKILL.md reference-file links. */
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { basename, dirname, join, normalize } from "node:path";

const ROOT = process.env.PLUGIN_ROOT ?? join(import.meta.dir, "..");
const SKILLS_DIR = join(ROOT, "skills");
const REF_RE =
  /\.\.\/(?:[a-zA-Z0-9_.-]+\/)*references\/[a-zA-Z0-9_.-]+\.md|references\/[a-zA-Z0-9_.-]+\.md/g;

const walkSkillFiles = (dir: string): string[] => {
  if (!existsSync(dir)) return [];
  const files: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...walkSkillFiles(path));
    if (entry.isFile() && entry.name === "SKILL.md") files.push(path);
  }
  return files;
};

let broken = 0;
for (const skillFile of walkSkillFiles(SKILLS_DIR)) {
  const body = readFileSync(skillFile, "utf8");
  for (const match of body.matchAll(REF_RE)) {
    const ref = match[0];
    const absPath = normalize(join(dirname(skillFile), ref));
    if (!existsSync(absPath)) {
      process.stdout.write(`BROKEN: ${skillFile} -> ${ref} (expected ${dirname(absPath)}/${basename(absPath)})\n`);
      broken += 1;
    }
  }
}

process.stdout.write(`Broken: ${broken}\n`);
process.exit(broken > 0 ? 1 : 0);
