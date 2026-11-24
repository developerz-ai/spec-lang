// Projected by hand from @store (src/apps/cpp.slg) over state tasks, state next_no (src/todo/board.slg): docs/contract.md section 3. Stand-in for spec-lang build output; change the spec, not this file.
#include <string>
#include <string_view>

#include "harness.hpp"
#include "support.hpp"
#include "todo/store/codec.hpp"

using namespace todo::model;
using support::no;
using support::tags;
using support::task;
using todo::board::World;

namespace {

constexpr std::string_view contract_example =
    R"({"version":1,"next_no":3,"tasks":[{"no":1,"title":"milk","status":"Open","priority":"Normal","tags":[]},{"no":2,"title":"bread \"fresh\"","status":"Done","priority":"High","tags":["home","shop"]}]})"
    "\n";

World example_world() {
  World w;
  w.tasks.emplace(no(1), task(1, "milk"));
  w.tasks.emplace(no(2), task(2, "bread \"fresh\"", Status::Done, Priority::High, tags({"shop", "home"})));
  w.next_no = no(3);
  return w;
}

std::string task_json(std::string_view fields) {
  return R"({"version":1,"next_no":2,"tasks":[{)" + std::string(fields) + "}]}";
}

}  // namespace

TEST(canonical_bytes_match_the_contract) {
  CHECK_EQ(todo::store::encode(example_world()).value(), contract_example);
}

TEST(initial_world_bytes) {
  CHECK_EQ(todo::store::encode(World{}).value(), "{\"version\":1,\"next_no\":1,\"tasks\":[]}\n");
}

TEST(round_trip) {
  const World w = example_world();
  CHECK(todo::store::decode(todo::store::encode(w).value()) == w);
  const auto counts = todo::store::count_statuses(contract_example);
  CHECK(counts.has_value());
  CHECK_EQ(counts->open, 1U);
  CHECK_EQ(counts->done, 1U);
}

TEST(non_canonical_reads_and_writes_canonical) {
  const std::string_view loose =
      R"( { "tasks" : [ {"tags":["b","a"],"priority":"High","status":"Done","title":"café 😀","no":2}, {"no":1,"title":"x","status":"Open","priority":"Low","tags":[]} ], "next_no" : 7, "version" : 1 } )";
  const auto w = todo::store::decode(loose);
  CHECK(w.has_value());
  CHECK_EQ(
      todo::store::encode(*w).value(),
      "{\"version\":1,\"next_no\":7,\"tasks\":[{\"no\":1,\"title\":\"x\",\"status\":\"Open\",\"priority\":"
      "\"Low\",\"tags\":[]},{\"no\":2,\"title\":\"café \U0001F600\",\"status\":\"Done\",\"priority\":"
      "\"High\",\"tags\":[\"a\",\"b\"]}]}\n");
}

TEST(escapes_decode) {
  const auto w = todo::store::decode(
      task_json(R"("no":1,"title":"a\/b\\c\"d‧","status":"Open","priority":"Normal","tags":["a"])"));
  CHECK(w.has_value());
  CHECK_EQ(w->tasks.at(no(1)).title.text(), "a/b\\c\"d‧");
  CHECK(w->tasks.at(no(1)).tags == tags({"a"}));
}

TEST(raw_c1_and_punctuation_are_stored_raw) {
  World w;
  w.tasks.emplace(no(1), task(1, "a\u2027b\u0084c"));
  w.next_no = no(2);
  CHECK_EQ(
      todo::store::encode(w).value(),
      "{\"version\":1,\"next_no\":2,\"tasks\":[{\"no\":1,\"title\":\"a\u2027b\u0084c\",\"status\":\"Open\","
      "\"priority\":\"Normal\",\"tags\":[]}]}\n");
}

TEST(two_to_the_53_is_the_last_next_no) {
  CHECK(todo::store::decode(R"({"version":1,"next_no":9007199254740992,"tasks":[]})").has_value());
  CHECK(!todo::store::decode(R"({"version":1,"next_no":9007199254740993,"tasks":[]})").has_value());
  CHECK(!todo::store::decode(R"({"version":1,"next_no":99999999999999999999999,"tasks":[]})").has_value());
  World w;
  w.next_no = no((std::uint64_t{1} << 53U) + 1);
  CHECK(todo::store::encode(w).error() == todo::store::StoreError::Full);
}

TEST(each_unreadable_shape) {
  const std::string_view bad[] = {
      "",
      "\n",
      "{}",
      "[1]",
      "todo",
      "null",
      R"({"version":2,"next_no":1,"tasks":[]})",
      R"({"version":1,"next_no":0,"tasks":[]})",
      R"({"version":1,"next_no":-1,"tasks":[]})",
      R"({"version":1,"next_no":-0,"tasks":[]})",
      R"({"version":1,"next_no":1.0,"tasks":[]})",
      R"({"version":1,"next_no":1e0,"tasks":[]})",
      R"({"version":1,"next_no":01,"tasks":[]})",
      R"({"version":1,"next_no":"1","tasks":[]})",
      R"({"version":1,"next_no":1,"tasks":{}})",
      R"({"version":1,"next_no":1,"tasks":[],"x":0})",
      R"({"version":1,"next_no":1})",
      R"({"version":1,"next_no":1,"next_no":1,"tasks":[]})",
      R"({"version":1,"next_no":1,"tasks":[]} x)",
      R"({"version":1,"next_no":1,"tasks":[],})",
      R"({"version":1,"next_no":1,"tasks":[]})"
      "\xef\xbb\xbf",
      "\xef\xbb\xbf{\"version\":1,\"next_no\":1,\"tasks\":[]}",
      R"({"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]},]})",
      R"({"version":1,"next_no":1,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}]})",
      R"({"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]},{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}]})",
  };
  for (const std::string_view b : bad) {
    if (todo::store::decode(b).has_value() || todo::store::count_statuses(b).has_value()) {
      harness::fail("read as a store: " + harness::show(b));
    }
  }
  const std::string_view bad_task[] = {
      R"("no":1,"no":1,"title":"a","status":"Open","priority":"Normal","tags":[])",
      R"("no":0,"title":"a","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":" ","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":"\t","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":"\u0000","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":"a","status":"open","priority":"Normal","tags":[])",
      R"("no":1,"title":"a","status":"Open","priority":"normal","tags":[])",
      R"("no":1,"title":"a","status":"Open","priority":"Normal","tags":["Home"])",
      R"("no":1,"title":"a","status":"Open","priority":"Normal","tags":["-x"])",
      "\"no\":1,\"title\":\"a\u2028b\",\"status\":\"Open\",\"priority\":\"Normal\",\"tags\":[]",
      R"("no":1,"title":"a\u0085b","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":"a","status":"Open","priority":"Normal","tags":["a","a"])",
      R"("no":1,"title":"a","status":"Open","priority":"Normal","tags":[1])",
      R"("no":1,"title":"a","status":"Open","priority":"Normal","tags":[],"due":null)",
      R"("no":1,"title":"a","status":"Open","priority":"Normal")",
      R"("no":"1","title":"a","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":"\ud800","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":"\udc00","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":"\ud800A","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":"\x","status":"Open","priority":"Normal","tags":[])",
      R"("no":1,"title":"\u12","status":"Open","priority":"Normal","tags":[])",
      "\"no\":1,\"title\":\"a\nb\",\"status\":\"Open\",\"priority\":\"Normal\",\"tags\":[]",
      "\"no\":1,\"title\":\"a\xff\",\"status\":\"Open\",\"priority\":\"Normal\",\"tags\":[]",
      "\"no\":1,\"title\":\"\xed\xa0\x80\",\"status\":\"Open\",\"priority\":\"Normal\",\"tags\":[]",
  };
  for (const std::string_view b : bad_task) {
    const std::string doc = task_json(b);
    if (todo::store::decode(doc).has_value() || todo::store::count_statuses(doc).has_value()) {
      harness::fail("read as a store: " + harness::show(doc));
    }
  }
}
