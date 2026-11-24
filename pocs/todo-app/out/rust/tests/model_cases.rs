// Projected by hand from the @cases of fn is_title, is_tag and the claims of type Title, Tag, TaskNo (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

mod support;

use todo::model::text::{is_white_space, size, trim};
use todo::model::{Priority, Status, Tag, Task, TaskNo, Title, is_tag, is_title};

#[test]
fn is_title_cases() {
    let cases = [
        ("milk", true),
        ("", false),
        ("   ", false),
        ("a\tb", false),
        ("\u{3000}", false),
        ("a\u{2028}b", false),
        ("a\u{0085}b", false),
    ];
    for (text, expected) in cases {
        assert_eq!(is_title(text), expected, "is_title({text:?})");
    }
}

#[test]
fn is_tag_cases() {
    let cases = [
        ("home", true),
        ("deep-work", true),
        ("Home", false),
        ("", false),
        ("a b", false),
        ("-x", false),
        ("--all", false),
    ];
    for (text, expected) in cases {
        assert_eq!(is_tag(text), expected, "is_tag({text:?})");
    }
}

/// The fused `is_title` against the spec's expression written clause by clause.
#[test]
fn is_title_is_the_clauses() {
    fn by_the_letter(t: &str) -> bool {
        size(trim(t)) >= 1
            && size(t) <= 200
            && t.bytes().all(|b| b >= 32 && b != 127)
            && !t.contains('\u{0085}')
            && !t.contains('\u{2028}')
            && !t.contains('\u{2029}')
    }
    let mut rng = support::Rng::new(7);
    let alphabet = [
        ' ',
        'a',
        '\t',
        '\u{7f}',
        '\u{85}',
        '\u{a0}',
        '\u{e9}',
        '\u{3000}',
        '\u{feff}',
        '\u{200b}',
        '\u{180e}',
        '\u{2028}',
        '\u{2029}',
        '\u{1F600}',
        '"',
        '\\',
    ];
    for _ in 0..5000 {
        let length = match rng.below(4) {
            0 => rng.below(4),
            1 => 198 + rng.below(5),
            2 => 790 + rng.below(20),
            _ => rng.below(40),
        };
        let filler = *rng.pick(&alphabet);
        let text: String = (0..length)
            .map(|_| {
                if rng.below(8) == 0 {
                    *rng.pick(&alphabet)
                } else {
                    filler
                }
            })
            .collect();
        assert_eq!(is_title(&text), by_the_letter(&text), "{text:?}");
    }
}

#[test]
fn title_size_counts_code_points() {
    assert!(is_title(&"x".repeat(200)));
    assert!(!is_title(&"x".repeat(201)));
    assert!(is_title(&"\u{e9}".repeat(200)));
    assert!(!is_title(&"\u{e9}".repeat(201)));
    assert!(is_title(&"\u{1F600}".repeat(200)));
    assert!(!is_title(&"\u{1F600}".repeat(201)));
}

#[test]
fn white_space_is_exactly_the_25_code_points() {
    let all: Vec<char> = (0..=0x10FFFF)
        .filter_map(char::from_u32)
        .filter(|c| is_white_space(*c))
        .collect();
    assert_eq!(all.len(), 25);
    for not_white in ['\u{feff}', '\u{180e}', '\u{200b}'] {
        assert!(!is_white_space(not_white));
        assert!(is_title(&not_white.to_string()));
    }
    assert_eq!(trim("\u{2028} a b\u{a0}\u{85}"), "a b");
}

#[test]
fn tag_size_is_one_to_32() {
    assert!(is_tag(&"a".repeat(32)));
    assert!(!is_tag(&"a".repeat(33)));
    assert!(is_tag("0-9"));
    assert!(!is_tag("caf\u{e9}"));
}

#[test]
fn constructors_check_the_claim() {
    assert_eq!(
        Title::new("milk").map(|t| t.to_string()),
        Some("milk".to_owned())
    );
    assert_eq!(Title::new(" "), None);
    assert_eq!(
        Tag::new("home").map(|t| t.to_string()),
        Some("home".to_owned())
    );
    assert_eq!(Tag::new("Home"), None);
    assert_eq!(TaskNo::new(0), None);
    assert_eq!(TaskNo::new(1), Some(TaskNo::FIRST));
}

#[test]
fn task_no_parses_ascii_digits_only() {
    for bad in [
        "", "0", "000", "-1", "+1", "1.5", "abc", " 1", "1 ", "\u{661}", "1e3",
    ] {
        assert_eq!(TaskNo::parse(bad), None, "{bad:?}");
    }
    assert_eq!(TaskNo::parse("7"), TaskNo::new(7));
    assert_eq!(TaskNo::parse("007"), TaskNo::new(7));
    assert_eq!(TaskNo::parse("18446744073709551615"), TaskNo::new(u64::MAX));
}

#[test]
fn task_no_past_the_word_is_still_a_task_no_and_in_no_world() {
    for huge in ["18446744073709551616", "99999999999999999999999999999999"] {
        let no = TaskNo::parse(huge).expect("a TaskNo");
        assert_eq!(no, TaskNo::BEYOND);
        assert!(no > TaskNo::LIMIT);
    }
}

#[test]
fn task_no_successor_stops_at_the_limit() {
    assert_eq!(TaskNo::FIRST.successor(), TaskNo::new(2));
    assert_eq!(
        TaskNo::new((1 << 53) - 1).and_then(TaskNo::successor),
        Some(TaskNo::LIMIT)
    );
    assert_eq!(TaskNo::LIMIT.successor(), None);
    assert_eq!(TaskNo::BEYOND.successor(), None);
}

#[test]
fn task_defaults() {
    let task = Task::new(support::no(1), support::title("milk"));
    assert_eq!(task.status, Status::Open);
    assert_eq!(task.priority, Priority::Normal);
    assert!(task.tags.is_empty());
}

#[test]
fn variant_names_round_trip() {
    for priority in Priority::ALL {
        assert_eq!(Priority::from_name(priority.name()), Some(priority));
    }
    for status in [Status::Open, Status::Done] {
        assert_eq!(Status::from_name(status.name()), Some(status));
    }
    assert_eq!(Status::from_name("open"), None);
    assert_eq!(Priority::from_name("high"), None);
}
