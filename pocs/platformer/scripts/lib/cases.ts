// Parse parity case files. Format: parity/README.md.

export type Step =
  | { kind: "run"; args: string[] }
  | { kind: "stdin"; text: string }
  | { kind: "stdout"; line: string }
  | { kind: "agree"; pattern: RegExp }
  | { kind: "stderr"; line: string }
  | { kind: "exit"; code: number }
  | { kind: "env"; key: string; value: string };

export type Case = { file: string; line: number; name: string; steps: Step[] };

export class CaseSyntaxError extends Error {}

export const HELP = [
  "usage: skyhop <command> [options]",
  "",
  "commands:",
  "  play [--level <n>]",
  "  run [--level <n>]",
  "  draw [--level <n>] [--at <tick>]",
  "  shot [--level <n>] [--at <tick>] [--out <file.png>]",
  "",
  "play opens a window: arrows or WASD to run, space to jump, Esc to quit.",
  "run, draw and shot read a script of controls on standard input.",
];

function jsonText(rest: string, where: string): string {
  const v: unknown = JSON.parse(rest);
  if (typeof v !== "string") throw new CaseSyntaxError(`${where}: needs a JSON string`);
  return v;
}

/** `readInput` resolves `<@ name` to the text of parity/inputs/<name>. */
export function parseCases(file: string, source: string, readInput: (name: string) => string): Case[] {
  const cases: Case[] = [];
  let current: Case | null = null;
  source.split("\n").forEach((raw, i) => {
    const where = `${file}:${i + 1}`;
    if (raw.startsWith("# case:")) {
      current = { file, line: i + 1, name: raw.slice(7).trim(), steps: [] };
      cases.push(current);
      return;
    }
    if (raw === "" || raw.startsWith("#")) return;
    if (!current) throw new CaseSyntaxError(`${where}: step before the first # case:`);
    const m = /^(\$|>~|>|!|\?|<@|<j|<|&)(?: (.*))?$/.exec(raw);
    if (!m) throw new CaseSyntaxError(`${where}: cannot read ${JSON.stringify(raw)}`);
    const sigil = m[1] as string;
    const rest = m[2] ?? "";
    const steps = current.steps;
    switch (sigil) {
      case "$": {
        const args: unknown = JSON.parse(rest);
        if (!Array.isArray(args) || !args.every((a) => typeof a === "string"))
          throw new CaseSyntaxError(`${where}: $ needs a JSON array of strings`);
        steps.push({ kind: "run", args });
        break;
      }
      case "<":
        steps.push({ kind: "stdin", text: `${rest}\n` });
        break;
      case "<j":
        steps.push({ kind: "stdin", text: jsonText(rest, where) });
        break;
      case "<@":
        steps.push({ kind: "stdin", text: readInput(rest) });
        break;
      case ">":
      case "!": {
        const kind = sigil === ">" ? "stdout" : "stderr";
        for (const line of rest === "@help" ? HELP : [rest]) steps.push({ kind, line });
        break;
      }
      case ">~":
        steps.push({ kind: "agree", pattern: new RegExp(`^(?:${rest})$`) });
        break;
      case "?":
        steps.push({ kind: "exit", code: Number.parseInt(rest, 10) });
        break;
      case "&": {
        const eq = rest.indexOf("=");
        if (eq < 1) throw new CaseSyntaxError(`${where}: & needs KEY=VALUE`);
        steps.push({ kind: "env", key: rest.slice(0, eq), value: rest.slice(eq + 1) });
        break;
      }
    }
  });
  return cases;
}
