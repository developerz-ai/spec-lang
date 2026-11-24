# Parity suite

> **Design, POC-local.** One set of command-line transcripts every backend must reproduce, byte for byte where the bytes are fixed, alike where they depend on the machine. It is the outside view of [the contract](../docs/contract.md); the spec's own tests run inside each backend.

```text
bun scripts/parity/run.ts                 # every built backend, every case
bun scripts/parity/run.ts --only cpp      # one backend (agreement is then checked among those run)
```

The runner starts one `Xvfb` for the whole run when a case calls `shot`, and passes `DISPLAY`, `SKYHOP_RAYLIB` and `SKYHOP_ASSETS` to every program. Programs run in the POC folder.

## Case files

`cases/*.txt`; inputs shared by several cases in `inputs/`.

| Line | Means |
|---|---|
| `# case: name` | a new case |
| `< text` | one line of standard input for the next command (`text` and `\n`) |
| `<j "..."` | standard input as a JSON string, exactly, no newline added |
| `<@ name` | standard input from `inputs/name` |
| `$ ["run", "--level", "2"]` | run `skyhop` with these arguments, a JSON array |
| `> text` | the next line of standard output; none at all means standard output must be empty |
| `>~ regex` | the next line of standard output matches `regex`, and every backend printed the same line |
| `! text` | the next line of standard error; likewise |
| `? n` | the exit code; absent means 0 |
| `& KEY=VALUE` | set an environment variable for the rest of the case; empty counts as unset |
| `> @help`, `! @help` | the help text from the contract |

## Where the expected bytes came from

Hand-written for `args.txt` and `input.txt`. For `run.txt` and `draw.txt`, from a throwaway transliteration of `hop.world.step` and `hop.render.scene` that the coordinator used to search for winning scripts; it is not kept. The spec engine confirms what it can: both winning runs end on the ticks and counts the cases show (`hop.tests.runs`). A backend that disagrees with a case is a question for the spec, not a reason to edit the case.

## Limits

- Transcripts check what a program prints. The laws and spec tests run inside each backend, and this runner does not read them.
- `shot` hashes agree only on one machine and one GL driver; they show the four bindings make the same calls, not that the picture is right. `assets/out/preview*.png` and a person's eyes do that.
- `play` is not covered.
