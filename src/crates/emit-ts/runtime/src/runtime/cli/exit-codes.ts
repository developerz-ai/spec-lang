export const EXIT = Object.freeze({ ok: 0, refused: 1, usage: 2, store: 3 } as const);
export type ExitCode = (typeof EXIT)[keyof typeof EXIT];
