// Projected by hand from the shot command (docs/contract.md section 2) and fn read_pad, rule present (src/hop/screen.slg), through the real raylib in a hidden window. Stand-in for spec-lang build output; change the spec, not this file.
#include "hop/screen/screen.hpp"

#include <sys/wait.h>
#include <unistd.h>

#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <string>
#include <vector>

#include "game/app/program.hpp"
#include "harness.hpp"
#include "hop/screen/session.hpp"

using game::app::Outcome;
using hop::levels::first_level;

namespace {

constexpr std::string_view meadow_win = "75 R\n20 RJ\n41 R\n20 RJ\n35 R\n20 RJ\n23 R\n20 RJ\n62 R\n";

std::string assets_dir() {
  const char* const env = std::getenv("SKYHOP_ASSETS");
  return env != nullptr && *env != '\0' ? env : SKYHOP_TEST_ASSETS;
}

// These tests open a window; without an X server there is nothing to open it on.
void need_a_display() {
  if (!hop::screen::has_display()) {
    harness::skip("no X server (neither DISPLAY nor WAYLAND_DISPLAY is set); run under xvfb-run -a");
  }
}

std::string read_file(const std::filesystem::path& path) {
  std::ifstream file(path, std::ios::binary);
  return {std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>()};
}

// A directory of its own for one test's files, removed when the test ends.
struct Scratch {
  std::filesystem::path dir;
  Scratch() {
    std::string name = (std::filesystem::temp_directory_path() / "skyhop-cpp-XXXXXX").string();
    if (mkdtemp(name.data()) == nullptr) {
      harness::fail("cannot make a temporary directory");
    }
    dir = name;
  }
  Scratch(const Scratch&) = delete;
  Scratch& operator=(const Scratch&) = delete;
  ~Scratch() {
    std::error_code ignored;  // a test's outcome is already decided; a leftover file is not a failure
    std::filesystem::remove_all(dir, ignored);
  }
};

// Runs the built skyhop as the parity suite does: its own process, a script on standard input.
// raylib has one window per process, and opening a second one after the first is closed crashes
// now and then in this GL stack (NOTES.md, Could not keep), so every window here is a process.
Outcome skyhop(const std::vector<std::string>& args, std::string_view input, const std::string& assets) {
  const Scratch scratch;
  const std::filesystem::path in = scratch.dir / "in";
  const std::filesystem::path out = scratch.dir / "out";
  const std::filesystem::path err = scratch.dir / "err";
  std::ofstream(in, std::ios::binary) << input;

  std::fflush(nullptr);  // the child must not inherit unwritten test output
  const pid_t child = fork();
  if (child < 0) {
    harness::fail("fork failed");
  }
  if (child == 0) {
    if (std::freopen(in.c_str(), "rb", stdin) == nullptr ||
        std::freopen(out.c_str(), "wb", stdout) == nullptr ||
        std::freopen(err.c_str(), "wb", stderr) == nullptr) {
      _exit(126);
    }
    setenv("SKYHOP_ASSETS", assets.c_str(), 1);
    std::vector<char*> argv{const_cast<char*>(SKYHOP_TEST_EXE)};
    for (const std::string& a : args) {
      argv.push_back(const_cast<char*>(a.c_str()));
    }
    argv.push_back(nullptr);
    execv(SKYHOP_TEST_EXE, argv.data());
    _exit(127);
  }
  int status = 0;
  if (waitpid(child, &status, 0) != child) {
    harness::fail("waitpid failed");
  }
  if (!WIFEXITED(status)) {
    harness::fail("skyhop was killed by signal " + std::to_string(WTERMSIG(status)));
  }
  return {WEXITSTATUS(status), read_file(out), read_file(err)};
}

Outcome shot(std::vector<std::string> flags, std::string_view input) {
  flags.insert(flags.begin(), "shot");
  return skyhop(flags, input, assets_dir());
}

bool is_shot_line(std::string_view out) {
  if (out.size() != 22 || !out.starts_with("shot ") || out.back() != '\n') {
    return false;
  }
  return out.substr(5, 16).find_first_not_of("0123456789abcdef") == std::string_view::npos;
}

}  // namespace

TEST(a_shot_prints_a_hash_and_is_the_same_twice) {
  need_a_display();
  const Outcome first = shot({}, "");
  CHECK_EQ(first.err, "");
  CHECK_EQ(first.code, 0);
  CHECK(is_shot_line(first.out));
  CHECK(first == shot({}, ""));
}

TEST(a_shot_follows_the_world) {
  need_a_display();
  const Outcome start = shot({"--at", "0"}, meadow_win);
  const Outcome mid = shot({"--at", "150"}, meadow_win);
  const Outcome won = shot({}, meadow_win);
  const Outcome ridge = shot({"--level", "2"}, "");
  CHECK(is_shot_line(start.out) && is_shot_line(mid.out) && is_shot_line(won.out) && is_shot_line(ridge.out));
  CHECK(start == shot({}, ""));
  CHECK(start.out != mid.out);
  CHECK(mid.out != won.out);
  CHECK(start.out != ridge.out);
}

TEST(a_shot_writes_the_png_it_hashed) {
  need_a_display();
  const Scratch scratch;
  const std::string png = (scratch.dir / "frame.png").string();
  const Outcome plain = shot({"--at", "150"}, meadow_win);
  const Outcome saved = shot({"--at", "150", "--out", png}, meadow_win);
  CHECK(is_shot_line(plain.out));
  CHECK(saved == plain);
  const std::string bytes = read_file(png);
  CHECK(bytes.starts_with("\x89PNG\r\n\x1a\n"));
  // The IHDR chunk: 320 by 180.
  CHECK(bytes.size() > 1000);
  CHECK_EQ(bytes.substr(16, 8), std::string("\x00\x00\x01\x40\x00\x00\x00\xb4", 8));
}

TEST(a_png_that_cannot_be_written_is_an_error) {
  need_a_display();
  const std::string nowhere = "/nonexistent-skyhop-dir/shot.png";
  CHECK(shot({"--out", nowhere}, "") == (Outcome{3, "", "error: " + nowhere + ": cannot write\n"}));
}

TEST(a_missing_asset_is_exit_3) {
  need_a_display();
  const Outcome o = skyhop({"shot"}, "", "/nonexistent-skyhop-assets");
  CHECK_EQ(o.code, 3);
  CHECK_EQ(o.out, "");
  CHECK_EQ(o.err, "error: asset /nonexistent-skyhop-assets/out/pip.glb: cannot load\n");
}

TEST(a_bad_script_is_refused_before_any_window_opens) {
  // In this process, with no display needed: nothing is opened.
  CHECK(game::app::shot(first_level, std::nullopt, std::nullopt, "1 Z\n", "/nonexistent-skyhop-assets") ==
        (Outcome{2, "", "error: input line 1: unknown key \"Z\"\n"}));
}

// Contract section 2, "No display": refused before InitWindow, which would crash.
TEST(no_display_is_exit_3) {
  const char* const x11 = std::getenv("DISPLAY");
  const char* const wayland = std::getenv("WAYLAND_DISPLAY");
  const std::string x11_was = x11 != nullptr ? x11 : "";
  const std::string wayland_was = wayland != nullptr ? wayland : "";
  unsetenv("DISPLAY");
  setenv("WAYLAND_DISPLAY", "", 1);
  const Outcome o = game::app::shot(first_level, std::nullopt, std::nullopt, "", assets_dir());
  if (x11 != nullptr) {
    setenv("DISPLAY", x11_was.c_str(), 1);
  }
  if (wayland != nullptr) {
    setenv("WAYLAND_DISPLAY", wayland_was.c_str(), 1);
  } else {
    unsetenv("WAYLAND_DISPLAY");
  }
  CHECK(o == (Outcome{3, "", "error: window: cannot open\n"}));
}

// The one window this process opens.
TEST(no_key_is_down_in_a_fresh_window) {
  need_a_display();
  auto session = hop::screen::Session::open(hop::screen::Purpose::Shot, game::app::shot_window, assets_dir());
  CHECK(session.has_value());
  CHECK(hop::screen::read_pad() == hop::model::Pad{});
}
