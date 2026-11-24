// The C++ printer's runtime, the same in every project `spec-lang build` prints. Do not edit.
// Data and the app manifest's lookups.
#include "rt/data.hpp"

#include "rt/meta.hpp"

namespace rt {

namespace {
[[noreturn]] void wrong(const char* wanted) {
  throw Stop{"E-TYPE-MISMATCH", std::string{"a value of another shape where "} + wanted + " was expected"};
}
}  // namespace

Data Data::some(Data value) {
  Some some;
  some.value.push_back(std::move(value));
  return Data{Held{std::move(some)}};
}

Data Data::entries(std::vector<std::pair<Data, Data>> entries) {
  return Data{Held{Entries{std::move(entries)}}};
}

bool Data::boolean() const {
  if (const auto* v = std::get_if<bool>(&held_)) return *v;
  wrong("a Bool");
}
Int Data::integer() const {
  if (const auto* v = std::get_if<Int>(&held_)) return *v;
  wrong("an Int");
}
const Text& Data::text() const {
  if (const auto* v = std::get_if<Text>(&held_)) return *v;
  wrong("a Text");
}
const Data& Data::some() const {
  if (const auto* v = std::get_if<Some>(&held_); v != nullptr && v->value.size() == 1) return v->value.front();
  wrong("Some");
}
const std::vector<Data>& Data::items() const {
  if (const auto* v = std::get_if<Items>(&held_)) return v->items;
  wrong("a List, a Set or a tuple");
}
const std::vector<std::pair<Data, Data>>& Data::entries() const {
  if (const auto* v = std::get_if<Entries>(&held_)) return v->entries;
  wrong("a Map");
}
std::size_t Data::variant() const {
  if (const auto* v = std::get_if<Fields>(&held_)) return v->variant;
  wrong("a record or a variant");
}
const std::vector<Data>& Data::values() const {
  if (const auto* v = std::get_if<Fields>(&held_)) return v->values;
  wrong("a record or a variant");
}

Data to_data(Unit) {
  return Data{unit};
}
Data to_data(bool value) {
  return Data{value};
}
Data to_data(Int value) {
  return Data{value};
}
Data to_data(const Text& value) {
  return Data{value};
}

void from_data(const Data&, Unit&) {}
void from_data(const Data& data, bool& value) {
  value = data.boolean();
}
void from_data(const Data& data, Int& value) {
  value = data.integer();
}
void from_data(const Data& data, Text& value) {
  value = data.text();
}

namespace meta {

namespace {
template <class Pairs>
const NoteValue* arg_of(const Pairs& args, std::string_view param) {
  for (const auto& [name, value] : args) {
    if (name == param) return &value;
  }
  return nullptr;
}
}  // namespace

std::string Type::shown() const {
  if (!name.empty()) return name;
  switch (kind) {
    case Kind::Unit:
      return "()";
    case Kind::Bool:
      return "Bool";
    case Kind::Int:
      return "Int";
    case Kind::Text:
      return "Text";
    case Kind::Option:
      return "Option";
    case Kind::List:
      return "List";
    case Kind::Set:
      return "Set";
    case Kind::Map:
      return "Map";
    case Kind::Tuple:
      return "tuple";
    case Kind::Record:
      return "record";
    case Kind::Sum:
      return "sum";
  }
  return "value";
}

const NoteValue* NoteValue::arg(std::string_view param) const {
  return arg_of(args, param);
}
const NoteValue* Note::arg(std::string_view param) const {
  return arg_of(args, param);
}

const Note* Callable::trigger(std::string_view library, std::string_view trigger_name) const {
  for (const Note& t : triggers) {
    if (t.library == library && t.name == trigger_name) return &t;
  }
  return nullptr;
}

const Note* App::note(std::string_view library, std::string_view note_name) const {
  for (const Note& n : notes) {
    if (n.library == library && n.name == note_name) return &n;
  }
  return nullptr;
}

const Callable* App::find(std::string_view wanted) const {
  for (const Callable& c : rules) {
    if (c.qualified == wanted) return &c;
  }
  for (const Callable& c : fns) {
    if (c.qualified == wanted) return &c;
  }
  return nullptr;
}

const Callable* App::named(std::string_view param) const {
  for (const auto& [name, decls] : params) {
    if (name == param && decls.size() == 1) return find(decls.front());
  }
  return nullptr;
}

}  // namespace meta
}  // namespace rt
