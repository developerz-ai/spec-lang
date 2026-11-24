// Projected by hand from rule add, list, done, reopen, rename, remove, clear, stats and fn has_tag (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstddef>
#include <expected>
#include <optional>
#include <set>
#include <string>
#include <vector>

#include "todo/board/errors.hpp"
#include "todo/board/world.hpp"
#include "todo/model/types.hpp"

namespace todo::board {

// A rule that writes: the next world, the out.line lines it sends after commit, and its reply.
template <class Reply>
struct Outcome {
  World world;
  std::vector<std::string> lines;
  Reply reply;
};

struct Nothing {
  bool operator==(const Nothing&) const = default;
};

// A rule with guards. On a refusal there is no next world: the world is unchanged by construction.
template <class Reply>
using Result = std::expected<Outcome<Reply>, RuleError>;

// Lines only. @uses(tasks.read): a read rule cannot return a world, so it cannot change one.
using Lines = std::vector<std::string>;

[[nodiscard]] bool has_tag(const model::Task& t, const std::optional<model::Tag>& tag);

// add has no guard. Precondition: w.next_no is below 2^64 - 1 (the store keeps it at most 2^53).
[[nodiscard]] Outcome<model::TaskNo> add(const World& w, model::Title title,
                                         model::Priority priority = model::Priority::Normal,
                                         std::set<model::Tag> tags = {});
[[nodiscard]] Lines list(const World& w, bool all = false,
                         const std::optional<model::Tag>& tag = std::nullopt);
[[nodiscard]] Result<Nothing> done(const World& w, model::TaskNo no);
[[nodiscard]] Result<Nothing> reopen(const World& w, model::TaskNo no);
[[nodiscard]] Result<Nothing> rename(const World& w, model::TaskNo no, model::Title title);
[[nodiscard]] Result<Nothing> remove(const World& w, model::TaskNo no);
[[nodiscard]] Outcome<std::size_t> clear(const World& w);
[[nodiscard]] Lines stats(const World& w);

// stats over counts alone: what the fused store read hands over when only status is needed.
[[nodiscard]] std::string stats_line(std::size_t open, std::size_t done);

}  // namespace todo::board
