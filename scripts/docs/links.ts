// Every relative link in every Markdown file resolves: the file exists, and a #fragment names a heading in it.
// Usage: bun scripts/docs/links.ts
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, join, normalize } from "node:path";
import { markdownFiles, ROOT } from "../lib/files.ts";
import { anchor, headings, links } from "../lib/markdown.ts";

const anchorsOf = new Map<string, Set<string>>();
function anchors(file: string): Set<string> {
  let set = anchorsOf.get(file);
  if (!set) {
    set = new Set(headings(readFileSync(join(ROOT, file), "utf8")).map((h) => anchor(h.title)));
    anchorsOf.set(file, set);
  }
  return set;
}

const dead: string[] = [];
const files = markdownFiles();
for (const file of files) {
  for (const { target, line } of links(readFileSync(join(ROOT, file), "utf8"))) {
    if (/^[a-z]+:/i.test(target)) continue;
    const [path = "", fragment] = target.split("#");
    const resolved = path === "" ? file : normalize(join(dirname(file), decodeURI(path)));
    const where = `${file}:${line}`;
    if (!existsSync(join(ROOT, resolved))) {
      dead.push(`${where}: ${target} -> no such file ${resolved}`);
      continue;
    }
    if (fragment && resolved.endsWith(".md") && statSync(join(ROOT, resolved)).isFile() && !anchors(resolved).has(fragment)) {
      dead.push(`${where}: ${target} -> no heading #${fragment} in ${resolved}`);
    }
  }
}
for (const d of dead) console.error(d);
console.log(`links: ${files.length} files, ${dead.length} dead`);
process.exit(dead.length ? 1 : 0);
