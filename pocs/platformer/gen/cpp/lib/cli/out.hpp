// The `cli` library's runtime for C++ (libs/cli/real/cpp): the `out` handler of an app. A rule's lines wait
// here until the rule commits; a refused rule takes its lines back with its writes, because the machine
// restores the handler it copied before the rule.
#pragma once

#include <memory>
#include <string>

#include "program/machine.hpp"

namespace lib::cli {

class BufferedOut final : public program::Out {
 public:
  void line(const rt::Text& text) override;
  // Read by tests only; an app has none.
  rt::Text text() override;
  std::unique_ptr<program::Out> clone() const override;

  // The lines sent since the last take, each ending in a newline; the handler keeps none.
  std::string take();

 private:
  std::string sent_;
};

// The lines the machine's `out` holds, taken; empty when another handler is installed.
std::string take_lines(program::Machine& machine);

}  // namespace lib::cli
