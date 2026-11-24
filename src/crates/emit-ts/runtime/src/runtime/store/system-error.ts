/** An error the operating system gave (it carries an errno code), as opposed to a bug in this program. */
export function isSystemError(e: unknown): e is Error & { readonly code: string } {
  return e instanceof Error && typeof (e as { code?: unknown }).code === "string";
}
