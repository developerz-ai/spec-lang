// Projected by hand from fn mark, fn badge, fn tag_text, fn urgency, fn rank, fn line (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>
#include <set>
#include <string>
#include <string_view>
#include <utility>

#include "todo/model/types.hpp"

namespace todo::render {

[[nodiscard]] constexpr std::string_view mark(model::Status s) noexcept {
  return s == model::Status::Open ? " " : "x";
}

[[nodiscard]] constexpr std::string_view badge(model::Priority p) noexcept {
  switch (p) {
    case model::Priority::Low:
      return " !low";
    case model::Priority::Normal:
      return "";
    case model::Priority::High:
      return " !high";
  }
  return "";
}

[[nodiscard]] constexpr int urgency(model::Priority p) noexcept {
  switch (p) {
    case model::Priority::High:
      return 0;
    case model::Priority::Normal:
      return 1;
    case model::Priority::Low:
      return 2;
  }
  return 1;
}

// Most urgent first, then oldest first.
[[nodiscard]] constexpr std::pair<int, std::uint64_t> rank(const model::Task& t) noexcept {
  return {urgency(t.priority), t.no.value()};
}

[[nodiscard]] std::string tag_text(const std::set<model::Tag>& tags);

[[nodiscard]] std::string line(const model::Task& t);

}  // namespace todo::render
