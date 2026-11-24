// Runtime shim of the bun backend of spec-lang build: failure paths: a failed or interrupted write leaves the old store or the new and no tmp file; locks are released. The same in every generated project; change the backend, not this file.

import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { replaceAtomically } from "../../src/runtime/store/atomic-write.ts";
import { lock } from "../../src/runtime/store/lock.ts";

function inTempDir(body: (dir: string) => void): void {
  const dir = mkdtempSync(join(tmpdir(), "spec-runtime-"));
  try {
    body(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test("a replace that fails leaves the old store and no tmp file", () => {
  inTempDir((dir) => {
    // A directory cannot be renamed over, so the last step fails after the tmp file was written.
    const path = join(dir, "store");
    mkdirSync(path);
    writeFileSync(join(path, "kept"), "old");
    expect(replaceAtomically(path, Buffer.from("new\n"), 7)).toBe(false);
    expect(readdirSync(dir)).toEqual(["store"]);
    expect(readFileSync(join(path, "kept"), "utf8")).toBe("old");
  });
});

test("a replace into a missing directory fails and leaves nothing", () => {
  inTempDir((dir) => {
    expect(replaceAtomically(join(dir, "none", "store"), Buffer.from("new\n"), 7)).toBe(false);
    expect(readdirSync(dir)).toEqual([]);
  });
});

test("a lock is released on every way out, so the next taker never waits", () => {
  inTempDir((dir) => {
    const path = join(dir, "store");
    for (let i = 0; i < 3; i++) {
      const held = lock(path, "exclusive", 0);
      if (!held.ok) throw new Error(held.error);
      // A second taker is refused at once while it is held: the descriptor is the lock.
      expect(lock(path, "exclusive", 0)).toEqual({ ok: false, error: "contended" });
      held.release();
    }
    expect(readdirSync(dir)).toEqual(["store.lock"]);
  });
});

test("an interrupt during a write leaves the old store or the new, whole, and no tmp file", async () => {
  const writer = join(import.meta.dir, "../../src/runtime/store/atomic-write.ts");
  const old = `${"o".repeat(1 << 16)}\n`;
  const fresh = `${"n".repeat(1 << 16)}\n`;
  for (let round = 0; round < 8; round++) {
    const dir = mkdtempSync(join(tmpdir(), "spec-runtime-"));
    try {
      const path = join(dir, "store");
      writeFileSync(path, old);
      const script = `import { replaceAtomically } from ${JSON.stringify(writer)};
        const bytes = Buffer.from(${JSON.stringify(fresh)});
        process.stdout.write("ready\\n");
        for (let i = 0; i < 40; i++) replaceAtomically(${JSON.stringify(path)}, bytes, process.pid);`;
      const child = Bun.spawn(["bun", "-e", script], { stdout: "pipe", stderr: "ignore" });
      await child.stdout.getReader().read();
      await Bun.sleep(round);
      child.kill("SIGINT");
      await child.exited;
      expect([old, fresh]).toContain(readFileSync(path, "utf8"));
      expect(readdirSync(dir)).toEqual(["store"]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  }
}, 30_000);
