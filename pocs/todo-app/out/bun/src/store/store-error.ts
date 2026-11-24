// Projected by hand from the store errors of the cli vocabulary (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

/** Why a store could not take part in a rule. `contended` is the language's `Contended`; the rest exit 3. */
export type StoreError = "unreadable" | "unwritable" | "full" | "contended";
