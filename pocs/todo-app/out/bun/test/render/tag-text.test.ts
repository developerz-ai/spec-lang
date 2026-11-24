// Projected by hand from @cases on tag_text (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { tagText } from "../../src/render/tag-text.ts";
import { tags } from "../support/values.ts";

test.each([
  [tags(), ""],
  [tags("work", "home"), " #home #work"],
])("tag_text(%j) -> %j", (set, want) => {
  expect(tagText(set)).toBe(want);
});
