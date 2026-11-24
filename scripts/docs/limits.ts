// House rule: every doc ends with Limits. The last level-2 heading of each design and POC doc must be "Limits".
// Indexes, agent files and backend notes are exempt. Usage: bun scripts/docs/limits.ts
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { markdownFiles, ROOT } from "../lib/files.ts";
import { headings } from "../lib/markdown.ts";

// Decision records end with "Revisit when" / "Not claimed"; a POC's program.md is a program, not prose.
const EXEMPT = /(^|\/)(README|CLAUDE|AGENTS|SKILL|NOTES|program)\.md$|^docs\/decisions\/|^\.claude\/|\/\.claude\/|^\.github\//;
// Known exceptions, older than this gate. A ratchet: the list may only shrink; a fixed doc must leave it.
const KNOWN = new Set(["docs/idea/01-language-card.md", "docs/idea/27-authorization.md"]);
const RULED = /^(docs|pocs)\//;

const lastIsLimits = (f: string) =>
  headings(readFileSync(join(ROOT, f), "utf8"))
    .filter((h) => h.level === 2)
    .at(-1)?.title === "Limits";

const missing = markdownFiles()
  .filter((f) => RULED.test(f) && !EXEMPT.test(f))
  .filter((f) => !KNOWN.has(f) && !lastIsLimits(f));
const stale = [...KNOWN].filter(lastIsLimits);
for (const f of missing) console.error(`${f}: last section is not "## Limits"`);
for (const f of stale) console.error(`${f}: now ends with Limits; remove it from KNOWN in scripts/docs/limits.ts`);
console.log(`limits: ${missing.length} docs without a closing Limits section, ${KNOWN.size} known exceptions`);
process.exit(missing.length || stale.length ? 1 : 0);
