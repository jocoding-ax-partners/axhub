#!/usr/bin/env bun
/**
 * Phase 3 US-204: Release manifest generator.
 *
 * Reads built binaries from bin/, computes sha256 per file, emits a JSON
 * manifest mapping {arch → filename, sha256, size_bytes}. Used by:
 *   - Cosign signing pipeline (signs the manifest itself for tamper detection)
 *   - User-side verify-release.sh (cross-checks downloaded files against manifest)
 *
 * Output: JSON to stdout (workflow redirects to bin/manifest.json).
 */
import { readFileSync, statSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";

import packageJson from "../../package.json" with { type: "json" };

interface ManifestEntry {
  filename: string;
  arch: string;
  sha256: string;
  size_bytes: number;
}

interface Manifest {
  schema_version: 1;
  plugin_version: string;
  helper_version: string;
  generated_at: string;
  binaries: ManifestEntry[];
}

const archFromFilename = (filename: string): string => {
  // axhub-helpers-darwin-arm64 → darwin-arm64
  // axhub-helpers-windows-amd64.exe → windows-amd64
  const stripped = filename.replace(/^axhub-helpers-/, "").replace(/\.exe$/, "");
  return stripped;
};

const sha256File = (path: string): string => {
  const buf = readFileSync(path);
  return createHash("sha256").update(buf).digest("hex");
};

const main = (): void => {
  const binDir = join(import.meta.dir, "..", "..", "bin");
  const files = readdirSync(binDir).filter((f) => f.startsWith("axhub-helpers-") && !f.endsWith(".sig"));
  files.sort();

  const binaries: ManifestEntry[] = files.map((f) => {
    const fullPath = join(binDir, f);
    const stats = statSync(fullPath);
    return {
      filename: f,
      arch: archFromFilename(f),
      sha256: sha256File(fullPath),
      size_bytes: stats.size,
    };
  });

  const manifest: Manifest = {
    schema_version: 1,
    plugin_version: packageJson.version,
    helper_version: packageJson.version,
    generated_at: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
    binaries,
  };

  process.stdout.write(JSON.stringify(manifest, null, 2) + "\n");
};

if (import.meta.main) {
  main();
}
