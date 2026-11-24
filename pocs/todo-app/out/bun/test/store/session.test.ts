// Projected by hand from "load before the rule, one lock around it, an atomic replace after it" (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { add } from "../../src/board/add.ts";
import { EMPTY_WORLD, worldEquals } from "../../src/board/world.ts";
import { lock } from "../../src/store/lock.ts";
import { withStore } from "../../src/store/session.ts";
import { useTempDir } from "../support/temp-dir.ts";
import { no, title } from "../support/values.ts";

const dir = useTempDir();
const CANONICAL_ONE =
  '{"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}]}\n';

function addA(path: string) {
  return withStore(path, "exclusive", (w) => {
    const o = add(w, title("a"));
    return { commit: o.ok ? o.world : undefined, value: o.ok ? o.reply : undefined };
  });
}

describe("store session", () => {
  test("a missing store is the initial world, and a read creates only the lock", () => {
    const path = join(dir(), ".todo.json");
    const r = withStore(path, "shared", (w) => ({ commit: undefined, value: worldEquals(w, EMPTY_WORLD) }));
    expect(r).toEqual({ ok: true, value: true });
    expect(readdirSync(dir())).toEqual([".todo.json.lock"]);
  });

  test("a write replaces the store with canonical bytes and leaves no tmp file", () => {
    const path = join(dir(), ".todo.json");
    expect(addA(path)).toEqual({ ok: true, value: no(1) });
    expect(readFileSync(path, "utf8")).toBe(CANONICAL_ONE);
    expect(readdirSync(dir()).sort()).toEqual([".todo.json", ".todo.json.lock"]);
  });

  test("a commit equal to the bytes on disk is not rewritten", () => {
    const path = join(dir(), ".todo.json");
    writeFileSync(path, CANONICAL_ONE);
    const before = statSync(path).ino;
    const r = withStore(path, "exclusive", (w) => ({ commit: w, value: null }));
    expect(r.ok).toBe(true);
    expect(statSync(path).ino).toBe(before);
  });

  test("a commit that changes nothing still makes a non-canonical store canonical", () => {
    const path = join(dir(), ".todo.json");
    writeFileSync(path, '{ "version": 1, "next_no": 1, "tasks": [] }');
    withStore(path, "exclusive", (w) => ({ commit: w, value: null }));
    expect(readFileSync(path, "utf8")).toBe('{"version":1,"next_no":1,"tasks":[]}\n');
  });

  test("an unreadable store is reported and left as it was", () => {
    const path = join(dir(), ".todo.json");
    writeFileSync(path, "{}\n");
    expect(addA(path)).toEqual({ ok: false, error: "unreadable" });
    expect(readFileSync(path, "utf8")).toBe("{}\n");
  });

  test("a directory where the store should be is unreadable", () => {
    const path = dir();
    expect(withStore(path, "shared", () => ({ commit: undefined, value: null }))).toEqual({
      ok: false,
      error: "unreadable",
    });
  });

  test("a lock that cannot be created is unwritable", () => {
    const path = join(dir(), "missing", "store.json");
    expect(addA(path)).toEqual({ ok: false, error: "unwritable" });
    expect(existsSync(join(dir(), "missing"))).toBe(false);
  });

  test("full: past 2^53 nothing is written", () => {
    const path = join(dir(), ".todo.json");
    const full = '{"version":1,"next_no":9007199254740992,"tasks":[]}\n';
    writeFileSync(path, full);
    expect(addA(path)).toEqual({ ok: false, error: "full" });
    expect(readFileSync(path, "utf8")).toBe(full);
  });

  test("an exclusive lock held elsewhere is contended after the wait", () => {
    const path = join(dir(), ".todo.json");
    const held = lock(path, "exclusive");
    if (!held.ok) throw new Error(held.error);
    try {
      const started = performance.now();
      const r = withStore(path, "shared", () => ({ commit: undefined, value: null }), { waitMs: 50 });
      expect(r).toEqual({ ok: false, error: "contended" });
      expect(performance.now() - started).toBeGreaterThanOrEqual(50);
    } finally {
      held.release();
    }
  });

  test("shared locks do not exclude each other; they do exclude a writer", () => {
    const path = join(dir(), ".todo.json");
    const reader = lock(path, "shared");
    if (!reader.ok) throw new Error(reader.error);
    try {
      const second = lock(path, "shared", 0);
      expect(second.ok).toBe(true);
      if (second.ok) second.release();
      expect(lock(path, "exclusive", 30)).toEqual({ ok: false, error: "contended" });
    } finally {
      reader.release();
    }
    const writer = lock(path, "exclusive", 0);
    expect(writer.ok).toBe(true);
    if (writer.ok) writer.release();
  });
});
