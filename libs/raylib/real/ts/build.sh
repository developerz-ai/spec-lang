#!/usr/bin/env bash
# The `raylib` library's runtime for TypeScript (libs/raylib/real/ts): compiles the by-value marshalling shim
# to build/libraylib-shim.so, which the app embeds. It needs raylib.h for the struct types only and links no raylib.
# raylib.h is looked for in $RAYLIB_INCLUDE, then ../../vendor/raylib/include, then the system.
set -euo pipefail
cd "$(dirname "$0")/../../.."
include="${RAYLIB_INCLUDE:-}"
for d in "$include" ../../vendor/raylib/include /usr/local/include /usr/include; do
  if [ -n "$d" ] && [ -f "$d/raylib.h" ]; then include="$d"; break; fi
done
[ -f "$include/raylib.h" ] || { echo "raylib.h not found: set RAYLIB_INCLUDE" >&2; exit 1; }
mkdir -p build
"${CC:-cc}" -std=c11 -O2 -fPIC -shared -Wall -Wextra -Werror -I"$include" src/lib/raylib/shim.c -o build/libraylib-shim.so
