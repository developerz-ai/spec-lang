// Projected by hand from the argument rules of the game vocabulary (docs/contract.md section 2, "Parsing, in this order") for app SkyhopCpp (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <algorithm>
#include <initializer_list>
#include <vector>

#include "game/cli/digits.hpp"
#include "game/cli/help.hpp"
#include "game/cli/parser.hpp"
#include "harness.hpp"

using namespace game::cli;

namespace {

Parsed parse_args(std::initializer_list<std::string_view> args) {
  const std::vector<std::string_view> list(args);
  return parse(list);
}

std::string error_of(std::initializer_list<std::string_view> args) {
  const Parsed parsed = parse_args(args);
  const auto* usage = std::get_if<UsageError>(&parsed);
  return usage != nullptr ? usage->message : "<no usage error>";
}

Invocation invocation_of(std::initializer_list<std::string_view> args) {
  const Parsed parsed = parse_args(args);
  const auto* invocation = std::get_if<Invocation>(&parsed);
  if (invocation == nullptr) {
    harness::fail("not accepted");
  }
  return *invocation;
}

}  // namespace

static_assert(digits_up_to("0042", 100) == 42U);
static_assert(digits_up_to("99999999999999999999999", 100) == 100U);
static_assert(!digits_up_to("", 100) && !digits_up_to("-1", 100) && !digits_up_to("1.5", 100));

TEST(no_arguments_is_help_on_standard_error) { CHECK(std::holds_alternative<NoArguments>(parse_args({}))); }

TEST(help_anywhere_wins) {
  CHECK(std::holds_alternative<HelpAsked>(parse_args({"--help"})));
  CHECK(std::holds_alternative<HelpAsked>(parse_args({"run", "--level", "9", "-h"})));
  CHECK(std::holds_alternative<HelpAsked>(parse_args({"draw", "--at", "-h"})));
  CHECK(std::holds_alternative<HelpAsked>(parse_args({"jump", "--help"})));
}

TEST(the_help_text_is_the_contract_s) {
  CHECK(help.starts_with("usage: skyhop <command> [options]\n\ncommands:\n  play [--level <n>]\n"));
  CHECK(help.ends_with("run, draw and shot read a script of controls on standard input.\n"));
  CHECK_EQ(std::ranges::count(help, '\n'), 10);
}

TEST(unknown_commands) {
  CHECK_EQ(error_of({"jump"}), "error: unknown command \"jump\"");
  CHECK_EQ(error_of({"--level"}), "error: unknown command \"--level\"");
  CHECK_EQ(error_of({""}), "error: unknown command \"\"");
}

TEST(unknown_flags) {
  CHECK_EQ(error_of({"run", "--fast"}), "error: unknown flag --fast");
  CHECK_EQ(error_of({"run", "--level=2"}), "error: unknown flag --level=2");
  CHECK_EQ(error_of({"run", "--at", "3"}), "error: unknown flag --at");
  CHECK_EQ(error_of({"play", "--out", "x.png"}), "error: unknown flag --out");
  CHECK_EQ(error_of({"draw", "--out", "x.png"}), "error: unknown flag --out");
  CHECK_EQ(error_of({"run", "--"}), "error: unknown flag --");
}

TEST(a_flag_needs_a_value_once) {
  CHECK_EQ(error_of({"run", "--level"}), "error: --level needs a value");
  CHECK_EQ(error_of({"draw", "--at", "1", "--at", "2"}), "error: --at given twice");
  CHECK_EQ(error_of({"draw", "--level", "1", "--level"}), "error: --level needs a value");
  CHECK_EQ(error_of({"shot", "--out", "a", "--out", "b"}), "error: --out given twice");
}

TEST(a_level_is_1_or_2_in_digits) {
  for (const std::string_view bad :
       {"0", "3", "-1", "one", "", "99999999999999999999999", "+1", "1 ", "0x1"}) {
    CHECK_EQ(error_of({"run", "--level", bad}), "error: --level: not a LevelNo");
  }
  CHECK_EQ(invocation_of({"run"}).level.value(), 1);
  CHECK_EQ(invocation_of({"run", "--level", "2"}).level.value(), 2);
  CHECK_EQ(invocation_of({"run", "--level", "001"}).level.value(), 1);
  CHECK_EQ(invocation_of({"play", "--level", "0000000000000000000000002"}).level.value(), 2);
}

TEST(a_tick_is_digits_zero_or_more) {
  for (const std::string_view bad : {"-1", "1.5", "", "ten", " 1"}) {
    CHECK_EQ(error_of({"draw", "--at", bad}), "error: --at: not a tick");
  }
  CHECK(!invocation_of({"draw"}).at.has_value());
  CHECK(invocation_of({"draw", "--at", "0"}).at == std::optional<std::uint32_t>(0));
  CHECK(invocation_of({"shot", "--at", "0150"}).at == std::optional<std::uint32_t>(150));
  // Of any length: a tick past the longest script is the end of the script.
  CHECK(invocation_of({"draw", "--at", "123456789012345678901234567890"}).at ==
        std::optional<std::uint32_t>(tick_ceiling));
}

TEST(an_output_path_cannot_be_empty) {
  CHECK_EQ(error_of({"shot", "--out", ""}), "error: --out: not a path");
  CHECK(invocation_of({"shot", "--out", "--at"}).out == std::optional<std::string>("--at"));
}

TEST(positionals_are_refused) {
  CHECK_EQ(error_of({"run", "meadow"}), "error: unexpected argument \"meadow\"");
  CHECK_EQ(error_of({"draw", "-"}), "error: unexpected argument \"-\"");
  CHECK_EQ(error_of({"run", ""}), "error: unexpected argument \"\"");
}

TEST(the_first_problem_from_the_left_wins) {
  CHECK_EQ(error_of({"run", "--level", "7", "--fast"}), "error: --level: not a LevelNo");
  CHECK_EQ(error_of({"run", "--fast", "--level", "7"}), "error: unknown flag --fast");
  CHECK_EQ(error_of({"draw", "--at", "x", "meadow"}), "error: --at: not a tick");
}

TEST(a_whole_command_line) {
  const Invocation shot = invocation_of({"shot", "--out", "a.png", "--at", "150", "--level", "2"});
  CHECK(shot.command == Command::Shot);
  CHECK_EQ(shot.level.value(), 2);
  CHECK(shot.at == std::optional<std::uint32_t>(150));
  CHECK(shot.out == std::optional<std::string>("a.png"));
  CHECK(invocation_of({"play"}).command == Command::Play);
  CHECK(invocation_of({"run"}).command == Command::Run);
  CHECK(invocation_of({"draw"}).command == Command::Draw);
}
