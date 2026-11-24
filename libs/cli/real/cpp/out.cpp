// The `cli` library's runtime for C++ (libs/cli/real/cpp): the `out` handler of an app.
#include "lib/cli/out.hpp"

#include <utility>

namespace lib::cli {

void BufferedOut::line(const rt::Text& text) {
  sent_ += text;
  sent_ += '\n';
}

rt::Text BufferedOut::text() {
  rt::stop("E-UNSUPPORTED", "`out.text` is read by tests only");
}

std::unique_ptr<program::Out> BufferedOut::clone() const {
  return std::make_unique<BufferedOut>(*this);
}

std::string BufferedOut::take() {
  return std::exchange(sent_, {});
}

std::string take_lines(program::Machine& machine) {
  auto* out = dynamic_cast<BufferedOut*>(machine.out.get());
  return out == nullptr ? std::string{} : out->take();
}

}  // namespace lib::cli
