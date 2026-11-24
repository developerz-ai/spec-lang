// Runtime shim of the bun backend of spec-lang build: exit codes. The same in every generated project; change the backend, not this file.

export const EXIT = Object.freeze({ ok: 0, refused: 1, usage: 2, store: 3 } as const);
export type ExitCode = (typeof EXIT)[keyof typeof EXIT];
