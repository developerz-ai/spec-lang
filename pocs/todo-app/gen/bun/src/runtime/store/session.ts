// Runtime shim of the bun backend of spec-lang build: load, lock, atomic replace. The same in every generated project; change the backend, not this file.

import { readFileSync } from "node:fs";
import { own } from "../values.ts";
import { replaceAtomically } from "./atomic-write.ts";
import type { StoreCodec } from "./codec.ts";
import { LOCK_WAIT_MS, type LockMode, lock } from "./lock.ts";
import type { StoreError } from "./store-error.ts";
import { isSystemError } from "./system-error.ts";

/** Where every state of the app lives between runs: its first world and its file format. */
export interface Store<W> {
  readonly initial: W;
  readonly codec: StoreCodec<W>;
}

/** What a rule hands back to its store: the world to commit (none for a read or a refusal) and its reply. */
export interface Step<W, T> {
  readonly commit: W | undefined;
  readonly value: T;
}

export type Session<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: StoreError };

export interface SessionOptions {
  readonly waitMs?: number;
  readonly pid?: number;
}

/** `bytes` is what was on disk, or undefined for a missing store, so a write can tell whether it changes anything. */
type Loaded<W> =
  | { readonly ok: true; readonly world: W; readonly bytes: Uint8Array | undefined }
  | { readonly ok: false; readonly error: "unreadable" };

/** Strict UTF-8; a byte-order mark is kept, so the JSON reader refuses it. */
const utf8 = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });

/** A missing store is the initial world; anything else that is not a valid store is unreadable. */
export function readStore<W>(store: Store<W>, path: string): Loaded<W> {
  let bytes: Uint8Array;
  try {
    bytes = readFileSync(path);
  } catch (e) {
    if (!isSystemError(e)) throw e;
    if (e.code === "ENOENT") return { ok: true, world: store.initial, bytes: undefined };
    return { ok: false, error: "unreadable" };
  }
  let text: string;
  try {
    text = utf8.decode(bytes);
  } catch (e) {
    if (!(e instanceof TypeError)) throw e;
    return { ok: false, error: "unreadable" };
  }
  const world = store.codec.decode(text);
  // The world was built here for this one rule call, so the rule may draft its maps in place.
  return world === undefined ? { ok: false, error: "unreadable" } : { ok: true, world: own(world), bytes };
}

/**
 * Runs `body` on the stored world with the lock held from before the read until after the write.
 * A shared lock is never followed by a write.
 */
export function withStore<W, T>(
  store: Store<W>,
  path: string,
  mode: LockMode,
  body: (world: W) => Step<W, T>,
  options: SessionOptions = {},
): Session<T> {
  const held = lock(path, mode, options.waitMs ?? LOCK_WAIT_MS);
  if (!held.ok) return held;
  try {
    const loaded = readStore(store, path);
    if (!loaded.ok) return loaded;
    const step = body(loaded.world);
    if (step.commit === undefined) return { ok: true, value: step.value };
    if (mode === "shared") throw new Error("a rule under a shared lock tried to write the store");
    const encoded = store.codec.encode(step.commit);
    if (!encoded.ok) return encoded;
    const bytes = Buffer.from(encoded.text, "utf8");
    // Byte-identical: the commit is already on disk, so the replace is skipped.
    if (loaded.bytes !== undefined && bytes.equals(loaded.bytes)) return { ok: true, value: step.value };
    if (!replaceAtomically(path, bytes, options.pid ?? process.pid)) return { ok: false, error: "unwritable" };
    return { ok: true, value: step.value };
  } finally {
    held.release();
  }
}
