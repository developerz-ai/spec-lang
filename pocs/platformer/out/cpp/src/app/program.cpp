// Projected by hand from app SkyhopCpp(start, tick, input, frame, dump, report), @bin("skyhop") (src/apps/cpp.slg): help, usage errors, dispatch and exit codes (docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.
#include "game/app/program.hpp"

#include <type_traits>
#include <variant>

#include "game/app/io.hpp"
#include "game/cli/exit_code.hpp"
#include "game/cli/help.hpp"

namespace game::app {
namespace {

int finish(const Outcome& outcome) {
  write_stderr(outcome.err);
  if (!outcome.out.empty() && !write_stdout(outcome.out)) {
    write_stderr("error: stdout: unwritable\n");
    return cli::exit_output;
  }
  return outcome.code;
}

}  // namespace

int main_program(const cli::Parsed& parsed, std::string (*input)(), const std::string& assets_dir) {
  if (std::holds_alternative<cli::NoArguments>(parsed)) {
    return finish({cli::exit_usage, "", std::string(cli::help)});
  }
  if (std::holds_alternative<cli::HelpAsked>(parsed)) {
    return finish({cli::exit_ok, std::string(cli::help), ""});
  }
  if (const auto* usage = std::get_if<cli::UsageError>(&parsed)) {
    return finish({cli::exit_usage, "", usage->message + "\n"});
  }
  const cli::Invocation& invocation = std::get<cli::Invocation>(parsed);
  switch (invocation.command) {
    case cli::Command::Play:
      return play(invocation.level, assets_dir);
    case cli::Command::Run:
      return finish(run(invocation.level, input()));
    case cli::Command::Draw:
      return finish(draw(invocation.level, invocation.at, input()));
    case cli::Command::Shot:
      return finish(shot(invocation.level, invocation.at, invocation.out, input(), assets_dir));
  }
  return cli::exit_usage;
}

}  // namespace game::app
