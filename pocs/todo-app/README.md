# todo-app: one spec, four backends

> **Two ways out of one source.** A program of about 260 lines of spec-lang in [`src/`](src/) becomes four production command-line programs (Rust, TypeScript on Bun, Ruby, C++) twice. In [`out/`](out/), four model sessions wrote them by hand, acting as the compiler before it existed. In [`gen/`](gen/), the real compiler wrote them: `spec-lang build`, from [`src/`](../../src/README.md) at the repository root. One suite judges all eight byte for byte, and `spec-lang test` checks the source itself.

## The source

```text
src/
  todo/model.slg            Title, Tag, TaskNo, Status, Priority, Task: data and claims
  todo/render.slg           one line per task, ranking
  todo/board.slg            the state and eight rules: add list done reopen rename remove clear stats
  todo/laws.slg             six laws that hold after any sequence of commands
  todo/tests/scenarios.slg  ten tests, one of them a property
  apps/{rust,bun,ruby,cpp}.slg   the only files that differ: @target(...)
```

A rule, as written:

```slg
@on(cli("done"))
@uses(tasks.write, out.write)
@guard(no in tasks else NoTask, tasks[no].status == Open else AlreadyDone)
pub rule done(no: TaskNo) = {
  tasks[no].status = Done;
  out.line(line(tasks[no]));
}
```

A law, and the app that picks a target:

```slg
@why("a number is never reused")
law numbers_below_next(t: Trace[Change]) =
  t.states.all(w => w.tasks.keys.all(it < w.next_no));

@target(rust)
@bin("todo")
@store(file(".todo.json"))
app TodoRust(rules: [board.add, board.list, board.done, board.reopen, board.rename, board.remove, board.clear, board.stats]);
```

Not written anywhere in `src/`: argument parsing, `--help`, exit codes, the JSON store, file locking, atomic writes, the store's strict reader. The [`cli` vocabulary](docs/cli-vocabulary.md) derives them; the [contract](docs/contract.md) fixes their bytes.

## What came out

The same rule in each backend (`out/<backend>/`, file shown):

```rust
// out/rust/src/board/done.rs
pub fn done(world: &mut World, out: &mut Out, no: TaskNo) -> Result<(), Refusal> {
    let task = world.tasks.get_mut(&no).ok_or(Refusal::NoTask)?;
    if task.status != Status::Open {
        return Err(Refusal::AlreadyDone);
    }
    task.status = Status::Done;
    out.line(Line(task));
    Ok(())
}
```

```ts
// out/bun/src/board/done.ts
export function done(w: World, no: TaskNo): Outcome {
  const t = w.tasks.get(no);
  if (t === undefined) return refuse("NoTask");
  if (t.status !== "Open") return refuse("AlreadyDone");
  const changed = Object.freeze({ ...t, status: "Done" as const });
  return commit(withTask(w, changed), [line(changed)], undefined);
}
```

```ruby
# out/ruby/lib/todo/board/done.rb
def self.done(world, no:)
  task = world.task(no, NoTask)
  raise AlreadyDone unless task.status == Model::Status::OPEN

  moved = task.with(status: Model::Status::DONE)
  Outcome.new(world: world.put(moved), lines: [Render.line(moved)], reply: nil)
end
```

```cpp
// out/cpp/src/board/done_reopen.cpp, condensed: done and reopen share one move
Result<Nothing> move(const World& w, model::TaskNo no, model::Status from, model::Status to, RuleError wrong) {
  const auto found = w.tasks.find(no);
  if (found == w.tasks.end()) return std::unexpected(RuleError::NoTask);
  if (found->second.status != from) return std::unexpected(wrong);
  World next = w;
  next.tasks.at(no).status = to;
  return Outcome<Nothing>{std::move(next), {render::line(next.tasks.at(no))}, {}};
}
```

Each backend has the layout of a production project: `model/ render/ board/` (pure), `cli/` (a static parameter table, parser, help, exit codes), `store/` (strict JSON reader, canonical writer, `flock`, atomic replace), and tests that translate every `@cases`, every `test` by name, and every `law` as a seeded property test. Each folder's `NOTES.md` lists its open choices and its optimizations, with the spec fact that licenses each one.

## Results

Measured on 2026-11-24, one machine (AMD Ryzen 7 5700X3D, 16 threads, Linux 7.0), warm cache, one run of each command below.

| | Rust | TypeScript (Bun) | Ruby | C++ |
|---|---|---|---|---|
| Projected by | Claude Fable 5.2 | Claude Opus 5.5 | Claude Fable 5.2 | Claude Opus 5.5 |
| Parity cases, alone | 65/65 | 65/65 | 65/65 | 65/65 |
| Own gate (`out/<b>/bin/check`) | green | green | green | green |
| Product / test lines | 2061 / 1536 | 1122 / 1005 | 1068 / 1041 | 1968 / 1035 |
| Third-party libraries | none | fast-check (tests) | minitest, rubocop (dev) | none |
| Binary | 1.3 MB static | 111 MB (runtime included) | interpreted | 0.96 MB static |
| Median wall time per command | 0.3 ms | 4.0 ms | 34.2 ms | 0.2 ms |

The spec is 179 lines of product and 71 of tests. All four together pass the `--mix` run, where each step of a case runs on the next backend against one shared store, and a race of 40 concurrent `add`s spread over the four, with numbers 1 to 40 and none lost.

Methods: lines from `bun scripts/stats/loc.ts` (non-blank, test by path; Rust inline test modules count as product). Times from `bun scripts/parity/run.ts`: wall time of each `spawnSync`, process start included, median over 160 small runs. Ruby's figure is mostly a version-manager shim on `PATH`: its notes measure 7.8 ms for `--help` with the interpreter called directly. Projection time: 13 to 20 minutes per backend, the agent run time the harness recorded, with one later course change mid-run (optimizations, libraries); not a controlled measurement.

What the projections found is in [findings.md](findings.md).

## Generated by the compiler

`bin/regen` runs `spec-lang build` for each app into `gen/<backend>/`; `bin/regen --check`, part of the gate, fails when the committed output is not what the current source builds. Every generated file names its declarations and `.slg` lines, every guard carries a `slg:` anchor, and `provenance.json` maps every emitted line to its declaration, its role and the strategy that shaped it ([37](../../docs/idea/37-optimizer.md)).

| | Rust | TypeScript (Bun) | Ruby | C++ |
|---|---|---|---|---|
| `spec-lang build` | 12 ms | 6 ms | 6 ms | 392 ms |
| Parity, strategies on and off | 68/68 | 68/68 | 68/68 | 68/68 |
| `stats` on 100,000 tasks, generated against hand-written | 23.6 against 20.5 ms | 179 against 143 ms | 1.45 against 0.97 s | 29.3 against 27.1 ms |

Method for the table: `hyperfine` on one machine (load from other builds varied), the 100,000-task store from [findings](findings.md). After a change to two claims, the generated backends followed with no edit; the hand-written ones needed 236 changed lines ([findings](findings.md)).

## Run it

```bash
bin/setup                                  # check toolchains, install deps, build all four
bin/todo rust add "ship the POC" --priority high --tags work
bin/todo ruby list                         # same store, another language
bin/check                                  # the gate: spec-lang test, gen/ current, eight backend gates, parity
bin/regen                                  # rebuild gen/ from src/ with spec-lang build
```

For agents: [CLAUDE.md](CLAUDE.md), the commands in `.claude/commands/`, the `projector` and `parity-reviewer` agents.

## Limits

- Models wrote every file under `out/`. The compiler's emitters carry runtime pieces (store, lock, argument engine) lifted from those references; what they derive from the source is the domain, the command table, the help text and the tests. The contract itself is still written by hand.
- `spec-lang test` checks names, notes, effects and ownership, and runs every case, test, property and law; it does not yet type-check expressions. Hand-written backends' tests were translated by the session that wrote the code.
- Parity covers 65 transcripts. Behaviour no case pins can differ; [findings](findings.md) lists the known differences.
- One small program. Size, change over time and teams are not tested.
- Times are one machine, one run, small stores; they compare start-up more than anything else.
