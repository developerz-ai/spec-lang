// Projected by hand from @on(cli(...)) dispatch and @store (docs/cli-vocabulary.md, src/apps/bun.slg, docs/contract.md sections 2 and 3). Stand-in for spec-lang build output; change the spec, not this file.

import { withStore } from "../store/session.ts";
import type { StoreError } from "../store/store-error.ts";
import type { Command } from "./commands.ts";
import { EXIT } from "./exit-codes.ts";
import { failure, type Output, success } from "./output.ts";

export interface ExecuteOptions {
  readonly waitMs?: number;
}

function storeFailure(error: StoreError, path: string): Output {
  if (error === "contended") return failure("Contended", EXIT.refused);
  return failure(`store ${path}: ${error}`, EXIT.store);
}

/** Runs one parsed command against the store at `path` (as configured, which is also how errors name it). */
export function execute(
  command: Command,
  values: readonly unknown[],
  path: string,
  options: ExecuteOptions = {},
): Output {
  const writes = command.access === "write";
  const session = withStore(
    path,
    writes ? "exclusive" : "shared",
    (world) => {
      const outcome = command.invoke(world, values);
      return { commit: writes && outcome.ok ? outcome.world : undefined, value: outcome };
    },
    options,
  );
  if (!session.ok) return storeFailure(session.error, path);
  const outcome = session.value;
  // Lines are printed only now, after the commit.
  return outcome.ok ? success(outcome.lines) : failure(outcome.refusal, EXIT.refused);
}
