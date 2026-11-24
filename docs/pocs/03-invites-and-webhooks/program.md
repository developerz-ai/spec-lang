# 03 Program

> **Design.** Nothing here runs yet. Invites only; webhooks are in [24](../../idea/24-examples.md#webhooks-tasks-retries-signing).

```slg
type Status = Pending | Accepted | Revoked | Expired;
type Invite(@pii email: Text, @secret token: Text, status: Status = Pending);

state invites: Map[Id[Invite], Invite];
state members: Map[Id[Invite], Id[User]];
state admins: Set[Id[User]];

fn admin(by: Id[User]) -> Bool = by in admins;

@edge(Pending -> Accepted, via: accept)
@edge(Pending -> Revoked, via: revoke, by: admin)
@edge(Pending -> Pending, via: resend, by: admin)
@edge(Pending -> Expired, after: 7d)
flow InviteLife(at: Invite.status);

event accepted(id: Id[Invite], user: Id[User]);

@why("a wrong token and a missing invite give the same answer")
@access(id in invites and invites[id].token == token)
@uses(invites.write, members.write)
@guard(invites[id].status == Pending else Gone)
pub rule accept(by: Id[User], id: Id[Invite], @secret token: Text) = {
  invites[id].status = Accepted;
  members[id] = by;
  accepted.emit(id, user: by);
}

@access(admin(by))
@uses(invites.write)
@guard(invites[id].status == Pending else Gone)
pub rule revoke(by: Id[User], id: Id[Invite]) = {
  invites[id].status = Revoked;
}

@why("a resent invite has a new link, a new week, and a dead old link")
@access(admin(by))
@uses(invites.write, random.read)
@guard(invites[id].status == Pending else Gone)
pub rule resend(by: Id[User], id: Id[Invite]) = {
  invites[id].token = random.token();
  invites[id].status = Pending;
}

law member_means_accepted(t: Trace[accept | revoke | resend | crash], i: Id[Invite]) =
  t.states.all(s => i in s.members implies i in s.invites and s.invites[i].status == Accepted);

law closed_stays_closed(t: Trace[accept | revoke | resend | crash], i: Id[Invite]) =
  t.steps.all(s => i in s.before.invites and s.before.invites[i].status != Pending implies
    s.after.invites[i].status == s.before.invites[i].status);

const boss: Id[User] = Id(1);
const ana: Id[User] = Id(2);
const eve: Id[User] = Id(3);
const i1: Id[Invite] = Id(1);

@given(clock.at(0s), admins -> Set(boss), invites[i1] -> Invite("ana@example.com", token: "t1"))
test invited = {
  assert invites[i1].status == Pending;
}

@given(invited)
test accepted_once = {
  accept(ana, id: i1, token: "t1");
  accept(eve, id: i1, token: "t1") -> fail Gone;
  revoke(boss, id: i1) -> fail Gone;
}

@given(invited)
test wrong_token_looks_like_no_invite = {
  accept(eve, id: i1, token: "guess") -> fail NotFound;
  accept(eve, id: Id(9), token: "t1") -> fail NotFound;
}

@given(invited)
test a_week_passes = {
  clock.advance(7d);
  accept(ana, id: i1, token: "t1") -> fail Gone;
  assert invites[i1].status == Expired;
}

@given(invited)
test resending_restarts_the_week = {
  clock.advance(6d);
  resend(boss, id: i1);
  clock.advance(6d);
  assert invites[i1].status == Pending;
  accept(ana, id: i1, token: "t1") -> fail NotFound;
}

@given(invited)
test only_admins = {
  revoke(eve, id: i1) -> fail NotFound;
  resend(eve, id: i1) -> fail NotFound;
}
```
