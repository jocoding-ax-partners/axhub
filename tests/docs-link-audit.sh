#!/usr/bin/env bash
# Audit all references/X.md mentions across SKILL.md files.
# Resolve each path against filesystem; report broken links.
set -euo pipefail
ROOT=${PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}
broken=0
while IFS= read -r line; do
  skill_file=$(echo "$line" | cut -d: -f1)
  ref=$(echo "$line" | grep -oE '\.\./[a-zA-Z0-9_/-]+/references/[a-zA-Z0-9_/-]+\.md|references/[a-zA-Z0-9_/-]+\.md' | head -1)
  [ -z "$ref" ] && continue
  skill_dir=$(dirname "$skill_file")
  abs_path=$(cd "$skill_dir" && cd "$(dirname "$ref")" 2>/dev/null && pwd)/$(basename "$ref")
  if [ ! -f "$abs_path" ]; then
    echo "BROKEN: $skill_file -> $ref"
    broken=$((broken+1))
  fi
done < <(grep -rn "references/" "$ROOT/skills" --include="SKILL.md" 2>/dev/null || true)
echo "Broken: $broken"
exit $broken
