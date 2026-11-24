# Systems: a kernel, a database, a compiler

> **Design.** Nothing here runs yet. Three range tests for "any software": what spec-lang would add over Rust, C++ or Zig, what stays hand-written at machine altitude, what would be foolish, and where the ceiling is.

**Rule: here the language buys checked laws, derived boundaries and simulation. It does not buy brevity.** Kernels, storage engines and compilers are one-off algorithms, not families of similar programs. Most of their code sits at define and machine altitude, and the token count is expected to be close to Rust's, more once laws are counted. That expectation is unmeasured.

Everything is still rules: an interrupt, a syscall, a page write, a replication message and a compiler query are all "state + event -> new state + effects + reply".

## A small kernel

Target for the sketch: one RISC-V core under an emulator. The page allocator is in [the examples](24-examples.md#kernel-page-allocator).

```slg
use kernel;

@mmio(0x1000_0000)
layout Uart(@ro rbr: u8, @wo @at(0) thr: u8, @ro @at(5) lsr: u8) {
  fn can_send(u: Uart) -> Bool = u.lsr band 0x20 != 0;
}

@on(syscall(3))
@access(fd in files[by] else BadFd)
@guard(len <= 1MiB else TooBig)
@fails(WouldBlock)
@uses(files.write, user.read)
pub rule write(by: Pid, fd: Fd, buf: UserPtr[u8], len: u64) -> u64 = machine {
  let src = user.borrow(buf, len);
  return files[by][fd].write(src);
}

@on(irq(10))
@within(20us)
@alloc(none)
@uses(Uart.read, rx.write)
rule uart_rx() = machine { rx.push(Uart.rbr.read()); }

@order(runqueue, zones, uart_tx)
locks Kernel;

fn pick(q: RunQueue) -> Option[Pid] = q.ready.sort(it.vruntime).get(0);

law no_starvation(t: Trace[tick | block | wake], p: Pid) =
  t.steps.all(s => p in s.before.ready implies t.within(100ms, ran(p)));

@edge(Owned -> Lent, via: dma_start)
@edge(Lent -> Owned, via: dma_done)
flow DmaLife(at: DmaBuf.holder);

pub rule switch_to(next: ref Context) = foreign asm "ctx_switch";
```

| Part | Altitude | What the language adds |
|---|---|---|
| Syscall ABI | declare | one `@on(syscall(n))` rule derives the dispatcher, user-pointer checks from `@guard` and `UserPtr`, user-space stubs for every target language, a fuzz harness and the ABI document. It is the boundary-type rule of [11](11-backends-and-parity.md) applied to a ring transition |
| Device registers, page-table entries | declare | `layout` gives offsets, widths and access modes as types; a read with a side effect is an effect, so it cannot be reordered or dropped |
| Page allocator | machine, with laws | `own Frame` is held exactly once; `never_shared` and `conserved` are trace laws checked exhaustively for small frame counts |
| Scheduler | policy define, mechanism machine | `pick` is pure, so it runs in the interpreter, replays under simulated time, and is checked for bounded starvation on small task sets |
| Interrupt context | `rule` | a rule never waits, so an `@on(irq(n))` rule cannot sleep by construction; `@alloc(none)` and `@within` bound the rest. A blocking syscall is a `task` whose steps are such rules, carried by kernel threads with `survives: none` ([11](11-backends-and-parity.md)) |
| Lock order | a kernel note | `@order` is one global declaration; an acquisition against it is an error at the site |
| Driver DMA | `flow` plus ownership | a buffer lent to a device cannot be touched until the device returns it; the mapping lives as long as the loan |
| Boot, trap entry, context switch | foreign | a contract around assembly, evidence `assumed`, trusting `hw:riscv64` |

**Hand-written at machine or foreign altitude:** the boot stub, trap entry and exit, the context switch, TLB shootdown, the linker script, and anything whose correctness depends on the hardware's weak memory ordering. That is about the assembly a Rust kernel also has. The difference is that each piece sits behind a stated contract and shows up in every report as `assumed`.

**Impossible or foolish:**

| Idea | Why not |
|---|---|
| Full functional correctness in the style of seL4 | that is years of proof work per kernel; here most kernel laws would honestly report `bounded` or `sampled` |
| Claims about real hardware | the hardware model is an assumption; nothing in the language can raise it |
| Worst-case execution time | needs a timing model of the processor; `@within` is `sampled` and monitored, never proven |
| A `kernel` vocabulary with `@scheduler(fair)` | a kernel chosen from a catalogue is a toy; the `kernel` vocabulary supplies `layout`, `@mmio`, `@on(irq)`, `@order`, not policies |
| Lock-free structures at declare altitude | their proofs depend on the memory model; write them at machine altitude and say what is assumed |

## A small database

The page format, write-ahead log and eviction rule are in [the examples](24-examples.md#b-tree-page-and-wal).

```slg
use storage;

state tree: BTree[Bytes, Bytes];
state model: Map[Bytes, Bytes];

law tree_is_a_map(t: Trace[insert | delete | split | merge]) =
  t.states.all(s => s.tree.entries == s.model.entries.sort(it.key));

law snapshot(t: Trace[begin | read | write | commit]) =
  t.committed.all(tx => tx.reads.all(r => r.value == t.at(tx.start).get(r.key)));

law pushdown(a: Rel, b: Rel, p: Pred) =
  p.cols.all(it in a.cols) implies a.join(b).filter(p) == a.filter(p).join(b);

@on(net.recv)
@guard(m.term >= term else Stale, log.matches(m.prev) else Gap)
@uses(log.write, term.write)
rule append(m: Append) -> Ack = {
  term = m.term;
  for e in m.entries {
    if log.conflicts(e) { log.truncate_from(e.index); }
    if e.index > log.size { log.push(e); }
  }
  return Ack(term, upto: log.size);
}

law one_leader(t: Trace[append | vote | timeout | net.drop | crash]) =
  t.states.all(s => s.nodes.filter(it.role == Leader).group_by(it.term).values.all(it.size <= 1));
```

| Part | Altitude | What the language adds |
|---|---|---|
| Page format | declare | `layout` with `fits`, `sorted` and `codec` laws; reader, writer and a consistency checker are derived |
| Pager, buffer pool | machine | pin and unpin as ownership. No other gain |
| B-tree | machine, against a model | `tree_is_a_map`: the tree must agree with a plain `Map` on every trace; node search is a fast and slow pair as in [11](11-backends-and-parity.md) |
| WAL and recovery | define and machine | disk is an effect with a handler, so the checker **enumerates crash points and faults** (torn write, lost sync, reorder) for small traces. The `durable` law is `bounded(k)` as a build step, not a separate test project |
| MVCC | define | the isolation level is a trace law; histories come from the same simulation |
| Query planner | laws | relational laws such as `pushdown` are the rewrite rules, each checked once; the bounded rewriting engine of [10](10-compiler-and-ir.md) ships as a runtime library; cost comes from runtime statistics |
| Replication | define | each message handler is an atomic `rule` whose reply is the acknowledgement; the protocol rules are checked over small clusters with dropped messages and crashes, and those same rules are the code that runs. There is no separate model to drift from |

**Hand-written at machine or foreign altitude:** asynchronous I/O glue, vectorised key comparison, latch coupling, any lock-free structure, cache eviction tuning.

**Impossible or foolish:**

| Idea | Why not |
|---|---|
| Matching a mature engine's planner | decades of heuristics and statistics; laws make rewrites safe, not good |
| Proving linearisability for unbounded clusters | the honest level is `bounded(k)` for small `k` |
| Trusting the simulated disk | a real device and file system fail in ways the handler does not model; durability also trusts `hw:` and `os:` tokens |
| Confusing the two sides | *using* a database from an application (`entity`, `@index`) is high density; *building* one, this section, is not |

Deterministic simulation as a property of the language, not of one heroic codebase, is the strongest argument for spec-lang in this domain.

## spec-lang in spec-lang

A `lang` vocabulary supplies `syntax` and `judgment` as kinds. Both expand to ordinary types, functions and laws.

```slg
use lang;

syntax Param(notes: List[Note], name: Name, of: Option[Expr], default: Option[Expr]);

law roundtrip(t: Tree) = parse(print(t)) == t;

law format_is_stable(src: Text) = format(format(src)) == format(src);

@rule(ctx.has(x, t) implies types(ctx, Var(x), t))
@rule(types(ctx, f, Fn(a, b)) and types(ctx, e, a) implies types(ctx, Call(f, e), b))
judgment types(ctx: Ctx, e: Expr, t: Type);

law fuse_keeps_meaning(p: Program, w: World) =
  well_typed(p) implies run(fuse(p), w) == run(p, w);

fn type_of(d: Decl) -> Type = check(d.body, against: d.result);
```

| Part | What the language adds |
|---|---|
| Grammar | one `syntax` declaration derives parser, lossless tree, printer and formatter; `roundtrip` is a law, not a hope |
| Type rules | a `judgment` derives the checker's skeleton and a generator of well-typed terms. That generator feeds every vocabulary suite and every backend's parity run |
| IR | each layer is a type with laws; they are checked between passes in test builds |
| Passes | each carries a meaning-preservation law against the interpreter, sampled on generated programs |
| Incrementality | `type_of` is a pure `fn`, so its result is memoisable by content hash with no hand-built cache |
| Diagnostics | error codes, causes and fix edits are typed values; the wording is hand-written |

**Hand-written or foreign:** the SMT solver, any machine-code generator reused from elsewhere, performance work in unification and hashing, the text of diagnostics.

**Impossible or foolish:**

| Idea | Why not |
|---|---|
| A verified compiler in the style of CompCert | pass laws here are `sampled`; a proof per pass per backend is out of reach |
| Self-hosting early | the language must stop moving first; a moving bootstrap doubles every change |
| Self-hosting the trusted base | core checker, interpreter, ledger and certificate checker stay small and stay in Rust ([10](10-compiler-and-ir.md)); a compiler that vouches for itself vouches for nothing |
| Skipping the cross-check | a self-hosted stage must reproduce the Rust stage's output hash for hash on the whole suite before it replaces it |

## The real ceiling

Estimates, unmeasured. [19-density-and-experiments](19-density-and-experiments.md) says how they would be tested.

| Software | Mostly at | Expected gain | Bounded by |
|---|---|---|---|
| Record-shaped web, jobs, adapters, infrastructure | declare | much less source | what the backend framework can do; beyond it, `foreign` |
| Game and control logic | define | less glue, units and budgets checked | the engine or board backend ([13-games](13-games.md)) |
| Novel algorithms | define | laws and tests beside the code | the author still writes the algorithm |
| Kernel, storage engine, compiler core | machine | laws, derived boundaries, simulation; little or no brevity | proof cost and the hardware model |
| Boot code, intrinsics, vendor blobs | foreign | a stated contract | everything is `assumed` |

Out of reach by design:

- claims about hardware, third-party services or foreign code stronger than `assumed`, or `sampled` with a monitor;
- performance stronger than `sampled`;
- anything whose shortest honest specification is as long as its implementation;
- quality nobody wrote down: how a screen looks, whether a planner picks well, whether a scheduler feels fair.

## Order of construction

Detail belongs to [20-roadmap](20-roadmap.md). The order that keeps each step useful:

| Step | Build | It proves |
|---|---|---|
| 0 | core calculus, canonical format, hashing, interpreter, holes, simulated handlers, incremental tests | logic runs and is tested before any target exists |
| 1 | vocabulary machinery, `data` running in the interpreter, projections | vocabularies are meaning first, code generation second |
| 2 | first backend (Ultimate), planner, `spec-lang.lock`, provenance, parity harness | day-to-day use inside a real application |
| 3 | prover, second backend for the same vocabulary, second vocabulary on one declaration | the backend contract and footprints are not shaped around one framework |
| 4 | boundary types across two apps, record and replay | several targets, one type system |
| 5 | SIR-M, ownership, layout, a Rust-source backend, `foreign` | machine altitude |
| 6 | `lang` vocabulary: formatter and parser self-hosted, then expansions | the compiler range test |
| 7 | key-value store with crash enumeration, then the emulator kernel | the other two range tests, and a measured ceiling to publish |

## Limits

- **None of the three has been attempted.** The tables are a design argument, not a result.
- **Bounded checking of crash and message traces explodes.** The scopes that finish in a build may be too small to find the bugs that matter; the useful `k` is unknown.
- **Machine altitude needs an ownership and layout system as capable as Rust's.** Designing one is a multi-year problem in itself ([03-types-and-claims](03-types-and-claims.md)); underestimating it is the main risk to this whole document.
- **Assumed contracts carry the dangerous code.** Context switches and lock-free paths are where kernels and databases break, and they are exactly the parts reported as `assumed`.
- **Density here may be negative.** If laws and contracts cost more tokens than they save in debugging, a model is better served writing Rust directly for this class of software. That would be an acceptable finding.
