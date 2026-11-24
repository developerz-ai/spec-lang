// The `cli` library's runtime for C++ (libs/cli/real/cpp): the store's file format, derived from the states'
// declared types alone:
//   {"version":1, <each state that is not a collection>, <each collection>}, each group in declaration order,
//   keyed by the state's name. One line of JSON and a newline, no other white space.
//   Int: an integer within 2^53 (a larger one is `full` when written, unreadable when read).
//   Text: a string; only `"` and `\` are escaped, and what JSON cannot hold raw.  Bool: true or false.
//   A record: an object of its fields in declared order.  A sum without data: its variant's name.
//   A sum with data: {"<variant>": {<fields>}}.  Option: null, or the value.  List, Set, tuple: an array.
//   Map[K, R] whose record R starts with a field of type K holding each entry's key: the array of its records;
//   any other Map: an array of [key, value] pairs.
// A read accepts any valid JSON with exactly these keys, then checks every value against its declared type.
#pragma once

#include <string>

#include "lib/cli/json.hpp"
#include "rt/meta.hpp"

namespace lib::cli {

// A value the store cannot hold: an Int past 2^53.
struct Full {};
// Valid JSON that is not a world of this program.
struct Unfit {};

// The world's bytes. Throws Full.
std::string encode_world(const rt::meta::App& app, const rt::Data& world);
// The world a stored document holds, each state checked against its declared type. Throws Unfit.
rt::Data decode_world(const rt::meta::App& app, const Json& json);

}  // namespace lib::cli
