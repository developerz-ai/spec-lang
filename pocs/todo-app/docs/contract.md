# Projection contract

> **Design, POC-local.** There is no `spec-lang build`. This page is what one would derive from [`src/`](../src/) and the [`cli` vocabulary](cli-vocabulary.md), written out by hand so that four people (here: four model sessions) projecting the same source by hand produce programs that cannot be told apart from outside. Where it says **must**, the [parity suite](../parity/) checks it.

## 1. Meaning

- The world is `board.tasks: Map[TaskNo, Task]` and `board.next_no: TaskNo`, initially empty and `1`.
- A rule is a function `(world, args) -> (world', lines, reply) | error`. Guards run first, in written order; the first failing one is the error. On an error the world is unchanged and no line is printed.
- `out.line` lines are printed only after the new world is committed to disk.
- `Text.size` counts Unicode code points, not bytes and not UTF-16 units. `trim` removes Unicode `White_Space` from both ends, exactly these 25 code points: U+0009..U+000D, U+0020, U+0085, U+00A0, U+1680, U+2000..U+200A, U+2028, U+2029, U+202F, U+205F, U+3000. **Not** U+FEFF, U+180E or U+200B, which several language `trim`s remove.
- `Int` is exact. A `TaskNo` argument of any length is a `TaskNo`; one larger than anything in the store is simply not in it (`NoTask`).

## 2. Command line

Binary `todo`. Exit codes: `0` success, `1` a rule refused (`fail`), `2` usage, `3` store.

### Help

Printed to standard output with exit 0 for `--help` or `-h`, or to standard error with exit 2 when there are no arguments. Exactly:

```text
usage: todo <command> [args]

commands:
  add <title> [--priority low|normal|high] [--tags <tag>]...
  list [--all] [--tag <tag>]
  done <no>
  reopen <no>
  rename <no> <title>
  remove <no>
  clear
  stats
```

### Parsing, in this order

1. No arguments: help to standard error, exit 2.
2. `--help` or `-h` anywhere before the first `--`: help to standard output, exit 0, nothing else checked.
3. The first argument is the command, even `--` or a flag. Unknown: `error: unknown command "X"`.
4. The remaining arguments, left to right; the **first** problem met is the one reported:
   - The first `--` makes every later argument positional.
   - An argument starting with `--` is a flag name (`--priority=high` is the unknown flag `--priority=high`). Not a flag of this command: `error: unknown flag --X`.
   - A switch (`--all`) seen before: `error: --all given twice`.
   - A value flag takes the **next argument, whatever it is**, as its value. None left: `error: --priority needs a value`. Any value flag other than a `Set` seen before (`Option`, or one with a plain default like `--priority`): `error: --tag given twice`. Value not of the type: `error: --priority: not a Priority`, `error: --tags: not a Tag`. `--tags` repeats; duplicates collapse.
   - Anything else (`-x`, `-`, `-1` included) is positional. One too many: `error: unexpected argument "X"`. Not of the type: `error: <no>: not a TaskNo`, `error: <title>: not a Title`.
5. After all arguments, the first missing positional: `error: missing <title>`.

Every usage error is one line on standard error, exit 2. `X` is printed as given.

| Type | Accepts |
|---|---|
| `TaskNo` | ASCII digits only, one or more, value at least 1; leading zeros allowed (`007` is 7) |
| `Title` | `is_title` in [model.slg](../src/todo/model.slg) |
| `Tag` | `is_tag` |
| `Priority` | exactly `low`, `normal`, `high` |

### Errors

| When | Standard error | Exit |
|---|---|---|
| a guard or `fail` | `error: NoTask` (the name) | 1 |
| the lock not had within 5 s | `error: Contended` | 1 |
| the store cannot be read or does not meet section 3 | `error: store <path>: unreadable` | 3 |
| the store or its lock cannot be created or written | `error: store <path>: unwritable` | 3 |
| `add` when `next_no` is 2^53 | `error: store <path>: full` | 3 |

`<path>` is the path as configured, not resolved. Standard output is empty on every error.

## 3. Store

- Path: `.todo.json` in the working directory; the environment variable `TODO_STORE` replaces it. An empty `TODO_STORE` counts as unset.
- Lock: `flock(2)` on `<path>.lock`, created if missing and never deleted. Exclusive for `add done reopen rename remove clear`, shared for `list stats`. Try without blocking every 10 ms for up to 5 s, then `Contended`. The lock is held from before the read until after the write.
- A missing store is the initial world. `list` and `stats` never create or rewrite the store (they do create the lock file).
- A rule that writes and succeeds replaces the store atomically, unless the bytes are unchanged (below): write `<path>.<pid>.tmp` in the same directory, `fsync`, `rename` over `<path>`. If the new bytes equal the bytes read, the write may be skipped ([32](../../../docs/idea/32-build-layout.md): a byte-identical file is not rewritten).

### Bytes written

One line of JSON and `\n`, no other white space, keys in this order, tasks by `no` ascending, tags in code-point order:

```text
{"version":1,"next_no":3,"tasks":[{"no":1,"title":"milk","status":"Open","priority":"Normal","tags":[]},{"no":2,"title":"bread \"fresh\"","status":"Done","priority":"High","tags":["home","shop"]}]}
```

Strings escape only `"` and `\`; everything else, non-ASCII included, is raw UTF-8. Two backends given the same world **must** write the same bytes.

### Accepted on read

Any valid JSON (white space, key order, task order, `\u` escapes and surrogate pairs) that meets all of these; anything else is `unreadable`:

| Field | Must |
|---|---|
| top level | an object with exactly `version`, `next_no`, `tasks`; no key twice in any object |
| `version` | the integer `1` |
| `next_no` | an integer, 1 to 2^53 (9007199254740992) |
| `tasks` | an array of objects with exactly `no`, `title`, `status`, `priority`, `tags` |
| `no` | an integer, at least 1, below `next_no`, unique |
| `title` | a string meeting `is_title` |
| `status` `priority` | `"Open"` `"Done"` · `"Low"` `"Normal"` `"High"` |
| `tags` | an array of strings meeting `is_tag`, none twice |

An integer is a JSON number with no fraction or exponent: `1.0` and `1e0` are not integers. A whole-number literal above 2^53 must be rejected, not rounded.

## 4. Each backend

| | Rust | TypeScript | Ruby | C++ |
|---|---|---|---|---|
| Folder | `out/rust` | `out/bun` | `out/ruby` | `out/cpp` |
| Toolchain | stable Rust, edition 2024 | Bun, TypeScript strict | Ruby 4.0, no gems at run time | C++23, CMake, g++ or clang |
| Executable after `bin/build` | `target/release/todo` | `dist/todo` (`bun build --compile`) | `exe/todo` | `build/todo` |
| Gate `bin/check` | `cargo fmt --check`, `clippy -D warnings`, `cargo test`, build | `biome check`, `tsc --noEmit`, `bun test`, build | `ruby -w` on every file, minitest, build | build with `-Wall -Wextra -Wpedantic -Werror`, ctest |

Layout is production style: many small files, one concern each, mirroring the spec's modules.

| Spec | Becomes |
|---|---|
| `todo.model` | types with validated constructors; `is_title`, `is_tag` |
| `todo.render` | the line format, `rank` |
| `todo.board` | the world and one file per rule, or a small group; pure, no I/O |
| `todo.laws` | property tests: at least 200 seeded traces of 40 random commands, every law checked after every step |
| `todo.tests.scenarios` | one native test per spec test, same names |
| `@cases` | table tests |
| `cli` vocabulary | `cli/`: argument parsing from a per-command parameter table, help, dispatch, exit codes |
| `@store` | `store/`: JSON codec (strict reader, canonical writer), lock, atomic file |

Rules:

- Every source file starts with a comment naming the declarations it projects and their spec file, and saying it is a hand projection: `Projected by hand from <decls> (<spec file>). Stand-in for spec-lang build output; change the spec, not this file.`
- The domain (`model`, `render`, `board`) never touches I/O, the clock or the environment.
- No mocks. Tests of the store use a temporary directory.
- Libraries: any well-known one a production team in that language would reach for (argument parsing excepted: the parser is derived from the spec and must match section 2 exactly). Pinned, and each listed in `NOTES.md` with what it replaces.
- Compiler-grade output: every optimization a senior engineer would make is allowed when a fact in the spec licenses it (an effect, a read set, a claim); `NOTES.md` lists each one with that fact.
- `NOTES.md` in the folder: every choice this contract or the spec left open and what was chosen, anything that could not be kept, the projecting model, and line counts with the command that counted them.

## Limits

- A real compiler would derive this page. Writing it by hand means its gaps are found by the backends that fall into them, which is the point of the POC but not proof the derivation is possible.
- The 2^53 store limit is a backend constraint carried as a refusal, not a language rule.
- `flock` is advisory and local. A store on a network file system is not covered.
