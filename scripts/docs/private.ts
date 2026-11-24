// House rule: private stays private. No home directories, machine paths or e-mail addresses in any tracked text.
// Usage: bun scripts/docs/private.ts
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { ROOT } from "../lib/files.ts";

const SKIP = new Set([".git", "node_modules", "target", "build", "dist", "vendor", ".bundle", ".codegraph", "tmp"]);
const TEXT = /\.(md|slg|rs|ts|rb|cpp|hpp|h|py|json|toml|yml|yaml|txt|sh)$|^(bin|exe)\//;
const PATTERNS: [RegExp, string][] = [
  [/\/(home|Users)\/[a-z0-9._-]+\//i, "a home directory path"],
  [/\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/i, "an e-mail address"],
];
const ALLOW = [/noreply@anthropic\.com/, /@[a-z]+\/[a-z-]+/, /example\.(com|org)/];

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    if (SKIP.has(name)) return [];
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return walk(p);
    const rel = relative(ROOT, p);
    return TEXT.test(name) || TEXT.test(rel) ? [rel] : [];
  });
}

const hits: string[] = [];
for (const file of walk(ROOT)) {
  readFileSync(join(ROOT, file), "utf8")
    .split("\n")
    .forEach((text, i) => {
      for (const [re, what] of PATTERNS) {
        const m = re.exec(text);
        if (m && !ALLOW.some((a) => a.test(m[0]))) hits.push(`${file}:${i + 1}: ${what}: ${m[0]}`);
      }
    });
}
for (const h of hits) console.error(h);
console.log(`private: ${hits.length} hits`);
process.exit(hits.length ? 1 : 0);
