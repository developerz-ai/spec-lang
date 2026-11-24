# 03 Invites and webhooks

> **Design.** Nothing here runs yet. A lifecycle with a deadline, and delivery to an unreliable outside.

**Why this one.** It covers what records do not: time, events, retries, calls that may or may not have happened.

| Part | Program | Must work |
|---|---|---|
| invites | [program.md](program.md) | `flow` with `via`, `by`, `after`; `clock.advance`; a race of accept and revoke |
| webhooks | [the example](../../idea/24-examples.md#webhooks-tasks-retries-signing) | events, `task`, `@retry`, `exhausted:`, crash and resume |

## Done when

- The race law holds over every order, including `crash`.
- A seven-day expiry test uses no real waiting.
- For the task: a crash between any two steps, a lost reply and a duplicate trigger are derived and run.
- "Delivered at most once" is reported as `assumed` where it rests on the receiver honouring the key, and nothing stronger is printed.

## Stop if

The run, attempt and step model in [11](../../idea/11-backends-and-parity.md) cannot explain a failing test in one paragraph.

## Not in scope

Mail, HTTP, signing beyond a `foreign` stub.
