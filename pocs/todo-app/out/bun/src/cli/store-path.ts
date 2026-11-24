// Projected by hand from @store(file(".todo.json")) (src/apps/bun.slg, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

export const DEFAULT_STORE = ".todo.json";

/** `TODO_STORE` replaces the default; an empty value counts as unset. Relative to the working directory. */
export function storePath(env: Readonly<Record<string, string | undefined>>): string {
  const configured = env.TODO_STORE;
  return configured === undefined || configured === "" ? DEFAULT_STORE : configured;
}
