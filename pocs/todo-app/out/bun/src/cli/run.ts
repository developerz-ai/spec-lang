// Projected by hand from the cli vocabulary: help, usage errors, dispatch (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { type ExecuteOptions, execute } from "./execute.ts";
import { EXIT } from "./exit-codes.ts";
import { HELP } from "./help.ts";
import { failure, type Output } from "./output.ts";
import { parse } from "./parse.ts";

/** One whole run of `todo`, as bytes and an exit code. `storePath` is the configured path. */
export function run(argv: readonly string[], storePath: string, options: ExecuteOptions = {}): Output {
  const parsed = parse(argv);
  switch (parsed.kind) {
    case "help":
      return parsed.stream === "stdout"
        ? { stdout: HELP, stderr: "", code: EXIT.ok }
        : { stdout: "", stderr: HELP, code: EXIT.usage };
    case "usage":
      return failure(parsed.message, EXIT.usage);
    case "run":
      return execute(parsed.command, parsed.values, storePath, options);
  }
}
