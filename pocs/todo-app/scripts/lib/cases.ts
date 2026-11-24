// Parse parity case files. Format: parity/README.md.

export type Step =
  | { kind: "run"; args: string[] }
  | { kind: "stdout"; line: string }
  | { kind: "stderr"; line: string }
  | { kind: "exit"; code: number }
  | { kind: "seed"; text: string }
  | { kind: "store"; text: string | null }
  | { kind: "env"; key: string; value: string };

export type Case = { file: string; line: number; name: string; steps: Step[] };

export class CaseSyntaxError extends Error {}

const HELP = [
  "usage: todo <command> [args]",
  "",
  "commands:",
  "  add <title> [--priority low|normal|high] [--tags <tag>]...",
  "  list [--all] [--tag <tag>]",
  "  done <no>",
  "  reopen <no>",
  "  rename <no> <title>",
  "  remove <no>",
  "  clear",
  "  stats",
];

function text(sigil: string, rest: string, where: string): string {
  if (!sigil.endsWith("j")) return rest;
  const v: unknown = JSON.parse(rest);
  if (typeof v !== "string") throw new CaseSyntaxError(`${where}: ${sigil} needs a JSON string`);
  return v;
}

export function parseCases(file: string, source: string): Case[] {
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
    const m = /^([$><=!?&]j?)(?: (.*))?$/.exec(raw);
    if (!m) throw new CaseSyntaxError(`${where}: cannot read ${JSON.stringify(raw)}`);
    const sigil = m[1] as string;
    const rest = m[2] ?? "";
    const steps = current.steps;
    switch (sigil[0]) {
      case "$": {
        const args: unknown = JSON.parse(rest);
        if (!Array.isArray(args) || !args.every((a) => typeof a === "string"))
          throw new CaseSyntaxError(`${where}: $ needs a JSON array of strings`);
        steps.push({ kind: "run", args });
        break;
      }
      case ">":
      case "!": {
        const kind = sigil[0] === ">" ? "stdout" : "stderr";
        const lines = rest === "@help" ? HELP : [text(sigil, rest, where)];
        for (const line of lines) steps.push({ kind, line });
        break;
      }
      case "?":
        steps.push({ kind: "exit", code: Number.parseInt(rest, 10) });
        break;
      case "<":
        steps.push({ kind: "seed", text: text(sigil, rest, where) });
        break;
      case "=":
        steps.push({ kind: "store", text: rest === "absent" ? null : text(sigil, rest, where) });
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
