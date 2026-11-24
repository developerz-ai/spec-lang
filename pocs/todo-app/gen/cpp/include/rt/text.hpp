// The C++ printer's runtime, the same in every project `spec-lang build` prints. Do not edit.
// Text by code point: sizes, take and drop count code points; trim removes the 25 White_Space code points;
// nothing is normalised.
#pragma once

#include <string_view>

#include "rt/value.hpp"

namespace rt {

Int size(const Text& text);
List<Int> bytes(const Text& text);
Text trim(const Text& text);
// Full Unicode lowercase mapping, no locale (the final sigma included).
Text lower(const Text& text);
bool starts_with(const Text& text, const Text& prefix);
bool contains(const Text& text, const Text& part);
// Pieces between separators, empty ones kept; an empty separator stops E-TYPE-ARITY.
List<Text> split(const Text& text, const Text& separator);
// Every non-overlapping match, left to right; an empty `old` stops E-TYPE-ARITY.
Text replace(const Text& text, const Text& old, const Text& with);
Text take(const Text& text, Int count);
Text drop(const Text& text, Int count);
Text join(const List<Text>& parts, const Text& separator);
// The parts in order, as one Text.
Text concat(std::initializer_list<Text> parts);

namespace unicode {
// One code point decoded from well-formed UTF-8 at `at`, which moves past it.
char32_t next(std::string_view text, std::size_t& at);
void append(Text& out, char32_t code_point);
bool is_white_space(char32_t c);
bool is_control(char32_t c);
}  // namespace unicode

}  // namespace rt
