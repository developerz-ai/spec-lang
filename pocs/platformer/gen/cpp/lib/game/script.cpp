// The `game` library's runtime for C++ (libs/game/real/cpp): the input script.
#include "lib/game/script.hpp"

#include <algorithm>
#include <cctype>
#include <optional>

#include "lib/cli/args.hpp"

namespace lib::game {

namespace {

constexpr std::string_view kBlanks = " \t";
// What a line that is not UTF-8 reports as its unknown key: U+FFFD, the replacement character.
constexpr std::string_view kReplacement = "\xEF\xBF\xBD";

// The first field of text; text moves past it.
std::string_view take_field(std::string_view& text) {
  const std::size_t start = std::min(text.find_first_not_of(kBlanks), text.size());
  const std::size_t end = std::min(text.find_first_of(kBlanks, start), text.size());
  const std::string_view field = text.substr(start, end - start);
  text.remove_prefix(end);
  return field;
}

// The bytes of the code point well-formed UTF-8 starts with.
std::size_t code_point_length(std::string_view text) {
  const auto lead = static_cast<unsigned char>(text[0]);
  return lead < 0x80 ? 1 : lead < 0xE0 ? 2 : lead < 0xF0 ? 3 : 4;
}

char letter_of(const rt::meta::Field& field) {
  return field.name.empty() ? '\0' : static_cast<char>(std::toupper(static_cast<unsigned char>(field.name[0])));
}

std::string unknown_key(std::string_view key) {
  return "unknown key \"" + std::string{key} + "\"";
}

// "-" (nothing held), or one or more key letters in any order, each at most once. Left to right, the first
// character that is a problem is the one reported.
std::expected<rt::Data, std::string> read_keys(const rt::meta::Type& controls, std::string_view field) {
  std::vector<bool> held(controls.fields.size(), false);
  if (field != "-") {
    for (std::string_view rest = field; !rest.empty();) {
      const std::size_t n = code_point_length(rest);
      const auto key = n == 1 ? std::ranges::find(controls.fields, rest[0], letter_of) : controls.fields.end();
      if (key == controls.fields.end()) return std::unexpected(unknown_key(rest.substr(0, n)));
      const auto at = static_cast<std::size_t>(key - controls.fields.begin());
      if (held[at]) return std::unexpected("key \"" + std::string{rest.substr(0, 1)} + "\" twice");
      held[at] = true;
      rest.remove_prefix(n);
    }
  }
  std::vector<rt::Data> values;
  for (const bool h : held) values.emplace_back(h);
  return rt::Data::fields(0, std::move(values));
}

// Digits only, 1 to 100000, leading zeros allowed.
std::optional<std::uint32_t> read_ticks(std::string_view field) {
  if (!std::ranges::all_of(field, [](char c) { return c >= '0' && c <= '9'; })) return std::nullopt;
  std::uint64_t n = 0;
  for (const char c : field)
    n = std::min<std::uint64_t>(n * 10 + static_cast<std::uint64_t>(c - '0'), kMaxLineTicks + 1);
  return n >= 1 && n <= kMaxLineTicks ? std::optional<std::uint32_t>{static_cast<std::uint32_t>(n)} : std::nullopt;
}

// A hold, nothing for a line to skip, or why the line is bad.
std::expected<std::optional<Hold>, std::string> read_line(const rt::meta::Type& controls, std::string_view line) {
  if (!lib::cli::is_utf8(line)) return std::unexpected(unknown_key(kReplacement));
  std::string_view text = line.substr(0, line.find('#'));
  const std::string_view ticks_field = take_field(text);
  if (ticks_field.empty()) return std::optional<Hold>{};
  const std::string_view keys_field = take_field(text);
  if (keys_field.empty() || !take_field(text).empty()) return std::unexpected(std::string{"expected <ticks> <keys>"});
  const auto ticks = read_ticks(ticks_field);
  if (!ticks) return std::unexpected(std::string{"ticks must be 1 to 100000"});
  auto keys = read_keys(controls, keys_field);
  if (!keys) return std::unexpected(std::move(keys.error()));
  return std::optional<Hold>{Hold{*ticks, std::move(*keys)}};
}

}  // namespace

std::expected<Script, std::string> read_script(const rt::meta::Type& controls, std::string_view input) {
  Script script;
  std::uint64_t total = 0;
  std::size_t number = 0;
  while (!input.empty()) {
    ++number;
    const std::size_t end = input.find('\n');
    auto line = read_line(controls, input.substr(0, end));
    input.remove_prefix(end == std::string_view::npos ? input.size() : end + 1);
    if (!line) return std::unexpected("input line " + std::to_string(number) + ": " + line.error());
    if (line->has_value()) {
      total += (*line)->ticks;
      script.holds.push_back(std::move(**line));
    }
  }
  // Every line is checked, in order, before the total.
  if (total > kMaxTotalTicks) return std::unexpected(std::string{"input: more than 1000000 ticks"});
  return script;
}

}  // namespace lib::game
