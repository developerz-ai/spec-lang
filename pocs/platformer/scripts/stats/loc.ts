// Lines of spec against lines of projected code, per backend. Counts non-blank lines.
// Usage: bun scripts/stats/loc.ts
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = join(import.meta.dir, "../..");
const SKIP = new Set(["node_modules", "target", "build", "dist", "vendor", "assets", ".bundle", ".git"]);
const CODE = /\.(slg|rs|ts|rb|cpp|hpp|h|cc|c|vs|fs)$|^skyhop$/;
const TEST = /(^|\/)(tests?|spec_tests)\/|_test\.|\.test\.|(^|\/)test_/;

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (SKIP.has(name)) return [];
    return statSync(p).isDirectory() ? walk(p) : CODE.test(name) ? [p] : [];
  });
}

function lines(file: string): number {
  return readFileSync(file, "utf8")
    .split("\n")
    .filter((l) => l.trim() !== "").length;
}

function count(dir: string) {
  let product = 0;
  let tests = 0;
  for (const f of walk(join(ROOT, dir))) {
    if (TEST.test(relative(ROOT, f))) tests += lines(f);
    else product += lines(f);
  }
  return { product, tests };
}

const rows = [["src (spec-lang)", count("src")], ...["rust", "bun", "ruby", "cpp"].map((b) => [b, count(`out/${b}`)] as const)] as const;
console.log("| source | product lines | test lines |\n|---|---|---|");
for (const [name, c] of rows) console.log(`| ${name} | ${c.product} | ${c.tests} |`);
console.log(
  "\nNon-blank lines; a file counts as test by path (tests/, test/, *_test.*, *.test.*). Inline Rust #[cfg(test)] modules count as product.",
);
