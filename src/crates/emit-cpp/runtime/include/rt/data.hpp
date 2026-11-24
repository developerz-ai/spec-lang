// A value of any declared type as a library's runtime sees it: the shape without the C++ type. An app's
// generated code turns its own types into Data and back; a library (arguments, a store, a script) works on
// Data and on the descriptions in rt/meta.hpp, and so knows no program.
#pragma once

#include <utility>
#include <variant>
#include <vector>

#include "rt/value.hpp"

namespace rt {

class Data {
 public:
  struct None {};
  struct Some {
    std::vector<Data> value;  // exactly one
  };
  // A List, a Set (ascending) or a tuple.
  struct Items {
    std::vector<Data> items;
  };
  // A Map, ascending by key.
  struct Entries {
    std::vector<std::pair<Data, Data>> entries;
  };
  // A record (variant 0), or a variant of a sum with its fields.
  struct Fields {
    std::size_t variant = 0;
    std::vector<Data> values;
  };

  Data() = default;
  Data(Unit) {}                                        // NOLINT
  Data(bool value) : held_(value) {}                   // NOLINT
  Data(Int value) : held_(value) {}                    // NOLINT
  Data(Text value) : held_(std::move(value)) {}        // NOLINT
  Data(const char* value) : held_(Text{value}) {}      // NOLINT
  static Data none() { return Data{Held{None{}}}; }
  static Data some(Data value);
  static Data items(std::vector<Data> items) { return Data{Held{Items{std::move(items)}}}; }
  static Data entries(std::vector<std::pair<Data, Data>> entries);
  static Data fields(std::size_t variant, std::vector<Data> values) {
    return Data{Held{Fields{variant, std::move(values)}}};
  }

  bool is_bool() const { return std::holds_alternative<bool>(held_); }
  bool is_int() const { return std::holds_alternative<Int>(held_); }
  bool is_text() const { return std::holds_alternative<Text>(held_); }
  bool is_none() const { return std::holds_alternative<None>(held_); }
  bool is_some() const { return std::holds_alternative<Some>(held_); }

  // Each stops E-TYPE-MISMATCH when the value has another shape.
  bool boolean() const;
  Int integer() const;
  const Text& text() const;
  const Data& some() const;
  const std::vector<Data>& items() const;
  const std::vector<std::pair<Data, Data>>& entries() const;
  std::size_t variant() const;
  const std::vector<Data>& values() const;

 private:
  using Held = std::variant<Unit, bool, Int, Text, None, Some, Items, Entries, Fields>;
  explicit Data(Held held) : held_(std::move(held)) {}
  Held held_;
};

// ---- the built-in types as Data and back; a generated type brings its own pair ----

Data to_data(Unit);
Data to_data(bool value);
Data to_data(Int value);
Data to_data(const Text& value);
template <class T>
Data to_data(const Option<T>& value);
template <class T>
Data to_data(const Box<T>& value);
template <class T>
Data to_data(const std::vector<T>& value);
template <class T>
Data to_data(const std::set<T>& value);
template <class K, class V>
Data to_data(const std::map<K, V>& value);
template <class... Ts>
Data to_data(const std::tuple<Ts...>& value);

void from_data(const Data& data, Unit& value);
void from_data(const Data& data, bool& value);
void from_data(const Data& data, Int& value);
void from_data(const Data& data, Text& value);
template <class T>
void from_data(const Data& data, Option<T>& value);
template <class T>
void from_data(const Data& data, Box<T>& value);
template <class T>
void from_data(const Data& data, std::vector<T>& value);
template <class T>
void from_data(const Data& data, std::set<T>& value);
template <class K, class V>
void from_data(const Data& data, std::map<K, V>& value);
template <class... Ts>
void from_data(const Data& data, std::tuple<Ts...>& value);

// Data of the right shape as a `T`.
template <class T>
T from(const Data& data) {
  T value{};
  from_data(data, value);
  return value;
}

template <class T>
Data to_data(const Option<T>& value) {
  return value.has_value() ? Data::some(to_data(*value)) : Data::none();
}
template <class T>
Data to_data(const Box<T>& value) {
  return to_data(*value);
}
template <class T>
Data to_data(const std::vector<T>& value) {
  std::vector<Data> items;
  items.reserve(value.size());
  for (const auto& x : value) {
    const T& item = x;  // a Bool list gives values, not references
    items.push_back(to_data(item));
  }
  return Data::items(std::move(items));
}
template <class T>
Data to_data(const std::set<T>& value) {
  std::vector<Data> items;
  for (const auto& x : value) items.push_back(to_data(x));
  return Data::items(std::move(items));
}
template <class K, class V>
Data to_data(const std::map<K, V>& value) {
  std::vector<std::pair<Data, Data>> entries;
  for (const auto& [k, v] : value) entries.emplace_back(to_data(k), to_data(v));
  return Data::entries(std::move(entries));
}
template <class... Ts>
Data to_data(const std::tuple<Ts...>& value) {
  std::vector<Data> items;
  std::apply([&](const Ts&... xs) { (items.push_back(to_data(xs)), ...); }, value);
  return Data::items(std::move(items));
}

template <class T>
void from_data(const Data& data, Option<T>& value) {
  value = data.is_none() ? Option<T>{} : Option<T>{from<T>(data.some())};
}
template <class T>
void from_data(const Data& data, Box<T>& value) {
  value = Box<T>{from<T>(data)};
}
template <class T>
void from_data(const Data& data, std::vector<T>& value) {
  value.clear();
  for (const auto& x : data.items()) value.push_back(from<T>(x));
}
template <class T>
void from_data(const Data& data, std::set<T>& value) {
  value.clear();
  for (const auto& x : data.items()) value.insert(from<T>(x));
}
template <class K, class V>
void from_data(const Data& data, std::map<K, V>& value) {
  value.clear();
  for (const auto& [k, v] : data.entries()) value.insert_or_assign(from<K>(k), from<V>(v));
}
template <class... Ts>
void from_data(const Data& data, std::tuple<Ts...>& value) {
  std::size_t i = 0;
  std::apply([&](Ts&... xs) { (from_data(data.items().at(i++), xs), ...); }, value);
}

}  // namespace rt
