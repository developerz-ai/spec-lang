// The `game` library's runtime for TypeScript (libs/game/real/ts): a fixed-tick game with a window, from the
// app's six fields. `start`'s one parameter is `--level <n>`; `tick` runs once per tick with the controls,
// from `input` in `play` and from a script in `run`, `draw` and `shot`; `frame` draws, `dump` writes the frame as
// text, `report` sums up a run. A refused tick ends the run. Exit codes: 0 done, 2 usage or input, 3 the window.
// The window itself is the `screen` handler's, which a drawing library (raylib) implements.

import { readFileSync, writeSync } from "node:fs";
import * as rt from "../../runtime/index.ts";
import { fnv1a64 } from "./hash.ts";
import { type Hold, Keys, MAX_TICKS, readScript } from "./script.ts";

const EXIT = { ok: 0, usage: 2, screen: 3 } as const;
const SHOT = { width: 320, height: 180 } as const;

interface Output {
  readonly stdout: string;
  readonly stderr: string;
  readonly code: number;
}

const printed = (lines: readonly string[]): Output => ({
  stdout: lines.map((l) => `${l}\n`).join(""),
  stderr: "",
  code: EXIT.ok,
});
const failure = (message: string, code: number): Output => ({ stdout: "", stderr: `error: ${message}\n`, code });

/** What `game` needs of the `screen` handler: a window. */
interface Screen extends rt.Handler {
  open(spec: {
    readonly bin: string;
    readonly width: number;
    readonly height: number;
    readonly title: string;
    readonly hidden: boolean;
    readonly env: Readonly<Record<string, string | undefined>>;
  }): string | undefined;
  fps(n: number): void;
  closing(): boolean;
  frame(draw: () => void): void;
  offscreen<T>(
    width: number,
    height: number,
    draw: () => void,
    out: string | null,
    read: (pixels: Uint8Array) => T,
  ): { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: string };
  close(): void;
}

/** What `game` needs of the `out` handler: the lines a tick sent. */
interface Lines extends rt.Handler {
  take(): string;
}

type Flag = "level" | "at" | "out";
const FLAGS: Readonly<Record<string, readonly Flag[]>> = {
  play: ["level"],
  run: ["level"],
  draw: ["level", "at"],
  shot: ["level", "at", "out"],
};
const HINTS: Readonly<Record<Flag, string>> = { level: "<n>", at: "<tick>", out: "<file.png>" };

/** The app's field `name`: the rule or `fn` it names. */
function field<W>(app: rt.App<W>, name: string): rt.Named<W> | undefined {
  return app.params.find((p) => p.name === name)?.decls[0];
}

function rule<W>(app: rt.App<W>, name: string): rt.RuleInfo<W> {
  const d = field(app, name);
  return d?.kind === "rule" ? d.rule : rt.stop("E-UNSUPPORTED", `game app ${app.name}: \`${name}\` names no rule`);
}

function fn<W>(app: rt.App<W>, name: string): rt.FnInfo<W> {
  const d = field(app, name);
  return d?.kind === "fn" ? d.fn : rt.stop("E-UNSUPPORTED", `game app ${app.name}: \`${name}\` names no fn`);
}

function note<W>(app: rt.App<W>, name: string): readonly rt.NoteValue[] {
  return app.notes.find((n) => n.library === "game" && n.name === name)?.args.map((a) => a.value) ?? [];
}

const int = (v: rt.NoteValue | undefined): number =>
  v?.kind === "int" || v?.kind === "quantity" ? Number(v.value) : 0;
const text = (v: rt.NoteValue | undefined): string => (v?.kind === "text" ? v.value : "");

function base(t: rt.Type): rt.Type {
  if (t instanceof rt.AliasType) return base(t.of);
  if (t instanceof rt.WhereType) return base(t.base);
  return t;
}

function typeName(t: rt.Type): string {
  return t instanceof rt.AliasType || t instanceof rt.RecordType || t instanceof rt.SumType ? t.name : t.key;
}

function help<W>(bin: string, app: rt.App<W>): string {
  const commands = Object.entries(FLAGS).map(
    ([c, flags]) => `  ${c}${flags.map((f) => ` [--${f} ${HINTS[f]}]`).join("")}`,
  );
  const how = fn(app, "input").why ?? "the controls";
  return [
    `usage: ${bin} <command> [options]`,
    "",
    "commands:",
    ...commands,
    "",
    `play opens a window: ${how}, Esc to quit.`,
    "run, draw and shot read a script of controls on standard input.",
    "",
  ].join("\n");
}

type Command = {
  readonly name: string;
  readonly level: unknown;
  readonly at: bigint | null;
  readonly out: string | null;
};

function parse<W>(app: rt.App<W>, argv: readonly string[]): Command | Output | "help" {
  const first = argv[0];
  if (first === undefined) return "help";
  if (argv.includes("--help") || argv.includes("-h")) return "help";
  const allowed = FLAGS[first];
  if (allowed === undefined) return failure(`unknown command "${first}"`, EXIT.usage);
  const start = rule(app, "start");
  const levelParam = start.params[0];
  let level: unknown;
  let at: bigint | null = null;
  let out: string | null = null;
  const seen = new Set<Flag>();
  for (let i = 1; i < argv.length; i++) {
    const arg = argv[i] as string;
    if (!arg.startsWith("--")) return failure(`unexpected argument "${arg}"`, EXIT.usage);
    const flag = allowed.find((f) => `--${f}` === arg);
    if (flag === undefined) return failure(`unknown flag ${arg}`, EXIT.usage);
    const value = argv[++i];
    if (value === undefined) return failure(`${arg} needs a value`, EXIT.usage);
    if (seen.has(flag)) return failure(`${arg} given twice`, EXIT.usage);
    seen.add(flag);
    if (flag === "level") {
      const t = levelParam?.type ?? rt.int;
      level = /^[0-9]+$/.test(value) && base(t) instanceof rt.IntType ? BigInt(value) : undefined;
      if (level === undefined || t.conform(level) !== undefined)
        return failure(`--level: not a ${typeName(t)}`, EXIT.usage);
    } else if (flag === "at") {
      if (!/^[0-9]+$/.test(value)) return failure("--at: not a tick", EXIT.usage);
      at = BigInt(value);
    } else {
      if (value === "") return failure("--out: not a path", EXIT.usage);
      out = value;
    }
  }
  return { name: first, level: level ?? firstValue(levelParam), at, out };
}

/** The level when none is given: the parameter's default, else the least value of its range. */
function firstValue(p: rt.ParamInfo | undefined): unknown {
  if (p === undefined || p.hasDefault) return undefined;
  const t = base(p.type);
  if (t instanceof rt.RangeType) return t.lo;
  return rt.stop("E-UNSUPPORTED", `\`${p.name}\` has no default and no least value`);
}

/** A game being played: the world, its handlers, and the app's declarations. */
class Game<W extends rt.Machine, V> {
  readonly w: W;
  readonly lines: Lines;
  readonly screen: Screen;
  readonly keys: Keys;
  private readonly tickRule: rt.RuleInfo<W>;
  ticks = 0;

  constructor(
    readonly app: rt.App<W>,
    program: rt.Program<W, V>,
    libraries: readonly rt.Library[],
  ) {
    const handlers = rt.realHandlers(program, libraries);
    this.lines = handlers.out as Lines;
    this.screen = handlers.screen as Screen;
    this.w = program.world(handlers);
    this.tickRule = rule(app, "tick");
    const pad = this.tickRule.params[0]?.type;
    const record = pad === undefined ? undefined : base(pad);
    if (!(record instanceof rt.RecordType)) rt.stop("E-UNSUPPORTED", "the tick rule takes no record of controls");
    this.keys = new Keys(record as rt.RecordType<unknown>);
  }

  start(level: unknown): string {
    rule(this.app, "start").call(this.w, [level]);
    return this.lines.take();
  }

  /** One tick; false when refused, which ends the run. */
  tick(pad: unknown): boolean {
    const reply = rt.reply(this.tickRule, this.w, [pad]);
    if (reply.kind === "Err") return false;
    this.ticks++;
    return true;
  }

  /** Plays a script from `start(level)`, stopping at tick `until` or when a tick is refused. */
  play(script: readonly Hold[], level: unknown, until: number, line: (text: string) => void): void {
    line(this.start(level));
    for (const hold of script) {
      for (let i = 0; i < hold.ticks; i++) {
        if (this.ticks >= until) return;
        if (!this.tick(hold.pad)) return;
        line(this.lines.take());
      }
    }
  }

  frame(): void {
    rule(this.app, "frame").call(this.w, []);
  }

  value(name: string): unknown {
    return fn(this.app, name).call(this.w, []);
  }

  screenSpec(
    width: number,
    height: number,
    hidden: boolean,
    bin: string,
    env: Readonly<Record<string, string | undefined>>,
  ) {
    return { bin, width, height, title: text(note(this.app, "window")[2]), hidden, env };
  }
}

/** `--at` as a tick to stop after; any value past the script's limit stops where the limit does. */
const stopAfter = (at: bigint | null): number => (at === null || at > BigInt(MAX_TICKS) ? MAX_TICKS : Number(at));

/** The lines of a value `dump` gives: a list of texts, or one text. */
function linesOf(v: unknown): readonly string[] {
  return Array.isArray(v) ? (v as readonly string[]) : [String(v)];
}

export function run<W extends rt.Machine, V>(
  app: rt.App<W>,
  program: rt.Program<W, V>,
  libraries: readonly rt.Library[],
  argv: readonly string[],
  env: Readonly<Record<string, string | undefined>>,
  stdin: () => Uint8Array,
  write: (text: string) => void,
): Output {
  const binNote = note(app, "bin")[0];
  const bin = binNote === undefined ? app.name : text(binNote);
  const parsed = parse(app, argv);
  if (parsed === "help") {
    const h = help(bin, app);
    return argv.length === 0 ? { stdout: "", stderr: h, code: EXIT.usage } : { stdout: h, stderr: "", code: EXIT.ok };
  }
  if (!("name" in parsed)) return parsed;
  const game = new Game(app, program, libraries);
  const [width, height] = note(app, "window");
  if (parsed.name === "play") {
    const error = game.screen.open(game.screenSpec(int(width), int(height), false, bin, env));
    if (error !== undefined) return failure(error, EXIT.screen);
    const rate = rule(app, "tick").triggers.find((t) => t.library === "game" && t.name === "tick")?.args[0]?.value;
    game.screen.fps(int(rate));
    write(game.start(parsed.level));
    while (!game.screen.closing()) {
      if (game.tick(game.value("input"))) write(game.lines.take());
      game.screen.frame(() => game.frame());
    }
    const summary = `${game.value("report")}\n`;
    game.screen.close();
    return { stdout: summary, stderr: "", code: EXIT.ok };
  }
  const script = readScript(game.keys, stdin());
  if (!script.ok) return failure(script.message, EXIT.usage);
  if (parsed.name === "run") {
    const lines: string[] = [];
    game.play(script.script, parsed.level, MAX_TICKS, (t) => lines.push(t));
    return { stdout: `${lines.join("")}${game.value("report")}\n`, stderr: "", code: EXIT.ok };
  }
  game.play(script.script, parsed.level, stopAfter(parsed.at), () => {});
  if (parsed.name === "draw") return printed(linesOf(game.value("dump")));
  const error = game.screen.open(game.screenSpec(SHOT.width, SHOT.height, true, bin, env));
  if (error !== undefined) return failure(error, EXIT.screen);
  const taken = game.screen.offscreen(SHOT.width, SHOT.height, () => game.frame(), parsed.out, fnv1a64);
  game.screen.close();
  return taken.ok ? printed([`shot ${taken.value}`]) : failure(taken.error, EXIT.screen);
}

function writeAll(fd: number, s: string): void {
  if (s === "") return;
  const bytes = Buffer.from(s, "utf8");
  for (let done = 0; done < bytes.length; ) done += writeSync(fd, bytes, done);
}

/** The process: arguments, environment and standard input in; standard output and error and an exit code out. */
export function main<W extends rt.Machine, V>(
  app: rt.App<W>,
  program: rt.Program<W, V>,
  libraries: readonly rt.Library[],
): void {
  const out = run(
    app,
    program,
    libraries,
    process.argv.slice(2),
    process.env,
    () => readFileSync(0),
    (t) => writeAll(1, t),
  );
  writeAll(1, out.stdout);
  writeAll(2, out.stderr);
  process.exit(out.code);
}
