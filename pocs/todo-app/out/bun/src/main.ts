// Projected by hand from app TodoBun with @bin("todo") (src/apps/bun.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { writeSync } from "node:fs";
import { run } from "./cli/run.ts";
import { storePath } from "./cli/store-path.ts";

function writeAll(fd: number, text: string): void {
  if (text === "") return;
  const bytes = Buffer.from(text, "utf8");
  for (let done = 0; done < bytes.length; ) done += writeSync(fd, bytes, done);
}

const out = run(process.argv.slice(2), storePath(process.env));
writeAll(1, out.stdout);
writeAll(2, out.stderr);
process.exit(out.code);
