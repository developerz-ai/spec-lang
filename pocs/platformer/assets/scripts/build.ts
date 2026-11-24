// Compile every recipe at LOD0 and write <out>/<name>.glb: vertex colour only, no textures, no
// animation. Prints one line per asset (triangles, draw calls, bounds, bytes) and fails on any
// ts23d QA finding. Run: bun scripts/build.ts [out-dir]   (default: out)

import { mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { type Asset, boundsOf, compile, runQa, toGlb } from "@developerz.ai/ts23d";
import { crawler } from "../recipes/crawler/crawler.ts";
import { flag } from "../recipes/flag/flag.ts";
import { gem } from "../recipes/gem/gem.ts";
import { pip } from "../recipes/pip/pip.ts";

const ASSETS: readonly Asset[] = [pip, crawler, gem, flag];

/** Draw calls of a GLB: the primitives of its meshes, read from the JSON chunk. */
function drawCalls(glb: Uint8Array): number {
  const view = new DataView(glb.buffer, glb.byteOffset, glb.byteLength);
  const json = new TextDecoder().decode(glb.subarray(20, 20 + view.getUint32(12, true)));
  const meshes = (JSON.parse(json) as { meshes?: { primitives: unknown[] }[] }).meshes ?? [];
  return meshes.reduce((n, m) => n + m.primitives.length, 0);
}

const out = resolve(process.argv[2] ?? join(import.meta.dir, "..", "out"));
mkdirSync(out, { recursive: true });

let failed = false;
for (const asset of ASSETS) {
  const compiled = compile(asset, { lod: 0 });
  const findings = runQa(compiled);
  for (const f of findings) {
    failed = true;
    console.error(`${asset.name}: ${f.gate}: ${f.message}\n  fix: ${f.fix}`);
  }
  const glb = toGlb(compiled);
  await Bun.write(join(out, `${asset.name}.glb`), glb);
  const meshes = compiled.parts.flatMap((p) => (p.mesh ? [p.mesh] : []));
  const triangles = meshes.reduce((n, m) => n + m.indices.length / 3, 0);
  const size = meshes
    .map((m) => boundsOf(m))
    .reduce((a, b) => ({
      min: [Math.min(a.min[0], b.min[0]), Math.min(a.min[1], b.min[1]), Math.min(a.min[2], b.min[2])],
      max: [Math.max(a.max[0], b.max[0]), Math.max(a.max[1], b.max[1]), Math.max(a.max[2], b.max[2])],
    }));
  const f = (v: readonly number[]): string => v.map((x) => x.toFixed(3)).join(" ");
  console.log(
    `${asset.name}: ${triangles} triangles, ${drawCalls(glb)} draw call(s), ` +
      `min ${f(size.min)} max ${f(size.max)} (recipe X Y Z), ${glb.byteLength} bytes`,
  );
}
if (failed) process.exit(1);
