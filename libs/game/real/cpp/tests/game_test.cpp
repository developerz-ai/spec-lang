// The `game` library's runtime for C++ (libs/game/real/cpp): its own tests, over the manifest of the app it
// was built into: the command line and the input script, without a window.
#include <iostream>

#include "lib/game/args.hpp"
#include "lib/game/script.hpp"
#include "program/manifest.hpp"

namespace {

int failures = 0;

void expect(bool holds, const char* what) {
  if (!holds) {
    ++failures;
    std::cerr << "FAIL " << what << "\n";
  }
}

std::string usage(const std::vector<std::string_view>& args) {
  const auto parsed = lib::game::parse(program::app(), args);
  const auto* usage = std::get_if<lib::game::Usage>(&parsed);
  return usage == nullptr ? std::string{} : usage->message;
}

void the_command_line() {
  const auto& app = program::app();
  const auto none = lib::game::parse(app, {});
  expect(std::holds_alternative<lib::game::Help>(none) && !std::get<lib::game::Help>(none).to_stdout,
         "no arguments: help on standard error");
  const auto help = lib::game::parse(app, {"run", "-h"});
  expect(std::holds_alternative<lib::game::Help>(help) && std::get<lib::game::Help>(help).to_stdout,
         "-h anywhere: help on standard output");
  expect(usage({"fly"}) == "unknown command \"fly\"", "an unknown command");
  expect(usage({"run", "x"}) == "unexpected argument \"x\"", "an argument that is no flag");
  expect(usage({"run", "--at", "3"}) == "unknown flag --at", "--at is not a flag of run");
  expect(usage({"draw", "--at"}) == "--at needs a value", "a flag with nothing after it");
  expect(usage({"draw", "--at", "1", "--at", "2"}) == "--at given twice", "a flag twice");
  expect(usage({"draw", "--at", "1", "--at"}) == "--at needs a value", "needs a value comes before given twice");
  expect(usage({"draw", "--at", "x"}) == "--at: not a tick", "a tick is digits");
  expect(usage({"shot", "--out", ""}) == "--out: not a path", "an empty path");
  const auto shot = lib::game::parse(app, {"shot", "--at", "007", "--out", "a.png"});
  const auto* invocation = std::get_if<lib::game::Invocation>(&shot);
  expect(invocation != nullptr && invocation->command == lib::game::Command::Shot && invocation->at == rt::Int{7} &&
             invocation->out == "a.png",
         "shot with its flags");
  expect(lib::game::help_text(app).starts_with("usage: " + lib::game::bin_of(app) + " <command> [options]\n"),
         "the help names the executable");
}

void the_input_script() {
  const rt::meta::Callable* tick = program::app().named("tick");
  expect(tick != nullptr && tick->params.size() == 1, "the app names its tick rule");
  if (tick == nullptr || tick->params.size() != 1) return;
  const rt::meta::Type& controls = (*tick->params.front().type)();
  const auto error = [&](std::string_view input) {
    const auto script = lib::game::read_script(controls, input);
    return script ? std::string{} : script.error();
  };
  const auto empty = lib::game::read_script(controls, "# nothing\n\n   \n");
  expect(empty && empty->holds.empty(), "comments and blank lines are skipped");
  const auto idle = lib::game::read_script(controls, "3 -\n2 -");
  expect(idle && idle->holds.size() == 2 && idle->holds[0].ticks == 3 && idle->holds[1].ticks == 2,
         "a final line without a newline counts");
  expect(error("5\n") == "input line 1: expected <ticks> <keys>", "one field");
  expect(error("\n0 -\n") == "input line 2: ticks must be 1 to 100000", "line numbers count skipped lines");
  expect(error("100001 -\n") == "input line 1: ticks must be 1 to 100000", "more ticks than a line may hold");
  expect(error("1 -?\n") == "input line 1: unknown key \"-\"", "- mixed with other characters");
  expect(error("1 \xFF\n") == "input line 1: unknown key \"\xEF\xBF\xBD\"", "bytes that are not UTF-8");
  std::string long_script;
  for (int i = 0; i < 11; ++i) long_script += "100000 -\n";
  expect(error(long_script) == "input: more than 1000000 ticks", "more ticks than a script may hold");
  if (!controls.fields.empty()) {
    const char letter = static_cast<char>(std::toupper(static_cast<unsigned char>(controls.fields.front().name[0])));
    const auto held = lib::game::read_script(controls, std::string{"1 "} + letter + "\n");
    expect(held && held->holds.size() == 1 && held->holds[0].controls.values().front().boolean(),
           "a key is its field's first letter");
    expect(
        error(std::string{"1 "} + letter + letter + "\n") == std::string{"input line 1: key \""} + letter + "\" twice",
        "a key twice");
  }
}

}  // namespace

int main() {
  the_command_line();
  the_input_script();
  if (failures == 0) std::cout << "lib/game: every test passed\n";
  return failures == 0 ? 0 : 1;
}
