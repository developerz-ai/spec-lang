# Dropping down

> **Design.** Nothing here runs yet. Mixing spec-lang with code in a backend's own language, the way C projects keep a little assembly.

**Most of a program is spec. A few pieces are written in the backend's language, each behind a contract.** C kept assembly for the places where it was needed and made the programmer state what it reads, writes and clobbers. Same here.

| In C | In spec-lang |
|---|---|
| inputs and outputs of an `asm` block | the signature |
| clobber list | `@uses`, `@fails`, `@alloc`, `@within` |
| "trust me" | evidence starts at `assumed`, and reports say so |
| one `asm` per architecture | one fast version per backend; the reference runs everywhere else |
| a `.s` file beside the `.c` | a real file in that language, with its own formatter, linter and debugger |

## The first choice: do not

Try in this order. Each step down costs portability or evidence.

| # | Step | Keeps |
|---|---|---|
| 1 | a pin or policy on the declaration: `@page(50, by: offset)`, `@memo(...)` | everything |
| 2 | a better algorithm in spec-lang | everything |
| 3 | `machine { }`: layout, ownership, no allocation | everything but brevity |
| 4 | a **pair**: spec-lang reference, foreign fast version | meaning, with the fast half tested against the reference |
| 5 | `foreign` alone | the contract only |

## The pair

The reference is the meaning. The fast version is an implementation of it for one backend.

```slg
@why("profiled: this is most of the import time")
@fast(parse_simd, on: rust)
@post(result.size <= text.size + 1)
fn parse(text: Bytes) -> List[Field] = {
  var out: List[Field] = [];
  for part in text.split(b",") { out.push(Field(part)); }
  return out;
}

fn parse_simd(text: Bytes) -> List[Field] = foreign rust "fast/parse.rs#parse_simd";

law same(text: Bytes) = parse(text) == parse_simd(text);
```

- The interpreter runs the reference. So do tests, holes and every backend without a fast version.
- `@fast` writes the law `same` by itself; it is shown here to be seen. The law runs as a differential test on generated inputs, on the real backend.
- Effects must match. A fast version that logs, allocates or fails differently is an error where the checker can see it, and a differential failure where it cannot.
- Delete the `@fast` line and the program means the same.

## Foreign alone

For things spec-lang cannot say: a vendor library, a boot instruction, a canvas renderer, a view with taste in it.

```slg
@uses(gpu)
@within(2ms)
rule draw(scene: Scene) -> Frame = foreign cpp "render/frame.cpp#draw";

fn sign(@secret key: Bytes, body: Bytes) -> Bytes = foreign ts "./sign.ts#hmac";
```

The text names a symbol (`path#name`), a linked symbol or an instruction. It never holds code: code in a string has no formatter, no linter, no tests and no debugger.

| It must state | Because |
|---|---|
| types at the edge | values are copied or lent, never shared |
| effects and failures | a caller is checked against them |
| what is assumed | it goes to every report that rests on it |

Results that come back are untrusted until decoded ([05](05-effects-and-trust.md)). Evidence can be raised by a gate, a differential test or a sandbox ([04](04-evidence.md)).

## Calling the other way

Hand-written code calls `pub` rules through generated typed clients and never writes a module's state. That is what keeps laws true when most of an app is still old code ([22](22-lifecycle.md)).

## How much is too much

A project reports the share of its code that is foreign, by module. It is a fact for the reader and no gate. A module that is mostly foreign is not worth writing in spec-lang ([12](12-frameworks.md)); K7 in [19](19-density-and-experiments.md) sets the pilot's limit.

| Place | Expected foreign part |
|---|---|
| records, flows, jobs | near none |
| provider adapters | the field maps and quirks |
| screens | the view |
| game | renderer, some geometry |
| database | asynchronous I/O, vectorised compare |
| kernel | boot, context switch |

Not measured.

## Limits

- Differential tests sample. A fast version can differ on an input nobody generated.
- Timing, memory ordering and undefined behaviour inside foreign code are invisible to the checker.
- Two languages in one project bring back some of what this language removes: two toolchains, and a person or model who knows both.
- A fast version tied to one backend is a reason not to add a second.
- `@fast` is proposed by this doc and the card does not list it.
