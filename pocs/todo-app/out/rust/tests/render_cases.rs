// Projected by hand from the @cases of fn tag_text, line and fn mark, badge, urgency, rank (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

mod support;

use support::{tags, task};
use todo::model::{Priority, Status};
use todo::render::{Line, badge, line, mark, rank, tag_text, urgency};

#[test]
fn tag_text_cases() {
    assert_eq!(tag_text(&tags(&[])), "");
    assert_eq!(tag_text(&tags(&["work", "home"])), " #home #work");
}

#[test]
fn line_cases() {
    let cases = [
        (
            task(1, "milk", Status::Open, Priority::Normal, &[]),
            "1 [ ] milk",
        ),
        (
            task(2, "ship", Status::Done, Priority::High, &["work"]),
            "2 [x] ship !high #work",
        ),
    ];
    for (task, expected) in cases {
        assert_eq!(line(&task), expected);
        assert_eq!(Line(&task).to_string(), expected);
    }
}

#[test]
fn mark_and_badge() {
    assert_eq!(mark(Status::Open), " ");
    assert_eq!(mark(Status::Done), "x");
    assert_eq!(badge(Priority::Low), " !low");
    assert_eq!(badge(Priority::Normal), "");
    assert_eq!(badge(Priority::High), " !high");
}

#[test]
fn rank_is_most_urgent_then_oldest() {
    assert!(urgency(Priority::High) < urgency(Priority::Normal));
    assert!(urgency(Priority::Normal) < urgency(Priority::Low));
    let old_low = task(1, "a", Status::Open, Priority::Low, &[]);
    let new_high = task(9, "b", Status::Open, Priority::High, &[]);
    let newer_high = task(10, "c", Status::Open, Priority::High, &[]);
    assert!(rank(&new_high) < rank(&old_low));
    assert!(rank(&new_high) < rank(&newer_high));
}
