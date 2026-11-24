import { expect, it } from "bun:test";
import { CaseSyntaxError, parseCases } from "./cases.ts";

const none = () => "";

it("reads runs, input, expected lines and agreement lines", () => {
  const [c] = parseCases("t", '# case: one\n< 3 R\n<j "1 J"\n$ ["run"]\n> 0 level meadow\n>~ shot [0-9a-f]{16}\n? 0\n', none);
  expect(c?.steps.map((s) => s.kind)).toEqual(["stdin", "stdin", "run", "stdout", "agree", "exit"]);
});

it("expands @help and reads named inputs", () => {
  const [c] = parseCases("t", "# case: h\n<@ x.txt\n$ []\n! @help\n? 2\n", (n) => `from ${n}`);
  expect(c?.steps[0]).toEqual({ kind: "stdin", text: "from x.txt" });
  expect(c?.steps.filter((s) => s.kind === "stderr").length).toBe(10);
});

it("refuses a step before a case and an unknown sigil", () => {
  expect(() => parseCases("t", '$ ["run"]\n', none)).toThrow(CaseSyntaxError);
  expect(() => parseCases("t", "# case: x\n% what\n", none)).toThrow(CaseSyntaxError);
});

it("reads an environment line for the rest of the case", () => {
  const [c] = parseCases("t", '# case: e\n& DISPLAY=\n$ ["shot"]\n? 3\n', none);
  expect(c?.steps[0]).toEqual({ kind: "env", key: "DISPLAY", value: "" });
});
