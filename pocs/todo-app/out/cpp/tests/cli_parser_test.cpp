// Projected by hand from the cli vocabulary (docs/cli-vocabulary.md) over the rules of todo.board (src/todo/board.slg): docs/contract.md section 2. Stand-in for spec-lang build output; change the spec, not this file.
#include <string>
#include <string_view>
#include <vector>

#include "harness.hpp"
#include "support.hpp"
#include "todo/cli/help.hpp"
#include "todo/cli/parser.hpp"

using namespace todo::cli;

namespace {

Parsed parse_all(std::vector<std::string_view> args) { return parse(args); }

std::string usage_of(std::vector<std::string_view> args) {
  const Parsed p = parse_all(std::move(args));
  const auto* u = std::get_if<UsageError>(&p);
  return u != nullptr ? u->message : "<no usage error>";
}

Command command_of(std::vector<std::string_view> args) {
  const Parsed p = parse_all(std::move(args));
  const auto* c = std::get_if<Command>(&p);
  harness::check(c != nullptr, "parsed as a command");
  return *c;
}

}  // namespace

TEST(help_text_is_the_contract_text) {
  CHECK_EQ(help_text,
           "usage: todo <command> [args]\n"
           "\n"
           "commands:\n"
           "  add <title> [--priority low|normal|high] [--tags <tag>]...\n"
           "  list [--all] [--tag <tag>]\n"
           "  done <no>\n"
           "  reopen <no>\n"
           "  rename <no> <title>\n"
           "  remove <no>\n"
           "  clear\n"
           "  stats\n");
}

TEST(help_requests) {
  CHECK(std::get<HelpRequest>(parse_all({})).to_stdout == false);
  CHECK(std::get<HelpRequest>(parse_all({"--help"})).to_stdout);
  CHECK(std::get<HelpRequest>(parse_all({"add", "-h"})).to_stdout);
  CHECK(std::get<HelpRequest>(parse_all({"nope", "--priority", "-h"})).to_stdout);
  CHECK(std::holds_alternative<Command>(parse_all({"add", "--", "--help"})));
}

TEST(usage_errors_table) {
  const std::vector<std::pair<std::vector<std::string_view>, std::string>> table = {
      {{"frobnicate"}, "unknown command \"frobnicate\""},
      {{"Add", "x"}, "unknown command \"Add\""},
      {{"add"}, "missing <title>"},
      {{"done"}, "missing <no>"},
      {{"rename", "1"}, "missing <title>"},
      {{"add", "buy", "milk"}, "unexpected argument \"milk\""},
      {{"clear", "now"}, "unexpected argument \"now\""},
      {{"done", "0"}, "<no>: not a TaskNo"},
      {{"done", "-1"}, "<no>: not a TaskNo"},
      {{"done", "+1"}, "<no>: not a TaskNo"},
      {{"done", "1.5"}, "<no>: not a TaskNo"},
      {{"done", ""}, "<no>: not a TaskNo"},
      {{"done", " 1"}, "<no>: not a TaskNo"},
      {{"done", "000"}, "<no>: not a TaskNo"},
      {{"done", "١"}, "<no>: not a TaskNo"},
      {{"add", "x", "--force"}, "unknown flag --force"},
      {{"add", "x", "--priority=high"}, "unknown flag --priority=high"},
      {{"done", "1", "--all"}, "unknown flag --all"},
      {{"add", "x", "--priority"}, "--priority needs a value"},
      {{"add", "x", "--priority", "High"}, "--priority: not a Priority"},
      {{"add", "x", "--priority", "--tags"}, "--priority: not a Priority"},
      {{"add", "x", "--priority", "low", "--priority", "high"}, "--priority given twice"},
      {{"add", "x", "--tags", "Home"}, "--tags: not a Tag"},
      {{"add", "x", "--tags", ""}, "--tags: not a Tag"},
      {{"add", "x", "--tags", "-x"}, "--tags: not a Tag"},
      {{"add", "x", "--tags", "--all"}, "--tags: not a Tag"},
      {{"list", "--tag", "--all"}, "--tag: not a Tag"},
      {{"list", "--all", "--all"}, "--all given twice"},
      {{"list", "--tag", "a", "--tag", "b"}, "--tag given twice"},
      {{"list", "--tag", "a", "--tag"}, "--tag needs a value"},
      {{"list", "--tag", "a", "--tag", "B"}, "--tag given twice"},
      {{"done", "abc", "--force"}, "<no>: not a TaskNo"},
      {{"done", "--force", "abc"}, "unknown flag --force"},
      {{"add", "x", "y", "--priority", "nope"}, "unexpected argument \"y\""},
      {{"add", "--priority", "nope"}, "--priority: not a Priority"},
      {{"rename", "x"}, "<no>: not a TaskNo"},
      {{"add", ""}, "<title>: not a Title"},
      {{"add", "a\tb"}, "<title>: not a Title"},
      {{"add", "a\u2028b"}, "<title>: not a Title"},
      {{"add", "a\u2029b"}, "<title>: not a Title"},
      {{"add", "a\u0085b"}, "<title>: not a Title"},
      {{"add", "\xff"}, "<title>: not a Title"},
  };
  for (const auto& [args, want] : table) {
    CHECK_EQ(usage_of(args), want);
  }
}

TEST(flags_before_positionals_and_double_dash) {
  const auto add = std::get<Add>(command_of({"add", "--priority", "high", "ship it", "--tags", "work"}));
  CHECK_EQ(add.title.text(), "ship it");
  CHECK(add.priority == todo::model::Priority::High);
  CHECK(add.tags == support::tags({"work"}));
  CHECK_EQ(std::get<Add>(command_of({"add", "--", "--all"})).title.text(), "--all");
  CHECK_EQ(std::get<Add>(command_of({"add", "-x"})).title.text(), "-x");
  CHECK_EQ(std::get<Add>(command_of({"add", "-"})).title.text(), "-");
  CHECK_EQ(usage_of({"add", "--", "a", "--"}), "unexpected argument \"--\"");
  CHECK_EQ(std::get<Add>(command_of({"add", "--", "--"})).title.text(), "--");
  CHECK_EQ(usage_of({"add", "--", "a", "b"}), "unexpected argument \"b\"");
}

TEST(tags_collapse_and_list_options) {
  const auto add = std::get<Add>(command_of({"add", "x", "--tags", "b", "--tags", "a", "--tags", "b"}));
  CHECK(add.tags == support::tags({"a", "b"}));
  const auto list = std::get<List>(command_of({"list", "--tag", "home", "--all"}));
  CHECK(list.all);
  CHECK(list.tag == support::tag("home"));
  CHECK(!std::get<List>(command_of({"list"})).all);
}

TEST(task_numbers) {
  CHECK_EQ(parse_task_no("007")->value(), 7U);
  CHECK_EQ(parse_task_no("9007199254740991")->value(), 9007199254740991U);
  CHECK_EQ(parse_task_no("18446744073709551615")->value(), 18446744073709551615U);
  CHECK_EQ(parse_task_no("18446744073709551617")->value(), 18446744073709551615U);
  CHECK_EQ(parse_task_no("99999999999999999999999999999999")->value(), 18446744073709551615U);
  CHECK(!parse_task_no("0").has_value());
}

TEST(rename_and_others_build) {
  const auto r = std::get<Rename>(command_of({"rename", "3", "urgent, really"}));
  CHECK_EQ(r.no.value(), 3U);
  CHECK_EQ(r.title.text(), "urgent, really");
  CHECK(std::holds_alternative<Clear>(command_of({"clear"})));
  CHECK(std::holds_alternative<Stats>(command_of({"stats"})));
  CHECK_EQ(std::get<Remove>(command_of({"remove", "2"})).no.value(), 2U);
  CHECK_EQ(std::get<Reopen>(command_of({"reopen", "2"})).no.value(), 2U);
}
