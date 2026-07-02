import { copyFileSync, existsSync, mkdirSync, readdirSync, rmSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

const REPO_ROOT = resolve(join(import.meta.dir, ".."));
const DEFAULT_OUT_DIR = join(REPO_ROOT, "dist", "axhub-plugin");
const ROOT_FILES = ["README.md", "LICENSE"] as const;
const ROOT_DIRS = [".claude-plugin", "skills"] as const;
const DENY_NAMES = new Set([
  ".DS_Store",
  ".axhub",
  ".axhub-state",
  ".autoplan",
  ".bun",
  ".cache",
  ".cargo",
  ".claude",
  ".codegraph",
  ".git",
  ".github",
  ".gitnexus",
  ".gjc",
  ".graphify_python",
  ".obsidian",
  ".omc",
  ".omx",
  ".ouroboros",
  ".plan",
  ".qa-live",
  ".serena",
  ".specify",
  ".understand-anything",
  "CHANGELOG.md",
  "CLAUDE.md",
  "AGENTS.md",
  "node_modules",
  "graphify-out",
  "tests",
  "scripts",
  "dist",
  "test-results.json",
]);

interface Options {
  outDir: string;
  json: boolean;
}

interface BundleStats {
  outDir: string;
  files: number;
  bytes: number;
}

const parseArgs = (): Options => {
  let outDir = DEFAULT_OUT_DIR;
  let json = false;
  const args = Bun.argv.slice(2);
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--json") {
      json = true;
    } else if (arg === "--out") {
      const value = args[i + 1];
      if (!value) throw new Error("--out requires a path");
      outDir = resolve(value);
      i += 1;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return { outDir, json };
};

const assertSafeOutDir = (outDir: string): void => {
  const normalized = resolve(outDir);
  if (normalized === REPO_ROOT || normalized === dirname(REPO_ROOT) || normalized === "/") {
    throw new Error(`refusing to clear unsafe output path: ${outDir}`);
  }
};

const isDenied = (path: string): boolean => {
  const parts = relative(REPO_ROOT, path).split("/");
  return parts.some((part) => DENY_NAMES.has(part));
};

const copyTree = (src: string, dest: string): void => {
  if (isDenied(src)) return;
  const stat = statSync(src);
  if (stat.isDirectory()) {
    mkdirSync(dest, { recursive: true });
    for (const entry of readdirSync(src)) {
      copyTree(join(src, entry), join(dest, entry));
    }
    return;
  }
  if (!stat.isFile()) return;
  mkdirSync(dirname(dest), { recursive: true });
  copyFileSync(src, dest);
};

const collectStats = (dir: string): { files: number; bytes: number } => {
  let files = 0;
  let bytes = 0;
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      const child = collectStats(path);
      files += child.files;
      bytes += child.bytes;
    } else if (stat.isFile()) {
      files += 1;
      bytes += stat.size;
    }
  }
  return { files, bytes };
};

const buildBundle = ({ outDir }: Options): BundleStats => {
  assertSafeOutDir(outDir);
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });

  for (const file of ROOT_FILES) {
    copyTree(join(REPO_ROOT, file), join(outDir, file));
  }

  for (const dir of ROOT_DIRS) {
    copyTree(join(REPO_ROOT, dir), join(outDir, dir));
  }

  const pluginJson = join(outDir, ".claude-plugin", "plugin.json");
  if (!existsSync(pluginJson)) {
    throw new Error(`bundle is missing ${relative(outDir, pluginJson)}`);
  }

  const stats = collectStats(outDir);
  return { outDir, ...stats };
};

const main = (): void => {
  const options = parseArgs();
  const stats = buildBundle(options);
  if (options.json) {
    console.log(JSON.stringify(stats));
  } else {
    console.log(`Built axhub plugin bundle at ${stats.outDir} (${stats.files} files, ${stats.bytes} bytes)`);
  }
};

main();
