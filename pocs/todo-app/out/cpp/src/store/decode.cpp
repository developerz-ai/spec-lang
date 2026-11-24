// Projected by hand from type Task, type Title, type Tag (src/todo/model.slg) and state tasks, state next_no (src/todo/board.slg), read per docs/contract.md section 3. Stand-in for spec-lang build output; change the spec, not this file.
#include <algorithm>
#include <array>
#include <cstring>
#include <vector>

#include "todo/model/text.hpp"
#include "todo/model/unicode.hpp"
#include "todo/store/codec.hpp"
#include "todo/store/json_lexer.hpp"

namespace todo::store {
namespace {

// A tag held without allocation: is_tag bounds it to 32 ASCII bytes.
struct SmallTag {
  std::array<char, 32> bytes{};
  std::size_t size = 0;
  [[nodiscard]] std::string_view view() const noexcept { return {bytes.data(), size}; }
  auto operator<=>(const SmallTag& o) const noexcept { return view() <=> o.view(); }
  bool operator==(const SmallTag& o) const noexcept { return view() == o.view(); }
};

// One task as read, before the sink decides what to keep.
struct TaskRead {
  std::uint64_t no = 0;
  std::string_view title;
  model::Status status = model::Status::Open;
  model::Priority priority = model::Priority::Normal;
  const std::vector<SmallTag>* tags = nullptr;
};

template <std::size_t N>
std::optional<std::size_t> key_index(std::string_view key, const std::array<std::string_view, N>& keys) {
  for (std::size_t i = 0; i < N; ++i) {
    if (keys[i] == key) {
      return i;
    }
  }
  return std::nullopt;
}

std::optional<model::Status> status_of(std::string_view s) {
  if (s == "Open") {
    return model::Status::Open;
  }
  if (s == "Done") {
    return model::Status::Done;
  }
  return std::nullopt;
}

std::optional<model::Priority> priority_of(std::string_view s) {
  if (s == "Low") {
    return model::Priority::Low;
  }
  if (s == "Normal") {
    return model::Priority::Normal;
  }
  if (s == "High") {
    return model::Priority::High;
  }
  return std::nullopt;
}

// Reads one store, schema first: there is no generic JSON tree, because every key and every value
// type is fixed by section 3, so anything else is unreadable the moment it is met.
template <class Sink>
class Reader {
 public:
  Reader(std::string_view bytes, Sink& sink) : lex_(bytes), sink_(sink) {}

  bool document() {
    static constexpr std::array<std::string_view, 3> keys{"version", "next_no", "tasks"};
    std::array<bool, 3> seen{};
    std::uint64_t next_no = 0;
    if (!lex_.eat('{')) {
      return false;
    }
    for (bool first = true; !lex_.eat('}'); first = false) {
      if (!first && !lex_.eat(',')) {
        return false;
      }
      const auto key = lex_.string(key_scratch_);
      if (!key || !lex_.eat(':')) {
        return false;
      }
      const auto index = key_index(*key, keys);
      if (!index || seen[*index]) {
        return false;  // a key outside the schema, or a key twice
      }
      seen[*index] = true;
      if (*index == 0) {
        if (lex_.integer(1) != 1U) {
          return false;
        }
      } else if (*index == 1) {
        const auto n = lex_.integer(max_next_no);
        if (!n || *n < 1) {
          return false;
        }
        next_no = *n;
      } else if (!tasks()) {
        return false;
      }
    }
    if (!lex_.at_end() || !std::ranges::all_of(seen, [](bool b) { return b; })) {
      return false;
    }
    // Task numbers are checked against next_no only now: keys may come in any order.
    std::ranges::sort(numbers_);
    if (std::ranges::adjacent_find(numbers_) != numbers_.end()) {
      return false;
    }
    if (!numbers_.empty() && numbers_.back() >= next_no) {
      return false;
    }
    sink_.next_no(next_no);
    return true;
  }

 private:
  bool tasks() {
    if (!lex_.eat('[')) {
      return false;
    }
    for (bool first = true; !lex_.eat(']'); first = false) {
      if (!first && !lex_.eat(',')) {
        return false;
      }
      if (!task()) {
        return false;
      }
    }
    return true;
  }

  bool task() {
    static constexpr std::array<std::string_view, 5> keys{"no", "title", "status", "priority", "tags"};
    std::array<bool, 5> seen{};
    TaskRead t;
    if (!lex_.eat('{')) {
      return false;
    }
    for (bool first = true; !lex_.eat('}'); first = false) {
      if (!first && !lex_.eat(',')) {
        return false;
      }
      const auto key = lex_.string(key_scratch_);
      if (!key || !lex_.eat(':')) {
        return false;
      }
      const auto index = key_index(*key, keys);
      if (!index || seen[*index]) {
        return false;
      }
      seen[*index] = true;
      if (!field(*index, t)) {
        return false;
      }
    }
    if (!std::ranges::all_of(seen, [](bool b) { return b; })) {
      return false;
    }
    numbers_.push_back(t.no);
    t.tags = &tags_;
    return sink_.task(t);
  }

  bool field(std::size_t index, TaskRead& t) {
    switch (index) {
      case 0: {
        const auto no = lex_.integer(max_next_no);
        if (!no || *no < 1) {
          return false;
        }
        t.no = *no;
        return true;
      }
      case 1: {
        const auto title = lex_.string(title_scratch_);
        if (!title || !model::is_title(*title)) {
          return false;
        }
        t.title = *title;
        return true;
      }
      case 2: {
        const auto s = lex_.string(value_scratch_);
        const auto status = s ? status_of(*s) : std::nullopt;
        t.status = status.value_or(model::Status::Open);
        return status.has_value();
      }
      case 3: {
        const auto s = lex_.string(value_scratch_);
        const auto priority = s ? priority_of(*s) : std::nullopt;
        t.priority = priority.value_or(model::Priority::Normal);
        return priority.has_value();
      }
      default:
        return tag_list();
    }
  }

  bool tag_list() {
    tags_.clear();
    if (!lex_.eat('[')) {
      return false;
    }
    for (bool first = true; !lex_.eat(']'); first = false) {
      if (!first && !lex_.eat(',')) {
        return false;
      }
      const auto tag = lex_.string(value_scratch_);
      if (!tag || !model::is_tag(*tag)) {
        return false;
      }
      SmallTag small;
      std::memcpy(small.bytes.data(), tag->data(), tag->size());
      small.size = tag->size();
      tags_.push_back(small);
    }
    std::ranges::sort(tags_);
    return std::ranges::adjacent_find(tags_) == tags_.end();  // none twice
  }

  json::Lexer lex_;
  Sink& sink_;
  std::string key_scratch_;
  std::string title_scratch_;
  std::string value_scratch_;
  std::vector<SmallTag> tags_;
  std::vector<std::uint64_t> numbers_;
};

// Keeps everything: the world a rule runs on.
struct WorldSink {
  board::World world;
  bool task(const TaskRead& r) {
    const auto no = model::TaskNo::make(r.no);
    auto title = model::Title::make(std::string(r.title));
    if (!no || !title) {
      return false;
    }
    std::set<model::Tag> tags;
    for (const SmallTag& s : *r.tags) {
      tags.insert(tags.end(), *model::Tag::make(s.view()));  // sorted already: hinted insert
    }
    world.tasks.insert_or_assign(*no, model::Task{.no = *no,
                                                  .title = std::move(*title),
                                                  .status = r.status,
                                                  .priority = r.priority,
                                                  .tags = std::move(tags)});
    return true;
  }
  void next_no(std::uint64_t n) { world.next_no = *model::TaskNo::make(n); }
};

// Keeps two counts: all that rule stats reads.
struct CountSink {
  StatusCounts counts;
  bool task(const TaskRead& r) {
    ++(r.status == model::Status::Open ? counts.open : counts.done);
    return true;
  }
  void next_no(std::uint64_t) {}
};

}  // namespace

std::optional<board::World> decode(std::string_view bytes) {
  if (!model::unicode::is_valid_utf8(bytes)) {
    return std::nullopt;
  }
  WorldSink sink;
  Reader<WorldSink> reader(bytes, sink);
  if (!reader.document()) {
    return std::nullopt;
  }
  return std::move(sink.world);
}

std::optional<StatusCounts> count_statuses(std::string_view bytes) {
  if (!model::unicode::is_valid_utf8(bytes)) {
    return std::nullopt;
  }
  CountSink sink;
  Reader<CountSink> reader(bytes, sink);
  if (!reader.document()) {
    return std::nullopt;
  }
  return sink.counts;
}

}  // namespace todo::store
