// Projected by hand from is_tag, Tag, Set[Tag] (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

declare const tagBrand: unique symbol;
export type Tag = string & { readonly [tagBrand]: true };

/** `Set[Tag]`, held sorted in code-point order and without repeats, so `==` is element-wise. */
export type TagSet = readonly Tag[];

/**
 * `1 <= t.size <= 32 and t.bytes.all(97 <= it <= 122 or 48 <= it <= 57 or it == 45) and not t.starts_with("-")`.
 * Every allowed byte is ASCII, so bytes, code points and UTF-16 units coincide and one regex decides it.
 */
const TAG = /^[a-z0-9][a-z0-9-]{0,31}$/;

export function isTag(t: string): boolean {
  return TAG.test(t);
}

export function asTag(t: string): Tag | undefined {
  return isTag(t) ? (t as Tag) : undefined;
}

/** Tags are ASCII, so the default UTF-16 sort is code-point order. */
export function tagSet(tags: Iterable<Tag>): TagSet {
  return Object.freeze([...new Set(tags)].sort());
}
