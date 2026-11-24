/** What a rule gives: `(world, args) -> (world', lines, reply) | error`. A refusal carries no world: nothing changed. */
export type RuleOutcome<W, E extends string, R = undefined> =
  | { readonly ok: true; readonly world: W; readonly lines: readonly string[]; readonly reply: R }
  | { readonly ok: false; readonly refusal: E };

export function commit<W, R>(world: W, lines: readonly string[], reply: R): RuleOutcome<W, never, R> {
  return { ok: true, world, lines, reply };
}

export function refuse<E extends string>(refusal: E): RuleOutcome<never, E, never> {
  return { ok: false, refusal };
}
