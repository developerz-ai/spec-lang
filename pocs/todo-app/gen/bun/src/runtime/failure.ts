// The spec-lang runtime of the TypeScript printer: how evaluation ends without a value.
// The same in every generated project; the printer copies it, never changes it.

/**
 * A language failure: `fail E(data)`, `Missing`, `DivByZero`, `Uneven`. `else`, a guard, a rule's refusal
 * and a test's `-> fail E` handle it.
 */
export class Failure extends Error {
  constructor(
    readonly failure: string,
    readonly data: readonly unknown[] = [],
    /** The type of each item of `data`. */
    readonly types: readonly unknown[] = [],
  ) {
    super(failure);
  }
}

/** Evaluation stopped with a diagnostic code (`E-CLAIM`, `E-RUNTIME`, ...): never handled. */
export class Stop extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(`${code}: ${message}`);
  }
}

/** A `?hole` was reached (`F-BLOCKED`), or a foreign body with no runtime here (`B-FOREIGN`). */
export class Blocked extends Error {
  constructor(
    readonly code: "F-BLOCKED" | "B-FOREIGN",
    message: string,
  ) {
    super(`${code}: ${message}`);
  }
}

/** `fail Name(data)`. */
export function fail(name: string, data: readonly unknown[] = [], types: readonly unknown[] = []): never {
  throw new Failure(name, data, types);
}

/** An absent key or index, an empty `first`, `min` or `max`. */
export function missing(): never {
  throw new Failure("Missing");
}

export function stop(code: string, message: string): never {
  throw new Stop(code, message);
}

/** `while` turns left; only the checks set a limit, so a loop that does not end is a failed check. */
let turns = Number.POSITIVE_INFINITY;

export function limitTurns(n: number): void {
  turns = n;
}

/** One turn of a `while`. */
export function turn(): void {
  if (--turns < 0) stop("E-RUNTIME", "a loop that does not end");
}

/** `?name`: the program is not finished here. */
export function hole(name: string): never {
  throw new Blocked("F-BLOCKED", `reached ?${name}`);
}

/** `value else fallback`: a failure of `value` gives `fallback`. */
export function orElse<T>(value: () => T, fallback: () => T): T {
  try {
    return value();
  } catch (e) {
    if (e instanceof Failure) return fallback();
    throw e;
  }
}

/** A guard's condition: a failure while evaluating it counts as false. */
export function holds(condition: () => boolean): boolean {
  try {
    return condition();
  } catch (e) {
    if (e instanceof Failure) return false;
    throw e;
  }
}

/** The failure a call ended with, or undefined when it gave a value: for `-> fail E` and replies. */
export function outcome<T>(
  call: () => T,
): { readonly ok: true; readonly value: T } | { readonly ok: false; readonly failure: Failure } {
  try {
    return { ok: true, value: call() };
  } catch (e) {
    if (e instanceof Failure) return { ok: false, failure: e };
    throw e;
  }
}
