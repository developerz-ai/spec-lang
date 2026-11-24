// Projected by hand from the reader of the input script (docs/game-vocabulary.md, docs/contract.md section 2 "The input script"). Stand-in for spec-lang build output; change the spec, not this file.

import { padOf } from "./keys.ts";
import { type Hold, MAX_HOLD, MAX_TICKS, type Script } from "./script.ts";

export type Read = { readonly ok: true; readonly script: Script } | { readonly ok: false; readonly message: string };

/** ASCII digits, value 1 to 100000, leading zeros allowed. Decided on the digits, so no length overflows. */
function holdTicks(text: string): number | null {
  if (!/^[0-9]+$/.test(text)) return null;
  const digits = text.replace(/^0+/, "");
  if (digits.length === 0 || digits.length > String(MAX_HOLD).length) return null;
  const n = Number(digits);
  return n <= MAX_HOLD ? n : null;
}

/** Invalid UTF-8 decodes to U+FFFD, which is not a key letter; a byte-order mark is kept and is not one either. */
const decoder = new TextDecoder("utf-8", { fatal: false, ignoreBOM: true });

/** Standard input, read whole and checked before the first tick. */
export function readScript(input: Uint8Array): Read {
  const holds: Hold[] = [];
  let total = 0;
  const lines = decoder.decode(input).split("\n");
  for (const [i, raw] of lines.entries()) {
    const bad = (why: string): Read => ({ ok: false, message: `input line ${i + 1}: ${why}` });
    const hash = raw.indexOf("#");
    const fields = (hash < 0 ? raw : raw.slice(0, hash)).split(/[ \t]+/).filter((f) => f !== "");
    if (fields.length === 0) continue;
    const [ticksText, keysText] = fields;
    if (fields.length !== 2 || ticksText === undefined || keysText === undefined) return bad("expected <ticks> <keys>");
    const ticks = holdTicks(ticksText);
    if (ticks === null) return bad(`ticks must be 1 to ${MAX_HOLD}`);
    const keys = padOf(keysText);
    if (!keys.ok) return bad(keys.why);
    holds.push({ ticks, pad: keys.pad });
    // Saturating: the total never grows past what the limit needs, whatever the number of lines.
    total = Math.min(total + ticks, MAX_TICKS + 1);
  }
  if (total > MAX_TICKS) return { ok: false, message: `input: more than ${MAX_TICKS} ticks` };
  return { ok: true, script: holds };
}
