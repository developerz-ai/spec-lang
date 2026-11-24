// Projected by hand from @store(file(".todo.json")) (src/apps/bun.slg, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

/** An error the operating system gave (it carries an errno code), as opposed to a bug in this program. */
export function isSystemError(e: unknown): e is Error & { readonly code: string } {
  return e instanceof Error && typeof (e as { code?: unknown }).code === "string";
}
