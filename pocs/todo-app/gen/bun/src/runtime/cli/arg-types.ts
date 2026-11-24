// Runtime shim of the bun backend of spec-lang build: argument types. The same in every generated project; change the backend, not this file.

import { isText } from "../text.ts";

/** How one argument becomes a typed value; `undefined` means "not a <name>". */
export interface ArgType<T> {
  readonly name: string;
  /** What `--help` shows after a flag of this type. */
  readonly hint: string;
  readonly parse: (text: string) => T | undefined;
}

const DIGITS = /^-?[0-9]+$/;

/** `Int where ...`: ASCII digits, leading zeros allowed, any length since `Int` is unbounded; then the claim. */
export function intArg<T extends bigint>(name: string, parse: (n: bigint) => T | undefined): ArgType<T> {
  return Object.freeze({
    name,
    hint: `<${name.toLowerCase()}>`,
    parse: (text: string) => (DIGITS.test(text) ? parse(BigInt(text)) : undefined),
  });
}

/** `Text where ...`: the argument as given, then the claim. */
export function textArg<T extends string>(name: string, parse: (s: string) => T | undefined): ArgType<T> {
  return Object.freeze({
    name,
    hint: `<${name.toLowerCase()}>`,
    parse: (text: string) => (isText(text) ? parse(text) : undefined),
  });
}

/** A sum type with no data: its variant names in lower case. */
export function sumArg<T extends string>(name: string, variants: readonly T[]): ArgType<T> {
  const byName: ReadonlyMap<string, T> = new Map(variants.map((v) => [v.toLowerCase(), v]));
  return Object.freeze({ name, hint: [...byName.keys()].join("|"), parse: (text: string) => byName.get(text) });
}
