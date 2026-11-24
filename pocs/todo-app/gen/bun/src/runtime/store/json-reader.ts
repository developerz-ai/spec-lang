// Runtime shim of the bun backend of spec-lang build: strict JSON reader. The same in every generated project; change the backend, not this file.

/**
 * A strict JSON reader (RFC 8259). JSON.parse cannot be used: it keeps the last of two equal keys
 * and rounds integers above 2^53, and the contract requires both to be refused.
 */
export type Json =
  | { readonly kind: "null" }
  | { readonly kind: "bool"; readonly value: boolean }
  /** `integer` is set when the literal has no fraction and no exponent; it is exact. */
  | { readonly kind: "number"; readonly integer: bigint | undefined }
  | { readonly kind: "string"; readonly value: string }
  | { readonly kind: "array"; readonly items: readonly Json[] }
  | { readonly kind: "object"; readonly entries: ReadonlyMap<string, Json> };

export type JsonResult = { readonly ok: true; readonly value: Json } | { readonly ok: false; readonly reason: string };

/** Deeper than any store; it bounds recursion so hostile input cannot overflow the stack. */
const MAX_DEPTH = 64;
const NUMBER = /-?(?:0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?/y;
const HEX4 = /^[0-9a-fA-F]{4}$/;
const WHITE_SPACE = /[ \t\n\r]*/y;
/** A run of string characters that need no decoding: one regex step instead of a step per character. */
// biome-ignore lint/suspicious/noControlCharactersInRegex: JSON forbids raw U+0000..U+001F inside a string.
const PLAIN_RUN = /[^"\\\x00-\x1f]*/y;
const SIMPLE_ESCAPES: Readonly<Record<string, string>> = Object.freeze({
  '"': '"',
  "\\": "\\",
  "/": "/",
  b: "\b",
  f: "\f",
  n: "\n",
  r: "\r",
  t: "\t",
});

class Malformed extends Error {}

export function readJson(text: string): JsonResult {
  const r = new Reader(text);
  try {
    r.space();
    const value = r.value(0);
    r.space();
    if (!r.atEnd()) r.fail("trailing characters");
    return { ok: true, value };
  } catch (e) {
    if (e instanceof Malformed) return { ok: false, reason: e.message };
    throw e;
  }
}

class Reader {
  private at = 0;
  constructor(private readonly s: string) {}

  atEnd(): boolean {
    return this.at >= this.s.length;
  }

  fail(why: string): never {
    throw new Malformed(`${why} at offset ${this.at}`);
  }

  space(): void {
    WHITE_SPACE.lastIndex = this.at;
    WHITE_SPACE.test(this.s);
    this.at = WHITE_SPACE.lastIndex;
  }

  private expect(c: string): void {
    if (this.s[this.at] !== c) this.fail(`expected ${JSON.stringify(c)}`);
    this.at++;
  }

  private word(w: string): void {
    if (!this.s.startsWith(w, this.at)) this.fail("unknown literal");
    this.at += w.length;
  }

  value(depth: number): Json {
    if (depth > MAX_DEPTH) this.fail("nested too deep");
    const c = this.s[this.at];
    switch (c) {
      case "{":
        return this.object(depth);
      case "[":
        return this.array(depth);
      case '"':
        return { kind: "string", value: this.string() };
      case "t":
        this.word("true");
        return { kind: "bool", value: true };
      case "f":
        this.word("false");
        return { kind: "bool", value: false };
      case "n":
        this.word("null");
        return { kind: "null" };
      default:
        return this.number();
    }
  }

  private object(depth: number): Json {
    this.expect("{");
    const entries = new Map<string, Json>();
    this.space();
    if (this.s[this.at] === "}") {
      this.at++;
      return { kind: "object", entries };
    }
    for (;;) {
      this.space();
      if (this.s[this.at] !== '"') this.fail("expected a key");
      const key = this.string();
      if (entries.has(key)) this.fail(`duplicate key ${JSON.stringify(key)}`);
      this.space();
      this.expect(":");
      this.space();
      entries.set(key, this.value(depth + 1));
      this.space();
      if (this.s[this.at] === "}") {
        this.at++;
        return { kind: "object", entries };
      }
      this.expect(",");
    }
  }

  private array(depth: number): Json {
    this.expect("[");
    const items: Json[] = [];
    this.space();
    if (this.s[this.at] === "]") {
      this.at++;
      return { kind: "array", items };
    }
    for (;;) {
      this.space();
      items.push(this.value(depth + 1));
      this.space();
      if (this.s[this.at] === "]") {
        this.at++;
        return { kind: "array", items };
      }
      this.expect(",");
    }
  }

  private number(): Json {
    NUMBER.lastIndex = this.at;
    const m = NUMBER.exec(this.s);
    if (m === null) this.fail("unexpected character");
    this.at += m[0].length;
    const integer = m[1] === undefined && m[2] === undefined ? BigInt(m[0]) : undefined;
    return { kind: "number", integer };
  }

  private hex4(): number {
    const h = this.s.slice(this.at, this.at + 4);
    if (!HEX4.test(h)) this.fail("bad \\u escape");
    this.at += 4;
    return Number.parseInt(h, 16);
  }

  private string(): string {
    this.expect('"');
    let out = "";
    for (;;) {
      PLAIN_RUN.lastIndex = this.at;
      PLAIN_RUN.test(this.s);
      out += this.s.slice(this.at, PLAIN_RUN.lastIndex);
      this.at = PLAIN_RUN.lastIndex;
      const c = this.s[this.at];
      this.at++;
      if (c === '"') return out;
      if (c === undefined) this.fail("unterminated string");
      if (c !== "\\") this.fail("raw control character in string");
      const e = this.s[this.at] ?? "";
      this.at++;
      if (e === "u") out += this.unicodeEscape();
      else {
        const simple = SIMPLE_ESCAPES[e];
        if (simple === undefined) this.fail("bad escape");
        out += simple;
      }
    }
  }

  /** `\\uXXXX`, joining a surrogate pair; a lone surrogate is not text. */
  private unicodeEscape(): string {
    const u = this.hex4();
    if (u >= 0xdc00 && u <= 0xdfff) this.fail("lone low surrogate");
    if (u < 0xd800 || u > 0xdbff) return String.fromCharCode(u);
    if (!this.s.startsWith("\\u", this.at)) this.fail("lone high surrogate");
    this.at += 2;
    const low = this.hex4();
    if (low < 0xdc00 || low > 0xdfff) this.fail("lone high surrogate");
    return String.fromCharCode(u, low);
  }
}
