// What can go wrong between a rule and the disk; none of these is a spec error.
#pragma once

#include <cstdint>

namespace rt {

enum class StoreError : std::uint8_t { Unreadable, Unwritable, Full, Contended };

}  // namespace rt
