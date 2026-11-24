// The `cli` library's runtime for TypeScript (libs/cli/real/ts): what the store keeps when something fails.

import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import * as rt from "../../../runtime/index.ts";
import { replaceAtomically } from "./atomic-write.ts";
import { decode, encode, Full, MAX_INT, Unfit } from "./codec.ts";
import { readJson } from "./json-reader.ts";
import { lock } from "./lock.ts";

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
  const writer = join(import.meta.dir, "atomic-write.ts");
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

test("a value past what the store holds is refused, never rounded", () => {
  expect(encode(rt.int, MAX_INT)).toBe("9007199254740992");
  expect(() => encode(rt.int, MAX_INT + 1n)).toThrow(Full);
  expect(decode(rt.int, { kind: "number", integer: MAX_INT })).toBe(MAX_INT);
  expect(() => decode(rt.int, { kind: "number", integer: MAX_INT + 1n })).toThrow(Unfit);
});

test("a map keyed by its records' first field is their array; any other is pairs; both read back", () => {
  interface Item {
    readonly no: bigint;
  }
  const item = rt.record<Item>("m.Item", () => [rt.field("no", "no", rt.int)]);
  const type = rt.map(rt.int, item);
  const keyed = rt.OrdMap.of<bigint, Item>(rt.int, [[2n, { no: 2n }]]);
  const other = rt.OrdMap.of<bigint, Item>(rt.int, [[1n, { no: 2n }]]);
  for (const [value, text] of [
    [keyed, '[{"no":2}]'],
    [other, '[[1,{"no":2}]]'],
  ] as const) {
    expect(encode(type, value)).toBe(text);
    const json = readJson(text);
    if (!json.ok) throw new Error(json.reason);
    expect(rt.equal(decode(type, json.value), value)).toBe(true);
  }
});
