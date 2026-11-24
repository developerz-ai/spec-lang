// Projected by hand from Text.size, Text.trim, Text.bytes (docs/idea/28-standard-library.md, docs/contract.md section 1). Stand-in for spec-lang build output; change the spec, not this file.

/** Unicode White_Space, exactly the 25 code points of contract section 1. Not U+FEFF, U+180E or U+200B. */
const WHITE_SPACE: ReadonlySet<number> = new Set([
  0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006,
  0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000,
]);

const encoder = new TextEncoder();

/** A Text is a sequence of Unicode scalar values; a JS string holding a lone surrogate is not one. */
export function isText(t: string): boolean {
  return t.isWellFormed();
}

/** `Text.size`: code points, not UTF-16 units. */
export function size(t: string): number {
  let n = 0;
  for (const _ of t) n++;
  return n;
}

export function isWhiteSpace(codePoint: number): boolean {
  return WHITE_SPACE.has(codePoint);
}

/** `Text.trim`: removes White_Space from both ends. */
export function trim(t: string): string {
  const points = Array.from(t);
  let start = 0;
  let end = points.length;
  while (start < end && isWhiteSpace(points[start]?.codePointAt(0) ?? -1)) start++;
  while (end > start && isWhiteSpace(points[end - 1]?.codePointAt(0) ?? -1)) end--;
  return points.slice(start, end).join("");
}

/** `Text.bytes`: UTF-8. Only defined on well-formed text. */
export function bytes(t: string): Uint8Array {
  return encoder.encode(t);
}
