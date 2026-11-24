// Projected by hand from the key letters of Pad: the first letter of each field, upper case (docs/game-vocabulary.md, docs/contract.md section 2 "The input script"). Stand-in for spec-lang build output; change the spec, not this file.

import { noPad, type Pad } from "../model/pad.ts";

const fieldOf: Readonly<Record<string, keyof Pad>> = { L: "left", R: "right", F: "forward", B: "back", J: "jump" };

export type Keys = { readonly ok: true; readonly pad: Pad } | { readonly ok: false; readonly why: string };

/** `-` (nothing held), or one or more of `L R F B J`, each at most once. Read left to right; the first problem wins. */
export function padOf(keys: string): Keys {
  if (keys === "-") return { ok: true, pad: noPad };
  const held: { -readonly [K in keyof Pad]: boolean } = { ...noPad };
  // By code point, so a key outside ASCII is named whole.
  for (const c of keys) {
    const field = Object.hasOwn(fieldOf, c) ? fieldOf[c] : undefined;
    if (field === undefined) return { ok: false, why: `unknown key "${c}"` };
    if (held[field]) return { ok: false, why: `key "${c}" twice` };
    held[field] = true;
  }
  return { ok: true, pad: held };
}
