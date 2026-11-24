// CI must run bin/check and nothing weaker: green locally means green in CI.
import { expect, it } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "../..");
const workflow = readFileSync(join(ROOT, "../../.github/workflows/todo-app.yml"), "utf8");

it("CI runs the local gate itself", () => {
  expect(workflow).toContain("pocs/todo-app/bin/check");
});

it("CI installs every toolchain bin/setup requires", () => {
  const setup = readFileSync(join(ROOT, "bin/setup"), "utf8");
  const needed = [...setup.matchAll(/^need (\S+)/gm)].map((m) => m[1]);
  const provides: Record<string, RegExp> = {
    bun: /setup-bun/,
    uv: /setup-uv/,
    cargo: /rust-toolchain|rustup/,
    ruby: /setup-ruby/,
    cmake: /cmake/,
    "c++": /g\+\+|clang|build-essential|ubuntu/,
  };
  for (const tool of needed) expect(workflow).toMatch(provides[tool as string] ?? new RegExp(`never-${tool}`));
});

it("third-party actions are pinned to a full commit SHA", () => {
  const uses = [...workflow.matchAll(/uses:\s*(\S+)/g)].map((m) => m[1] as string);
  expect(uses.length).toBeGreaterThan(0);
  for (const u of uses) expect(u).toMatch(/@[0-9a-f]{40}$/);
});
