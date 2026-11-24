// The C++ printer's runtime, the same in every project `spec-lang build` prints. Do not edit.
// Why evaluation did not give a value: a stop (never handled), a blocked check, a false test step.
// A language failure (`fail E`, Missing, DivByZero, Uneven) is rt::Failure, in rt/value.hpp.
#pragma once

#include <string>

namespace rt {

// Evaluation stopped with a diagnostic code (E-RUNTIME, E-CLAIM, E-TYPE-MISMATCH, ...): never handled.
struct Stop {
  std::string code;
  std::string message;
};

// A `?hole` (F-BLOCKED) or a foreign body in a test build (B-FOREIGN).
struct Blocked {
  std::string code;
  std::string message;
};

// A test step or an assert that did not hold (F-TEST).
struct CheckFailed {
  std::string message;
};

}  // namespace rt
