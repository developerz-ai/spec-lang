// Run the parity transcripts against every built backend. Usage: parity/README.md.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { type Case, parseCases } from "../lib/cases.ts";
import { startDisplay } from "./display.ts";

const ROOT = resolve(import.meta.dir, "../..");
type Backend = { name: string; run: string[] };

function backends(only: string | undefined): Backend[] {
  const table: Record<string, { run: string[] }> = JSON.parse(readFileSync(join(ROOT, "parity/backends.json"), "utf8"));
  const all = Object.entries(table).map(([name, b]) => ({ name, run: b.run.map((p, i) => (i === 0 ? join(ROOT, p) : p)) }));
  const wanted = only ? all.filter((b) => only.split(",").includes(b.name)) : all;
  const missing = wanted.filter((b) => !existsSync(b.run[0] as string));
  for (const b of missing) console.error(`skip ${b.name}: ${b.run[0]} not built (run its bin/build)`);
  return wanted.filter((b) => !missing.includes(b));
}

const timings = new Map<string, number[]>();

function invoke(b: Backend, args: string[], stdin: string, env: Record<string, string>) {
  const started = performance.now();
  const r = Bun.spawnSync([...b.run, ...args], { cwd: ROOT, env, stdin: Buffer.from(stdin), stdout: "pipe", stderr: "pipe" });
  const list = timings.get(b.name) ?? [];
  list.push(performance.now() - started);
  timings.set(b.name, list);
  return { out: r.stdout.toString(), err: r.stderr.toString(), code: r.exitCode ?? -1 };
}

function baseEnv(display: string | null): Record<string, string> {
  const env: Record<string, string> = {
    PATH: process.env.PATH ?? "/usr/bin:/bin",
    HOME: process.env.HOME ?? "/tmp",
    LANG: "C.UTF-8",
    SKYHOP_RAYLIB: join(ROOT, "vendor/raylib/lib/libraylib.so"),
    SKYHOP_ASSETS: join(ROOT, "assets"),
  };
  if (display) env.DISPLAY = display;
  return env;
}

/** Lines that every backend must print alike, keyed by case, run and line number. */
const agreed = new Map<string, Map<string, string>>();

function runCase(c: Case, b: Backend, env: Record<string, string>): string | null {
  let stdin = "";
  let expect: { out: (string | RegExp)[]; err: string[]; code: number } = { out: [], err: [], code: 0 };
  let last: { args: string[]; out: string; err: string; code: number; no: number } | null = null;
  let runs = 0;
  const settle = (): string | null => {
    if (!last) return null;
    const ctx = `[${b.name}] skyhop ${JSON.stringify(last.args)}`;
    const got = last.out.split("\n");
    if (got.at(-1) !== "") return `${ctx}\n  stdout does not end in a newline: ${JSON.stringify(last.out.slice(-80))}`;
    got.pop();
    if (got.length !== expect.out.length)
      return `${ctx}\n  stdout want ${expect.out.length} lines got ${got.length}\n  got ${JSON.stringify(last.out.slice(0, 2000))}`;
    for (const [i, want] of expect.out.entries()) {
      const line = got[i] as string;
      if (typeof want === "string") {
        if (line !== want)
          return `${ctx}\n  stdout line ${i + 1} want ${JSON.stringify(want)}\n  stdout line ${i + 1} got  ${JSON.stringify(line)}`;
      } else {
        if (!want.test(line)) return `${ctx}\n  stdout line ${i + 1} ${JSON.stringify(line)} does not match ${want}`;
        const key = `${c.file}:${c.line} run ${last.no} line ${i + 1}`;
        const seen = agreed.get(key) ?? new Map<string, string>();
        seen.set(b.name, line);
        agreed.set(key, seen);
      }
    }
    const wantErr = expect.err.map((l) => `${l}\n`).join("");
    if (last.err !== wantErr) return `${ctx}\n  stderr want ${JSON.stringify(wantErr)}\n  stderr got  ${JSON.stringify(last.err)}`;
    if (last.code !== expect.code) return `${ctx}\n  exit want ${expect.code} got ${last.code}`;
    last = null;
    return null;
  };
  for (const s of c.steps) {
    if (s.kind === "stdout") expect.out.push(s.line);
    else if (s.kind === "agree") expect.out.push(s.pattern);
    else if (s.kind === "stderr") expect.err.push(s.line);
    else if (s.kind === "exit") expect.code = s.code;
    else if (s.kind === "env") {
      const bad = settle();
      if (bad) return bad;
      env[s.key] = s.value;
    } else if (s.kind === "stdin") {
      const bad = settle();
      if (bad) return bad;
      stdin += s.text;
    } else {
      const bad = settle();
      if (bad) return bad;
      last = { args: s.args, no: runs++, ...invoke(b, s.args, stdin, env) };
      stdin = "";
      expect = { out: [], err: [], code: 0 };
    }
  }
  return settle();
}

function median(xs: number[]): number {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)] ?? 0;
}

function loadCases(): Case[] {
  const dir = join(ROOT, "parity/cases");
  const input = (name: string) => readFileSync(join(ROOT, "parity/inputs", name), "utf8");
  return readdirSync(dir)
    .filter((f) => f.endsWith(".txt"))
    .sort()
    .flatMap((f) => parseCases(`parity/cases/${f}`, readFileSync(join(dir, f), "utf8"), input));
}

async function main(): Promise<number> {
  const argv = process.argv.slice(2);
  const i = argv.indexOf("--only");
  const list = backends(i >= 0 ? argv[i + 1] : undefined);
  if (list.length === 0) {
    console.error("no backend built");
    return 1;
  }
  const cases = loadCases();
  const needsDisplay = cases.some((c) => c.steps.some((s) => s.kind === "run" && s.args[0] === "shot"));
  const display = needsDisplay ? await startDisplay() : null;
  const failures: string[] = [];
  try {
    for (const b of list) {
      let ok = 0;
      for (const c of cases) {
        const bad = runCase(c, b, baseEnv(display?.name ?? null));
        if (bad) failures.push(`${b.name}: ${c.file}:${c.line} ${c.name}\n  ${bad}`);
        else ok++;
      }
      console.log(`${b.name.padEnd(5)} ${ok}/${cases.length} cases`);
    }
  } finally {
    display?.stop();
  }
  let differ = 0;
  for (const [key, seen] of agreed) {
    if (new Set(seen.values()).size > 1) {
      differ++;
      failures.push(`agreement: ${key}\n  ${[...seen].map(([n, l]) => `${n}: ${l}`).join("\n  ")}`);
    }
  }
  console.log(`agree ${agreed.size - differ}/${agreed.size} lines alike across ${list.map((b) => b.name).join(", ")}`);
  for (const [name, ts] of timings) console.log(`${name.padEnd(5)} median ${median(ts).toFixed(1)} ms over ${ts.length} runs`);
  for (const f of failures.slice(0, 30)) console.error(`\nFAIL ${f}`);
  if (failures.length > 30) console.error(`\n... ${failures.length - 30} more`);
  return failures.length ? 1 : 0;
}

process.exit(await main());
