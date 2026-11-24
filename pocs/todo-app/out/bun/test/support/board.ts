// Projected by hand from the scenario semantics of tests and the out handler (docs/idea/01-language-card.md "Tests", docs/cli-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.

import type { Outcome, Refusal } from "../../src/board/outcome.ts";
import { EMPTY_WORLD, type World, worldEquals } from "../../src/board/world.ts";

/**
 * A test's world: rules are called directly, `out.line` collects into `outText`.
 * `call` is "when", its return value is "then"; `refused` is `-> fail E`.
 */
export class TestBoard {
  private lines: string[] = [];
  constructor(public world: World = EMPTY_WORLD) {}

  call<R>(rule: (w: World) => Outcome<R>): R {
    const outcome = rule(this.world);
    if (!outcome.ok) throw new Error(`expected success, got fail ${outcome.refusal}`);
    this.world = outcome.world;
    this.lines.push(...outcome.lines);
    return outcome.reply;
  }

  refused<R>(rule: (w: World) => Outcome<R>): Refusal {
    const before = this.world;
    const outcome = rule(this.world);
    if (outcome.ok) throw new Error(`expected a refusal, got ${JSON.stringify(outcome.lines)}`);
    if (!worldEquals(before, this.world)) throw new Error("a refusal changed the world");
    return outcome.refusal;
  }

  get outText(): string {
    return this.lines.map((l) => `${l}\n`).join("");
  }
}
