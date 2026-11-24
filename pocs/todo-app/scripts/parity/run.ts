// Run the parity transcripts against every built backend. Usage: parity/README.md.

import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { type Case, parseCases } from "../lib/cases.ts";

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

function invoke(b: Backend, args: string[], cwd: string, env: Record<string, string>) {
  const started = performance.now();
  const r = Bun.spawnSync([...b.run, ...args], { cwd, env, stdout: "pipe", stderr: "pipe" });
  const list = timings.get(b.name) ?? [];
  list.push(performance.now() - started);
  timings.set(b.name, list);
  return { out: r.stdout.toString(), err: r.stderr.toString(), code: r.exitCode ?? -1 };
}

function baseEnv(): Record<string, string> {
  return { PATH: process.env.PATH ?? "/usr/bin:/bin", HOME: process.env.HOME ?? "/tmp", LANG: "C.UTF-8" };
}

function runCase(c: Case, pick: (step: number) => Backend): string | null {
  const dir = mkdtempSync(join(tmpdir(), "todo-parity-"));
  const env = baseEnv();
  const storePath = () => join(dir, env.TODO_STORE || ".todo.json");
  let expectOut: string[] = [];
  let expectErr: string[] = [];
  let expectCode = 0;
  let last: { b: Backend; args: string[]; out: string; err: string; code: number } | null = null;
  let runs = 0;
  const settle = (): string | null => {
    if (!last) return null;
    const want = { out: expectOut.map((l) => `${l}\n`).join(""), err: expectErr.map((l) => `${l}\n`).join("") };
    const ctx = `[${last.b.name}] todo ${JSON.stringify(last.args)}`;
    if (last.out !== want.out) return `${ctx}\n  stdout want ${JSON.stringify(want.out)}\n  stdout got  ${JSON.stringify(last.out)}`;
    if (last.err !== want.err) return `${ctx}\n  stderr want ${JSON.stringify(want.err)}\n  stderr got  ${JSON.stringify(last.err)}`;
    if (last.code !== expectCode) return `${ctx}\n  exit want ${expectCode} got ${last.code}`;
    last = null;
    return null;
  };
  try {
    for (const s of c.steps) {
      if (s.kind === "stdout") expectOut.push(s.line);
      else if (s.kind === "stderr") expectErr.push(s.line);
      else if (s.kind === "exit") expectCode = s.code;
      else {
        const bad = settle();
        if (bad) return bad;
        if (s.kind === "run") {
          const b = pick(runs++);
          last = { b, args: s.args, ...invoke(b, s.args, dir, env) };
          expectOut = [];
          expectErr = [];
          expectCode = 0;
        } else if (s.kind === "seed") writeFileSync(storePath(), `${s.text}\n`);
        else if (s.kind === "env") env[s.key] = s.value;
        else if (s.kind === "store") {
          const have = existsSync(storePath()) ? readFileSync(storePath(), "utf8") : null;
          const want = s.text === null ? null : `${s.text}\n`;
          if (have !== want)
            return `store ${env.TODO_STORE || ".todo.json"}\n  want ${JSON.stringify(want)}\n  got  ${JSON.stringify(have)}`;
        }
      }
    }
    return settle();
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

async function race(list: Backend[], n: number): Promise<string | null> {
  const dir = mkdtempSync(join(tmpdir(), "todo-race-"));
  try {
    const results = await Promise.all(
      Array.from({ length: n }, async (_, i) => {
        const b = list[i % list.length] as Backend;
        const p = Bun.spawn([...b.run, "add", `job ${i}`], { cwd: dir, env: baseEnv(), stdout: "pipe", stderr: "pipe" });
        const [code, out, err] = await Promise.all([p.exited, new Response(p.stdout).text(), new Response(p.stderr).text()]);
        return { backend: b.name, code, out, err };
      }),
    );
    const failed = results.filter((r) => r.code !== 0);
    if (failed.length) return `race: ${failed.length} of ${n} failed, first: ${JSON.stringify(failed[0])}`;
    const nos = results.map((r) => Number(r.out.trim().replace("added ", ""))).sort((a, b) => a - b);
    if (nos.some((no, i) => no !== i + 1)) return `race: numbers ${JSON.stringify(nos)}`;
    const stats = invoke(list[0] as Backend, ["stats"], dir, baseEnv());
    if (stats.out !== `${n} open, 0 done\n`) return `race: stats said ${JSON.stringify(stats.out)}`;
    return null;
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

function median(xs: number[]): number {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)] ?? 0;
}

async function main(): Promise<number> {
  const argv = process.argv.slice(2);
  const flag = (name: string) => {
    const i = argv.indexOf(name);
    return i >= 0 ? (argv[i + 1] ?? "") : undefined;
  };
  const list = backends(flag("--only"));
  if (list.length === 0) {
    console.error("no backend built");
    return 1;
  }
  const raceN = flag("--race");
  if (raceN !== undefined) {
    const n = Number.parseInt(raceN, 10);
    const bad = await race(list, n);
    console.log(bad ?? `race: ${n} concurrent adds over ${list.map((b) => b.name).join(", ")}: numbers 1..${n}, none lost`);
    return bad ? 1 : 0;
  }
  const dir = join(ROOT, "parity/cases");
  const cases = readdirSync(dir)
    .filter((f) => f.endsWith(".txt"))
    .sort()
    .flatMap((f) => parseCases(`parity/cases/${f}`, readFileSync(join(dir, f), "utf8")));
  const failures: string[] = [];
  // --mix starts each case on the next backend, so every backend meets every other on a shared store.
  const modes: { label: string; pick: (caseNo: number, step: number) => Backend }[] = argv.includes("--mix")
    ? [{ label: "mix", pick: (c, i) => list[(c + i) % list.length] as Backend }]
    : list.map((b) => ({ label: b.name, pick: () => b }));
  for (const m of modes) {
    let ok = 0;
    for (const [n, c] of cases.entries()) {
      const bad = runCase(c, (step) => m.pick(n, step));
      if (bad) failures.push(`${m.label}: ${c.file}:${c.line} ${c.name}\n  ${bad}`);
      else ok++;
    }
    console.log(`${m.label.padEnd(5)} ${ok}/${cases.length} cases`);
  }
  for (const [name, ts] of timings) console.log(`${name.padEnd(5)} median ${median(ts).toFixed(1)} ms over ${ts.length} runs`);
  for (const f of failures.slice(0, 30)) console.error(`\nFAIL ${f}`);
  if (failures.length > 30) console.error(`\n... ${failures.length - 30} more`);
  return failures.length ? 1 : 0;
}

process.exit(await main());
