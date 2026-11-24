//! The front types the whole todo program and derives the facts the
//! hand projections' Optimizations tables name.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use spec_lang_emit_rust::front::{self, cli};

#[test]
fn todo_types_and_derives_facts() {
    let (prog, app) = common::todo("TodoRust");
    let f = match front::build(&prog, app) {
        Ok(f) => f,
        Err(ds) => panic!(
            "{}",
            ds.iter()
                .map(|d| format!("{d} at {:?}: {:?} {:?}", d.span.map(|s| prog.location(s)), d.decl, d.expected))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    };
    assert_eq!(f.errors, ["NoTask", "AlreadyDone", "NotDone"]);
    let name = |i: usize| prog.rule(f.rules[i].id).info.name.clone();
    let read_only: Vec<String> = (0..f.rules.len()).filter(|i| f.facts.rules[*i].read_only()).map(name).collect();
    assert_eq!(read_only, ["list", "stats"]);
    let entry: Vec<String> = (0..f.rules.len()).filter(|i| f.facts.rules[*i].entry.is_some()).map(name).collect();
    assert_eq!(entry, ["done", "reopen", "rename", "remove"]);
    let retain: Vec<String> = (0..f.rules.len()).filter(|i| f.facts.rules[*i].retain.is_some()).map(name).collect();
    assert_eq!(retain, ["clear"]);
    assert_eq!(f.facts.keyed.len(), 1);
    let invariants: Vec<&str> = f.facts.invariants.iter().map(|l| prog.law(*l).info.name.as_str()).collect();
    assert_eq!(invariants, ["numbers_below_next", "stored_under_its_number"]);
    // `stats` reads `status`; the store's invariants add the key field `no`.
    let stats = f.rules.iter().position(|r| prog.rule(r.id).info.name == "stats").unwrap();
    assert_eq!(f.facts.rules[stats].projection.as_ref().map(|p| p.fields.clone()), Some(vec![0, 2]));
    assert_eq!(
        cli::help(&f, "todo"),
        "usage: todo <command> [args]\n\ncommands:\n  add <title> [--priority low|normal|high] [--tags <tag>]...\n  list [--all] [--tag <tag>]\n  done <no>\n  reopen <no>\n  rename <no> <title>\n  remove <no>\n  clear\n  stats\n"
    );
}
