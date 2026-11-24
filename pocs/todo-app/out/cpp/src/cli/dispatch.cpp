// Projected by hand from the app's rules list (src/apps/cpp.slg) over todo.board (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "todo/cli/dispatch.hpp"

#include <utility>

#include "todo/board/rules.hpp"

namespace todo::cli {
namespace {

template <class... F>
struct Overloaded : F... {
  using F::operator()...;
};

template <class Reply>
Applied wrote(board::Outcome<Reply> o) {
  return {std::move(o.world), std::move(o.lines), std::nullopt};
}

template <class Reply>
Applied wrote(board::Result<Reply> r) {
  if (!r) {
    return {std::nullopt, {}, r.error()};
  }
  return wrote(std::move(*r));
}

Applied read(board::Lines lines) { return {std::nullopt, std::move(lines), std::nullopt}; }

}  // namespace

Applied apply(const Command& command, const board::World& world) {
  return std::visit(Overloaded{
                        [&](const Add& c) { return wrote(board::add(world, c.title, c.priority, c.tags)); },
                        [&](const List& c) { return read(board::list(world, c.all, c.tag)); },
                        [&](const Done& c) { return wrote(board::done(world, c.no)); },
                        [&](const Reopen& c) { return wrote(board::reopen(world, c.no)); },
                        [&](const Rename& c) { return wrote(board::rename(world, c.no, c.title)); },
                        [&](const Remove& c) { return wrote(board::remove(world, c.no)); },
                        [&](const Clear&) { return wrote(board::clear(world)); },
                        [&](const Stats&) { return read(board::stats(world)); },
                    },
                    command);
}

}  // namespace todo::cli
