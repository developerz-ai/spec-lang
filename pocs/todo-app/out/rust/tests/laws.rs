// Projected by hand from law numbers_below_next, next_only_grows, others_untouched, clear_keeps_every_open_task, reads_change_nothing, refusal_changes_nothing (src/todo/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.
// Each law is checked after every step of 240 seeded traces of 40 random commands.

mod support;

use support::{Rng, random_command, three_open_one_done};
use todo::board::{Command, Halt, Out, World, apply};
use todo::model::{Status, TaskNo};

const TRACES: u64 = 240;
const STEPS: usize = 40;

/// One element of `t.steps`.
struct Step {
    event: Command,
    reply: Result<(), Halt>,
    before: World,
    after: World,
    said: String,
}

/// A trace from the initial world, or from a world the rules can reach.
fn trace(seed: u64) -> Vec<Step> {
    let mut rng = Rng::new(seed);
    let mut world = if seed.is_multiple_of(3) {
        three_open_one_done()
    } else {
        World::default()
    };
    let mut steps = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let event = random_command(&mut rng, &world);
        let before = world.clone();
        let mut out = Out::default();
        let reply = apply(&mut world, &mut out, event.clone());
        steps.push(Step {
            event,
            reply,
            before,
            after: world.clone(),
            said: out.text().to_owned(),
        });
    }
    steps
}

/// Checks `law` after every step of every trace.
fn holds_on_every_step(law: impl Fn(&Step) -> bool) {
    let mut checked = 0;
    for seed in 0..TRACES {
        for (index, step) in trace(seed).iter().enumerate() {
            assert!(law(step), "seed {seed}, step {index}: {:?}", step.event);
            checked += 1;
        }
    }
    assert_eq!(checked, TRACES as usize * STEPS);
}

fn is_change(event: &Command) -> bool {
    event.writes()
}

/// The `no` of the events that name a task.
fn named(event: &Command) -> Option<TaskNo> {
    match event {
        Command::Done { no } | Command::Reopen { no } | Command::Remove { no } => Some(*no),
        Command::Rename { no, .. } => Some(*no),
        _ => None,
    }
}

#[test]
fn numbers_below_next() {
    let below = |world: &World| world.tasks().keys().all(|no| *no < world.next_no());
    holds_on_every_step(|step| below(&step.before) && below(&step.after));
}

#[test]
fn next_only_grows() {
    holds_on_every_step(|step| step.after.next_no() >= step.before.next_no());
}

#[test]
fn others_untouched() {
    holds_on_every_step(|step| {
        let Some(no) = named(&step.event) else {
            return true;
        };
        // Every k that is in either world; for any other k both sides are none.
        step.before
            .tasks()
            .keys()
            .chain(step.after.tasks().keys())
            .filter(|k| **k != no)
            .all(|k| step.after.tasks().get(k) == step.before.tasks().get(k))
    });
}

#[test]
fn clear_keeps_every_open_task() {
    holds_on_every_step(|step| {
        if step.event != Command::Clear {
            return true;
        }
        let open_before = step
            .before
            .tasks()
            .values()
            .filter(|t| t.status == Status::Open);
        step.after
            .tasks()
            .values()
            .all(|t| t.status == Status::Open)
            && step.after.tasks().len() == open_before.count()
    });
}

#[test]
fn reads_change_nothing() {
    holds_on_every_step(|step| is_change(&step.event) || step.after == step.before);
}

#[test]
fn refusal_changes_nothing() {
    holds_on_every_step(|step| {
        step.reply.is_ok() || (step.after == step.before && step.said.is_empty())
    });
}

/// Not a law of the spec: the traces must actually visit what the laws talk about.
#[test]
fn traces_reach_every_rule_and_every_refusal() {
    let steps: Vec<Step> = (0..TRACES).flat_map(trace).collect();
    let seen = |wanted: &dyn Fn(&Step) -> bool| steps.iter().any(wanted);
    for name in [
        "Add", "List", "Done", "Reopen", "Rename", "Remove", "Clear", "Stats",
    ] {
        assert!(
            seen(&|s| s.reply.is_ok() && format!("{:?}", s.event).starts_with(name)),
            "{name}"
        );
    }
    for refusal in ["NoTask", "AlreadyDone", "NotDone"] {
        assert!(
            seen(&|s| s.reply.is_err_and(|halt| halt.to_string() == refusal)),
            "{refusal}"
        );
    }
    assert!(seen(
        &|s| s.event == Command::Clear && s.after.tasks().len() < s.before.tasks().len()
    ));
}

/// The backend limit behaves as a refusal: at 2^53 `add` changes nothing.
#[test]
fn add_at_the_limit_changes_nothing() {
    let mut world = World::restore(TaskNo::LIMIT, []).expect("a world at the limit");
    let before = world.clone();
    let mut out = Out::default();
    let add = Command::Add {
        title: support::title("one too many"),
        priority: Default::default(),
        tags: Default::default(),
    };
    assert_eq!(apply(&mut world, &mut out, add), Err(Halt::Full));
    assert_eq!(world, before);
    assert_eq!(out.text(), "");
}
