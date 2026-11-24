// The spec-lang runtime of the TypeScript printer: `Text`, a JS string counted by code point.

import { stop } from "./failure.ts";
import type { Int } from "./int.ts";

/** Unicode White_Space, exactly these 25 code points. Not U+FEFF, U+180E or U+200B, which `String#trim` removes. */
const WHITE_SPACE: ReadonlySet<number> = new Set([
  0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006,
  0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000,
]);

const SURROGATE = /[\ud800-\udfff]/;
const encoder = new TextEncoder();

export function isWhiteSpace(codePoint: number): boolean {
  return WHITE_SPACE.has(codePoint);
}

/** Code points, not UTF-16 units. Without a surrogate the two counts are the same. */
export function textSize(t: string): Int {
  if (!SURROGATE.test(t)) return BigInt(t.length);
  let n = 0n;
  for (const _ of t) n++;
  return n;
}

/** Removes White_Space from both ends. Every White_Space code point is one UTF-16 unit. */
export function trim(t: string): string {
  let start = 0;
  let end = t.length;
  while (start < end && WHITE_SPACE.has(t.charCodeAt(start))) start++;
  while (end > start && WHITE_SPACE.has(t.charCodeAt(end - 1))) end--;
  return t.slice(start, end);
}

/** The UTF-8 bytes. */
export function bytes(t: string): readonly Int[] {
  return Array.from(encoder.encode(t), (b) => BigInt(b));
}

/** The first `n` code points; a negative `n` is 0. */
export function takeText(t: string, n: Int): string {
  if (n <= 0n) return "";
  if (!SURROGATE.test(t)) return t.slice(0, n > BigInt(t.length) ? t.length : Number(n));
  return Array.from(t).slice(0, Number(n)).join("");
}

/** All but the first `n` code points; a negative `n` is 0. */
export function dropText(t: string, n: Int): string {
  if (n <= 0n) return t;
  if (!SURROGATE.test(t)) return n > BigInt(t.length) ? "" : t.slice(Number(n));
  return Array.from(t).slice(Number(n)).join("");
}

export function split(t: string, sep: string): readonly string[] {
  if (sep === "") stop("E-TYPE-ARITY", "`split` takes one non-empty Text");
  return t.split(sep);
}

/** Every non-overlapping match, left to right. */
export function replace(t: string, old: string, by: string): string {
  if (old === "") stop("E-TYPE-ARITY", "`replace` takes two Text, the first not empty");
  return t.split(old).join(by);
}

/** UTF-16 order differs from code-point order only where a surrogate meets a unit above U+DFFF. */
export function compareText(a: string, b: string): number {
  if (a === b) return 0;
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const x = a.charCodeAt(i);
    const y = b.charCodeAt(i);
    if (x !== y) return codeOrder(x) - codeOrder(y);
  }
  return a.length - b.length;
}

function codeOrder(unit: number): number {
  return unit >= 0xd800 && unit <= 0xdfff ? unit + 0x2000 : unit >= 0xe000 ? unit - 0x800 : unit;
}

export function textLt(a: string, b: string): boolean {
  return compareText(a, b) < 0;
}

export function textLe(a: string, b: string): boolean {
  return compareText(a, b) <= 0;
}

export function textGt(a: string, b: string): boolean {
  return compareText(a, b) > 0;
}

export function textGe(a: string, b: string): boolean {
  return compareText(a, b) >= 0;
}

/** A control character (general category Cc). */
function isControl(c: number): boolean {
  return c < 0x20 || (c >= 0x7f && c < 0xa0);
}

/** Text as spec-lang source: quoted, with `\" \\ \n \t \{` and `\u{X}` for other control or white space. */
export function quote(t: string): string {
  let out = '"';
  for (const ch of t) {
    const c = ch.codePointAt(0) ?? 0;
    if (ch === '"') out += '\\"';
    else if (ch === "\\") out += "\\\\";
    else if (ch === "\n") out += "\\n";
    else if (ch === "\t") out += "\\t";
    else if (ch === "{") out += "\\{";
    else if (isControl(c) || (WHITE_SPACE.has(c) && c !== 0x20)) out += `\\u{${c.toString(16).toUpperCase()}}`;
    else out += ch;
  }
  return `${out}"`;
}
