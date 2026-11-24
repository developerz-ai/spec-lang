// Runtime shim of the bun backend of spec-lang build: store errors. The same in every generated project; change the backend, not this file.

/** Why a store could not take part in a rule. `contended` is the language's `Contended`; the rest exit 3. */
export type StoreError = "unreadable" | "unwritable" | "full" | "contended";
