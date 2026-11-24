// The catalog of scripts. Usage: bun scripts/help.ts
console.log(`bin/setup                         toolchains, deps, build all four backends
bin/build [rust|bun|ruby|cpp]...  build backends
bin/check                         the gate: spec parses, scripts, every backend's gate, parity
bin/todo <backend> <cmd> [args]   run one backend's binary

bun scripts/parity/run.ts [--only rust,bun] [--mix] [--race N]   parity transcripts (parity/README.md)
bun scripts/stats/loc.ts                                          spec lines against projected lines
../../bin/slg-check <path>...                                       parse .slg files and slg fences (repo root tool)`);
