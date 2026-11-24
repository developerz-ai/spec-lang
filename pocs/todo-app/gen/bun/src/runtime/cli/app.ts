// Runtime shim of the bun backend of spec-lang build: the app record and its store path. The same in every generated project; change the backend, not this file.

import type { Store } from "../store/session.ts";
import { type Command, helpText } from "./command.ts";

/** Everything the `app` declaration fixes: its name, its commands, where its state lives. */
export interface App<W> {
  readonly bin: string;
  readonly help: string;
  readonly commands: ReadonlyMap<string, Command<W>>;
  readonly store: Store<W>;
  /** `@store(file(...))`, relative to the working directory. */
  readonly storeFile: string;
  /** The environment variable that replaces it; an empty value counts as unset. */
  readonly storeEnv: string;
}

export function app<W>(
  bin: string,
  commands: readonly Command<W>[],
  store: Store<W>,
  storeFile: string,
  storeEnv: string,
): App<W> {
  return Object.freeze({
    bin,
    help: helpText(bin, commands),
    commands: new Map(commands.map((c) => [c.name, c])),
    store,
    storeFile,
    storeEnv,
  });
}

/** The store path as configured, never resolved: errors print it this way. */
export function storePath<W>(a: App<W>, env: Readonly<Record<string, string | undefined>>): string {
  const configured = env[a.storeEnv];
  return configured === undefined || configured === "" ? a.storeFile : configured;
}
