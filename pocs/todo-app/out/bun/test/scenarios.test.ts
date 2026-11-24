// Projected by hand from every test in todo.tests.scenarios (src/todo/tests/scenarios.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import fc from "fast-check";
import { add } from "../src/board/add.ts";
import { clear } from "../src/board/clear.ts";
import { done, reopen } from "../src/board/done.ts";
import { list } from "../src/board/list.ts";
import { remove } from "../src/board/remove.ts";
import { rename } from "../src/board/rename.ts";
import { stats } from "../src/board/stats.ts";
import { taskEquals } from "../src/model/task.ts";
import { TestBoard } from "./support/board.ts";
import { no, tag, tags, task, title } from "./support/values.ts";
import { threeOpenOneDone } from "./support/worlds.ts";

describe("todo.tests.scenarios", () => {
  test("three_open_one_done", () => {
    const b = new TestBoard(threeOpenOneDone());
    expect(b.world.tasks.size).toBe(4);
    expect(b.world.nextNo).toBe(no(5));
    expect(b.outText).toBe("");
  });

  test("numbers_start_at_one", () => {
    const b = new TestBoard();
    expect(b.call((w) => add(w, title("milk")))).toBe(no(1));
    expect(b.call((w) => add(w, title("bread"), "Normal", tags("shop", "home")))).toBe(no(2));
    expect(b.outText).toBe("added 1\nadded 2\n");
  });

  test("numbers_are_never_reused", () => {
    const b = new TestBoard();
    expect(b.call((w) => add(w, title("a")))).toBe(no(1));
    expect(b.call((w) => add(w, title("b")))).toBe(no(2));
    b.call((w) => remove(w, no(2)));
    expect(b.call((w) => add(w, title("c")))).toBe(no(3));
  });

  test("empty_board_says_so", () => {
    const b = new TestBoard();
    b.call((w) => list(w));
    b.call((w) => stats(w));
    expect(b.outText).toBe("nothing to do\n0 open, 0 done\n");
  });

  test("list_ranks_urgent_first", () => {
    const b = new TestBoard(threeOpenOneDone());
    b.call((w) => list(w));
    expect(b.outText).toBe("3 [ ] urgent !high #work\n2 [ ] plain\n1 [ ] low one !low\n");
  });

  test("list_all_and_by_tag", () => {
    const b = new TestBoard(threeOpenOneDone());
    b.call((w) => list(w, true, tag("work")));
    expect(b.outText).toBe("3 [ ] urgent !high #work\n4 [x] finished #home #work\n");
  });

  test("done_and_reopen_move_once", () => {
    const b = new TestBoard(threeOpenOneDone());
    b.call((w) => done(w, no(2)));
    expect(b.refused((w) => done(w, no(2)))).toBe("AlreadyDone");
    b.call((w) => reopen(w, no(2)));
    expect(b.refused((w) => reopen(w, no(2)))).toBe("NotDone");
    expect(b.outText).toBe("2 [x] plain\n2 [ ] plain\n");
  });

  test("rename_keeps_the_rest", () => {
    const b = new TestBoard(threeOpenOneDone());
    b.call((w) => rename(w, no(3), title("urgent, really")));
    const want = task(3, "urgent, really", { priority: "High", tags: tags("work") });
    const got = b.world.tasks.get(no(3));
    expect(got !== undefined && taskEquals(got, want)).toBe(true);
  });

  test("clear_removes_only_done", () => {
    const b = new TestBoard(threeOpenOneDone());
    expect(b.call((w) => clear(w))).toBe(1);
    b.call((w) => stats(w));
    expect(b.outText).toBe("cleared 1\n3 open, 0 done\n");
    expect(b.world.tasks.has(no(4))).toBe(false);
  });

  test("unknown_numbers_are_refused", () => {
    // `no: TaskNo` is generated from its type: any Int >= 1, here up to 2^80 to pass every machine word.
    fc.assert(
      fc.property(fc.bigInt({ min: 1n, max: 2n ** 80n }), (n) => {
        if (n <= 4n) return;
        const b = new TestBoard(threeOpenOneDone());
        expect(b.refused((w) => done(w, no(n)))).toBe("NoTask");
        expect(b.refused((w) => reopen(w, no(n)))).toBe("NoTask");
        expect(b.refused((w) => rename(w, no(n), title("x")))).toBe("NoTask");
        expect(b.refused((w) => remove(w, no(n)))).toBe("NoTask");
        expect(b.outText).toBe("");
      }),
      { seed: 20261124, numRuns: 200 },
    );
  });
});
