import { writeSync } from "node:fs";
import { type SessionOptions, withStore } from "../store/session.ts";
import type { StoreError } from "../store/store-error.ts";
import { type App, storePath } from "./app.ts";
import type { Command } from "./command.ts";
import { EXIT } from "./exit-codes.ts";
import { failure, type Output, success } from "./output.ts";
import { parse } from "./parse.ts";

function storeFailure(error: StoreError, path: string): Output {
  if (error === "contended") return failure("Contended", EXIT.refused);
  return failure(`store ${path}: ${error}`, EXIT.store);
}

/** Runs one parsed command against the store at `path` (as configured, which is also how errors name it). */
function execute<W>(a: App<W>, command: Command<W>, values: readonly unknown[], path: string, options: SessionOptions): Output {
  const writes = command.access === "write";
  const session = withStore(
    a.store,
    path,
    writes ? "exclusive" : "shared",
    (world) => {
      const outcome = command.invoke(world, values);
      return { commit: writes && outcome.ok ? outcome.world : undefined, value: outcome };
    },
    options,
  );
  if (!session.ok) return storeFailure(session.error, path);
  const outcome = session.value;
  // Lines are printed only now, after the commit.
  return outcome.ok ? success(outcome.lines) : failure(outcome.refusal, EXIT.refused);
}

/** One whole run of the program, as bytes and an exit code. `path` is the configured store path. */
export function run<W>(a: App<W>, argv: readonly string[], path: string, options: SessionOptions = {}): Output {
  const parsed = parse(a.commands, argv);
  switch (parsed.kind) {
    case "help":
      return parsed.stream === "stdout"
        ? { stdout: a.help, stderr: "", code: EXIT.ok }
        : { stdout: "", stderr: a.help, code: EXIT.usage };
    case "usage":
      return failure(parsed.message, EXIT.usage);
    case "run":
      return execute(a, parsed.command, parsed.values, path, options);
  }
}

function writeAll(fd: number, text: string): void {
  if (text === "") return;
  const bytes = Buffer.from(text, "utf8");
  for (let done = 0; done < bytes.length; ) done += writeSync(fd, bytes, done);
}

/** The process: argv and the environment in, fds 1 and 2 and an exit code out. */
export function main<W>(a: App<W>): never {
  const out = run(a, process.argv.slice(2), storePath(a, process.env));
  writeAll(1, out.stdout);
  writeAll(2, out.stderr);
  process.exit(out.code);
}
