// Projected by hand from vocabulary cli: "every cli rule: a line of --help" and the help text (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

use todo::cli::{COMMANDS, FlagKind, HELP, ValueType};
use todo::model::Priority;

/// How a value is shown: a sum type as its variants, anything else as `<type>`.
fn shown(value: ValueType) -> String {
    match value {
        ValueType::Priority => Priority::ALL.map(|p| p.name().to_lowercase()).join("|"),
        other => format!("<{}>", other.name().to_lowercase()),
    }
}

/// The help text derived from the table, the way the build would fold it.
fn derived() -> String {
    let mut text = String::from("usage: todo <command> [args]\n\ncommands:\n");
    for spec in &COMMANDS {
        text.push_str("  ");
        text.push_str(spec.name);
        for positional in spec.positionals {
            text.push_str(&format!(" <{}>", positional.name));
        }
        for flag in spec.flags {
            text.push_str(&match flag.kind {
                FlagKind::Switch => format!(" [--{}]", flag.name),
                FlagKind::Option(value) => format!(" [--{} {}]", flag.name, shown(value)),
                FlagKind::Set(value) => format!(" [--{} {}]...", flag.name, shown(value)),
            });
        }
        text.push('\n');
    }
    text
}

#[test]
fn the_constant_is_what_the_table_derives() {
    assert_eq!(HELP, derived());
}

#[test]
fn the_help_text_of_the_contract() {
    let contract = "usage: todo <command> [args]\n\ncommands:\n  add <title> [--priority low|normal|high] [--tags <tag>]...\n  list [--all] [--tag <tag>]\n  done <no>\n  reopen <no>\n  rename <no> <title>\n  remove <no>\n  clear\n  stats\n";
    assert_eq!(HELP, contract);
}
