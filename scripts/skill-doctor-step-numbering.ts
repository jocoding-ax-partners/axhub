/**
 * FU-3 (Plan F′ follow-up) — top-level step numbering collision detection.
 *
 * Extracted from scripts/skill-doctor.ts so tests can import without triggering
 * the script's top-level immediate execution (process.exit).
 *
 * Detects duplicate top-level step numbers in a SKILL's `## Workflow` section.
 *
 * Exempt:
 *  - Sub-step numbering like `3.5. **...**` (regex `^\d+\.\s+\*\*` won't match `3.5.`)
 *  - H3 subsection numbering inside `### Subsection` blocks
 *    (e.g. dependency install with local `D1./D2./.../D5.` or `1./2./3./4./5.`)
 *
 * Workflow scope ends at:
 *  - `## NEVER`
 *  - `## v0.X`
 *  - `## Additional`
 *  - `## Vibe Coder Visibility` (FU-2 block, separate section)
 */
export function findTopLevelStepCollisions(content: string): number[] {
  const lines = content.split("\n");
  const workflowIdx = lines.findIndex((l) => /^##\s+Workflow\s*$/.test(l));
  if (workflowIdx < 0) return [];

  let inH3 = false;
  const seen = new Map<number, number>();

  for (let i = workflowIdx + 1; i < lines.length; i++) {
    const line = lines[i];
    if (/^##\s+(NEVER|v0\.\d|Additional|Vibe Coder Visibility)/.test(line)) break;
    if (/^###\s/.test(line)) {
      inH3 = true;
      continue;
    }
    if (/^##\s/.test(line)) {
      inH3 = false;
      continue;
    }
    if (inH3) continue;

    const m = line.match(/^(\d+)\.\s+\*\*/);
    if (m) {
      const n = parseInt(m[1], 10);
      seen.set(n, (seen.get(n) ?? 0) + 1);
    }
  }

  const collisions: number[] = [];
  for (const [num, count] of seen) {
    if (count > 1) collisions.push(num);
  }
  return collisions.sort((a, b) => a - b);
}
