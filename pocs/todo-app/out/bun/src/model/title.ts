// Projected by hand from is_title, Title (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { bytes, isText, size, trim } from "./text.ts";

declare const titleBrand: unique symbol;
export type Title = string & { readonly [titleBrand]: true };

/** Printable ASCII: one byte, one code point and one UTF-16 unit each, and the only White_Space in it is U+0020. */
const PRINTABLE_ASCII = /^[\x20-\x7e]*$/;
const NOT_SPACE = /[^ ]/;
/** The line breaks above U+007F that a title refuses. None is printable ASCII, so the fast path never meets one. */
const LINE_BREAK = /[\u0085\u2028\u2029]/;

/**
 * `t.trim().size >= 1 and t.size <= 200 and t.bytes.all(it >= 32 and it != 127)
 * and not t.contains("\u{0085}") and not t.contains("\u{2028}") and not t.contains("\u{2029}")`
 */
export function isTitle(t: string): boolean {
  if (PRINTABLE_ASCII.test(t)) return t.length <= 200 && NOT_SPACE.test(t);
  if (!isText(t)) return false;
  return size(trim(t)) >= 1 && size(t) <= 200 && bytes(t).every((b) => b >= 32 && b !== 127) && !LINE_BREAK.test(t);
}

export function asTitle(t: string): Title | undefined {
  return isTitle(t) ? (t as Title) : undefined;
}
