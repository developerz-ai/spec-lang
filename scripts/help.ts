// The catalog. Usage: bun scripts/help.ts
console.log(`Repository
  bin/setup                  check toolchains, install script deps, set up every code area
  bin/check [docs|src|pocs]  the gate; no argument runs all three. CI runs the same commands
  bin/slg-check [path...]    parse .slg files and slg fences (default: docs pocs)

Docs gates (bin/check docs runs all of them)
  bun scripts/docs/links.ts    every relative link and #fragment resolves
  bun scripts/docs/limits.ts   every design and POC doc ends with "## Limits"
  bun scripts/docs/private.ts  no home paths or e-mail addresses in tracked text

Code areas, each with its own CLAUDE.md, bin/check and CI workflow
  src/                 the toolchain (Rust)
  pocs/todo-app/       one spec, four hand-projected backends; bun pocs/todo-app/scripts/help.ts`);
