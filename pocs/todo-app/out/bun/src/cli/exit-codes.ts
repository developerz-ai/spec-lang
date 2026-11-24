// Projected by hand from the exit codes of the cli vocabulary (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

export const EXIT = Object.freeze({ ok: 0, refused: 1, usage: 2, store: 3 } as const);
export type ExitCode = (typeof EXIT)[keyof typeof EXIT];
