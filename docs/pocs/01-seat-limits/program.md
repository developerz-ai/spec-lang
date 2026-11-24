# 01 Program

> **Design.** Nothing here runs yet. The whole module.

```slg
type Role = Owner | Member;
type Seat(org: Id[Org], user: Id[User]);
type Org(limit: 1..=500 = 3);

state orgs: Map[Id[Org], Org];
state seats: Map[Seat, Role];

fn taken(org: Id[Org]) -> Int = seats.keys.filter(it.org == org).size;

fn owners(org: Id[Org]) -> Int =
  seats.entries.filter(it.key.org == org and it.value == Owner).size;

fn owns(by: Id[User], org: Id[Org]) -> Bool =
  seats.get(Seat(org, user: by)) == Some(Owner);

@why("a plan sells a number of seats")
@access(owns(by, org: org))
@uses(seats.write, orgs.read)
@guard(not (Seat(org, user: user) in seats) else AlreadyIn)
@guard(taken(org) < orgs[org].limit else Full)
pub rule add(by: Id[User], org: Id[Org], user: Id[User]) = {
  seats[Seat(org, user: user)] = Member;
}

@why("an organisation nobody owns cannot be repaired by its members")
@access(owns(by, org: org))
@uses(seats.write)
@guard(Seat(org, user: user) in seats else NotIn)
@guard(seats[Seat(org, user: user)] == Member or owners(org) > 1 else LastOwner)
pub rule remove(by: Id[User], org: Id[Org], user: Id[User]) = {
  seats.remove(Seat(org, user: user));
}

@access(owns(by, org: org))
@uses(seats.write)
@guard(Seat(org, user: user) in seats else NotIn)
pub rule promote(by: Id[User], org: Id[Org], user: Id[User]) = {
  seats[Seat(org, user: user)] = Owner;
}

@access(owns(by, org: org))
@uses(orgs.write, seats.read)
@guard(taken(org) <= limit else TooMany)
pub rule set_limit(by: Id[User], org: Id[Org], limit: 1..=500) = {
  orgs[org].limit = limit;
}

law never_over(t: Trace[add | remove | promote | set_limit], o: Id[Org]) =
  t.states.all(s => o in s.orgs implies
    s.seats.keys.filter(it.org == o).size <= s.orgs[o].limit);

law always_owned(t: Trace[add | remove | promote | set_limit], o: Id[Org]) =
  t.steps.all(s =>
    s.before.seats.entries.any(it.key.org == o and it.value == Owner) implies
    s.after.seats.entries.any(it.key.org == o and it.value == Owner));

const acme: Id[Org] = Id(1);
const ana: Id[User] = Id(1);
const bob: Id[User] = Id(2);
const cat: Id[User] = Id(3);
const dan: Id[User] = Id(4);

@given(orgs[acme] -> Org(limit: 3), seats[Seat(acme, user: ana)] -> Owner)
test one_owner = {
  assert taken(acme) == 1;
}

@given(one_owner)
test full = {
  add(ana, org: acme, user: bob);
  add(ana, org: acme, user: cat);
  add(ana, org: acme, user: dan) -> fail Full;
}

@given(full)
test room_after_a_removal = {
  remove(ana, org: acme, user: bob);
  add(ana, org: acme, user: dan);
  assert taken(acme) == 3;
}

@given(one_owner)
test the_last_owner_stays = {
  remove(ana, org: acme, user: ana) -> fail LastOwner;
  add(ana, org: acme, user: bob);
  promote(ana, org: acme, user: bob);
  remove(ana, org: acme, user: ana);
}

@given(full)
test members_and_strangers_are_denied = {
  add(bob, org: acme, user: dan) -> fail NotFound;
  add(dan, org: acme, user: dan) -> fail NotFound;
}

@given(full)
test the_limit_cannot_drop_below_use = {
  set_limit(ana, org: acme, limit: 2) -> fail TooMany;
  set_limit(ana, org: acme, limit: 4);
  add(ana, org: acme, user: dan);
}
```

`never_over` must hold from any allowed world, so `set_limit` and `add` each need their guard. Remove `set_limit`'s guard and the shortest counterexample is one call.
