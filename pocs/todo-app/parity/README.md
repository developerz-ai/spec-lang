# Parity suite

> **Design, POC-local.** One set of command-line transcripts that every backend must reproduce byte for byte. It is the outside view of [the contract](../docs/contract.md); the spec's own tests are inside each backend.

```text
bun scripts/parity/run.ts                 # every built backend, each case alone
bun scripts/parity/run.ts --only rust     # one backend
bun scripts/parity/run.ts --mix           # each step of a case on the next backend in turn: one store, four programs
bun scripts/parity/run.ts --race 40       # 40 concurrent adds spread over every backend, then one list
```

## Case files

`cases/*.txt`. Each `# case:` starts in a new empty directory with no `TODO_STORE`.

| Line | Means |
|---|---|
| `$ ["add", "x"]` | run `todo` with these arguments (a JSON array) |
| `> text` | the next line of standard output; none at all means standard output must be empty |
| `! text` | the next line of standard error; likewise |
| `? n` | the exit code; absent means 0 |
| `< text` | write the store (`text` and `\n`) before the next command |
| `= text` | the store now holds exactly `text` and `\n`; `= absent` means no store file |
| `& K=V` | set an environment variable for the rest of the case |
| `>j` `!j` `<j` `=j` | as above, with the text written as a JSON string literal |
| `> @help` `! @help` | the help text from the contract |

## Limits

- Transcripts check what a user sees and what lands on disk. Inside a backend, the laws and spec tests are checked by its own suite, which this runner does not read.
- The race check shows that numbers stay unique under 40 concurrent processes on one machine. It does not prove the lock correct.
