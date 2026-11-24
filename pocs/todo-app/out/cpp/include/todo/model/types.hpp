// Projected by hand from type Title, type Tag, type TaskNo, type Status, type Priority, type Task (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <compare>
#include <cstdint>
#include <optional>
#include <set>
#include <string>
#include <string_view>
#include <utility>

#include "todo/model/text.hpp"

namespace todo::model {

// type Title = Text where is_title(self). Only make() builds one, so every Title meets the claim.
class Title {
 public:
  [[nodiscard]] static std::optional<Title> make(std::string text) {
    if (!is_title(text)) {
      return std::nullopt;
    }
    return Title(std::move(text));
  }
  [[nodiscard]] const std::string& text() const noexcept { return text_; }
  auto operator<=>(const Title&) const = default;

 private:
  explicit Title(std::string text) : text_(std::move(text)) {}
  std::string text_;
};

// type Tag = Text where is_tag(self). Ordered by bytes, which for ASCII is code-point order.
class Tag {
 public:
  [[nodiscard]] static std::optional<Tag> make(std::string_view text) {
    if (!is_tag(text)) {
      return std::nullopt;
    }
    return Tag(std::string(text));
  }
  [[nodiscard]] const std::string& text() const noexcept { return text_; }
  auto operator<=>(const Tag&) const = default;

 private:
  explicit Tag(std::string text) : text_(std::move(text)) {}
  std::string text_;
};

// type TaskNo = Int where self >= 1. Int is unbounded; 64 bits hold every number a store can
// hold (at most 2^53), and the command line saturates larger ones, which no store contains.
class TaskNo {
 public:
  [[nodiscard]] static constexpr std::optional<TaskNo> make(std::uint64_t value) noexcept {
    if (value < 1) {
      return std::nullopt;
    }
    return TaskNo(value);
  }
  [[nodiscard]] static constexpr TaskNo first() noexcept { return TaskNo(1); }
  [[nodiscard]] constexpr std::uint64_t value() const noexcept { return value_; }
  [[nodiscard]] constexpr TaskNo next() const noexcept { return TaskNo(value_ + 1); }
  auto operator<=>(const TaskNo&) const = default;

 private:
  explicit constexpr TaskNo(std::uint64_t value) noexcept : value_(value) {}
  std::uint64_t value_;
};

enum class Status : std::uint8_t { Open, Done };
enum class Priority : std::uint8_t { Low, Normal, High };

struct Task {
  TaskNo no;
  Title title;
  Status status = Status::Open;
  Priority priority = Priority::Normal;
  std::set<Tag> tags = {};

  bool operator==(const Task&) const = default;
};

}  // namespace todo::model
