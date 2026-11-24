# 06 Webhooks on Ultimate

> **Design.** Nothing here runs yet. The first backend, on the one feature already measured.

**Why this one.** It is the claim in the README: 6,705 tokens of TypeScript against 2,509 of spec-lang, for one feature, never run ([12](../../idea/12-frameworks.md)).

**Program:** [webhooks](../../idea/24-examples.md#webhooks-tasks-retries-signing). **Target:** the reference app in the public [Ultimate](https://github.com/developerz-ai/ultimate) repository, in a fresh clone.

## Build twice

| Arm | Who | Rules |
|---|---|---|
| A | an agent, plain Ultimate | best tools, docs and tests it can use |
| B | the same model, spec-lang and the backend | the card, the engine, no hand edits to output |

Same model, same effort, clean sessions, both with the same acceptance tests written by a third party. At least three runs per arm. Count the tokens of everything read and written, the card and every repair included.

## Done when

- The same tests agree in the engine and on the built app, and Ultimate's own gate is green.
- No generated file was edited by hand.
- A guard over a count survives two forced concurrent writers, or the backend refused it with a reason.
- Tokens, turns, time and escaped defects are reported for both arms, with spread.

## Stop if

B is not cheaper than A at the same defects by the thresholds in [19](../../idea/19-density-and-experiments.md), or one hand edit was needed to ship.

## Not in scope

Screens.
