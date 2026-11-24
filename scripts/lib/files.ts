// The Markdown files a docs gate reads: tracked or new, never build output or dependencies.
import { readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

export const ROOT = join(import.meta.dir, "../..");
const SKIP = new Set([".git", "node_modules", "target", "build", "dist", "vendor", ".bundle", ".codegraph", "tmp"]);

export function markdownFiles(dir = ROOT): string[] {
  return readdirSync(dir)
    .sort()
    .flatMap((name) => {
      if (SKIP.has(name)) return [];
      const p = join(dir, name);
      if (statSync(p).isDirectory()) return markdownFiles(p);
      return name.endsWith(".md") ? [relative(ROOT, p)] : [];
    });
}
