// The `cli` library's runtime for TypeScript (libs/cli/real/ts): the `out` handler. A rule's lines wait in
// the handler until the rule commits; a refused rule takes its lines back with its writes.

import * as rt from "../../runtime/index.ts";

export class Out implements rt.Handler {
  private sent: readonly string[] = [];

  line(text: string): null {
    this.sent = [...this.sent, text];
    return null;
  }

  /** Read by tests only; an app has none. */
  text(): string {
    return rt.stop("E-UNSUPPORTED", "`out.text` is read by tests only");
  }

  /** The lines sent since the last take, each ending in a newline; the handler keeps none. */
  take(): string {
    const text = this.sent.map((l) => `${l}\n`).join("");
    this.sent = [];
    return text;
  }

  save(): unknown {
    return this.sent;
  }

  restore(saved: unknown): void {
    this.sent = saved as readonly string[];
  }
}
