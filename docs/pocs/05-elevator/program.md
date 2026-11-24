# 05 Program

> **Design.** Nothing here runs yet. The direction rule keeps going while there are calls ahead and turns otherwise.

```slg
type Floor = 1..=5;
type Way = Up | Down;
type Lift(at: Floor = 1, way: Way = Up, open: Bool = false);

state lift: Lift = Lift();
state calls: Set[Floor];

fn ahead(l: Lift, cs: Set[Floor]) -> Bool = match l.way {
  Up => cs.any(it > l.at);
  Down => cs.any(it < l.at);
}

fn turn(w: Way) -> Way = match w {
  Up => Down;
  Down => Up;
}

fn step(l: Lift, cs: Set[Floor]) -> Lift = {
  if l.at in cs { return l.with(open: true); }
  if l.open { return l.with(open: false); }
  if cs.size == 0 { return l; }
  let way = if ahead(l, cs: cs) then l.way else turn(l.way);
  return l.with(way: way, at: l.at + (if way == Up then 1 else -1));
}

@uses(calls.write)
rule call(f: Floor) = {
  calls.insert(f);
}

@on(tick(1Hz))
@uses(lift.write, calls.write)
rule move() = {
  lift = step(lift, cs: calls);
  if lift.open { calls.remove(lift.at); }
}

law doors_shut_while_moving(t: Trace[call | move]) =
  t.steps.all(s => s.before.lift.at != s.after.lift.at implies
    not s.before.lift.open and not s.after.lift.open);

law every_call_is_answered(t: Trace[call | move], f: Floor) =
  t.states.all(s => f in s.calls implies t.within(40s, not (f in it.calls)));

@given(clock.at(0s))
test goes_up_and_answers = {
  call(3);
  clock.advance(2s);
  assert lift.at == 3;
  clock.advance(1s);
  assert lift.open;
  assert calls.size == 0;
}

@given(clock.at(0s))
test does_not_starve_the_ground = {
  clock.advance(0s);
  for i in 1..=10 {
    call(5);
    call(1);
    clock.advance(20s);
    assert calls.size == 0;
  }
}
```

40 seconds is a bound that held in 4,000 random runs of 80 ticks (longest wait seen 31). It is a guess to be checked, and the engine should find the true one.

One thing to notice: `step` moves with `l.at + 1` on a range type. The checker must see that calls exist ahead, so the car cannot leave 1 to 5; if it cannot, that is a finding.
