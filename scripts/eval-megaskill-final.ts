#!/usr/bin/env bun
import { join } from "node:path";
import { runEval } from "./eval-megaskill-common";
runEval(join(import.meta.dir, "../tests/eval/megaskill-final.yaml"), "megaskill-final-baseline.json", { overall: 0.6, perSkill: 0.5, falsePositive: 0.2 });
