import { describe, expect, it } from "bun:test";
import { anchor, headings, links, stripFences } from "./markdown.ts";

describe("markdown", () => {
  it("ignores links and headings inside fences and code spans", () => {
    const text = "# Top\n```md\n[a](x.md)\n# not\n```\n`[b](y.md)` [c](z.md#part)\n## Limits\n";
    expect(links(text)).toEqual([{ target: "z.md#part", line: 6 }]);
    expect(headings(text).map((h) => h.title)).toEqual(["Top", "Limits"]);
  });

  it("keeps line numbers when blanking fences", () => {
    expect(stripFences("a\n```\nb\n```\nc").split("\n")).toHaveLength(5);
  });

  it("makes GitHub anchors", () => {
    expect(anchor("Minimal and sufficient")).toBe("minimal-and-sufficient");
    expect(anchor("`@guard` and `fail`: errors")).toBe("guard-and-fail-errors");
    expect(anchor("Rules, in order")).toBe("rules-in-order");
  });
});
