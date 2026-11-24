// An app as data: its notes, its rules with their parameters and triggers, its states, and a way to call
// each rule with values a library parsed. The generated manifest fills these in; a library's runtime reads
// them and never names a program.
#pragma once

#include <functional>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "rt/data.hpp"

namespace program {
struct Machine;
}  // namespace program

namespace rt::meta {

struct Type;
// A description, by the function that gives it: two declared types are the same when their functions are.
using TypeRef = const Type& (*)();

enum class Kind { Unit, Bool, Int, Text, Option, List, Set, Map, Tuple, Record, Sum };

struct Field {
  std::string name;
  TypeRef type;
};

struct Variant {
  std::string name;
  std::vector<Field> fields;
};

// A declared type: its shape, its name when it has one (`TaskNo`, `Task`), and whether a value of the
// shape conforms to it (ranges and claims).
struct Type {
  Kind kind = Kind::Unit;
  std::string name = {};
  // Option, List, Set: the item; Map: key and value; Tuple: each item.
  std::vector<TypeRef> args = {};
  // Record.
  std::vector<Field> fields = {};
  // Sum.
  std::vector<Variant> variants = {};
  // Unset: every value of the shape conforms.
  std::function<bool(const Data&)> conforms = {};

  // The name a message uses: the declared one, or the shape's.
  std::string shown() const;
};

// The value of a note's argument: `"todo"`, `60Hz`, `file(".todo.json")`.
struct NoteValue {
  enum class Kind { Int, Text, Quantity, Variant } kind = Kind::Int;
  Int number = {};
  // The text, the unit, or the variant's name.
  std::string text = {};
  std::vector<std::pair<std::string, NoteValue>> args = {};
  const NoteValue* arg(std::string_view param) const;
};

// A library's note or trigger with its arguments by parameter name: `bin("todo")`, `cli("add")`.
struct Note {
  std::string library;
  std::string name;
  std::vector<std::pair<std::string, NoteValue>> args = {};
  const NoteValue* arg(std::string_view param) const;
};

struct Param {
  std::string name;
  TypeRef type;
  // The default, when the parameter has one that reads no other parameter.
  std::optional<Data> fallback = {};
};

// A rule or a `fn` a library may call with values of its parameters' types, in order.
struct Callable {
  std::string qualified;
  std::string name;
  bool rule = false;
  std::vector<Note> triggers = {};
  std::vector<Param> params = {};
  TypeRef result = nullptr;
  std::function<Data(program::Machine&, const std::vector<Data>&)> call = {};

  const Note* trigger(std::string_view library, std::string_view name) const;
};

struct State {
  std::string qualified;
  std::string name;
  TypeRef type;
};

// One `app`.
struct App {
  std::string name;
  std::string qualified;
  std::string module;
  std::string target;
  // The library whose app shape it has; empty for `rules: [...]`.
  std::string shape;
  std::vector<Note> notes;
  // `rules: [...]`, then the shape's parameters: each names declarations.
  std::vector<std::pair<std::string, std::vector<std::string>>> params;
  // Every rule the named declarations reach.
  std::vector<Callable> rules;
  // Every public `fn` of plain data.
  std::vector<Callable> fns;
  std::vector<State> states;
  // The world as a record of the states, in their order, and back.
  std::function<Data(const program::Machine&)> world;
  std::function<void(program::Machine&, const Data&)> set_world;
  // Whether every law about one world alone holds on the machine's world.
  std::function<bool(const program::Machine&)> lawful;

  const Note* note(std::string_view library, std::string_view name) const;
  // A rule or a fn by its qualified name.
  const Callable* find(std::string_view qualified) const;
  // The one declaration an app parameter names (`tick: world.step`).
  const Callable* named(std::string_view param) const;
};

}  // namespace rt::meta
