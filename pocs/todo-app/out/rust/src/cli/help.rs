// Projected by hand from vocabulary cli: "every cli rule: a line of --help" (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

/// The help text, folded at build time from the command table: no run of the
/// program formats it. `tests/cli_help.rs` derives it again from [`super::COMMANDS`]
/// and fails if the two differ.
pub const HELP: &str = "\
usage: todo <command> [args]

commands:
  add <title> [--priority low|normal|high] [--tags <tag>]...
  list [--all] [--tag <tag>]
  done <no>
  reopen <no>
  rename <no> <title>
  remove <no>
  clear
  stats
";
