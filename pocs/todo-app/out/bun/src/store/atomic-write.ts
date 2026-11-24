// Projected by hand from "an atomic replace after it" of @store (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import { closeSync, fsyncSync, openSync, renameSync, rmSync, writeSync } from "node:fs";
import { isSystemError } from "./system-error.ts";

/**
 * Writes `<path>.<pid>.tmp` in the same directory, fsyncs it and renames it over `<path>`.
 * Returns false when any step fails; the old store is then untouched and the tmp file removed.
 */
export function replaceAtomically(path: string, bytes: Uint8Array, pid: number): boolean {
  const tmp = `${path}.${pid}.tmp`;
  try {
    const fd = openSync(tmp, "w", 0o666);
    try {
      for (let done = 0; done < bytes.length; ) done += writeSync(fd, bytes, done);
      fsyncSync(fd);
    } finally {
      closeSync(fd);
    }
    renameSync(tmp, path);
    return true;
  } catch (e) {
    if (!isSystemError(e)) throw e;
    rmSync(tmp, { force: true });
    return false;
  }
}
