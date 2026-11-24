// The `cli` library's runtime for TypeScript (libs/cli/real/ts): the store of an app's world between runs.

/** Why a store could not take part in a rule. `contended` is the language's `Contended`; the rest exit 3. */
export type StoreError = "unreadable" | "unwritable" | "full" | "contended";
