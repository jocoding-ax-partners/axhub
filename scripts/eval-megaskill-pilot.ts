#!/usr/bin/env bun
import { join } from "node:path";
import { runEval } from "./eval-megaskill-common";
runEval(join(import.meta.dir, "../tests/eval/megaskill-pilot.yaml"), "megaskill-pilot-baseline.json", { overall: 0.6 });
