// Projected by hand from state tasks, state next_no (src/todo/board.slg), written per "Bytes written" in docs/contract.md section 3. Stand-in for spec-lang build output; change the spec, not this file.
#include <charconv>

#include "todo/store/codec.hpp"

namespace todo::store {
namespace {

// Digits straight into the output buffer: no temporary.
void put_number(std::string& out, std::uint64_t n) {
  const std::size_t at = out.size();
  out.resize(at + 20);  // 20 digits hold every uint64_t, so to_chars cannot fail
  const auto result = std::to_chars(out.data() + at, out.data() + out.size(), n);
  out.resize(static_cast<std::size_t>(result.ptr - out.data()));
}

// Escapes only " and \; everything else, non-ASCII included, is raw UTF-8.
void put_string(std::string& out, std::string_view s) {
  out.push_back('"');
  for (const char c : s) {
    if (c == '"' || c == '\\') {
      out.push_back('\\');
    }
    out.push_back(c);
  }
  out.push_back('"');
}

std::string_view status_name(model::Status s) { return s == model::Status::Open ? "Open" : "Done"; }

std::string_view priority_name(model::Priority p) {
  switch (p) {
    case model::Priority::Low:
      return "Low";
    case model::Priority::Normal:
      return "Normal";
    case model::Priority::High:
      return "High";
  }
  return "Normal";
}

std::size_t estimate(const board::World& w) {
  std::size_t size = 64;
  for (const auto& [no, t] : w.tasks) {
    size += 96 + t.title.text().size() + 2;
    for (const model::Tag& tag : t.tags) {
      size += tag.text().size() + 3;
    }
  }
  return size;
}

}  // namespace

std::expected<std::string, StoreError> encode(const board::World& w) {
  if (w.next_no.value() > max_next_no) {
    return std::unexpected(StoreError::Full);
  }
  std::string out;
  out.reserve(estimate(w));
  out += R"({"version":1,"next_no":)";
  put_number(out, w.next_no.value());
  out += R"(,"tasks":[)";
  bool first_task = true;
  for (const auto& [no, t] : w.tasks) {  // std::map: by no ascending
    out += first_task ? R"({"no":)" : R"(,{"no":)";
    first_task = false;
    put_number(out, no.value());
    out += R"(,"title":)";
    put_string(out, t.title.text());
    out += R"(,"status":")";
    out += status_name(t.status);
    out += R"(","priority":")";
    out += priority_name(t.priority);
    out += R"(","tags":[)";
    bool first_tag = true;
    for (const model::Tag& tag : t.tags) {  // std::set: code-point order
      if (!first_tag) {
        out.push_back(',');
      }
      first_tag = false;
      put_string(out, tag.text());
    }
    out += "]}";
  }
  out += "]}\n";
  return out;
}

}  // namespace todo::store
