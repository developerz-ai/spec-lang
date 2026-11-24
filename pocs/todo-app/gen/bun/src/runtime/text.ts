// Runtime shim of the bun backend of spec-lang build: Text: size, trim, bytes. The same in every generated project; change the backend, not this file.

/** Unicode White_Space, exactly these 25 code points. Not U+FEFF, U+180E or U+200B, which `String#trim` removes. */
const WHITE_SPACE: ReadonlySet<number> = new Set([
  0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006,
  0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000,
]);

const encoder = new TextEncoder();

/** A Text is a sequence of Unicode scalar values; a JS string holding a lone surrogate is not one. */
export function isText(t: string): boolean {
  return t.isWellFormed();
}

const SURROGATE = /[\ud800-\udfff]/;

/** `Text.size`: code points, not UTF-16 units. Without a surrogate the two counts are the same. */
export function size(t: string): number {
  if (!SURROGATE.test(t)) return t.length;
  let n = 0;
  for (const _ of t) n++;
  return n;
}

/** `Text.trim`: removes White_Space from both ends. */
export function trim(t: string): string {
  // Every White_Space code point is one UTF-16 unit, so the two ends decide whether there is anything to do.
  if (t === "" || !(WHITE_SPACE.has(t.charCodeAt(0)) || WHITE_SPACE.has(t.charCodeAt(t.length - 1)))) return t;
  const points = Array.from(t);
  let start = 0;
  let end = points.length;
  while (start < end && WHITE_SPACE.has(points[start]?.codePointAt(0) ?? -1)) start++;
  while (end > start && WHITE_SPACE.has(points[end - 1]?.codePointAt(0) ?? -1)) end--;
  return points.slice(start, end).join("");
}

/** `Text.bytes`: UTF-8. Only defined on well-formed text. */
export function bytes(t: string): readonly number[] {
  return Array.from(encoder.encode(t));
}

/** `t.bytes.all(p)` without the byte array: a unit below U+0080 is its own byte. */
export function everyByte(t: string, holds: (byte: number) => boolean): boolean {
  for (let i = 0; i < t.length; i++) {
    const unit = t.charCodeAt(i);
    if (unit >= 0x80) return encoder.encode(t).every(holds);
    if (!holds(unit)) return false;
  }
  return true;
}

/** `t.bytes.any(p)` without the byte array. */
export function someByte(t: string, holds: (byte: number) => boolean): boolean {
  return !everyByte(t, (byte) => !holds(byte));
}
