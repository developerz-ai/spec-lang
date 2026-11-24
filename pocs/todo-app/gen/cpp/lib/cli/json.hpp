// The `cli` library's runtime for C++ (libs/cli/real/cpp): a strict JSON reader (RFC 8259) for the store.
// It refuses what a lenient parser lets through: a key given twice, a lone surrogate, a raw control character.
// An integer literal is kept exact; a number with a fraction or an exponent is not an integer.
#pragma once

#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "rt/int.hpp"

namespace lib::cli {

struct Json {
  enum class Kind { Null, Bool, Number, String, Array, Object } kind = Kind::Null;
  bool boolean = false;
  // Set when the number is an integer literal that the runtime's Int holds.
  std::optional<rt::Int> integer = {};
  std::string string = {};
  std::vector<Json> items = {};
  // In written order; no key twice.
  std::vector<std::pair<std::string, Json>> entries = {};

  const Json* get(std::string_view key) const;
};

// The document, or nothing when the text is not one JSON value. The text must be valid UTF-8.
std::optional<Json> read_json(std::string_view text);

}  // namespace lib::cli
