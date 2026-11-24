// The canonical writer's pieces (docs/contract.md section 3, "Bytes written"): one line, no white space,
// only " and \ escaped, everything else raw UTF-8.
#pragma once

#include <charconv>
#include <cstdint>
#include <string>
#include <string_view>

namespace rt::json {

// The largest Int the store carries: 2^53.
inline constexpr std::int64_t int_limit = std::int64_t{1} << 53U;

// Digits straight into the buffer. False when the store cannot carry the value.
[[nodiscard]] inline bool put_int(std::string& out, std::int64_t n) {
  if (n < 0 || n > int_limit) {
    return false;
  }
  const std::size_t at = out.size();
  out.resize(at + 20);  // 20 digits hold every int64_t, so to_chars cannot fail
  const auto r = std::to_chars(out.data() + at, out.data() + out.size(), n);
  out.resize(static_cast<std::size_t>(r.ptr - out.data()));
  return true;
}

inline void put_string(std::string& out, std::string_view s) {
  out.push_back('"');
  for (const char c : s) {
    if (c == '"' || c == '\\') {
      out.push_back('\\');
    }
    out.push_back(c);
  }
  out.push_back('"');
}

}  // namespace rt::json
