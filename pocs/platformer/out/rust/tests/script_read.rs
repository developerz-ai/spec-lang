// Projected by hand from type Pad (src/hop/model.slg) and the input script of contract section 2. Stand-in for spec-lang build output; change the spec, not this file.

use skyhop::model::Pad;
use skyhop::script::{Hold, Script, read};

fn err(input: &[u8]) -> String {
    read(input).unwrap_err().to_string()
}

fn pad(keys: &str) -> Pad {
    Pad {
        left: keys.contains('L'),
        right: keys.contains('R'),
        forward: keys.contains('F'),
        back: keys.contains('B'),
        jump: keys.contains('J'),
    }
}

#[test]
fn comments_blank_lines_tabs_and_leading_zeros() {
    let s = read(b"# wait, then walk\n\n  002\t-   # two ticks of nothing\n0003 R\n").unwrap();
    assert_eq!(
        s,
        Script {
            holds: vec![
                Hold {
                    ticks: 2,
                    pad: Pad::default()
                },
                Hold {
                    ticks: 3,
                    pad: pad("R")
                },
            ]
        }
    );
    assert_eq!(s.pads().count(), 5);
}

#[test]
fn a_last_line_without_a_newline_counts() {
    assert_eq!(read(b"4 -\n1 R").unwrap().pads().count(), 5);
    assert_eq!(read(b"").unwrap(), Script::default());
}

#[test]
fn keys_in_any_order() {
    assert_eq!(read(b"3 JR").unwrap().holds[0].pad, pad("RJ"));
    assert_eq!(read(b"1 BFJRL").unwrap().holds[0].pad, pad("LRFBJ"));
}

#[test]
fn every_refusal_and_its_line() {
    let cases: &[(&[u8], &str)] = &[
        (b"5", "input line 1: expected <ticks> <keys>"),
        (b"5 R J", "input line 1: expected <ticks> <keys>"),
        (b"0 R", "input line 1: ticks must be 1 to 100000"),
        (b"100001 R", "input line 1: ticks must be 1 to 100000"),
        (b"-3 R", "input line 1: ticks must be 1 to 100000"),
        (b"R 3", "input line 1: ticks must be 1 to 100000"),
        (
            b"99999999999999999999999 R",
            "input line 1: ticks must be 1 to 100000",
        ),
        (b"1 r", "input line 1: unknown key \"r\""),
        (b"1 R-", "input line 1: unknown key \"-\""),
        (b"1 --", "input line 1: unknown key \"-\""),
        (b"1 RJR", "input line 1: key \"R\" twice"),
        (b"1 RRZ", "input line 1: key \"R\" twice"),
        (b"1 RZR", "input line 1: unknown key \"Z\""),
        (
            b"# header\n\n3 R\n2 X\n0 R\n",
            "input line 4: unknown key \"X\"",
        ),
        (b"300 R\n1 Q\n", "input line 2: unknown key \"Q\""),
        (
            "1 R\n1 \u{ff}\n".as_bytes(),
            "input line 2: unknown key \"\u{ff}\"",
        ),
        (b"1 R\xff\n", "input line 1: unknown key \"\u{fffd}\""),
    ];
    for (input, want) in cases {
        assert_eq!(err(input), *want, "{:?}", String::from_utf8_lossy(input));
    }
}

#[test]
fn at_most_a_million_ticks() {
    let ten = "100000 R\n".repeat(10);
    assert_eq!(read(ten.as_bytes()).unwrap().pads().count(), 1_000_000);
    assert_eq!(
        err(format!("{ten}1 R\n").as_bytes()),
        "input: more than 1000000 ticks"
    );
}
