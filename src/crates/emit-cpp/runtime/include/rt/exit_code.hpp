// Exit codes of the cli vocabulary (docs/contract.md section 2).
#pragma once

namespace rt {

enum class ExitCode : int { Ok = 0, Refused = 1, Usage = 2, Store = 3 };

}  // namespace rt
