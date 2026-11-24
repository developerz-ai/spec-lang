// Projected by hand from "The input script" (docs/contract.md section 2) for rule step(pad: Pad) (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "game/script/reader.hpp"

#include <algorithm>
#include <cstddef>
#include <optional>
#include <variant>

#include "game/cli/digits.hpp"
#include "game/script/utf8.hpp"

namespace game::script {
namespace {

constexpr std::string_view blanks = " \t";
// What a line that is not UTF-8 reports as its unknown key: U+FFFD, the replacement character.
constexpr std::string_view replacement = "\xEF\xBF\xBD";

struct Skipped {};
struct Bad {
  std::string why;
};
using Line = std::variant<Skipped, Hold, Bad>;

Bad unknown_key(std::string_view key) { return {std::string("unknown key \"").append(key).append("\"")}; }

// The first field of text and the rest after it, both without leading blanks.
std::string_view take_field(std::string_view& text) {
  const std::size_t start = std::min(text.find_first_not_of(blanks), text.size());
  const std::size_t end = std::min(text.find_first_of(blanks, start), text.size());
  const std::string_view field = text.substr(start, end - start);
  text.remove_prefix(end);
  return field;
}

// "-" (nothing held), or one or more of the key letters in any order, each at most once.
std::variant<hop::model::Pad, Bad> read_keys(std::string_view field) {
  hop::model::Pad pad;
  if (field == "-") {
    return pad;
  }
  // Left to right, the first character that is a problem: not a key letter, or seen before.
  for (std::string_view rest = field; !rest.empty();) {
    const std::size_t n = utf8_length(rest);
    const auto key = n == 1 ? std::ranges::find(keys, rest[0], &Key::letter) : keys.end();
    if (key == keys.end()) {
      return unknown_key(rest.substr(0, n));
    }
    bool& held = pad.*(key->field);
    if (held) {
      return Bad{std::string("key \"") + key->letter + "\" twice"};
    }
    held = true;
    rest.remove_prefix(n);
  }
  return pad;
}

Line read_line(std::string_view line) {
  if (!is_utf8(line)) {
    return unknown_key(replacement);
  }
  std::string_view text = line.substr(0, line.find('#'));
  const std::string_view ticks_field = take_field(text);
  if (ticks_field.empty()) {
    return Skipped{};
  }
  const std::string_view keys_field = take_field(text);
  if (keys_field.empty() || !take_field(text).empty()) {
    return Bad{"expected <ticks> <keys>"};
  }
  const auto ticks = cli::digits_up_to(ticks_field, max_line_ticks + 1);
  if (!ticks || *ticks < 1 || *ticks > max_line_ticks) {
    return Bad{"ticks must be 1 to 100000"};
  }
  auto pad = read_keys(keys_field);
  if (Bad* bad = std::get_if<Bad>(&pad)) {
    return std::move(*bad);
  }
  return Hold{*ticks, std::get<hop::model::Pad>(pad)};
}

}  // namespace

std::expected<Script, std::string> read(std::string_view input) {
  Script script;
  std::uint64_t total = 0;
  std::size_t number = 0;
  while (!input.empty()) {
    ++number;
    const std::size_t end = input.find('\n');
    Line line = read_line(input.substr(0, end));
    input.remove_prefix(end == std::string_view::npos ? input.size() : end + 1);
    if (const Bad* bad = std::get_if<Bad>(&line)) {
      return std::unexpected("error: input line " + std::to_string(number) + ": " + bad->why);
    }
    if (const Hold* hold = std::get_if<Hold>(&line)) {
      total += hold->ticks;
      script.holds.push_back(*hold);
    }
  }
  // Every line is checked, in order, before the total.
  if (total > max_total_ticks) {
    return std::unexpected(std::string("error: input: more than 1000000 ticks"));
  }
  script.total_ticks = static_cast<std::uint32_t>(total);
  return script;
}

}  // namespace game::script
