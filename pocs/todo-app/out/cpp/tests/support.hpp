// Projected by hand from type Task, type Title, type Tag (src/todo/model.slg): literals for tests. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <initializer_list>
#include <set>
#include <string>
#include <string_view>

#include "harness.hpp"
#include "todo/board/world.hpp"
#include "todo/model/types.hpp"

namespace support {

inline todo::model::Title title(std::string text) {
  auto t = todo::model::Title::make(std::move(text));
  harness::check(t.has_value(), "test literal is a Title");
  return *t;
}

inline todo::model::Tag tag(std::string_view text) {
  auto t = todo::model::Tag::make(text);
  harness::check(t.has_value(), "test literal is a Tag");
  return *t;
}

inline todo::model::TaskNo no(std::uint64_t n) {
  auto t = todo::model::TaskNo::make(n);
  harness::check(t.has_value(), "test literal is a TaskNo");
  return *t;
}

inline std::set<todo::model::Tag> tags(std::initializer_list<std::string_view> names) {
  std::set<todo::model::Tag> out;
  for (const auto n : names) {
    out.insert(tag(n));
  }
  return out;
}

inline todo::model::Task task(std::uint64_t n, std::string t,
                              todo::model::Status status = todo::model::Status::Open,
                              todo::model::Priority priority = todo::model::Priority::Normal,
                              std::set<todo::model::Tag> tag_set = {}) {
  return {.no = no(n),
          .title = title(std::move(t)),
          .status = status,
          .priority = priority,
          .tags = std::move(tag_set)};
}

}  // namespace support
