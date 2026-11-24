import { closeSync, fsyncSync, openSync, renameSync, rmSync, writeSync } from "node:fs";
import { isSystemError } from "./system-error.ts";

const DEFERRED = ["SIGINT", "SIGTERM", "SIGHUP"] as const;

/** A signal that arrives during the commit waits: its handler cannot run before this synchronous code returns. */
function hold(): void {}

/**
 * Writes `<path>.<pid>.tmp` in the same directory, fsyncs it and renames it over `<path>`.
 * Returns false when any step fails. On every way out, failure, bug or signal, the store is
 * either the old bytes or the new, whole, and the tmp file is gone: a signal is deferred until
 * the rename is done, and the tmp file is removed in `finally` unless it was renamed.
 */
export function replaceAtomically(path: string, bytes: Uint8Array, pid: number): boolean {
  const tmp = `${path}.${pid}.tmp`;
  let renamed = false;
  for (const signal of DEFERRED) process.on(signal, hold);
  try {
    const fd = openSync(tmp, "w", 0o666);
    try {
      for (let done = 0; done < bytes.length; ) done += writeSync(fd, bytes, done);
      fsyncSync(fd);
    } finally {
      closeSync(fd);
    }
    renameSync(tmp, path);
    renamed = true;
    return true;
  } catch (e) {
    if (!isSystemError(e)) throw e;
    return false;
  } finally {
    if (!renamed) rmSync(tmp, { force: true });
    for (const signal of DEFERRED) process.off(signal, hold);
  }
}
