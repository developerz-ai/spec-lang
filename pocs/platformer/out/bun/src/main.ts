// Projected by hand from app SkyhopBun with @bin("skyhop") (src/apps/bun.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { readFileSync, writeSync } from "node:fs";
import { execute } from "./app/execute.ts";

function writeAll(fd: number, text: string): void {
  if (text === "") return;
  const bytes = Buffer.from(text, "utf8");
  for (let done = 0; done < bytes.length; ) done += writeSync(fd, bytes, done);
}

const result = execute(process.argv.slice(2), {
  env: process.env,
  stdin: () => readFileSync(0),
  write: (text) => writeAll(1, text),
});
writeAll(1, result.stdout);
writeAll(2, result.stderr);
process.exit(result.code);
