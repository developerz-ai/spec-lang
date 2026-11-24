// Projected by hand from fn tag_text, fn line (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "todo/render/render.hpp"

#include <charconv>

namespace todo::render {

std::string tag_text(const std::set<model::Tag>& tags) {
  std::string out;
  for (const model::Tag& t : tags) {  // a std::set is already sorted: tags.sort(it)
    out += " #";
    out += t.text();
  }
  return out;
}

std::string line(const model::Task& t) {
  std::string out;
  out.reserve(20 + t.title.text().size() + 16 + 34 * t.tags.size());
  out.resize(20);  // 20 digits hold every uint64_t, so to_chars cannot fail
  const auto result = std::to_chars(out.data(), out.data() + out.size(), t.no.value());
  out.resize(static_cast<std::size_t>(result.ptr - out.data()));
  out += " [";
  out += mark(t.status);
  out += "] ";
  out += t.title.text();
  out += badge(t.priority);
  out += tag_text(t.tags);
  return out;
}

}  // namespace todo::render
