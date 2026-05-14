import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { findTopLevelStepCollisions } from "../scripts/skill-doctor-step-numbering";

const REPO_ROOT = join(import.meta.dir, "..");

describe("skill-doctor step-numbering collision detection (FU-3)", () => {
  test("findTopLevelStepCollisions catches duplicate Step 5 in workflow", () => {
    const synthetic = `## Workflow

0. **Init**

5. **First five**

5. **Second five — collision**

## NEVER
`;
    expect(findTopLevelStepCollisions(synthetic)).toEqual([5]);
  });

  test("sub-step like 5.5. is exempt (FU-3 invariant)", () => {
    const synthetic = `## Workflow

0. **Init**

5. **Step five**

5.5. **Sub-step**

6. **Step six**

## NEVER
`;
    expect(findTopLevelStepCollisions(synthetic)).toEqual([]);
  });

  test("H3 subsection numbering (### Dependency install) is exempt", () => {
    const synthetic = `## Workflow

0. **Init**

1. **Step one**

### Dependency install (lockfile-aware)

1. plan 을 조회해요.

2. AskUserQuestion fire.

## NEVER
`;
    expect(findTopLevelStepCollisions(synthetic)).toEqual([]);
  });

  test("Vibe Coder Visibility Rules H2 (FU-2 block) does not trigger workflow scan past it", () => {
    const synthetic = `## Vibe Coder Visibility Rules

5. NOT a workflow step.

## Workflow

0. **Init**

1. **Step one**

## NEVER
`;
    expect(findTopLevelStepCollisions(synthetic)).toEqual([]);
  });

  test("multiple distinct collision numbers detected", () => {
    const synthetic = `## Workflow

3. **First three**

3. **Second three**

5. **First five**

5. **Second five**

## NEVER
`;
    expect(findTopLevelStepCollisions(synthetic)).toEqual([3, 5]);
  });

  test("all shipped SKILLs pass step-numbering check (FU-3 baseline)", () => {
    const skillsDir = join(REPO_ROOT, "skills");
    const slugs = readdirSync(skillsDir).filter((d) => {
      try {
        readFileSync(join(skillsDir, d, "SKILL.md"), "utf8");
        return true;
      } catch {
        return false;
      }
    });
    const failures: { slug: string; collisions: number[] }[] = [];
    for (const slug of slugs) {
      const content = readFileSync(join(skillsDir, slug, "SKILL.md"), "utf8");
      const collisions = findTopLevelStepCollisions(content);
      if (collisions.length > 0) failures.push({ slug, collisions });
    }
    expect(failures).toEqual([]);
  });
});
