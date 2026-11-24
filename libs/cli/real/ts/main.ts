// The `cli` library's runtime for TypeScript (libs/cli/real/ts): a program its user runs from a shell.
// From the manifest: `@bin(name)` names it, `@store(file(path))` keeps its world (`<BIN>_STORE` moves it), and
// each rule with `@on(cli("name"))` is a command. Exit codes: 0 done, 1 refused, 2 usage, 3 store.

import { writeSync } from "node:fs";
import * as rt from "../../runtime/index.ts";
import { commands, helpText, parse } from "./args.ts";
import { Out } from "./out.ts";
import { layout } from "./store/codec.ts";
import { withStore } from "./store/session.ts";
import type { StoreError } from "./store/store-error.ts";

const EXIT = { ok: 0, refused: 1, usage: 2, store: 3 } as const;

interface Output {
  readonly stdout: string;
  readonly stderr: string;
  readonly code: number;
}

function failure(message: string, code: number): Output {
  return { stdout: "", stderr: `error: ${message}\n`, code };
}

function storeFailure(error: StoreError, path: string): Output {
  if (error === "contended") return failure("Contended", EXIT.refused);
  return failure(`store ${path}: ${error}`, EXIT.store);
}

/** A note's text argument: `@bin("todo")`. */
function text(app: rt.App<unknown>, name: string): string | undefined {
  const v = app.notes.find((n) => n.library === "cli" && n.name === name)?.args[0]?.value;
  return v?.kind === "text" ? v.value : undefined;
}

/** `@store(file(path))`. */
function storeFile(app: rt.App<unknown>): string | undefined {
  const v = app.notes.find((n) => n.library === "cli" && n.name === "store")?.args[0]?.value;
  const path = v?.kind === "variant" && v.name === "file" ? v.args[0]?.value : undefined;
  return path?.kind === "text" ? path.value : undefined;
}

/** One whole run, as bytes and an exit code. */
export function run<W extends rt.Machine, V>(
  app: rt.App<W>,
  program: rt.Program<W, V>,
  libraries: readonly rt.Library[],
  argv: readonly string[],
  env: Readonly<Record<string, string | undefined>>,
): Output {
  const bin = text(app as rt.App<unknown>, "bin") ?? app.name;
  const cmds = commands(app);
  const help = helpText(bin, cmds);
  const parsed = parse(cmds, argv);
  if (parsed.kind === "help") {
    return parsed.stream === "stdout"
      ? { stdout: help, stderr: "", code: EXIT.ok }
      : { stdout: "", stderr: help, code: EXIT.usage };
  }
  if (parsed.kind === "usage") return failure(parsed.message, EXIT.usage);

  const out = new Out();
  const w = program.world({ ...rt.realHandlers(program, libraries), out });
  const rule = parsed.command.rule;
  const configured = env[`${bin.toUpperCase()}_STORE`];
  const file = storeFile(app as rt.App<unknown>);
  const invoke = (): { readonly commit: boolean; readonly value: rt.Reply } => {
    const reply = rt.reply(rule, w, parsed.values);
    return { commit: reply.kind === "Ok" && !rule.reads, value: reply };
  };
  let reply: rt.Reply;
  if (file === undefined) {
    reply = invoke().value;
  } else {
    const path = configured === undefined || configured === "" ? file : configured;
    const session = withStore(program, layout(program.states), w, path, rule.reads ? "shared" : "exclusive", invoke);
    if (!session.ok) return storeFailure(session.error, path);
    reply = session.value;
  }
  if (reply.kind === "Err") return failure(reply.error.name, EXIT.refused);
  // Lines are printed only now, after the commit.
  return { stdout: out.take(), stderr: "", code: EXIT.ok };
}

function writeAll(fd: number, text: string): void {
  if (text === "") return;
  const bytes = Buffer.from(text, "utf8");
  for (let done = 0; done < bytes.length; ) done += writeSync(fd, bytes, done);
}

/** The process: argv and the environment in, standard output and error and an exit code out. */
export function main<W extends rt.Machine, V>(
  app: rt.App<W>,
  program: rt.Program<W, V>,
  libraries: readonly rt.Library[],
): void {
  const out = run(app, program, libraries, process.argv.slice(2), process.env);
  writeAll(1, out.stdout);
  writeAll(2, out.stderr);
  process.exit(out.code);
}
