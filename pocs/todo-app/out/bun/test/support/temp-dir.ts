// Projected by hand from "Tests of the store use a temporary directory" (docs/contract.md section 4). Stand-in for spec-lang build output; change the spec, not this file.

import { afterEach } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/** A fresh directory per test, removed after it. */
export function useTempDir(): () => string {
  let dir: string | undefined;
  afterEach(() => {
    if (dir !== undefined) rmSync(dir, { recursive: true, force: true });
    dir = undefined;
  });
  return () => {
    dir ??= mkdtempSync(join(tmpdir(), "todo-bun-"));
    return dir;
  };
}
