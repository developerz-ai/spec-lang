// The `cli` library's runtime for C++ (libs/cli/real/cpp): commands from the manifest.
#include "lib/cli/args.hpp"

#include <algorithm>
#include <cctype>

namespace lib::cli {

namespace {

using rt::meta::Kind;

std::string lower(std::string text) {
  std::ranges::transform(text, text.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
  return text;
}

bool is_enum(const rt::meta::Type& type) {
  return type.kind == Kind::Sum && std::ranges::all_of(type.variants, [](const auto& v) { return v.fields.empty(); });
}

// What `--help` shows after a flag of this type.
std::string hint(const rt::meta::Type& type) {
  if (!is_enum(type)) return "<" + lower(type.shown()) + ">";
  std::string out;
  for (const auto& v : type.variants) out += (out.empty() ? "" : "|") + lower(v.name);
  return out;
}

// An optional minus and ASCII digits, of any length the runtime's Int holds.
std::optional<rt::Int> parse_int(std::string_view text) {
  const bool negative = text.starts_with('-');
  const std::string_view digits = text.substr(negative ? 1 : 0);
  if (digits.empty() || !std::ranges::all_of(digits, [](char c) { return c >= '0' && c <= '9'; })) return std::nullopt;
  try {
    rt::Int n;
    for (const char c : digits) n = n * rt::Int{10} - rt::Int{c - '0'};
    return negative ? n : -n;
  } catch (const rt::Stop&) {
    return std::nullopt;
  }
}

Param param_of(const rt::meta::Param& p) {
  const rt::meta::Type& type = (*p.type)();
  if (!p.has_default) return {p.name, Shape::Positional, p.type, false, "<" + p.name + ">"};
  if (type.kind == Kind::Bool) return {p.name, Shape::Switch, nullptr, false, "[--" + p.name + "]"};
  if (type.kind == Kind::Set) {
    return {p.name, Shape::Repeat, type.args.at(0), false, "[--" + p.name + " " + hint((*type.args.at(0))()) + "]..."};
  }
  const bool some = type.kind == Kind::Option;
  const rt::meta::TypeRef one = some ? type.args.at(0) : p.type;
  return {p.name, Shape::Once, one, some, "[--" + p.name + " " + hint((*one)()) + "]"};
}

Usage usage(std::string message) {
  return Usage{std::move(message)};
}

std::string quoted(std::string_view text) {
  return "\"" + std::string{text} + "\"";
}

Parsed parse_args(const Command& command, const std::vector<std::string_view>& args) {
  const auto& params = command.params;
  std::vector<std::optional<rt::Data>> values(params.size());
  std::vector<std::vector<rt::Data>> repeated(params.size());
  std::vector<bool> seen(params.size(), false);
  std::vector<std::size_t> positionals;
  for (std::size_t i = 0; i < params.size(); ++i) {
    if (params[i].shape == Shape::Positional) positionals.push_back(i);
  }
  std::size_t filled = 0;
  bool flags_ended = false;

  for (std::size_t i = 1; i < args.size(); ++i) {
    const std::string_view arg = args[i];
    if (!flags_ended && arg == "--") {
      flags_ended = true;
      continue;
    }
    if (!flags_ended && arg.starts_with("--")) {
      const auto found = std::ranges::find_if(
          params, [&](const Param& p) { return p.shape != Shape::Positional && p.name == arg.substr(2); });
      if (found == params.end()) return usage("unknown flag " + std::string{arg});
      const auto at = static_cast<std::size_t>(found - params.begin());
      const Param& p = *found;
      if (p.shape == Shape::Switch) {
        if (seen[at]) return usage(std::string{arg} + " given twice");
        values[at] = rt::Data{true};
      } else {
        // A value flag takes the next argument, whatever it is.
        if (i + 1 == args.size()) return usage(std::string{arg} + " needs a value");
        const std::string_view text = args[++i];
        if (p.shape == Shape::Once && seen[at]) return usage(std::string{arg} + " given twice");
        const rt::meta::Type& type = (*p.type)();
        auto value = parse_value(type, text);
        if (!value) return usage(std::string{arg} + ": not a " + type.shown());
        if (p.shape == Shape::Repeat) {
          repeated[at].push_back(std::move(*value));
          values[at] = rt::Data::items(repeated[at]);
        } else {
          values[at] = p.some ? rt::Data::some(std::move(*value)) : std::move(*value);
        }
      }
      seen[at] = true;
      continue;
    }
    if (filled == positionals.size()) return usage("unexpected argument " + quoted(arg));
    const std::size_t at = positionals[filled];
    const rt::meta::Type& type = (*params[at].type)();
    auto value = parse_value(type, arg);
    if (!value) return usage("<" + params[at].name + ">: not a " + type.shown());
    values[at] = std::move(*value);
    ++filled;
  }
  if (filled < positionals.size()) return usage("missing <" + params[positionals[filled]].name + ">");

  Call call{&command, {}};
  for (std::size_t i = 0; i < params.size(); ++i) {
    if (values[i]) {
      call.values.push_back(std::move(*values[i]));
      continue;
    }
    const auto& fallback = command.rule->params[i].fallback;
    if (!fallback) return usage("--" + params[i].name + ": its default reads another parameter, so it must be given");
    call.values.push_back(*fallback);
  }
  return call;
}

}  // namespace

bool is_utf8(std::string_view bytes) {
  const auto byte = [&](std::size_t i) {
    return static_cast<unsigned char>(bytes[i]);
  };
  for (std::size_t i = 0; i < bytes.size();) {
    const unsigned char lead = byte(i);
    std::size_t more = 0;
    char32_t c = 0;
    char32_t least = 0;
    if (lead < 0x80) {
      ++i;
      continue;
    }
    if (lead >= 0xC2 && lead < 0xE0) {
      more = 1, c = lead & 0x1FU, least = 0x80;
    } else if (lead >= 0xE0 && lead < 0xF0) {
      more = 2, c = lead & 0x0FU, least = 0x800;
    } else if (lead >= 0xF0 && lead < 0xF5) {
      more = 3, c = lead & 0x07U, least = 0x10000;
    } else {
      return false;
    }
    if (bytes.size() - i <= more) return false;
    for (std::size_t k = 1; k <= more; ++k) {
      if ((byte(i + k) & 0xC0U) != 0x80) return false;
      c = (c << 6U) | (byte(i + k) & 0x3FU);
    }
    if (c < least || c > 0x10FFFF || (c >= 0xD800 && c <= 0xDFFF)) return false;
    i += more + 1;
  }
  return true;
}

std::optional<rt::Data> parse_value(const rt::meta::Type& type, std::string_view text) {
  std::optional<rt::Data> value;
  if (is_enum(type)) {
    for (std::size_t i = 0; i < type.variants.size() && !value; ++i) {
      if (lower(type.variants[i].name) == text) value = rt::Data::fields(i, {});
    }
  } else if (type.kind == Kind::Int) {
    if (const auto n = parse_int(text)) value = rt::Data{*n};
  } else if (type.kind == Kind::Bool) {
    if (text == "true" || text == "false") value = rt::Data{text == "true"};
  } else if (type.kind == Kind::Text) {
    if (is_utf8(text)) value = rt::Data{rt::Text{text}};
  }
  if (!value) return std::nullopt;
  try {
    if (type.conforms && !type.conforms(*value)) return std::nullopt;
  } catch (const rt::Failure&) {
    return std::nullopt;
  }
  return value;
}

std::vector<Command> commands(const rt::meta::App& app) {
  std::vector<Command> out;
  for (const rt::meta::Callable& rule : app.rules) {
    const rt::meta::Note* trigger = rule.trigger("cli", "cli");
    const rt::meta::NoteValue* name = trigger == nullptr ? nullptr : trigger->arg("command");
    if (name == nullptr || name->kind != rt::meta::NoteValue::Kind::Text) continue;
    Command command{name->text, &rule, {}};
    for (const rt::meta::Param& p : rule.params) command.params.push_back(param_of(p));
    out.push_back(std::move(command));
  }
  return out;
}

std::string help_text(const std::string& bin, const std::vector<Command>& commands) {
  std::string out = "usage: " + bin + " <command> [args]\n\ncommands:\n";
  for (const Command& c : commands) {
    out += "  " + c.name;
    for (const Param& p : c.params) out += " " + p.usage;
    out += "\n";
  }
  return out;
}

Parsed parse(const std::vector<Command>& commands, const std::vector<std::string_view>& args) {
  if (args.empty()) return Help{false};
  for (const std::string_view a : args) {
    if (a == "--") break;
    if (a == "--help" || a == "-h") return Help{true};
  }
  const auto command = std::ranges::find_if(commands, [&](const Command& c) { return c.name == args[0]; });
  if (command == commands.end()) return usage("unknown command " + quoted(args[0]));
  return parse_args(*command, args);
}

}  // namespace lib::cli
