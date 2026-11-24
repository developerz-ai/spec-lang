// Projected by hand from note @store: "Bytes written" and "Accepted on read" (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

mod support;

use support::{Rng, no, random_command, task, three_open_one_done};
use todo::board::{Out, Tally, World, apply, stats};
use todo::model::{Priority, Status, TaskNo};
use todo::store::codec::{Unreadable, decode, encode, scan};

const FIXTURE: &str = r#"{"version":1,"next_no":5,"tasks":[{"no":1,"title":"low one","status":"Open","priority":"Low","tags":[]},{"no":2,"title":"plain","status":"Open","priority":"Normal","tags":[]},{"no":3,"title":"urgent","status":"Open","priority":"High","tags":["work"]},{"no":4,"title":"finished","status":"Done","priority":"Normal","tags":["home","work"]}]}"#;

fn text(world: &World) -> String {
    String::from_utf8(encode(world)).expect("the canonical form is UTF-8")
}

/// A world some sequence of commands can reach.
fn random_world(seed: u64) -> World {
    let mut rng = Rng::new(seed);
    let mut world = World::default();
    for _ in 0..rng.below(60) {
        let command = random_command(&mut rng, &world);
        let _ = apply(&mut world, &mut Out::default(), command);
    }
    world
}

#[test]
fn canonical_bytes_of_the_contract_example() {
    let world = World::restore(
        no(3),
        [
            task(
                2,
                "bread \"fresh\"",
                Status::Done,
                Priority::High,
                &["shop", "home"],
            ),
            task(1, "milk", Status::Open, Priority::Normal, &[]),
        ],
    )
    .expect("a world");
    let expected = r#"{"version":1,"next_no":3,"tasks":[{"no":1,"title":"milk","status":"Open","priority":"Normal","tags":[]},{"no":2,"title":"bread \"fresh\"","status":"Done","priority":"High","tags":["home","shop"]}]}"#;
    assert_eq!(text(&world), format!("{expected}\n"));
}

#[test]
fn canonical_bytes_of_the_initial_and_the_given_world() {
    assert_eq!(
        text(&World::default()),
        "{\"version\":1,\"next_no\":1,\"tasks\":[]}\n"
    );
    assert_eq!(text(&three_open_one_done()), format!("{FIXTURE}\n"));
}

#[test]
fn only_quote_and_backslash_are_escaped() {
    let title = "  caf\u{e9} \u{2615} \"q\" back\\slash a\u{2027}b\u{84}c \u{1F600}  ";
    let world = World::restore(no(2), [task(1, title, Status::Open, Priority::Normal, &[])])
        .expect("a world");
    let escaped = title.replace('\\', "\\\\").replace('"', "\\\"");
    assert!(text(&world).contains(&format!("\"title\":\"{escaped}\",")));
    assert_eq!(decode(&encode(&world)).expect("readable"), world);
}

#[test]
fn round_trip() {
    for seed in 0..300 {
        let world = random_world(seed);
        let bytes = encode(&world);
        let back = decode(&bytes).unwrap_or_else(|why| panic!("seed {seed}: {why}"));
        assert_eq!(back, world, "seed {seed}");
        assert_eq!(encode(&back), bytes, "seed {seed}");
        assert_eq!(bytes.iter().filter(|b| **b == b'\n').count(), 1);
    }
}

#[test]
fn the_fold_sees_what_the_world_holds() {
    for seed in 0..300 {
        let world = random_world(seed);
        let mut tally = Tally::default();
        let mut seen = Vec::new();
        let next_no = scan(&encode(&world), |view| {
            tally.count(view.status);
            seen.push((view.no, view.title.to_owned(), view.tags.len()));
            Ok(())
        })
        .expect("readable");
        assert_eq!(next_no, world.next_no());
        let held: Vec<_> = world
            .tasks()
            .values()
            .map(|t| (t.no, t.title.to_string(), t.tags.len()))
            .collect();
        assert_eq!(seen, held);
        let (mut folded, mut by_rule) = (Out::default(), Out::default());
        tally.report(&mut folded);
        stats(&world, &mut by_rule);
        assert_eq!(folded, by_rule, "seed {seed}");
    }
}

#[test]
fn reads_any_valid_json_that_meets_the_schema() {
    let loose = "\t{ \"tasks\" : [ {\"tags\":[\"b\",\"a\"],\"priority\":\"High\",\"status\":\"Done\",\"title\":\"caf\\u00e9 \\ud83d\\ude00 \\/ \\u0041\",\"no\":2},\r\n {\"no\":1,\"title\":\"x\",\"status\":\"Open\",\"priority\":\"Low\",\"tags\":[]} ], \"next_no\" : 7, \"version\" : 1 }\n\n";
    let expected = World::restore(
        no(7),
        [
            task(1, "x", Status::Open, Priority::Low, &[]),
            task(
                2,
                "caf\u{e9} \u{1F600} / A",
                Status::Done,
                Priority::High,
                &["a", "b"],
            ),
        ],
    )
    .expect("a world");
    assert_eq!(decode(loose.as_bytes()).expect("readable"), expected);
}

#[test]
fn reads_keys_given_with_escapes() {
    let escaped = r#"{"version":1,"next_no":1,"tasks":[]}"#;
    assert_eq!(
        decode(escaped.as_bytes()).expect("readable"),
        World::default()
    );
}

#[test]
fn next_no_may_be_exactly_two_to_the_53() {
    let at_limit = r#"{"version":1,"next_no":9007199254740992,"tasks":[{"no":9007199254740991,"title":"a","status":"Open","priority":"Normal","tags":[]}]}"#;
    let world = decode(at_limit.as_bytes()).expect("readable");
    assert_eq!(world.next_no(), TaskNo::LIMIT);
    assert_eq!(text(&world), format!("{at_limit}\n"));
}

fn one_task(fields: &str) -> String {
    format!(r#"{{"version":1,"next_no":2,"tasks":[{{{fields}}}]}}"#)
}

#[test]
fn each_unreadable_shape() {
    type Is = fn(&Unreadable) -> bool;
    let good = r#""no":1,"title":"a","status":"Open","priority":"Normal","tags":[]"#;
    assert!(decode(one_task(good).as_bytes()).is_ok());
    let long_title = format!(
        r#""no":1,"title":"{}","status":"Open","priority":"Normal","tags":[]"#,
        "x".repeat(201)
    );
    let cases: Vec<(String, Is)> = vec![
        ("".into(), |e| matches!(e, Unreadable::Syntax(0))),
        ("\n".into(), |e| matches!(e, Unreadable::Syntax(_))),
        ("todo".into(), |e| matches!(e, Unreadable::Syntax(_))),
        ("[1]".into(), |e| matches!(e, Unreadable::Syntax(_))),
        ("null".into(), |e| matches!(e, Unreadable::Syntax(_))),
        ("{}".into(), |e| matches!(e, Unreadable::MissingKey)),
        (r#"{"version":1,"next_no":1}"#.into(), |e| matches!(e, Unreadable::MissingKey)),
        (r#"{"version":2,"next_no":1,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Version)),
        (r#"{"version":1.0,"next_no":1,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":"1","next_no":1,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":0,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":1.0,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":1e0,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":1E0,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":"1","tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":-1,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":01,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":9007199254740993,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":99999999999999999999999999,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::Integer)),
        (r#"{"version":1,"next_no":1,"tasks":{}}"#.into(), |e| matches!(e, Unreadable::Syntax(_))),
        (r#"{"version":1,"next_no":1,"tasks":[],"x":0}"#.into(), |e| matches!(e, Unreadable::UnknownKey)),
        (r#"{"version":1,"next_no":1,"next_no":1,"tasks":[]}"#.into(), |e| matches!(e, Unreadable::DuplicateKey)),
        (r#"{"version":1,"next_no":1,"tasks":[],"tasks":[]}"#.into(), |e| matches!(e, Unreadable::DuplicateKey)),
        (r#"{"version":1,"next_no":1,"tasks":[],}"#.into(), |e| matches!(e, Unreadable::Syntax(_))),
        (r#"{"version":1,"next_no":1,"tasks":[]} x"#.into(), |e| matches!(e, Unreadable::Syntax(_))),
        (r#"{"version":1,"next_no":1,"tasks":[]}{}"#.into(), |e| matches!(e, Unreadable::Syntax(_))),
        ("\u{feff}{\"version\":1,\"next_no\":1,\"tasks\":[]}".into(), |e| matches!(e, Unreadable::Syntax(0))),
        (r#"{"version":1,"next_no":1,"tasks":[1]}"#.into(), |e| matches!(e, Unreadable::Syntax(_))),
        (one_task(r#""no":1,"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::DuplicateKey)),
        (one_task(r#""no":1,"title":"a","status":"Open","priority":"Normal","tags":[],"due":null"#), |e| matches!(e, Unreadable::UnknownKey)),
        (one_task(r#""no":1,"title":"a","status":"Open","priority":"Normal""#), |e| matches!(e, Unreadable::MissingKey)),
        (one_task(r#""no":2,"title":"a","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::NumberNotBelowNext)),
        (one_task(r#""no":0,"title":"a","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Integer)),
        (one_task(r#""no":1.0,"title":"a","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Integer)),
        (one_task(r#""no":1,"title":" ","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Title)),
        (one_task(r#""no":1,"title":"a\tb","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Title)),
        (one_task(&long_title), |e| matches!(e, Unreadable::Title)),
        (one_task(r#""no":1,"title":7,"status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Syntax(_))),
        (one_task("\"no\":1,\"title\":\"a\u{1}b\",\"status\":\"Open\",\"priority\":\"Normal\",\"tags\":[]"), |e| matches!(e, Unreadable::Syntax(_))),
        (one_task(r#""no":1,"title":"\ud800","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Escape(_))),
        (one_task(r#""no":1,"title":"\udc00\ud800","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Escape(_))),
        (one_task(r#""no":1,"title":"\ud800A","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Escape(_))),
        (one_task(r#""no":1,"title":"\x41","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Escape(_))),
        (one_task(r#""no":1,"title":"\u00g1","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Escape(_))),
        (one_task(r#""no":1,"title":"a","status":"open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Status)),
        (one_task(r#""no":1,"title":"a","status":"Open","priority":"high","tags":[]"#), |e| matches!(e, Unreadable::Priority)),
        (one_task(r#""no":1,"title":"a","status":"Open","priority":"Normal","tags":["Home"]"#), |e| matches!(e, Unreadable::Tag)),
        (one_task(r#""no":1,"title":"a","status":"Open","priority":"Normal","tags":["-x"]"#), |e| matches!(e, Unreadable::Tag)),
        (one_task("\"no\":1,\"title\":\"a\u{2028}b\",\"status\":\"Open\",\"priority\":\"Normal\",\"tags\":[]"), |e| matches!(e, Unreadable::Title)),
        (one_task(r#""no":1,"title":"a\u0085b","status":"Open","priority":"Normal","tags":[]"#), |e| matches!(e, Unreadable::Title)),
        (one_task(r#""no":1,"title":"a","status":"Open","priority":"Normal","tags":["a","a"]"#), |e| matches!(e, Unreadable::TagTwice)),
        (one_task(r#""no":1,"title":"a","status":"Open","priority":"Normal","tags":["b","a","b"]"#), |e| matches!(e, Unreadable::TagTwice)),
        (one_task(r#""no":1,"title":"a","status":"Open","priority":"Normal","tags":[1]"#), |e| matches!(e, Unreadable::Syntax(_))),
        (
            r#"{"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]},{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}]}"#.into(),
            |e| matches!(e, Unreadable::NumberTwice),
        ),
        (
            r#"{"tasks":[{"no":2,"title":"a","status":"Open","priority":"Normal","tags":[]},{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]},{"no":2,"title":"a","status":"Open","priority":"Normal","tags":[]}],"version":1,"next_no":3}"#.into(),
            |e| matches!(e, Unreadable::NumberTwice),
        ),
        (
            r#"{"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}],"version":1,"next_no":1}"#.into(),
            |e| matches!(e, Unreadable::NumberNotBelowNext),
        ),
    ];
    for (input, is_expected) in cases {
        let from_decode = decode(input.as_bytes()).expect_err(&input);
        assert!(is_expected(&from_decode), "{input}: {from_decode:?}");
        let from_scan = scan(input.as_bytes(), |_| Ok(())).expect_err(&input);
        assert!(is_expected(&from_scan), "{input}: {from_scan:?}");
    }
}

#[test]
fn bytes_that_are_not_utf8_are_unreadable() {
    let mut bytes = one_task(r#""no":1,"title":"a","status":"Open","priority":"Normal","tags":[]"#)
        .into_bytes();
    let at = bytes.iter().position(|b| *b == b'a').expect("the title");
    bytes[at] = 0xFF;
    assert!(matches!(decode(&bytes), Err(Unreadable::NotUtf8)));
}
