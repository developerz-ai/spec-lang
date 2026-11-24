# p03-invites-and-webhooks

Port of `docs/pocs/03-invites-and-webhooks/program.md` (invites only, as the design says). `spec-lang test src/corpus/p03-invites-and-webhooks --deny warnings`: 8 checks (2 laws, 6 scenarios), all pass.

| File | Holds |
|---|---|
| `invites.slg` | the whole module |

## Cuts

| Form in the design | The engine says | The port |
|---|---|---|
| `Id[Invite]`, `Id[User]`, `Id(1)` | `E-UNSUPPORTED` unsupported: the type `Id[Invite]`; `E-NAME-UNKNOWN` no `Id` in scope | `type InviteId = 1..=4;`, `type UserId = 1..=4;`, literals |
| `flow InviteLife(at: Invite.status);` with four `@edge` notes | `E-UNSUPPORTED` unsupported: kind `flow` | dropped. The three moves made by rules are still refused elsewhere by each rule's `Pending` guard; nothing refuses a fourth writer with `IllegalMove` |
| `@edge(Pending -> Expired, after: 7d)` | with the flow; `7d` alone: `E-UNSUPPORTED` unsupported: the number `7d`: only whole numbers without a unit | dropped: no invite expires. `Expired` stays in the type |
| `event accepted(id: Id[Invite], user: Id[User]);` | `E-UNSUPPORTED` unsupported: kind `event` | dropped |
| `accepted.emit(id, user: by);` | `E-NAME-UNKNOWN` no `accepted` in scope | dropped; nothing in the design listens to it |
| `@access(id in invites and invites[id].token == token)`, `@access(admin(by))` | `E-UNSUPPORTED` unsupported: the note `@access` on a `rule` | **a model, not the form**: a first guard with the same condition, `else NotFound` |
| `@secret token: Text` on a rule parameter | `E-UNSUPPORTED` unsupported: notes on parameters | the parameter without its label |
| `random.token()` | `E-NAME-UNKNOWN` no `random` in scope | **a model**: `resend` takes the new token as an argument, `fresh: Text` |
| `@uses(invites.write, random.read)` | accepted; `W-LINT-EFFECT-UNUSED` once the body has no such effect | `@uses(invites.write, admins.read)` |
| `Trace[accept \| revoke \| resend \| crash]` | `E-NAME-UNKNOWN` no `crash` in scope | `Trace[accept \| revoke \| resend]` |
| `@given(clock.at(0s), ...)` | `E-UNSUPPORTED` unsupported: the `@given` item `clock.at(0s)` (handlers are not implemented) | dropped |
| `clock.advance(7d);` | `E-NAME-UNKNOWN` no `clock` in scope | dropped |
| `@given(invites[i1] -> Invite(...))` | `E-UNSUPPORTED` unsupported: the `@given` item `invites[i1] -> ...` | `invites -> Map(i1 -> Invite(...))` |
| test `a_week_passes` | rests on the clock and the flow | dropped |
| test `resending_restarts_the_week` | rests on the clock | kept in part as `resending_kills_the_old_link`: the old token is `NotFound`, the new one is accepted |

Kept as written and accepted without effect: `@pii email` and `@secret token` on the fields of `Invite`.

## Other differences

- `member_means_accepted` quantifies inside (`s.members.keys.all(i => ...)`): with the parameter `i: Id[Invite]` it fails at a start world the engine calls lawful (the finding in `../p02-ledger/README.md`).
- One test added, `a_missing_invite_is_gone_to_an_admin`: `invites[id]` fails inside the guard's condition, so the reply is that guard's `Gone`.
- `@why` written for `revoke` and the two laws; `resend`'s lost "a new week".

## Findings

- `type Change = accept | revoke | resend;` in the module that declares those rules is `E-NAME-AMBIG` ("accept is declared twice"): the names are read as new variants. The card's example names rules of another module (`board.add | board.done`). The port writes the set inside `Trace[...]`.
- Notes on record fields (`@pii`, `@secret`) are accepted while the same notes on a rule's parameter are `E-UNSUPPORTED`.

## Limits

- Half of the design's subject (expiry, the week restarted by a resend, the event) is not in the port.
- A token passed as an argument is not a random token: nothing here says it is unguessable or fresh.
- Mutants (each `Pending` guard dropped in turn, `accept` not setting `Accepted`) were all caught by a law; run once, by hand, with seed 0.
