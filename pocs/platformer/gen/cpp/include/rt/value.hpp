// The C++ printer's runtime, the same in every project `spec-lang build` prints. Do not edit.
// The values of spec-lang in C++: Unit, Text, Option, the ordered collections, ranges, failures, and `show`.
// Equality is structural and `<=>` is the language's value order: false < true, numbers by value, Text by
// code point, records field by field, sums by declared variant then fields, collections as their sequences.
#pragma once

#include <compare>
#include <cstddef>
#include <map>
#include <memory>
#include <optional>
#include <set>
#include <string>
#include <tuple>
#include <typeinfo>
#include <utility>
#include <vector>

#include "rt/halt.hpp"
#include "rt/int.hpp"

namespace rt {

struct Unit {
  friend constexpr std::strong_ordering operator<=>(Unit, Unit) = default;
  friend constexpr bool operator==(Unit, Unit) = default;
};
inline constexpr Unit unit{};

// UTF-8; its byte order is its code point order.
using Text = std::string;
template <class T>
using List = std::vector<T>;
template <class T>
using Set = std::set<T>;
template <class K, class V>
using Map = std::map<K, V>;

// `Option[T] = Some(value) | none`: Some orders before none, as declared.
template <class T>
class Option {
 public:
  constexpr Option() = default;
  constexpr explicit Option(T value) : value_(std::move(value)) {}

  constexpr bool has_value() const {
    return value_.has_value();
  }
  constexpr const T& operator*() const {
    return *value_;
  }
  constexpr const T* operator->() const {
    return &*value_;
  }
  constexpr T value_or(T fallback) const {
    return value_ ? *value_ : fallback;
  }

  friend constexpr std::strong_ordering operator<=>(const Option& a, const Option& b) {
    if (a.value_ && b.value_) return *a.value_ <=> *b.value_;
    return b.has_value() <=> a.has_value();
  }
  friend constexpr bool operator==(const Option& a, const Option& b) {
    return a.value_ == b.value_;
  }

 private:
  std::optional<T> value_;
};

template <class T>
constexpr Option<std::decay_t<T>> some(T&& value) {
  return Option<std::decay_t<T>>{std::forward<T>(value)};
}

// A value kept behind a pointer, so a type can hold itself (`Tagged(name: Text, inner: Shape)`).
// It copies, compares and shows as the value it holds.
template <class T>
class Box {
 public:
  Box() : held_(std::make_unique<T>()) {}
  Box(T value) : held_(std::make_unique<T>(std::move(value))) {}  // NOLINT: a value is its box
  Box(const Box& other) : held_(std::make_unique<T>(*other.held_)) {}
  Box(Box&&) noexcept = default;
  Box& operator=(const Box& other) {
    held_ = std::make_unique<T>(*other.held_);
    return *this;
  }
  Box& operator=(Box&&) noexcept = default;
  ~Box() = default;

  const T& operator*() const {
    return *held_;
  }
  T& operator*() {
    return *held_;
  }

  friend std::strong_ordering operator<=>(const Box& a, const Box& b) {
    return *a.held_ <=> *b.held_;
  }
  friend bool operator==(const Box& a, const Box& b) {
    return *a.held_ == *b.held_;
  }

 private:
  std::unique_ptr<T> held_;
};

// `lo..=hi` or `lo..<hi` as a value.
struct Range {
  Int lo;
  Int hi;
  bool inclusive = true;
  constexpr bool has(Int n) const {
    return n >= lo && (inclusive ? n <= hi : n < hi);
  }
  friend constexpr std::strong_ordering operator<=>(const Range&, const Range&) = default;
  friend constexpr bool operator==(const Range&, const Range&) = default;
};

// ---- show: a value as spec-lang source, which "{x}" prints for anything but Text ----

Text show(Unit);
Text show(bool value);
Text show(Int value);
Text show(const Text& value);
Text show(const Range& value);
template <class T>
Text show(const Option<T>& value);
template <class T>
Text show(const Box<T>& value);
template <class T>
Text show(const std::vector<T>& value);
template <class T>
Text show(const std::set<T>& value);
template <class K, class V>
Text show(const std::map<K, V>& value);
template <class... Ts>
Text show(const std::tuple<Ts...>& value);

// `show` of any value, the generated types included (theirs is found by argument-dependent lookup).
template <class T>
Text str(const T& value) {
  return show(value);
}

namespace detail {
template <class Range, class F>
Text joined(const char* open, const Range& items, const char* close, F&& one) {
  Text out = open;
  bool first = true;
  for (const auto& item : items) {
    if (!first) out += ", ";
    first = false;
    out += one(item);
  }
  return out + close;
}
}  // namespace detail

template <class T>
Text show(const Option<T>& value) {
  return value.has_value() ? "Some(" + show(*value) + ")" : Text{"none"};
}
template <class T>
Text show(const Box<T>& value) {
  return show(*value);
}
template <class T>
Text show(const std::vector<T>& value) {
  return detail::joined("[", value, "]", [](const T& x) { return show(x); });
}
template <class T>
Text show(const std::set<T>& value) {
  return detail::joined("Set(", value, ")", [](const T& x) { return show(x); });
}
template <class K, class V>
Text show(const std::map<K, V>& value) {
  return detail::joined("Map(", value, ")", [](const auto& e) { return show(e.first) + " -> " + show(e.second); });
}
template <class... Ts>
Text show(const std::tuple<Ts...>& value) {
  Text out = "(";
  std::size_t i = 0;
  std::apply([&](const Ts&... xs) { ((out += (i++ == 0 ? "" : ", ") + show(xs)), ...); }, value);
  return out + ")";
}

// A record or a variant with fields, as source: the first field positional, the rest named.
class Shown {
 public:
  explicit Shown(const char* name) : text_(name) {}
  template <class T>
  Shown& field(const char* name, const T& value) {
    text_ += count_ == 0 ? "(" : ", ";
    if (count_++ > 0) text_ += Text{name} + ": ";
    text_ += str(value);
    return *this;
  }
  Text done() {
    return count_ == 0 ? text_ : text_ + ")";
  }

 private:
  Text text_;
  int count_ = 0;
};

// ---- a value of any type, where the type is only known at run time: failure data, event arguments ----

class Dyn {
 public:
  template <class T>
    requires(!std::same_as<std::decay_t<T>, Dyn>)
  explicit Dyn(T value) : held_(std::make_shared<Holder<std::decay_t<T>>>(std::move(value))) {}

  // The value as a `T`; stops E-TYPE-MISMATCH when it is of another type.
  template <class T>
  const T& as() const {
    if (const auto* h = dynamic_cast<const Holder<T>*>(held_.get())) return h->value;
    throw Stop{"E-TYPE-MISMATCH", "a value of another type than the one asked for: " + held_->show()};
  }

  friend std::strong_ordering operator<=>(const Dyn& a, const Dyn& b) {
    return a.held_->compare(*b.held_);
  }
  friend bool operator==(const Dyn& a, const Dyn& b) {
    return (a <=> b) == std::strong_ordering::equal;
  }
  friend Text show(const Dyn& value) {
    return value.held_->show();
  }

 private:
  struct Base {
    virtual ~Base() = default;
    virtual std::strong_ordering compare(const Base& other) const = 0;
    virtual Text show() const = 0;
  };
  template <class T>
  struct Holder final : Base {
    explicit Holder(T v) : value(std::move(v)) {}
    std::strong_ordering compare(const Base& other) const override {
      if (const auto* same = dynamic_cast<const Holder*>(&other)) return value <=> same->value;
      return typeid(*this).before(typeid(other)) ? std::strong_ordering::less : std::strong_ordering::greater;
    }
    Text show() const override {
      return str(value);
    }
    T value;
  };
  std::shared_ptr<const Base> held_;
};

// A language failure: `fail E(data)`, Missing, DivByZero, Uneven. Thrown; handled by `else`, by a guard,
// reported by a rule as its refusal.
struct Failure {
  std::string name;
  std::vector<Dyn> data = {};
  friend std::strong_ordering operator<=>(const Failure&, const Failure&) = default;
  friend bool operator==(const Failure&, const Failure&) = default;
  friend Text show(const Failure& f) {
    return "fail " + f.name +
           (f.data.empty() ? Text{} : detail::joined("(", f.data, ")", [](const Dyn& d) { return show(d); }));
  }
};

// The type of an expression that never gives a value; it stands where any value is expected.
struct Never {
  template <class T>
  [[noreturn]] operator T() const {  // NOLINT: by design
    throw Stop{"E-RUNTIME", "a value that cannot exist was used"};
  }
};

template <class... Ts>
[[noreturn]] Never fail(const char* name, Ts&&... data) {
  throw Failure{name, std::vector<Dyn>{Dyn{std::forward<Ts>(data)}...}};
}
[[noreturn]] Never missing();
[[noreturn]] Never hole(const char* name);
[[noreturn]] Never no_arm(const Text& shown);
// A foreign body in a build that has no library runtime linked: the check is blocked.
[[noreturn]] Never foreign(const char* decl, const char* written);
[[noreturn]] Never stop(const char* code, const Text& message);

// `conform` broke: stops E-CLAIM.
void claim(bool holds, const char* what);

// A value that conforms to its declared type, or a stop with E-CLAIM.
template <class T, class F>
T conforming(T value, F&& conforms, const char* what) {
  claim(conforms(value), what);
  return value;
}

// `assert` in a test: false fails the check.
void check(bool holds);

// One turn of a `while` loop: a loop that does not end stops with E-RUNTIME instead of hanging.
void tick();

// A guard: false, or a failure while evaluating it, fails with the guard's error.
void guard(bool holds, const char* error);
template <class F>
  requires std::is_invocable_r_v<bool, F>
void guard(F&& condition, const char* error) {
  bool holds = false;
  try {
    holds = condition();
  } catch (const Failure&) {
  }
  guard(holds, error);
}

// A claim `T where p`: true holds; false or a failure does not.
template <class F>
bool holds(F&& claim_body) {
  try {
    return claim_body();
  } catch (const Failure&) {
    return false;
  }
}

// `value else fallback`: a failure of the value gives the fallback.
template <class F, class G>
auto or_else(F&& value, G&& fallback) -> std::invoke_result_t<F> {
  try {
    return value();
  } catch (const Failure&) {
  }
  return fallback();
}

// `option else fallback`: a failure or `none` gives the fallback, `Some(v)` gives `v`.
template <class F, class G>
auto unwrap_or(F&& option, G&& fallback) -> std::decay_t<decltype(*option())> {
  try {
    if (auto value = option(); value.has_value()) return *value;
  } catch (const Failure&) {
  }
  return fallback();
}

// ---- traces: what a law reads ----

// One call of a rule, as a trace records it. `rule` is the rule's number in the program.
struct Event {
  int rule = 0;
  std::vector<Dyn> args = {};
  // An argument by name: its position per rule; stops E-TYPE-MISMATCH where the rule has none.
  template <class T>
  const T& arg(const char* name, std::initializer_list<std::pair<int, int>> by_rule) const {
    for (const auto& [r, index] : by_rule) {
      if (r == rule && index >= 0) return args.at(static_cast<std::size_t>(index)).template as<T>();
    }
    throw Stop{"E-TYPE-MISMATCH", Text{"the event has no argument `"} + name + "`"};
  }
  friend std::strong_ordering operator<=>(const Event&, const Event&) = default;
  friend bool operator==(const Event&, const Event&) = default;
  friend Text show(const Event& e);
};

// What `show` prints for an event: the rule's module, name and parameter names.
struct RuleName {
  const char* module;
  const char* name;
  std::vector<const char*> params;
};
void name_rules(std::vector<RuleName> names);

// A step's reply: Ok(value) or Err(failure).
class Reply {
 public:
  explicit Reply(Dyn value) : ok_(true), value_(std::move(value)) {}
  explicit Reply(Failure failure) : ok_(false), value_(Dyn{unit}), failure_(std::move(failure)) {}
  bool ok() const {
    return ok_;
  }
  const Dyn& value() const {
    return value_;
  }
  const Failure& failure() const {
    return failure_;
  }
  friend std::strong_ordering operator<=>(const Reply& a, const Reply& b) {
    if (a.ok_ != b.ok_) return b.ok_ <=> a.ok_;
    return a.ok_ ? a.value_ <=> b.value_ : a.failure_ <=> b.failure_;
  }
  friend bool operator==(const Reply& a, const Reply& b) {
    return (a <=> b) == std::strong_ordering::equal;
  }
  friend Text show(const Reply& r) {
    return r.ok_ ? "Ok(" + show(r.value_) + ")" : "Err(" + show(r.failure_) + ")";
  }

 private:
  bool ok_;
  Dyn value_;
  Failure failure_ = {};
};

// Runs one call from outside: its value as Ok, its refusal as Err. Anything else passes through.
template <class F>
Reply reply_of(F&& call) {
  try {
    if constexpr (std::is_void_v<std::invoke_result_t<F>>) {
      call();
      return Reply{Dyn{unit}};
    } else {
      return Reply{Dyn{call()}};
    }
  } catch (const Failure& f) {
    return Reply{f};
  }
}

template <class World>
struct Step {
  Event event;
  Reply reply;
  World before;
  World after;
  friend std::strong_ordering operator<=>(const Step&, const Step&) = default;
  friend bool operator==(const Step&, const Step&) = default;
  friend Text show(const Step& s) {
    return show(s.event) + " -> " + show(s.reply);
  }
};

template <class World>
struct Trace {
  std::vector<Step<World>> steps;
  // The start world, then the world after each step.
  std::vector<World> states;
  friend std::strong_ordering operator<=>(const Trace&, const Trace&) = default;
  friend bool operator==(const Trace&, const Trace&) = default;
  friend Text show(const Trace& t) {
    return "<trace of " + std::to_string(t.steps.size()) + " steps>";
  }
};

}  // namespace rt
