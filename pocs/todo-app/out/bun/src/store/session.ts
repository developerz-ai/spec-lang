// Projected by hand from "load before the rule, one lock around it, an atomic replace after it" of @store (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import type { World } from "../board/world.ts";
import { replaceAtomically } from "./atomic-write.ts";
import { encodeWorld } from "./encode.ts";
import { LOCK_WAIT_MS, type LockMode, lock } from "./lock.ts";
import { readStore } from "./read-store.ts";
import type { StoreError } from "./store-error.ts";

/** What a rule hands back to its store: the world to commit (none for a read or a refusal) and its reply. */
export interface Step<T> {
  readonly commit: World | undefined;
  readonly value: T;
}

export type Session<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: StoreError };

export interface SessionOptions {
  readonly waitMs?: number;
  readonly pid?: number;
}

/**
 * Runs `body` on the stored world with the lock held from before the read until after the write.
 * A shared lock is never followed by a write.
 */
export function withStore<T>(
  path: string,
  mode: LockMode,
  body: (world: World) => Step<T>,
  options: SessionOptions = {},
): Session<T> {
  const held = lock(path, mode, options.waitMs ?? LOCK_WAIT_MS);
  if (!held.ok) return held;
  try {
    const loaded = readStore(path);
    if (!loaded.ok) return loaded;
    const step = body(loaded.world);
    if (step.commit === undefined) return { ok: true, value: step.value };
    if (mode === "shared") throw new Error("a rule under a shared lock tried to write the store");
    const encoded = encodeWorld(step.commit);
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
