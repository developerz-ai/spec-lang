// Projected by hand from the exit codes of the cli vocabulary (docs/cli-vocabulary.md; docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

namespace todo::cli {

enum class ExitCode : int { Ok = 0, Refused = 1, Usage = 2, Store = 3 };

}  // namespace todo::cli
