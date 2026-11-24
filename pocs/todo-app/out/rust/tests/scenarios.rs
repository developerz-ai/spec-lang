// Projected by hand from test three_open_one_done, numbers_start_at_one, numbers_are_never_reused, empty_board_says_so, list_ranks_urgent_first, list_all_and_by_tag, done_and_reopen_move_once, rename_keeps_the_rest, clear_removes_only_done, unknown_numbers_are_refused (src/todo/tests/scenarios.slg). Stand-in for spec-lang build output; change the spec, not this file.
// No mocks: the rules are called directly on a world.

mod support;

use std::collections::BTreeSet;

use support::{Rng, no, tag, tags, task, three_open_one_done, title};
use todo::board::{self, Out, Refusal, World};
use todo::model::{Priority, Status, TaskNo};

#[test]
fn three_open_one_done_holds() {
    let world = three_open_one_done();
    assert_eq!(world.tasks().len(), 4);
    assert_eq!(world.next_no(), no(5));
    let open = world.tasks().values().filter(|t| t.status == Status::Open);
    assert_eq!(open.count(), 3);
}

#[test]
fn numbers_start_at_one() {
    let (mut world, mut out) = (World::default(), Out::default());
    let first = board::add(
        &mut world,
        &mut out,
        title("milk"),
        Priority::Normal,
        BTreeSet::new(),
    );
    assert_eq!(first, Ok(no(1)));
    let second = board::add(
        &mut world,
        &mut out,
        title("bread"),
        Priority::Normal,
        tags(&["shop", "home"]),
    );
    assert_eq!(second, Ok(no(2)));
    assert_eq!(out.text(), "added 1\nadded 2\n");
}

#[test]
fn numbers_are_never_reused() {
    let (mut world, mut out) = (World::default(), Out::default());
    let mut add = |world: &mut World, text| {
        board::add(
            world,
            &mut out,
            title(text),
            Priority::Normal,
            BTreeSet::new(),
        )
    };
    assert_eq!(add(&mut world, "a"), Ok(no(1)));
    assert_eq!(add(&mut world, "b"), Ok(no(2)));
    assert_eq!(
        board::remove(&mut world, &mut Out::default(), no(2)),
        Ok(())
    );
    assert_eq!(add(&mut world, "c"), Ok(no(3)));
}

#[test]
fn empty_board_says_so() {
    let (world, mut out) = (World::default(), Out::default());
    board::list(&world, &mut out, false, None);
    board::stats(&world, &mut out);
    assert_eq!(out.text(), "nothing to do\n0 open, 0 done\n");
}

#[test]
fn list_ranks_urgent_first() {
    let (world, mut out) = (three_open_one_done(), Out::default());
    board::list(&world, &mut out, false, None);
    assert_eq!(
        out.text(),
        "3 [ ] urgent !high #work\n2 [ ] plain\n1 [ ] low one !low\n"
    );
}

#[test]
fn list_all_and_by_tag() {
    let (world, mut out) = (three_open_one_done(), Out::default());
    board::list(&world, &mut out, true, Some(&tag("work")));
    assert_eq!(
        out.text(),
        "3 [ ] urgent !high #work\n4 [x] finished #home #work\n"
    );
}

#[test]
fn done_and_reopen_move_once() {
    let (mut world, mut out) = (three_open_one_done(), Out::default());
    assert_eq!(board::done(&mut world, &mut out, no(2)), Ok(()));
    assert_eq!(
        board::done(&mut world, &mut out, no(2)),
        Err(Refusal::AlreadyDone)
    );
    assert_eq!(board::reopen(&mut world, &mut out, no(2)), Ok(()));
    assert_eq!(
        board::reopen(&mut world, &mut out, no(2)),
        Err(Refusal::NotDone)
    );
    assert_eq!(out.text(), "2 [x] plain\n2 [ ] plain\n");
}

#[test]
fn rename_keeps_the_rest() {
    let (mut world, mut out) = (three_open_one_done(), Out::default());
    assert_eq!(
        board::rename(&mut world, &mut out, no(3), title("urgent, really")),
        Ok(())
    );
    let expected = task(3, "urgent, really", Status::Open, Priority::High, &["work"]);
    assert_eq!(world.tasks().get(&no(3)), Some(&expected));
}

#[test]
fn clear_removes_only_done() {
    let (mut world, mut out) = (three_open_one_done(), Out::default());
    assert_eq!(board::clear(&mut world, &mut out), 1);
    board::stats(&world, &mut out);
    assert_eq!(out.text(), "cleared 1\n3 open, 0 done\n");
    assert!(!world.tasks().contains_key(&no(4)));
}

/// A property: `no` is generated from its type, small numbers, large ones and the one past the word included.
#[test]
fn unknown_numbers_are_refused() {
    let mut rng = Rng::new(2026);
    let mut numbers: Vec<TaskNo> = (1..=12).map(no).collect();
    numbers.extend([TaskNo::LIMIT, TaskNo::BEYOND]);
    numbers.extend((0..300).map(|_| no(rng.next().max(1))));
    for number in numbers {
        if number <= no(4) {
            continue;
        }
        let (mut world, mut out) = (three_open_one_done(), Out::default());
        assert_eq!(
            board::done(&mut world, &mut out, number),
            Err(Refusal::NoTask)
        );
        assert_eq!(
            board::reopen(&mut world, &mut out, number),
            Err(Refusal::NoTask)
        );
        assert_eq!(
            board::rename(&mut world, &mut out, number, title("x")),
            Err(Refusal::NoTask)
        );
        assert_eq!(
            board::remove(&mut world, &mut out, number),
            Err(Refusal::NoTask)
        );
        assert_eq!(out.text(), "");
        assert_eq!(world, three_open_one_done());
    }
}
