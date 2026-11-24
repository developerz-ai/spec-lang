// The `cli` library's runtime for TypeScript (libs/cli/real/ts): the store of an app's world between runs.
// One rule call against the store: the lock from before the read until after the write, the world read
// strictly (its types, then the program's laws about a world alone), an atomic replace when it changed.

import { readFileSync } from "node:fs";
import * as rt from "../../../runtime/index.ts";
import { replaceAtomically } from "./atomic-write.ts";
import { decodeWorld, encodeWorld, Full, type Stored, Unfit } from "./codec.ts";
import { readJson } from "./json-reader.ts";
import { LOCK_WAIT_MS, type LockMode, lock } from "./lock.ts";
import type { StoreError } from "./store-error.ts";
import { isSystemError } from "./system-error.ts";

export type Session<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: StoreError };

/** Strict UTF-8; a byte-order mark is kept, so the JSON reader refuses it. */
const utf8 = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });

/** A missing store is the initial world; anything else that is not a world of the program is unreadable. */
function load<W extends rt.Machine, V>(
  program: rt.Program<W, V>,
  layout: readonly Stored<W>[],
  w: W,
  path: string,
): { readonly ok: true; readonly bytes: Uint8Array | undefined } | { readonly ok: false } {
  let bytes: Uint8Array;
  try {
    bytes = readFileSync(path);
  } catch (e) {
    if (!isSystemError(e)) throw e;
    return e.code === "ENOENT" ? { ok: true, bytes: undefined } : { ok: false };
  }
  let text: string;
  try {
    text = utf8.decode(bytes);
  } catch (e) {
    if (!(e instanceof TypeError)) throw e;
    return { ok: false };
  }
  const json = readJson(text);
  if (!json.ok) return { ok: false };
  try {
    decodeWorld(layout, json.value, w);
    if (!rt.lawful(program, w, rt.still)) return { ok: false };
  } catch (e) {
    if (e instanceof Unfit || e instanceof rt.Failure || e instanceof rt.Stop) return { ok: false };
    throw e;
  }
  return { ok: true, bytes };
}

/**
 * Runs `body` on the stored world with the lock held from before the read until after the write. `commit` false
 * (a refusal, or a rule that only reads) leaves the store as it is. A shared lock is never followed by a write.
 */
export function withStore<W extends rt.Machine, V, T>(
  program: rt.Program<W, V>,
  layout: readonly Stored<W>[],
  w: W,
  path: string,
  mode: LockMode,
  body: () => { readonly commit: boolean; readonly value: T },
): Session<T> {
  const held = lock(path, mode, LOCK_WAIT_MS);
  if (!held.ok) return held;
  try {
    const loaded = load(program, layout, w, path);
    if (!loaded.ok) return { ok: false, error: "unreadable" };
    const step = body();
    if (!step.commit) return { ok: true, value: step.value };
    if (mode === "shared") throw new Error("a rule under a shared lock tried to write the store");
    let text: string;
    try {
      text = encodeWorld(layout, program.value(w));
    } catch (e) {
      if (e instanceof Full) return { ok: false, error: "full" };
      throw e;
    }
    const bytes = Buffer.from(text, "utf8");
    // Byte-identical: the commit is already on disk, so the replace is skipped.
    if (loaded.bytes !== undefined && bytes.equals(loaded.bytes)) return { ok: true, value: step.value };
    if (!replaceAtomically(path, bytes, process.pid)) return { ok: false, error: "unwritable" };
    return { ok: true, value: step.value };
  } finally {
    held.release();
  }
}
