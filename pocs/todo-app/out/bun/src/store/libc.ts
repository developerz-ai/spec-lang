// Projected by hand from "one lock around it" of @store (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import { dlopen, FFIType, read } from "bun:ffi";

/** Linux values from <sys/file.h> and <errno.h>. */
export const LOCK_SH = 1;
export const LOCK_EX = 2;
export const LOCK_NB = 4;
export const EINTR = 4;
export const EWOULDBLOCK = 11;

function open() {
  return dlopen("libc.so.6", {
    flock: { args: [FFIType.i32, FFIType.i32], returns: FFIType.i32 },
    __errno_location: { args: [], returns: FFIType.ptr },
  });
}

let lib: ReturnType<typeof open> | undefined;

/** Loaded on the first lock, so help and usage errors never pay for dlopen. */
function libc(): ReturnType<typeof open>["symbols"] {
  lib ??= open();
  return lib.symbols;
}

/** `flock(2)`: 0, or the errno of the failure. */
export function flock(fd: number, operation: number): number {
  const c = libc();
  if (c.flock(fd, operation) === 0) return 0;
  const errno = c.__errno_location();
  if (errno === null) throw new Error("libc returned no errno location");
  return read.i32(errno, 0);
}
