// The `game` library's runtime for TypeScript (libs/game/real/ts): the script of controls `run`, `draw` and
// `shot` read on standard input. The tick rule's one parameter is a record of Bool fields; each field is a key,
// the field's first letter in upper case. Lifted from pocs/platformer/out/bun/src/script/, made generic.

import * as rt from "../../runtime/index.ts";

/** One line of the script: the same controls for `ticks` ticks. */
export interface Hold {
  readonly ticks: number;
  readonly pad: unknown;
}

export const MAX_HOLD = 100_000;
export const MAX_TICKS = 1_000_000;

export type Read =
  | { readonly ok: true; readonly script: readonly Hold[] }
  | { readonly ok: false; readonly message: string };

/** The controls: a record of Bool fields, by key letter. */
export class Keys {
  private readonly letters: ReadonlyMap<string, number>;

  constructor(private readonly pad: rt.RecordType<unknown>) {
    this.letters = new Map(pad.fields.map((f, i) => [f.name.charAt(0).toUpperCase(), i]));
    if (this.letters.size !== pad.fields.length)
      rt.stop("E-UNSUPPORTED", `the fields of ${pad.name} share a first letter`);
  }

  /** Nothing held. */
  none(): unknown {
    return this.pad.make(this.pad.fields.map(() => false));
  }

  /** The controls with these fields held. */
  of(held: readonly boolean[]): unknown {
    return this.pad.make(held);
  }

  /** `-`, or one or more key letters, each at most once; read left to right, the first problem wins. */
  read(keys: string): { readonly ok: true; readonly pad: unknown } | { readonly ok: false; readonly why: string } {
    if (keys === "-") return { ok: true, pad: this.none() };
    const held = this.pad.fields.map(() => false);
    // By code point, so a key outside ASCII is named whole.
    for (const c of keys) {
      const i = this.letters.get(c);
      if (i === undefined) return { ok: false, why: `unknown key "${c}"` };
      if (held[i]) return { ok: false, why: `key "${c}" twice` };
      held[i] = true;
    }
    return { ok: true, pad: this.of(held) };
  }
}

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
export function readScript(keys: Keys, input: Uint8Array): Read {
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
    const pad = keys.read(keysText);
    if (!pad.ok) return bad(pad.why);
    holds.push({ ticks, pad: pad.pad });
    // Saturating: the total never grows past what the limit needs, whatever the number of lines.
    total = Math.min(total + ticks, MAX_TICKS + 1);
  }
  if (total > MAX_TICKS) return { ok: false, message: `input: more than ${MAX_TICKS} ticks` };
  return { ok: true, script: holds };
}
