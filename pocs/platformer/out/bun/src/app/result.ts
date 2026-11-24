// Projected by hand from what a command gives back: standard output, standard error, exit code (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { EXIT_OK, EXIT_SCREEN, EXIT_USAGE } from "../cli/exit-codes.ts";

export type Result = { readonly stdout: string; readonly stderr: string; readonly code: number };

export const printed = (lines: readonly string[]): Result => ({
  stdout: lines.map((l) => `${l}\n`).join(""),
  stderr: "",
  code: EXIT_OK,
});

/** Standard output is empty on every error. */
export const usageError = (message: string): Result => ({
  stdout: "",
  stderr: `error: ${message}\n`,
  code: EXIT_USAGE,
});

export const screenError = (message: string): Result => ({
  stdout: "",
  stderr: `error: ${message}\n`,
  code: EXIT_SCREEN,
});
