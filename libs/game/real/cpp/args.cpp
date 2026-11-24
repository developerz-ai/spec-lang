// The `game` library's runtime for C++ (libs/game/real/cpp): the command line.
#include "lib/game/args.hpp"

#include <algorithm>
#include <array>

#include "lib/cli/args.hpp"

namespace lib::game {

namespace {

struct CommandSpec {
  std::string_view name;
  Command command;
  bool at;
  bool out;
};

constexpr std::array<CommandSpec, 4> kCommands{{
    {"play", Command::Play, false, false},
    {"run", Command::Run, false, false},
    {"draw", Command::Draw, true, false},
    {"shot", Command::Shot, true, true},
}};

std::string quoted(std::string_view text) {
  return "\"" + std::string{text} + "\"";
}

const std::vector<rt::meta::Param>& start_params(const rt::meta::App& app) {
  static const std::vector<rt::meta::Param> none;
  const rt::meta::Callable* start = app.named("start");
  return start == nullptr ? none : start->params;
}

// What a `start` parameter is when its flag is not given: its default, else the first of 1 and 0 its type admits.
std::optional<rt::Data> start_default(const rt::meta::Param& p) {
  if (p.fallback) return p.fallback;
  for (const char* text : {"1", "0"}) {
    if (auto value = lib::cli::parse_value((*p.type)(), text)) return value;
  }
  return std::nullopt;
}

// ASCII digits of any length; past a number no script reaches, the value is that number.
std::optional<rt::Int> parse_tick(std::string_view text) {
  if (text.empty() || !std::ranges::all_of(text, [](char c) { return c >= '0' && c <= '9'; })) return std::nullopt;
  const rt::Int ceiling{1'000'000'000'000LL};
  rt::Int n;
  for (const char c : text) n = rt::min(n * rt::Int{10} + rt::Int{c - '0'}, ceiling);
  return n;
}

}  // namespace

std::string bin_of(const rt::meta::App& app) {
  const rt::meta::Note* note = app.note("game", "bin");
  const rt::meta::NoteValue* name = note == nullptr ? nullptr : note->arg("name");
  return name == nullptr ? app.name : name->text;
}

std::string help_text(const rt::meta::App& app) {
  std::string start;
  for (const auto& p : start_params(app)) start += " [--" + p.name + " <n>]";
  std::string out = "usage: " + bin_of(app) + " <command> [options]\n\ncommands:\n";
  for (const CommandSpec& c : kCommands) {
    out += "  " + std::string{c.name} + start;
    if (c.at) out += " [--at <tick>]";
    if (c.out) out += " [--out <file.png>]";
    out += "\n";
  }
  // What the keys do is the `@why` of the fn that reads them.
  const rt::meta::Callable* input = app.named("input");
  out += "\nplay opens a window: ";
  if (input != nullptr && !input->why.empty()) out += input->why + ", ";
  out += "Esc to quit.\nrun, draw and shot read a script of controls on standard input.\n";
  return out;
}

Parsed parse(const rt::meta::App& app, const std::vector<std::string_view>& args) {
  if (args.empty()) return Help{false};
  if (std::ranges::any_of(args, [](std::string_view a) { return a == "--help" || a == "-h"; })) return Help{true};
  const auto spec = std::ranges::find(kCommands, args[0], &CommandSpec::name);
  if (spec == kCommands.end()) return Usage{"unknown command " + quoted(args[0])};

  const auto& params = start_params(app);
  std::vector<std::optional<rt::Data>> start(params.size());
  Invocation invocation{spec->command, {}, std::nullopt, std::nullopt};
  std::vector<std::string> seen;
  for (std::size_t i = 1; i < args.size(); ++i) {
    const std::string_view arg = args[i];
    if (!arg.starts_with("--")) return Usage{"unexpected argument " + quoted(arg)};
    const std::string_view name = arg.substr(2);
    const auto param = std::ranges::find(params, name, &rt::meta::Param::name);
    const bool known = param != params.end() || (spec->at && name == "at") || (spec->out && name == "out");
    if (!known) return Usage{"unknown flag " + std::string{arg}};
    // A flag takes the next argument, whatever it is.
    if (i + 1 == args.size()) return Usage{std::string{arg} + " needs a value"};
    if (std::ranges::find(seen, name) != seen.end()) return Usage{std::string{arg} + " given twice"};
    seen.emplace_back(name);
    const std::string_view value = args[++i];
    if (param != params.end()) {
      const rt::meta::Type& type = (*param->type)();
      auto parsed = lib::cli::parse_value(type, value);
      if (!parsed) return Usage{std::string{arg} + ": not a " + type.shown()};
      start[static_cast<std::size_t>(param - params.begin())] = std::move(parsed);
    } else if (name == "at") {
      invocation.at = parse_tick(value);
      if (!invocation.at) return Usage{std::string{arg} + ": not a tick"};
    } else {
      if (value.empty()) return Usage{std::string{arg} + ": not a path"};
      invocation.out = std::string{value};
    }
  }
  for (std::size_t i = 0; i < params.size(); ++i) {
    if (!start[i]) start[i] = start_default(params[i]);
    if (!start[i]) return Usage{"missing --" + params[i].name};
    invocation.start.push_back(std::move(*start[i]));
  }
  return invocation;
}

}  // namespace lib::game
