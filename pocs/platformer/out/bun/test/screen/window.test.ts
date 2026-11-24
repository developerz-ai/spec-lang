// Projected by hand from shot, the window, load_assets, read_pad and present through the real library (src/hop/screen.slg, docs/contract.md sections 2 to 5). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execute, type Io } from "../../src/app/execute.ts";
import { noPad } from "../../src/model/pad.ts";
import { readPad } from "../../src/screen/read-pad.ts";
import { closeWindow, FLAG_WINDOW_HIDDEN, openWindow } from "../../src/screen/window.ts";
import { assets, display, library, skipWithoutDisplay } from "../support/screen.ts";

skipWithoutDisplay("shot, the window and read_pad");

const env = {
  SKYHOP_RAYLIB: library,
  SKYHOP_ASSETS: assets,
  DISPLAY: process.env.DISPLAY,
  WAYLAND_DISPLAY: process.env.WAYLAND_DISPLAY,
};

function io(stdin: string, over: Record<string, string | undefined> = {}): Io {
  return {
    env: { ...env, ...over },
    stdin: () => new TextEncoder().encode(stdin),
    write: () => {
      throw new Error("only play writes as it goes");
    },
  };
}

const spec = { library, assets, display: true, width: 320, height: 180, title: "Skyhop", flags: FLAG_WINDOW_HIDDEN };

test.skipIf(!display)("shot: the same frame twice gives the same hash, and another frame another", () => {
  const first = execute(["shot"], io(""));
  expect(first.stderr).toBe("");
  expect(first.code).toBe(0);
  expect(first.stdout).toMatch(/^shot [0-9a-f]{16}\n$/);
  expect(execute(["shot", "--at", "0"], io("30 R\n"))).toEqual(first);
  const later = execute(["shot"], io("30 R\n"));
  expect(later.stdout).toMatch(/^shot [0-9a-f]{16}\n$/);
  expect(later.stdout).not.toBe(first.stdout);
  expect(execute(["shot", "--level", "2"], io("")).stdout).not.toBe(first.stdout);
});

test.skipIf(!display)("shot --out writes a 320 by 180 PNG and prints the same hash", () => {
  const dir = mkdtempSync(join(tmpdir(), "skyhop-bun-"));
  try {
    const file = join(dir, "frame.png");
    expect(execute(["shot", "--out", file], io(""))).toEqual(execute(["shot"], io("")));
    const png = readFileSync(file);
    expect([...png.subarray(0, 8)]).toEqual([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    expect([png.readUInt32BE(16), png.readUInt32BE(20)]).toEqual([320, 180]);
    expect(execute(["shot", "--out", join(dir, "no/such/folder.png")], io(""))).toEqual({
      stdout: "",
      stderr: `error: image ${join(dir, "no/such/folder.png")}: cannot write\n`,
      code: 3,
    });
  } finally {
    rmSync(dir, { recursive: true });
  }
});

test.skipIf(!display)("a model that does not load: error: asset <path>: cannot load, exit 3", () => {
  const dir = mkdtempSync(join(tmpdir(), "skyhop-bun-"));
  try {
    expect(execute(["shot"], io("", { SKYHOP_ASSETS: dir }))).toEqual({
      stdout: "",
      stderr: `error: asset ${dir}/out/pip.glb: cannot load\n`,
      code: 3,
    });
  } finally {
    rmSync(dir, { recursive: true });
  }
});

test.skipIf(!display)("read_pad: nothing is held in a window nobody types in", () => {
  const opened = openWindow(spec);
  if (!opened.ok) throw new Error(`window: ${JSON.stringify(opened.error)}`);
  expect(readPad(opened.window.rl)).toEqual(noPad);
  expect(opened.window.assets.timeLoc).toBeGreaterThanOrEqual(0);
  closeWindow(opened.window);
});

test("no display named: error: window: cannot open, exit 3, and raylib is not asked", () => {
  expect(execute(["shot"], io("", { DISPLAY: undefined, WAYLAND_DISPLAY: "" }))).toEqual({
    stdout: "",
    stderr: "error: window: cannot open\n",
    code: 3,
  });
});

test("a library that does not load: exit 3", () => {
  expect(execute(["shot"], io("", { SKYHOP_RAYLIB: "/no/such/libraylib.so" }))).toEqual({
    stdout: "",
    stderr: "error: raylib /no/such/libraylib.so: cannot load\n",
    code: 3,
  });
});
