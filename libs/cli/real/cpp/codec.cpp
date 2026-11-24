// The `cli` library's runtime for C++ (libs/cli/real/cpp): the store's file format.
#include "lib/cli/codec.hpp"

#include <algorithm>

namespace lib::cli {

namespace {

using rt::Data;
using rt::meta::Kind;
using rt::meta::Type;

// The largest magnitude of a stored Int: a store constraint, not a language rule.
const rt::Int kMaxInt{9007199254740992LL};

void quote(std::string& out, std::string_view text) {
  static const char* const hex = "0123456789abcdef";
  out.push_back('"');
  for (const char c : text) {
    const auto byte = static_cast<unsigned char>(c);
    if (c == '"' || c == '\\') {
      out.push_back('\\');
      out.push_back(c);
    } else if (byte < 0x20) {
      out += "\\u00";
      out.push_back(hex[byte >> 4U]);
      out.push_back(hex[byte & 0xFU]);
    } else {
      out.push_back(c);
    }
  }
  out.push_back('"');
}

// A map stored as the list of its records: its values are records whose first field has the key's type.
bool keyed_records(const Type& map) {
  const Type& value = (*map.args.at(1))();
  return value.kind == Kind::Record && !value.fields.empty() && value.fields.front().type == map.args.at(0);
}

void encode(std::string& out, const Type& type, const Data& data);

void encode_fields(std::string& out, const std::vector<rt::meta::Field>& fields, const std::vector<Data>& values) {
  out.push_back('{');
  for (std::size_t i = 0; i < fields.size(); ++i) {
    if (i > 0) out.push_back(',');
    quote(out, fields[i].name);
    out.push_back(':');
    encode(out, (*fields[i].type)(), values.at(i));
  }
  out.push_back('}');
}

std::string encoded(const Type& type, const Data& data) {
  std::string out;
  encode(out, type, data);
  return out;
}

void encode(std::string& out, const Type& type, const Data& data) {
  switch (type.kind) {
    case Kind::Unit:
      out += "null";
      return;
    case Kind::Bool:
      out += data.boolean() ? "true" : "false";
      return;
    case Kind::Int:
      if (data.integer() > kMaxInt || data.integer() < -kMaxInt) throw Full{};
      out += rt::to_string(data.integer());
      return;
    case Kind::Text:
      quote(out, data.text());
      return;
    case Kind::Option:
      if (data.is_none()) {
        out += "null";
      } else {
        encode(out, (*type.args.at(0))(), data.some());
      }
      return;
    case Kind::List:
    case Kind::Set:
    case Kind::Tuple: {
      out.push_back('[');
      const auto& items = data.items();
      for (std::size_t i = 0; i < items.size(); ++i) {
        if (i > 0) out.push_back(',');
        encode(out, (*type.args.at(type.kind == Kind::Tuple ? i : 0))(), items[i]);
      }
      out.push_back(']');
      return;
    }
    case Kind::Map: {
      const Type& key = (*type.args.at(0))();
      const Type& value = (*type.args.at(1))();
      const auto& entries = data.entries();
      const bool keyed = keyed_records(type) && std::ranges::all_of(entries, [&](const auto& e) {
                           return encoded(key, e.first) == encoded(key, e.second.values().at(0));
                         });
      out.push_back('[');
      for (std::size_t i = 0; i < entries.size(); ++i) {
        if (i > 0) out.push_back(',');
        if (keyed) {
          encode(out, value, entries[i].second);
        } else {
          out.push_back('[');
          encode(out, key, entries[i].first);
          out.push_back(',');
          encode(out, value, entries[i].second);
          out.push_back(']');
        }
      }
      out.push_back(']');
      return;
    }
    case Kind::Record:
      encode_fields(out, type.fields, data.values());
      return;
    case Kind::Sum: {
      const auto& variant = type.variants.at(data.variant());
      if (variant.fields.empty()) {
        quote(out, variant.name);
      } else {
        out.push_back('{');
        quote(out, variant.name);
        out.push_back(':');
        encode_fields(out, variant.fields, data.values());
        out.push_back('}');
      }
      return;
    }
  }
}

Data decode(const Type& type, const Json& json);

std::vector<Data> decode_fields(const std::vector<rt::meta::Field>& fields, const Json& json) {
  if (json.kind != Json::Kind::Object || json.entries.size() != fields.size()) throw Unfit{};
  std::vector<Data> values;
  for (const auto& f : fields) {
    const Json* item = json.get(f.name);
    if (item == nullptr) throw Unfit{};
    values.push_back(decode((*f.type)(), *item));
  }
  return values;
}

Data decode_map(const Type& type, const Json& json) {
  const Type& key = (*type.args.at(0))();
  const Type& value = (*type.args.at(1))();
  const bool records = keyed_records(type);
  std::vector<std::pair<Data, Data>> entries;
  std::vector<std::string> keys;
  for (const Json& item : json.items) {
    if (item.kind == Json::Kind::Object && records) {
      Data v = decode(value, item);
      Data k = v.values().at(0);
      entries.emplace_back(std::move(k), std::move(v));
    } else if (item.kind == Json::Kind::Array && item.items.size() == 2) {
      entries.emplace_back(decode(key, item.items[0]), decode(value, item.items[1]));
    } else {
      throw Unfit{};
    }
    keys.push_back(encoded(key, entries.back().first));
  }
  std::ranges::sort(keys);
  if (std::ranges::adjacent_find(keys) != keys.end()) throw Unfit{};
  return Data::entries(std::move(entries));
}

Data decode(const Type& type, const Json& json) {
  const bool array = json.kind == Json::Kind::Array;
  switch (type.kind) {
    case Kind::Unit:
      if (json.kind != Json::Kind::Null) throw Unfit{};
      return Data{rt::unit};
    case Kind::Bool:
      if (json.kind != Json::Kind::Bool) throw Unfit{};
      return Data{json.boolean};
    case Kind::Int:
      if (json.kind != Json::Kind::Number || !json.integer) throw Unfit{};
      if (*json.integer > kMaxInt || *json.integer < -kMaxInt) throw Unfit{};
      return Data{*json.integer};
    case Kind::Text:
      if (json.kind != Json::Kind::String) throw Unfit{};
      return Data{json.string};
    case Kind::Option:
      return json.kind == Json::Kind::Null ? Data::none() : Data::some(decode((*type.args.at(0))(), json));
    case Kind::List:
    case Kind::Set: {
      if (!array) throw Unfit{};
      const Type& of = (*type.args.at(0))();
      std::vector<Data> items;
      std::vector<std::string> seen;
      for (const Json& item : json.items) {
        items.push_back(decode(of, item));
        if (type.kind == Kind::Set) seen.push_back(encoded(of, items.back()));
      }
      std::ranges::sort(seen);
      if (std::ranges::adjacent_find(seen) != seen.end()) throw Unfit{};
      return Data::items(std::move(items));
    }
    case Kind::Tuple: {
      if (!array || json.items.size() != type.args.size()) throw Unfit{};
      std::vector<Data> items;
      for (std::size_t i = 0; i < json.items.size(); ++i) items.push_back(decode((*type.args[i])(), json.items[i]));
      return Data::items(std::move(items));
    }
    case Kind::Map:
      if (!array) throw Unfit{};
      return decode_map(type, json);
    case Kind::Record:
      return Data::fields(0, decode_fields(type.fields, json));
    case Kind::Sum: {
      const auto variant = [&](std::string_view name) {
        const auto found = std::ranges::find_if(type.variants, [&](const auto& v) { return v.name == name; });
        if (found == type.variants.end()) throw Unfit{};
        return static_cast<std::size_t>(found - type.variants.begin());
      };
      if (json.kind == Json::Kind::String) {
        const std::size_t at = variant(json.string);
        if (!type.variants[at].fields.empty()) throw Unfit{};
        return Data::fields(at, {});
      }
      if (json.kind == Json::Kind::Object && json.entries.size() == 1) {
        const std::size_t at = variant(json.entries.front().first);
        if (type.variants[at].fields.empty()) throw Unfit{};
        return Data::fields(at, decode_fields(type.variants[at].fields, json.entries.front().second));
      }
      throw Unfit{};
    }
  }
  throw Unfit{};
}

struct Stored {
  std::string key;
  std::size_t state;
};

// The states in file order: every one that is not a collection, then the collections.
std::vector<Stored> layout(const rt::meta::App& app) {
  const auto collection = [&](std::size_t i) {
    const Kind kind = (*app.states[i].type)().kind;
    return kind == Kind::List || kind == Kind::Set || kind == Kind::Map;
  };
  const auto key = [&](std::size_t i) {
    const auto same = std::ranges::count_if(app.states, [&](const auto& s) { return s.name == app.states[i].name; });
    return same > 1 ? app.states[i].qualified : app.states[i].name;
  };
  std::vector<Stored> out;
  for (const bool want : {false, true}) {
    for (std::size_t i = 0; i < app.states.size(); ++i) {
      if (collection(i) == want) out.push_back({key(i), i});
    }
  }
  return out;
}

}  // namespace

std::string encode_world(const rt::meta::App& app, const Data& world) {
  std::string out = "{\"version\":1";
  for (const Stored& s : layout(app)) {
    out.push_back(',');
    quote(out, s.key);
    out.push_back(':');
    encode(out, (*app.states[s.state].type)(), world.values().at(s.state));
  }
  return out + "}\n";
}

Data decode_world(const rt::meta::App& app, const Json& json) {
  const auto stored = layout(app);
  if (json.kind != Json::Kind::Object || json.entries.size() != stored.size() + 1) throw Unfit{};
  const Json* version = json.get("version");
  if (version == nullptr || version->kind != Json::Kind::Number || version->integer != rt::Int{1}) throw Unfit{};
  std::vector<Data> states(app.states.size());
  for (const Stored& s : stored) {
    const Json* item = json.get(s.key);
    if (item == nullptr) throw Unfit{};
    const Type& type = (*app.states[s.state].type)();
    states[s.state] = decode(type, *item);
    if (type.conforms && !type.conforms(states[s.state])) throw Unfit{};
  }
  return Data::fields(0, std::move(states));
}

}  // namespace lib::cli
