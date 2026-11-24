// The catalog of scripts. Usage: bun scripts/help.ts
console.log(`bin/setup                          toolchains, raylib, deps, build all four backends
bin/raylib                         build the pinned raylib into vendor/raylib
bin/build [rust|bun|ruby|cpp]...   build backends, hand-written and generated
bin/regen [--check] [backend]...   rebuild gen/ with spec-lang build, or check it is current
bin/check                          the gate: spec, scripts, assets, every backend's gate, parity
bin/skyhop <backend> <cmd> [args]  run one backend's binary, e.g. bin/skyhop cpp play

bun scripts/parity/run.ts [--only rust,bun]   parity transcripts and pixel agreement (parity/README.md)
bun scripts/stats/loc.ts                       spec lines against projected lines
../../bin/slg-check <path>...                  parse .slg files and slg fences (repo root tool)`);
