// Projected by hand from "The input script" (docs/contract.md section 2) for rule step(pad: Pad) (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "game/script/reader.hpp"
#include "game/script/utf8.hpp"
#include "harness.hpp"

using namespace game::script;
using hop::model::Pad;

namespace {

std::string error_of(std::string_view input) {
  const auto script = read(input);
  return script ? "<accepted>" : script.error();
}

Script script_of(std::string_view input) {
  auto script = read(input);
  if (!script) {
    harness::fail("refused: " + script.error());
  }
  return std::move(*script);
}

}  // namespace

static_assert(is_utf8("1 \xC3\xBF") && !is_utf8("1 \xFF") && !is_utf8("\xC0\x80") &&
              !is_utf8("\xED\xA0\x80"));
static_assert(!is_utf8("\xE2\x82") && is_utf8("\xF0\x9F\x98\x80") && !is_utf8("\xF4\x90\x80\x80"));

TEST(an_empty_script) {
  CHECK(script_of("").holds.empty());
  CHECK_EQ(script_of("\n\n# nothing\n   \t\n").total_ticks, 0U);
}

TEST(comments_blank_lines_tabs_and_leading_zeros) {
  const Script s = script_of("# wait, then walk\n\n  002\t-   # two ticks of nothing\n0003 R\n");
  CHECK_EQ(s.holds.size(), std::size_t{2});
  CHECK(s.holds[0] == (Hold{2, Pad{}}));
  CHECK(s.holds[1] == (Hold{3, Pad{.right = true}}));
  CHECK_EQ(s.total_ticks, 5U);
}

TEST(a_last_line_without_a_newline_counts) {
  const Script s = script_of("4 -\n1 R");
  CHECK_EQ(s.holds.size(), std::size_t{2});
  CHECK_EQ(s.total_ticks, 5U);
}

TEST(keys_in_any_order) {
  CHECK(script_of("3 JR\n").holds[0].pad == (Pad{.right = true, .jump = true}));
  CHECK(script_of("1 JBFRL\n").holds[0].pad ==
        (Pad{.left = true, .right = true, .forward = true, .back = true, .jump = true}));
  CHECK(script_of("1 F\n").holds[0].pad == (Pad{.forward = true}));
  CHECK(script_of("1 B\n").holds[0].pad == (Pad{.back = true}));
  CHECK(script_of("1 L\n").holds[0].pad == (Pad{.left = true}));
}

TEST(a_line_must_be_two_fields) {
  CHECK_EQ(error_of("5\n"), "error: input line 1: expected <ticks> <keys>");
  CHECK_EQ(error_of("5 R J\n"), "error: input line 1: expected <ticks> <keys>");
  CHECK_EQ(error_of("x\n"), "error: input line 1: expected <ticks> <keys>");
  CHECK_EQ(error_of("5 # R\n"), "error: input line 1: expected <ticks> <keys>");
}

TEST(ticks_are_1_to_100000) {
  for (const std::string_view bad :
       {"0 R", "100001 R", "-3 R", "R 3", "1e3 R", "99999999999999999999 R", "+1 R"}) {
    CHECK_EQ(error_of(bad), "error: input line 1: ticks must be 1 to 100000");
  }
  CHECK_EQ(script_of("100000 R\n").total_ticks, 100000U);
  CHECK_EQ(script_of("000000000000000000000001 R\n").total_ticks, 1U);
}

TEST(unknown_keys) {
  CHECK_EQ(error_of("1 r\n"), "error: input line 1: unknown key \"r\"");
  CHECK_EQ(error_of("1 R-\n"), "error: input line 1: unknown key \"-\"");
  CHECK_EQ(error_of("1 --\n"), "error: input line 1: unknown key \"-\"");
  CHECK_EQ(error_of("1 RxJ\n"), "error: input line 1: unknown key \"x\"");
  // A carriage return is not a blank: it is part of the keys.
  CHECK_EQ(error_of("1 R\r\n"), "error: input line 1: unknown key \"\r\"");
}

TEST(a_key_twice) {
  CHECK_EQ(error_of("1 RJR\n"), "error: input line 1: key \"R\" twice");
  CHECK_EQ(error_of("1 JJ\n"), "error: input line 1: key \"J\" twice");
  // Left to right, the first character that is a problem wins.
  CHECK_EQ(error_of("1 RRZ\n"), "error: input line 1: key \"R\" twice");
  CHECK_EQ(error_of("1 RZR\n"), "error: input line 1: unknown key \"Z\"");
}

TEST(the_first_that_applies_per_line) {
  CHECK_EQ(error_of("0 x\n"), "error: input line 1: ticks must be 1 to 100000");
  CHECK_EQ(error_of("0 x y\n"), "error: input line 1: expected <ticks> <keys>");
}

TEST(the_line_number_counts_every_line_and_the_first_bad_line_wins) {
  CHECK_EQ(error_of("# header\n\n3 R\n2 X\n0 R\n"), "error: input line 4: unknown key \"X\"");
  CHECK_EQ(error_of("300 R\n1 Q\n"), "error: input line 2: unknown key \"Q\"");
}

TEST(at_most_a_million_ticks_in_all) {
  std::string ten;
  for (int i = 0; i < 10; ++i) {
    ten += "100000 R\n";
  }
  CHECK_EQ(script_of(ten).total_ticks, max_total_ticks);
  CHECK_EQ(error_of(ten + "1 R\n"), "error: input: more than 1000000 ticks");
  // Every line is checked before the total.
  CHECK_EQ(error_of(ten + "1 R\n1 Q\n"), "error: input line 12: unknown key \"Q\"");
}

TEST(a_key_outside_ascii_is_an_unknown_key) {
  CHECK_EQ(error_of("1 R\n1 \xC3\xBF\n"), "error: input line 2: unknown key \"\xC3\xBF\"");
  CHECK_EQ(error_of("1 R\xE2\x86\x92\n"), "error: input line 1: unknown key \"\xE2\x86\x92\"");
}

TEST(invalid_utf8_is_an_unknown_key_at_its_line) {
  CHECK_EQ(error_of("1 R\n1 \xFF\n"), "error: input line 2: unknown key \"\xEF\xBF\xBD\"");
  CHECK_EQ(error_of("1 R # \xC3\n"), "error: input line 1: unknown key \"\xEF\xBF\xBD\"");
}
