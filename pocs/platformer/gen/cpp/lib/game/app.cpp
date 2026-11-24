// The `game` library's runtime for C++ (libs/game/real/cpp): the four commands, over the app's six fields:
// `start` (a rule), `tick` (the rule with `@on(tick(rate))`), `input` (a fn that reads the controls), `frame`
// (a rule that draws), `dump` (a fn: the frame as text), `report` (a fn: one line after a run).
#include <unistd.h>

#include <cstdio>
#include <cstdlib>

#include "lib/cli/out.hpp"
#include "lib/game/args.hpp"
#include "lib/game/main.hpp"
#include "lib/game/screen.hpp"
#include "lib/game/script.hpp"
#include "program/machine.hpp"

namespace lib::game {

namespace {

constexpr int kUsage = 2;
constexpr int kScreen = 3;
constexpr int kFailed = 1;
// The size of the frame `shot` draws.
constexpr int kShotWidth = 320;
constexpr int kShotHeight = 180;

struct Outcome {
  int code = 0;
  std::string out;
  std::string err;
};

Outcome failure(int code, const std::string& message) {
  return {code, {}, "error: " + message + "\n"};
}

// The app's fields, each the declaration it names.
struct Game {
  const rt::meta::App& app;
  const rt::meta::Callable* start;
  const rt::meta::Callable* tick;
  const rt::meta::Callable* input;
  const rt::meta::Callable* frame;
  const rt::meta::Callable* dump;
  const rt::meta::Callable* report;

  explicit Game(const rt::meta::App& a)
      : app(a),
        start(a.named("start")),
        tick(a.named("tick")),
        input(a.named("input")),
        frame(a.named("frame")),
        dump(a.named("dump")),
        report(a.named("report")) {}

  bool complete() const {
    return start != nullptr && tick != nullptr && input != nullptr && frame != nullptr && dump != nullptr &&
           report != nullptr && tick->params.size() == 1;
  }

  // The record of controls the tick rule takes.
  const rt::meta::Type& controls() const {
    return (*tick->params.front().type)();
  }
};

Window window_of(const rt::meta::App& app, const rt::meta::Callable& tick) {
  Window window{960, 540, bin_of(app), 60};
  if (const rt::meta::Note* note = app.note("game", "window")) {
    if (const auto* width = note->arg("width")) window.width = static_cast<int>(width->number.to_long());
    if (const auto* height = note->arg("height")) window.height = static_cast<int>(height->number.to_long());
    if (const auto* title = note->arg("title")) window.title = title->text;
  }
  if (const rt::meta::Note* trigger = tick.trigger("game", "tick")) {
    if (const auto* rate = trigger->arg("rate")) window.rate = static_cast<int>(rate->number.to_long());
  }
  return window;
}

std::string read_stdin() {
  std::string input;
  char chunk[65536];
  for (;;) {
    const std::size_t n = std::fread(chunk, 1, sizeof chunk, stdin);
    input.append(chunk, n);
    // End of input, or a read error: what was read is the script, and the reader judges it.
    if (n < sizeof chunk) return input;
  }
}

bool write_stdout(std::string_view text) {
  return std::fwrite(text.data(), 1, text.size(), stdout) == text.size() && std::fflush(stdout) == 0;
}

void write_stderr(std::string_view text) {
  std::fwrite(text.data(), 1, text.size(), stderr);
}

// One tick. False: the rule refused, which is the end of the run.
bool tick_once(const Game& game, program::Machine& machine, const rt::Data& controls) {
  try {
    game.tick->call(machine, {controls});
    return true;
  } catch (const rt::Failure&) {
    return false;
  }
}

// The script's ticks in order, at most `until` of them (tick 0 is right after start), until one is refused.
void replay(const Game& game, program::Machine& machine, const Script& script, const std::optional<rt::Int>& until) {
  rt::Int done;
  for (const Hold& hold : script.holds) {
    for (std::uint32_t i = 0; i < hold.ticks; ++i) {
      if (until && done >= *until) return;
      if (!tick_once(game, machine, hold.controls)) return;
      done = done + rt::Int{1};
    }
  }
}

// A fn's value as lines: a Text is one line, a List[Text] one line each.
std::string lines_of(const rt::meta::Callable& fn, program::Machine& machine) {
  const rt::Data value = fn.call(machine, {});
  if (value.is_text()) return value.text() + "\n";
  std::string out;
  for (const rt::Data& line : value.items()) out += line.text() + "\n";
  return out;
}

program::Machine fresh_machine() {
  program::Machine machine;
  machine.out = std::make_unique<lib::cli::BufferedOut>();
  return machine;
}

std::string hex16(std::uint64_t value) {
  std::string out(16, '0');
  for (std::size_t i = 16; i-- > 0;) {
    out[i] = "0123456789abcdef"[value & 0xFU];
    value >>= 4U;
  }
  return out;
}

// FNV-1a, 64 bits.
std::uint64_t fnv1a64(const std::vector<std::uint8_t>& bytes) {
  std::uint64_t hash = 0xcbf29ce484222325ULL;
  for (const std::uint8_t byte : bytes) {
    hash ^= byte;
    hash *= 0x100000001b3ULL;
  }
  return hash;
}

// `run`: start, every tick of the script, every line, then the report.
Outcome run(const Game& game, const Invocation& invocation, std::string_view input) {
  const auto script = read_script(game.controls(), input);
  if (!script) return failure(kUsage, script.error());
  program::Machine machine = fresh_machine();
  game.start->call(machine, invocation.start);
  replay(game, machine, *script, std::nullopt);
  std::string out = lib::cli::take_lines(machine);
  out += lines_of(*game.report, machine);
  return {0, std::move(out), {}};
}

// `draw`: the same, silent, then the frame as text. It opens no window.
Outcome draw(const Game& game, const Invocation& invocation, std::string_view input) {
  const auto script = read_script(game.controls(), input);
  if (!script) return failure(kUsage, script.error());
  program::Machine machine = fresh_machine();
  game.start->call(machine, invocation.start);
  replay(game, machine, *script, invocation.at);
  return {0, lines_of(*game.dump, machine), {}};
}

// `shot`: the same, silent, then the frame drawn off screen and its pixels hashed.
Outcome shot(const Game& game, const Invocation& invocation, std::string_view input, const std::string& assets_dir) {
  const auto script = read_script(game.controls(), input);
  if (!script) return failure(kUsage, script.error());
  program::Machine machine = fresh_machine();
  game.start->call(machine, invocation.start);
  replay(game, machine, *script, invocation.at);

  Screen* const display = screen();
  if (display == nullptr) return failure(kScreen, "window: cannot open");
  Window window = window_of(game.app, *game.tick);
  window.width = kShotWidth;
  window.height = kShotHeight;
  if (const auto error = display->open(Purpose::Shot, window, game.app, assets_dir)) return failure(kScreen, *error);
  const Capture frame =
      display->capture(kShotWidth, kShotHeight, [&] { game.frame->call(machine, {}); }, invocation.out);
  display->close();
  // The vocabulary names no error for a frame that cannot be read back or a picture that cannot be written;
  // both end like the other screen failures, with nothing on standard output.
  if (frame.rgba.empty()) return failure(kScreen, "window: cannot read the frame");
  if (!frame.exported) return failure(kScreen, invocation.out.value_or("") + ": cannot write");
  return {0, "shot " + hex16(fnv1a64(frame.rgba)) + "\n", {}};
}

// `play`: a window, one tick per frame at the tick rate with the controls read that frame; a refused tick
// ends the game and the last frame stays.
int play(const Game& game, const Invocation& invocation, const std::string& assets_dir) {
  Screen* const display = screen();
  if (display == nullptr) {
    write_stderr("error: window: cannot open\n");
    return kScreen;
  }
  if (const auto error = display->open(Purpose::Play, window_of(game.app, *game.tick), game.app, assets_dir)) {
    write_stderr("error: " + *error + "\n");
    return kScreen;
  }
  program::Machine machine = fresh_machine();
  game.start->call(machine, invocation.start);
  bool written = write_stdout(lib::cli::take_lines(machine));
  bool over = false;
  while (!display->should_close()) {
    if (!over) {
      over = !tick_once(game, machine, game.input->call(machine, {}));
      if (const std::string lines = lib::cli::take_lines(machine); !lines.empty()) {
        written = write_stdout(lines) && written;
      }
    }
    display->draw([&] { game.frame->call(machine, {}); });
  }
  written = write_stdout(lines_of(*game.report, machine)) && written;
  display->close();
  if (!written) {
    write_stderr("error: stdout: unwritable\n");
    return kFailed;
  }
  return 0;
}

int finish(const Outcome& outcome) {
  write_stderr(outcome.err);
  if (!outcome.out.empty() && !write_stdout(outcome.out)) {
    write_stderr("error: stdout: unwritable\n");
    return kFailed;
  }
  return outcome.code;
}

// `<BIN>_ASSETS` names the folder of assets; else `assets` in the working directory.
std::string assets_dir(const rt::meta::App& app) {
  std::string variable = bin_of(app) + "_ASSETS";
  for (char& c : variable) c = static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
  const char* configured = std::getenv(variable.c_str());
  return configured != nullptr && *configured != '\0' ? configured : "assets";
}

}  // namespace

namespace {
Screen* installed = nullptr;
}  // namespace

Screen* screen() {
  return installed;
}

void install_screen(Screen* screen) {
  installed = screen;
}

int main(const rt::meta::App& app, int argc, char** argv) {
  const std::vector<std::string_view> args(argv + std::min(argc, 1), argv + argc);
  const Parsed parsed = parse(app, args);
  if (const auto* help = std::get_if<Help>(&parsed)) {
    return finish(help->to_stdout ? Outcome{0, help_text(app), {}} : Outcome{kUsage, {}, help_text(app)});
  }
  if (const auto* usage = std::get_if<Usage>(&parsed)) return finish(failure(kUsage, usage->message));
  const Invocation& invocation = std::get<Invocation>(parsed);
  const Game game{app};
  if (!game.complete()) return finish(failure(kUsage, "app " + app.name + " does not name the six fields of a game"));
  try {
    switch (invocation.command) {
      case Command::Play:
        return play(game, invocation, assets_dir(app));
      case Command::Run:
        return finish(run(game, invocation, read_stdin()));
      case Command::Draw:
        return finish(draw(game, invocation, read_stdin()));
      case Command::Shot:
        return finish(shot(game, invocation, read_stdin(), assets_dir(app)));
    }
  } catch (const rt::Failure& refused) {
    return finish(failure(kFailed, refused.name));
  } catch (const rt::Stop& stopped) {
    return finish(failure(kFailed, stopped.code + ": " + stopped.message));
  }
  return kUsage;
}

}  // namespace lib::game
