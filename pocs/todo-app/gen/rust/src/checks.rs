//! Every check of the program, as `spec-lang test` names and runs it.
use crate::rt::{self, R};
use crate::rt::rng::Rng;
use crate::rt::runner::{Body, Check, Given, LawBody, LawDecl, TestDecl, Trace, What};
use crate::machine::{Machine, Pools};
use std::collections::BTreeSet;
/// `@given(todo.board.tasks -> ..)` of `todo.board.three_open_one_done`.
fn given_0_0(m: &mut Machine) -> R<()> {
    let v = rt::map_of(
        vec![
            (
                1,
                crate::todo::model::Task::new(
                    1,
                    String::from("low one"),
                    None,
                    Some(crate::todo::model::Priority::Low),
                    None,
                )?,
            ),
            (
                2,
                crate::todo::model::Task::new(
                    2,
                    String::from("plain"),
                    None,
                    None,
                    None,
                )?,
            ),
            (
                3,
                crate::todo::model::Task::new(
                    3,
                    String::from("urgent"),
                    None,
                    Some(crate::todo::model::Priority::High),
                    Some(BTreeSet::from([String::from("work")])),
                )?,
            ),
            (
                4,
                crate::todo::model::Task::new(
                    4,
                    String::from("finished"),
                    Some(crate::todo::model::Status::Done),
                    None,
                    Some(BTreeSet::from([String::from("work"), String::from("home")])),
                )?,
            ),
        ],
    );
    rt::claim(
        crate::conform::map_task_no_task(&v)?,
        "the `@given` value of todo.board.tasks",
    )?;
    m.world.tasks = v;
    Ok(())
}
/// `@given(todo.board.next_no -> ..)` of `todo.board.three_open_one_done`.
fn given_0_1(m: &mut Machine) -> R<()> {
    let v = 5;
    rt::claim(crate::conform::task_no(&v)?, "the `@given` value of todo.board.next_no")?;
    m.world.next_no = v;
    Ok(())
}
/// A test with parameters has no one end world.
fn no_end(_: &mut Machine) -> R<()> {
    Err(rt::stop("E-TYPE-MISMATCH", "a test with parameters has no one end world"))
}
/// The parameters of `todo.laws.numbers_below_next` besides its trace, drawn after it.
fn law_0(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::todo::laws::numbers_below_next(
            m,
            t,
        )),
    )
}
/// The parameters of `todo.laws.next_only_grows` besides its trace, drawn after it.
fn law_1(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::todo::laws::next_only_grows(
            m,
            t,
        )),
    )
}
/// The parameters of `todo.laws.others_untouched` besides its trace, drawn after it.
fn law_2(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<LawBody> {
    let a1 = crate::draw::task_no(m, rng, pools)?;
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::todo::laws::others_untouched(
            m,
            t,
            a1,
        )),
    )
}
/// The parameters of `todo.laws.clear_keeps_every_open_task` besides its trace, drawn after it.
fn law_3(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::todo::laws::clear_keeps_every_open_task(
            m,
            t,
        )),
    )
}
/// The parameters of `todo.laws.reads_change_nothing` besides its trace, drawn after it.
fn law_4(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::todo::laws::reads_change_nothing(
            m,
            t,
        )),
    )
}
/// The parameters of `todo.laws.refusal_changes_nothing` besides its trace, drawn after it.
fn law_5(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::todo::laws::refusal_changes_nothing(
            m,
            t,
        )),
    )
}
/// The parameters of `todo.laws.stored_under_its_number` besides its trace, drawn after it.
fn law_6(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::todo::laws::stored_under_its_number(
            m,
            t,
        )),
    )
}
/// `todo.model.is_title case[0]`.
fn case_8(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_title(String::from("milk")))?;
    let want: bool = true;
    Ok(got == Ok(want))
}
/// `todo.model.is_title case[1]`.
fn case_9(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_title(String::from("")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_title case[2]`.
fn case_10(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_title(String::from("   ")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_title case[3]`.
fn case_11(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_title(String::from("a\tb")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_title case[4]`.
fn case_12(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_title(String::from("\u{3000}")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_title case[5]`.
fn case_13(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_title(String::from("a\u{2028}b")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_title case[6]`.
fn case_14(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_title(String::from("a\u{85}b")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_tag case[0]`.
fn case_15(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_tag(String::from("home")))?;
    let want: bool = true;
    Ok(got == Ok(want))
}
/// `todo.model.is_tag case[1]`.
fn case_16(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_tag(String::from("deep-work")))?;
    let want: bool = true;
    Ok(got == Ok(want))
}
/// `todo.model.is_tag case[2]`.
fn case_17(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_tag(String::from("Home")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_tag case[3]`.
fn case_18(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_tag(String::from("")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_tag case[4]`.
fn case_19(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_tag(String::from("a b")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_tag case[5]`.
fn case_20(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_tag(String::from("-x")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.model.is_tag case[6]`.
fn case_21(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::model::is_tag(String::from("--all")))?;
    let want: bool = false;
    Ok(got == Ok(want))
}
/// `todo.render.tag_text case[0]`.
fn case_22(m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::todo::render::tag_text(BTreeSet::<String>::new()))?;
    let want: String = String::from("");
    Ok(got == Ok(want))
}
/// `todo.render.tag_text case[1]`.
fn case_23(m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::todo::render::tag_text(
            BTreeSet::from([String::from("work"), String::from("home")]),
        ),
    )?;
    let want: String = String::from(" #home #work");
    Ok(got == Ok(want))
}
/// `todo.render.line case[0]`.
fn case_24(m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::todo::render::line(
            crate::todo::model::Task::new(1, String::from("milk"), None, None, None)?,
        ),
    )?;
    let want: String = String::from("1 [ ] milk");
    Ok(got == Ok(want))
}
/// `todo.render.line case[1]`.
fn case_25(m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::todo::render::line(
            crate::todo::model::Task::new(
                2,
                String::from("ship"),
                Some(crate::todo::model::Status::Done),
                Some(crate::todo::model::Priority::High),
                Some(BTreeSet::from([String::from("work")])),
            )?,
        ),
    )?;
    let want: String = String::from("2 [x] ship !high #work");
    Ok(got == Ok(want))
}
/// Arguments of `todo.tests.scenarios.unknown_numbers_are_refused` drawn from its parameters' types.
fn draw_34(m: &mut Machine, rng: &mut Rng) -> R<Body> {
    let pools = Pools::default();
    let a0 = crate::draw::task_no(m, rng, &pools)?;
    Ok(
        Box::new(move |m: &mut Machine| crate::todo::tests::scenarios::unknown_numbers_are_refused(
            m,
            a0,
        )),
    )
}
/// Every test, by number.
pub static TESTS: &[TestDecl] = &[
    TestDecl {
        name: "todo.board.three_open_one_done",
        has_params: false,
        given: &[Given::State(given_0_0), Given::State(given_0_1)],
        run: crate::todo::board::three_open_one_done,
    },
    TestDecl {
        name: "todo.tests.scenarios.numbers_start_at_one",
        has_params: false,
        given: &[],
        run: crate::todo::tests::scenarios::numbers_start_at_one,
    },
    TestDecl {
        name: "todo.tests.scenarios.numbers_are_never_reused",
        has_params: false,
        given: &[],
        run: crate::todo::tests::scenarios::numbers_are_never_reused,
    },
    TestDecl {
        name: "todo.tests.scenarios.empty_board_says_so",
        has_params: false,
        given: &[],
        run: crate::todo::tests::scenarios::empty_board_says_so,
    },
    TestDecl {
        name: "todo.tests.scenarios.list_ranks_urgent_first",
        has_params: false,
        given: &[Given::Test(0)],
        run: crate::todo::tests::scenarios::list_ranks_urgent_first,
    },
    TestDecl {
        name: "todo.tests.scenarios.list_all_and_by_tag",
        has_params: false,
        given: &[Given::Test(0)],
        run: crate::todo::tests::scenarios::list_all_and_by_tag,
    },
    TestDecl {
        name: "todo.tests.scenarios.done_and_reopen_move_once",
        has_params: false,
        given: &[Given::Test(0)],
        run: crate::todo::tests::scenarios::done_and_reopen_move_once,
    },
    TestDecl {
        name: "todo.tests.scenarios.rename_keeps_the_rest",
        has_params: false,
        given: &[Given::Test(0)],
        run: crate::todo::tests::scenarios::rename_keeps_the_rest,
    },
    TestDecl {
        name: "todo.tests.scenarios.clear_removes_only_done",
        has_params: false,
        given: &[Given::Test(0)],
        run: crate::todo::tests::scenarios::clear_removes_only_done,
    },
    TestDecl {
        name: "todo.tests.scenarios.unknown_numbers_are_refused",
        has_params: true,
        given: &[Given::Test(0)],
        run: no_end,
    },
];
/// Every law, by number.
pub static LAWS: &[LawDecl] = &[
    LawDecl {
        name: "todo.laws.numbers_below_next",
        traced: &[0, 2, 3, 4, 5, 6],
        start: &[0, 1, 2, 3, 4, 5, 6, 7],
        draw: law_0,
    },
    LawDecl {
        name: "todo.laws.next_only_grows",
        traced: &[0],
        start: &[0, 1, 2, 3, 4, 5, 6, 7],
        draw: law_1,
    },
    LawDecl {
        name: "todo.laws.others_untouched",
        traced: &[2, 3, 4, 5],
        start: &[0, 1, 2, 3, 4, 5, 6, 7],
        draw: law_2,
    },
    LawDecl {
        name: "todo.laws.clear_keeps_every_open_task",
        traced: &[0, 2, 3, 4, 5, 6],
        start: &[0, 1, 2, 3, 4, 5, 6, 7],
        draw: law_3,
    },
    LawDecl {
        name: "todo.laws.reads_change_nothing",
        traced: &[1, 7],
        start: &[0, 1, 2, 3, 4, 5, 6, 7],
        draw: law_4,
    },
    LawDecl {
        name: "todo.laws.refusal_changes_nothing",
        traced: &[0, 2, 3, 4, 5, 6],
        start: &[0, 1, 2, 3, 4, 5, 6, 7],
        draw: law_5,
    },
    LawDecl {
        name: "todo.laws.stored_under_its_number",
        traced: &[0, 2, 3, 4, 5, 6],
        start: &[0, 1, 2, 3, 4, 5, 6, 7],
        draw: law_6,
    },
];
/// Every check, in the order `spec-lang test` runs them.
pub static CHECKS: &[Check] = &[
    Check {
        name: "todo.board.three_open_one_done",
        what: What::Test(0),
    },
    Check {
        name: "todo.laws.numbers_below_next",
        what: What::Law(0),
    },
    Check {
        name: "todo.laws.next_only_grows",
        what: What::Law(1),
    },
    Check {
        name: "todo.laws.others_untouched",
        what: What::Law(2),
    },
    Check {
        name: "todo.laws.clear_keeps_every_open_task",
        what: What::Law(3),
    },
    Check {
        name: "todo.laws.reads_change_nothing",
        what: What::Law(4),
    },
    Check {
        name: "todo.laws.refusal_changes_nothing",
        what: What::Law(5),
    },
    Check {
        name: "todo.laws.stored_under_its_number",
        what: What::Law(6),
    },
    Check {
        name: "todo.model.is_title case[0]",
        what: What::Case(case_8),
    },
    Check {
        name: "todo.model.is_title case[1]",
        what: What::Case(case_9),
    },
    Check {
        name: "todo.model.is_title case[2]",
        what: What::Case(case_10),
    },
    Check {
        name: "todo.model.is_title case[3]",
        what: What::Case(case_11),
    },
    Check {
        name: "todo.model.is_title case[4]",
        what: What::Case(case_12),
    },
    Check {
        name: "todo.model.is_title case[5]",
        what: What::Case(case_13),
    },
    Check {
        name: "todo.model.is_title case[6]",
        what: What::Case(case_14),
    },
    Check {
        name: "todo.model.is_tag case[0]",
        what: What::Case(case_15),
    },
    Check {
        name: "todo.model.is_tag case[1]",
        what: What::Case(case_16),
    },
    Check {
        name: "todo.model.is_tag case[2]",
        what: What::Case(case_17),
    },
    Check {
        name: "todo.model.is_tag case[3]",
        what: What::Case(case_18),
    },
    Check {
        name: "todo.model.is_tag case[4]",
        what: What::Case(case_19),
    },
    Check {
        name: "todo.model.is_tag case[5]",
        what: What::Case(case_20),
    },
    Check {
        name: "todo.model.is_tag case[6]",
        what: What::Case(case_21),
    },
    Check {
        name: "todo.render.tag_text case[0]",
        what: What::Case(case_22),
    },
    Check {
        name: "todo.render.tag_text case[1]",
        what: What::Case(case_23),
    },
    Check {
        name: "todo.render.line case[0]",
        what: What::Case(case_24),
    },
    Check {
        name: "todo.render.line case[1]",
        what: What::Case(case_25),
    },
    Check {
        name: "todo.tests.scenarios.numbers_start_at_one",
        what: What::Test(1),
    },
    Check {
        name: "todo.tests.scenarios.numbers_are_never_reused",
        what: What::Test(2),
    },
    Check {
        name: "todo.tests.scenarios.empty_board_says_so",
        what: What::Test(3),
    },
    Check {
        name: "todo.tests.scenarios.list_ranks_urgent_first",
        what: What::Test(4),
    },
    Check {
        name: "todo.tests.scenarios.list_all_and_by_tag",
        what: What::Test(5),
    },
    Check {
        name: "todo.tests.scenarios.done_and_reopen_move_once",
        what: What::Test(6),
    },
    Check {
        name: "todo.tests.scenarios.rename_keeps_the_rest",
        what: What::Test(7),
    },
    Check {
        name: "todo.tests.scenarios.clear_removes_only_done",
        what: What::Test(8),
    },
    Check {
        name: "todo.tests.scenarios.unknown_numbers_are_refused",
        what: What::Property {
            test: 9,
            draw: draw_34,
        },
    },
];
