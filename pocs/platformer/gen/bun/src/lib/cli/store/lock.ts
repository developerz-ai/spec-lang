// The `cli` library's runtime for TypeScript (libs/cli/real/ts): the store of an app's world between runs.

import { closeSync, constants, openSync } from "node:fs";
import { EINTR, EWOULDBLOCK, flock, LOCK_EX, LOCK_NB, LOCK_SH } from "./libc.ts";
import type { StoreError } from "./store-error.ts";
import { isSystemError } from "./system-error.ts";

export type LockMode = "shared" | "exclusive";

export const LOCK_WAIT_MS = 5000;
const RETRY_MS = 10;

export type Locked =
  | { readonly ok: true; readonly release: () => void }
  | { readonly ok: false; readonly error: StoreError };

/**
 * `flock(2)` on `<path>.lock`, created if missing and never deleted. Tries without blocking every 10 ms
 * until `waitMs` has passed, then `contended`. Closing the descriptor releases the lock.
 */
export function lock(path: string, mode: LockMode, waitMs: number = LOCK_WAIT_MS): Locked {
  let fd: number;
  try {
    fd = openSync(`${path}.lock`, constants.O_RDWR | constants.O_CREAT, 0o666);
  } catch (e) {
    if (!isSystemError(e)) throw e;
    return { ok: false, error: "unwritable" };
  }
  const operation = (mode === "shared" ? LOCK_SH : LOCK_EX) | LOCK_NB;
  const deadline = performance.now() + waitMs;
  for (;;) {
    const errno = flock(fd, operation);
    if (errno === 0) return { ok: true, release: () => closeSync(fd) };
    if (errno === EINTR) continue;
    if (errno !== EWOULDBLOCK || performance.now() >= deadline) {
      closeSync(fd);
      return { ok: false, error: errno === EWOULDBLOCK ? "contended" : "unwritable" };
    }
    Bun.sleepSync(RETRY_MS);
  }
}
