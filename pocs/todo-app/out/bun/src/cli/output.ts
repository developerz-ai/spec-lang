// Projected by hand from out.line and the error lines of the cli vocabulary (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { EXIT, type ExitCode } from "./exit-codes.ts";

/** Everything one run prints, and how it exits. */
export interface Output {
  readonly stdout: string;
  readonly stderr: string;
  readonly code: ExitCode;
}

export function success(lines: readonly string[]): Output {
  return { stdout: lines.map((l) => `${l}\n`).join(""), stderr: "", code: EXIT.ok };
}

/** Every error is one line on standard error; standard output stays empty. */
export function failure(message: string, code: ExitCode): Output {
  return { stdout: "", stderr: `error: ${message}\n`, code };
}
