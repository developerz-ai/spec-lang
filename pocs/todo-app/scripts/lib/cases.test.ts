import { describe, expect, it } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { CaseSyntaxError, parseCases } from "./cases.ts";

describe("parseCases", () => {
  it("reads every sigil", () => {
    const [c] = parseCases("t", '# case: one\n& TODO_STORE=a.json\n< {}\n$ ["list"]\n> x\n!j "e\\u0085"\n? 3\n= absent\n');
    expect(c?.steps).toEqual([
      { kind: "env", key: "TODO_STORE", value: "a.json" },
      { kind: "seed", text: "{}" },
      { kind: "run", args: ["list"] },
      { kind: "stdout", line: "x" },
      { kind: "stderr", line: "e\u0085" },
      { kind: "exit", code: 3 },
      { kind: "store", text: null },
    ]);
  });

  it("expands @help", () => {
    const [c] = parseCases("t", '# case: h\n$ ["-h"]\n> @help\n');
    expect(c?.steps.filter((s) => s.kind === "stdout")).toHaveLength(11);
  });

  it("refuses a step outside a case and a bad array", () => {
    expect(() => parseCases("t", '$ ["x"]\n')).toThrow(CaseSyntaxError);
    expect(() => parseCases("t", "# case: x\n$ [1]\n")).toThrow(CaseSyntaxError);
  });

  it("parses every committed case file", () => {
    const dir = join(import.meta.dir, "../../parity/cases");
    const n = readdirSync(dir).flatMap((f) => parseCases(f, readFileSync(join(dir, f), "utf8"))).length;
    expect(n).toBeGreaterThan(50);
  });
});
