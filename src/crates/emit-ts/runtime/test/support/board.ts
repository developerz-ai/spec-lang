import type { RuleOutcome } from "../../src/runtime/outcome.ts";
import { equals } from "../../src/runtime/values.ts";

/**
 * A test's world: rules are called directly, `out.line` collects into `outText`.
 * `call` is "when", its return value is "then"; `refused` is `-> fail E`. Not a mock: the rules are the real ones.
 */
export class TestBoard<W> {
  private lines: string[] = [];
  constructor(public world: W) {}

  call<R>(rule: (w: W) => RuleOutcome<W, string, R>): R {
    const outcome = rule(this.world);
    if (!outcome.ok) throw new Error(`expected success, got fail ${outcome.refusal}`);
    this.world = outcome.world;
    this.lines.push(...outcome.lines);
    return outcome.reply;
  }

  refused<R>(rule: (w: W) => RuleOutcome<W, string, R>): string {
    const before = this.world;
    const outcome = rule(this.world);
    if (outcome.ok) throw new Error(`expected a refusal, got ${JSON.stringify(outcome.lines)}`);
    if (!equals(before, this.world)) throw new Error("a refusal changed the world");
    return outcome.refusal;
  }

  /** A test that starts from another test's end state starts with nothing printed. */
  forgetOutput(): void {
    this.lines = [];
  }

  get outText(): string {
    return this.lines.map((l) => `${l}\n`).join("");
  }
}
