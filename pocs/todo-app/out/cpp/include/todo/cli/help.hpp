// Projected by hand from "every cli rule -> a line of --help" (docs/cli-vocabulary.md) over todo.board (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <array>
#include <cstddef>
#include <string_view>

#include "todo/cli/params.hpp"

namespace todo::cli {
namespace help_detail {

// Counts when out is null, writes otherwise: one function, two compile-time passes.
struct Writer {
  char* out = nullptr;
  std::size_t size = 0;
  constexpr void put(std::string_view s) {
    for (const char c : s) {
      if (out != nullptr) {
        out[size] = c;
      }
      ++size;
    }
  }
};

constexpr void put_lower(Writer& w, std::string_view s) {
  for (const char c : s) {
    const char lower = (c >= 'A' && c <= 'Z') ? static_cast<char>(c - 'A' + 'a') : c;
    w.put(std::string_view(&lower, 1));
  }
}

constexpr void put_value(Writer& w, ValueType t) {
  if (t == ValueType::Priority) {
    for (std::size_t i = 0; i < priority_names.size(); ++i) {
      w.put(i == 0 ? "" : "|");
      w.put(priority_names[i].first);
    }
    return;
  }
  w.put("<");
  put_lower(w, type_name(t));
  w.put(">");
}

constexpr std::size_t write(char* out) {
  Writer w{out};
  w.put("usage: todo <command> [args]\n\ncommands:\n");
  for (const CommandSpec& c : commands) {
    w.put("  ");
    w.put(c.name);
    for (const Positional& p : c.positionals) {
      w.put(" <");
      w.put(p.name);
      w.put(">");
    }
    for (const Flag& f : c.flags) {
      w.put(" [--");
      w.put(f.name);
      if (f.kind != FlagKind::Switch) {
        w.put(" ");
        put_value(w, f.type);
      }
      w.put(f.kind == FlagKind::Repeated ? "]..." : "]");
    }
    w.put("\n");
  }
  return w.size;
}

inline constexpr std::size_t size = write(nullptr);

consteval std::array<char, size> make() {
  std::array<char, size> text{};
  write(text.data());
  return text;
}

inline constexpr std::array<char, size> storage = make();

}  // namespace help_detail

// The whole help text, newline-terminated, built at compile time from the parameter table.
inline constexpr std::string_view help_text{help_detail::storage.data(), help_detail::storage.size()};

}  // namespace todo::cli
