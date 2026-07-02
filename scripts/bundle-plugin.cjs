#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const out = path.join(root, "dist", "axhub-plugin");

fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });

for (const entry of ["skills", "hooks"]) {
  const source = path.join(root, entry);
  if (fs.existsSync(source)) {
    fs.cpSync(source, path.join(out, entry), { recursive: true });
  }
}

for (const entry of ["README.md", "LICENSE"]) {
  const source = path.join(root, entry);
  if (fs.existsSync(source)) {
    fs.copyFileSync(source, path.join(out, entry));
  }
}

console.log(`Bundled axhub plugin to ${path.relative(root, out)}`);
