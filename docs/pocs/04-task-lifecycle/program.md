# 04 Program

> **Design.** Nothing here runs yet. The whole module.

```slg
type Life = Queued | Running(box: Id[Box], until: Time) | Done | Failed;
type Task(life: Life = Queued, tries: 0..=3 = 0);

state tasks: Map[Id[Task], Task];
const LEASE: Time = 5min;

fn busy(box: Id[Box]) -> Bool =
  tasks.values.any(it.life.is(Running(box, _)));

@uses(tasks.write, clock.read)
@guard(tasks[id].life == Queued else NotQueued)
@guard(not busy(box) else Busy)
@guard(tasks[id].tries < 3 else Spent)
rule claim(id: Id[Task], box: Id[Box]) = {
  tasks[id].tries = tasks[id].tries + 1;
  tasks[id].life = Running(box, until: now() + LEASE);
}

@uses(tasks.write, clock.read)
@guard(tasks[id].life.is(Running(box, until)) and now() < until else NotYours)
rule beat(id: Id[Task], box: Id[Box]) = {
  tasks[id].life = Running(box, until: now() + LEASE);
}

@why("work that outlived its lease may already be someone else's")
@uses(tasks.write, clock.read)
@guard(tasks[id].life.is(Running(box, until)) and now() < until else NotYours)
rule finish(id: Id[Task], box: Id[Box]) = {
  tasks[id].life = Done;
}

@on(every(1s))
@uses(tasks.write, clock.read)
rule expire() = {
  for e in tasks.entries {
    if e.value.life.is(Running(_, until)) and now() >= until {
      tasks[e.key].life = if e.value.tries < 3 then Queued else Failed;
    }
  }
}

law one_live_task_per_box(t: Trace[claim | beat | finish | expire | crash], b: Id[Box]) =
  t.states.all(s => s.tasks.values.filter(it.life.is(Running(b, _))).size <= 1);

law ended_holds_nothing(t: Trace[claim | beat | finish | expire | crash], i: Id[Task]) =
  t.steps.all(s => i in s.before.tasks and s.before.tasks[i].life in [Done, Failed] implies
    s.after.tasks[i].life == s.before.tasks[i].life);

law a_dead_worker_lets_go(t: Trace[claim | expire], i: Id[Task]) =
  t.states.all(s => i in s.tasks and s.tasks[i].life.is(Running(_, until)) implies
    t.within(LEASE + 1s, not it.tasks[i].life.is(Running(_, until))));

const t1: Id[Task] = Id(1);
const t2: Id[Task] = Id(2);
const b1: Id[Box] = Id(1);
const b2: Id[Box] = Id(2);

@given(clock.at(0s), tasks[t1] -> Task(), tasks[t2] -> Task())
test two_waiting = {
  claim(t1, box: b1);
  claim(t1, box: b2) -> fail NotQueued;
  claim(t2, box: b1) -> fail Busy;
}

@given(two_waiting)
test a_silent_worker_loses_it = {
  clock.advance(5min);
  finish(t1, box: b1) -> fail NotYours;
  assert tasks[t1].life == Queued;
  claim(t2, box: b1);
}

@given(two_waiting)
test beats_keep_it = {
  for i in 1..=3 {
    clock.advance(4min);
    beat(t1, box: b1);
  }
  finish(t1, box: b1);
  assert tasks[t1].life == Done;
}

@given(clock.at(0s), tasks[t1] -> Task())
test three_silent_workers_end_it = {
  for i in 1..=3 {
    claim(t1, box: b1);
    clock.advance(5min);
  }
  assert tasks[t1].life == Failed;
  claim(t1, box: b1) -> fail NotQueued;
}
```

The `a_dead_worker_lets_go` law is the hardest to state, and as written it fails from an allowed start: nothing stops a world where `until` lies years ahead. The engine should show that world and offer the missing law (`until` is never more than `LEASE` away). Adding it is part of the exercise. Python runs covered the first two laws and the ending of every task under a fair claimer, and did not cover this one.
