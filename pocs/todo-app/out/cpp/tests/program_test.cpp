// Projected by hand from app TodoCpp (src/apps/cpp.slg): one run end to end over a store in a temporary directory. Stand-in for spec-lang build output; change the spec, not this file.
#include "todo/app/program.hpp"

#include <filesystem>
#include <fstream>
#include <iterator>
#include <string>
#include <string_view>
#include <vector>

#include "harness.hpp"
#include "temp_dir.hpp"

namespace {

todo::app::RunResult run(const std::string& store, std::vector<std::string_view> args) {
  return todo::app::run(args, store);
}

std::string slurp(const std::string& path) {
  std::ifstream in(path, std::ios::binary);
  return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
}

}  // namespace

TEST(reads_never_create_the_store_but_create_the_lock) {
  const support::TempDir dir;
  const std::string store = dir.file(".todo.json");
  const auto r = run(store, {"list"});
  CHECK_EQ(r.out, "nothing to do\n");
  CHECK_EQ(run(store, {"stats"}).out, "0 open, 0 done\n");
  CHECK(!std::filesystem::exists(store));
  CHECK(std::filesystem::exists(store + ".lock"));
}

TEST(a_write_commits_then_prints) {
  const support::TempDir dir;
  const std::string store = dir.file(".todo.json");
  const auto r = run(store, {"add", "milk"});
  CHECK_EQ(r.code, 0);
  CHECK_EQ(r.out, "added 1\n");
  CHECK_EQ(slurp(store),
           "{\"version\":1,\"next_no\":2,\"tasks\":[{\"no\":1,\"title\":\"milk\",\"status\":\"Open\","
           "\"priority\":\"Normal\",\"tags\":[]}]}\n");
}

TEST(refusal_and_store_errors) {
  const support::TempDir dir;
  const std::string store = dir.file(".todo.json");
  const auto refused = run(store, {"done", "1"});
  CHECK_EQ(refused.code, 1);
  CHECK_EQ(refused.err, "error: NoTask\n");
  CHECK_EQ(refused.out, "");
  {
    std::ofstream(store) << "{}\n";
  }
  const auto bad = run(store, {"stats"});
  CHECK_EQ(bad.code, 3);
  CHECK_EQ(bad.err, "error: store " + store + ": unreadable\n");
  const auto lost = run(dir.file("no/such.json"), {"list"});
  CHECK_EQ(lost.code, 3);
  CHECK_EQ(lost.err, "error: store " + dir.file("no/such.json") + ": unwritable\n");
}

TEST(full_at_two_to_the_53) {
  const support::TempDir dir;
  const std::string store = dir.file(".todo.json");
  {
    std::ofstream(store) << "{\"version\":1,\"next_no\":9007199254740992,\"tasks\":[]}\n";
  }
  const auto r = run(store, {"add", "b"});
  CHECK_EQ(r.code, 3);
  CHECK_EQ(r.err, "error: store " + store + ": full\n");
}

TEST(an_unchanged_world_is_not_rewritten) {
  const support::TempDir dir;
  const std::string store = dir.file(".todo.json");
  CHECK_EQ(run(store, {"clear"}).out, "cleared 0\n");
  CHECK(std::filesystem::exists(store));  // a writing rule creates a missing store
  const auto before = std::filesystem::last_write_time(store);
  const auto inode_before = std::filesystem::hard_link_count(store);
  CHECK_EQ(run(store, {"clear"}).out, "cleared 0\n");
  CHECK(std::filesystem::last_write_time(store) == before);
  CHECK_EQ(std::filesystem::hard_link_count(store), inode_before);
}

TEST(usage_and_help) {
  const support::TempDir dir;
  const std::string store = dir.file(".todo.json");
  const auto none = run(store, {});
  CHECK_EQ(none.code, 2);
  CHECK(none.err.starts_with("usage: todo <command> [args]\n"));
  const auto help = run(store, {"add", "-h"});
  CHECK_EQ(help.code, 0);
  CHECK(help.out.starts_with("usage: todo"));
  const auto usage = run(store, {"add"});
  CHECK_EQ(usage.err, "error: missing <title>\n");
  CHECK(!std::filesystem::exists(store + ".lock"));  // a usage error never reaches the store
}
