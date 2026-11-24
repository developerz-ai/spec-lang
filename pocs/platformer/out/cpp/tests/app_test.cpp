// Projected by hand from the commands run and draw of the game vocabulary (docs/contract.md section 2) for app SkyhopCpp (src/apps/cpp.slg), and the frame hash of shot. Stand-in for spec-lang build output; change the spec, not this file.
#include <array>

#include "game/app/fnv.hpp"
#include "game/app/program.hpp"
#include "game/cli/parser.hpp"
#include "harness.hpp"

using game::app::Outcome;
using hop::levels::first_level;
using hop::levels::second_level;

namespace {

constexpr std::string_view meadow_win = "75 R\n20 RJ\n41 R\n20 RJ\n35 R\n20 RJ\n23 R\n20 RJ\n62 R\n";

std::string never_read() { harness::fail("standard input was read"); }

std::string canned_input;
std::string canned() { return canned_input; }

}  // namespace

// FNV-1a 64: the published test vectors.
static_assert(game::app::fnv1a64({}) == 0xcbf29ce484222325ULL);
static_assert(game::app::fnv1a64(std::array<std::uint8_t, 1>{'a'}) == 0xaf63dc4c8601ec8cULL);
static_assert(game::app::fnv1a64(std::array<std::uint8_t, 6>{'f', 'o', 'o', 'b', 'a', 'r'}) ==
              0x85944171f73967e8ULL);

TEST(a_hash_prints_as_16_lower_case_hex_digits) {
  CHECK_EQ(game::app::hex16(0xcbf29ce484222325ULL), "cbf29ce484222325");
  CHECK_EQ(game::app::hex16(0x1AULL), "000000000000001a");
}

TEST(run_nothing_held) {
  CHECK(game::app::run(first_level, "30 -\n") ==
        (Outcome{0, "0 level meadow\n1 land\nend 30 playing gems 0/3 lives 3 at 0,0,0\n", ""}));
}

TEST(run_an_empty_script_still_starts_the_level) {
  CHECK(game::app::run(first_level, "") ==
        (Outcome{0, "0 level meadow\nend 0 playing gems 0/3 lives 3 at 0,0,0\n", ""}));
  CHECK(game::app::run(second_level, "") ==
        (Outcome{0, "0 level ridge\nend 0 playing gems 0/3 lives 3 at 0,0,0\n", ""}));
}

TEST(run_the_meadow_won_with_every_gem) {
  const Outcome o = game::app::run(first_level, meadow_win);
  CHECK_EQ(o.code, 0);
  CHECK_EQ(o.err, "");
  CHECK_EQ(o.out,
           "0 level meadow\n1 land\n76 jump\n114 land\n137 jump\n140 gem 1\n167 land\n192 jump\n231 land\n"
           "235 jump\n238 gem 2\n279 land\n284 gem 3\n316 won\nend 316 won gems 3/3 lives 3 at 4992,0,0\n");
}

TEST(run_stops_reading_when_the_game_is_over) {
  const Outcome o = game::app::run(first_level, std::string(meadow_win) + "500 L\n");
  CHECK(o.out.ends_with("316 won\nend 316 won gems 3/3 lives 3 at 4992,0,0\n"));
}

TEST(run_refuses_a_bad_script_with_nothing_on_standard_output) {
  CHECK(game::app::run(first_level, "300 R\n1 Q\n") ==
        (Outcome{2, "", "error: input line 2: unknown key \"Q\"\n"}));
}

TEST(draw_the_meadow_at_tick_0) {
  const Outcome o = game::app::draw(first_level, std::nullopt, "");
  CHECK_EQ(o.code, 0);
  CHECK(o.out.starts_with("sky 135,206,235\ncamera 0,704,2048 0,192,0\ntime 0\n"));
  CHECK(
      o.out.ends_with("mesh crawler 3072,256,0 0\nmesh flag 5120,0,0 0\nmesh pip 0,0,0 0\n"
                      "label 16 12 20 255,255,255 gems 0/3\nlabel 16 38 20 255,255,255 lives 3\n"));
}

TEST(draw_stops_after_the_tick_asked_for) {
  const Outcome at_0 = game::app::draw(first_level, 0, meadow_win);
  CHECK(at_0 == game::app::draw(first_level, std::nullopt, ""));
  const Outcome at_150 = game::app::draw(first_level, 150, meadow_win);
  CHECK(at_150.out.find("\ntime 150\n") != std::string::npos);
  // A script that ends first, or a game that ends first, stops there.
  CHECK(game::app::draw(first_level, 9999, "30 -\n").out.find("\ntime 30\n") != std::string::npos);
  const Outcome won = game::app::draw(first_level, game::cli::tick_ceiling, meadow_win);
  CHECK(won.out.find("\ntime 316\n") != std::string::npos);
  CHECK(won.out.ends_with("label 380 250 40 255,255,255 level clear\n"));
  CHECK(won == game::app::draw(first_level, std::nullopt, meadow_win));
}

TEST(draw_refuses_a_bad_script) {
  CHECK(game::app::draw(first_level, std::nullopt, "2 X\n") ==
        (Outcome{2, "", "error: input line 1: unknown key \"X\"\n"}));
}

// "Before any input is read": a usage error and help never touch standard input.
TEST(usage_errors_come_before_any_input_is_read) {
  using game::cli::parse;
  const std::array<std::string_view, 4> bad{"run", "--level", "7", "--fast"};
  CHECK_EQ(game::app::main_program(parse(bad), &never_read, "assets"), 2);
  const std::array<std::string_view, 1> help{"--help"};
  CHECK_EQ(game::app::main_program(parse(help), &never_read, "assets"), 0);
  CHECK_EQ(game::app::main_program(parse({}), &never_read, "assets"), 2);
  const std::array<std::string_view, 1> draw{"draw"};
  canned_input = "1 Z\n";
  CHECK_EQ(game::app::main_program(parse(draw), &canned, "assets"), 2);
}
