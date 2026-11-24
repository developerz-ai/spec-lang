// Projected by hand from the cli vocabulary end to end, and law refusal_changes_nothing on disk (docs/cli-vocabulary.md, src/todo/laws.slg, docs/contract.md sections 2 and 3). Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import fc from "fast-check";
import { HELP } from "../../src/cli/help.ts";
import { run } from "../../src/cli/run.ts";
import { lock } from "../../src/store/lock.ts";
import { useTempDir } from "../support/temp-dir.ts";

const dir = useTempDir();

describe("run", () => {
  test("help and usage errors touch no file", () => {
    const path = join(dir(), ".todo.json");
    expect(run([], path)).toEqual({ stdout: "", stderr: HELP, code: 2 });
    expect(run(["add", "-h"], path)).toEqual({ stdout: HELP, stderr: "", code: 0 });
    expect(run(["add"], path)).toEqual({ stdout: "", stderr: "error: missing <title>\n", code: 2 });
    expect(existsSync(`${path}.lock`)).toBe(false);
  });

  test("a refusal prints its name and exits 1", () => {
    const path = join(dir(), ".todo.json");
    expect(run(["done", "1"], path)).toEqual({ stdout: "", stderr: "error: NoTask\n", code: 1 });
  });

  test("store errors name the path as configured", () => {
    const path = join(dir(), ".todo.json");
    writeFileSync(path, "[1]\n");
    expect(run(["list"], path)).toEqual({ stdout: "", stderr: `error: store ${path}: unreadable\n`, code: 3 });
  });

  test("a full store exits 3 and prints nothing on stdout", () => {
    const path = join(dir(), ".todo.json");
    writeFileSync(path, '{"version":1,"next_no":9007199254740992,"tasks":[]}\n');
    expect(run(["add", "b"], path)).toEqual({ stdout: "", stderr: `error: store ${path}: full\n`, code: 3 });
  });

  test("Contended exits 1", () => {
    const path = join(dir(), ".todo.json");
    const held = lock(path, "exclusive");
    if (!held.ok) throw new Error(held.error);
    try {
      expect(run(["stats"], path, { waitMs: 20 })).toEqual({ stdout: "", stderr: "error: Contended\n", code: 1 });
    } finally {
      held.release();
    }
  });

  test("refusal_changes_nothing and reads_change_nothing, on the bytes of the store", () => {
    const words = ["add", "list", "done", "reopen", "rename", "remove", "clear", "stats"];
    const argv = fc.oneof(
      fc.tuple(fc.constantFrom(...words), fc.constantFrom("1", "2", "3", "9", "x", "Ünïcode")),
      fc.tuple(
        fc.constantFrom("list", "add"),
        fc.constantFrom("--all", "--tag", "--tags", "--priority"),
        fc.constantFrom("work", "high", "1"),
      ),
    );
    fc.assert(
      fc.property(fc.array(argv, { minLength: 20, maxLength: 20 }), (commands) => {
        const path = join(dir(), `${crypto.randomUUID()}.json`);
        for (const args of commands) {
          const before = existsSync(path) ? readFileSync(path) : undefined;
          const out = run(args, path);
          const after = existsSync(path) ? readFileSync(path) : undefined;
          const read = args[0] === "list" || args[0] === "stats";
          if (out.code !== 0 || read) expect(after).toEqual(before);
          if (out.code !== 0) expect(out.stdout).toBe("");
        }
      }),
      { seed: 20261124, numRuns: 25 },
    );
  });
});
