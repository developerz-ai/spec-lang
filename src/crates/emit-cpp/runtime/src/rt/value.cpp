// Int beyond its operators, `show` of the built-in values, and the ways evaluation ends without a value.
#include "rt/value.hpp"

#include <algorithm>

#include "rt/collections.hpp"
#include "rt/text.hpp"

namespace rt {

namespace {
[[noreturn]] void overflow() {
  throw Stop{"E-RUNTIME", "an Int beyond the 128-bit range of this runtime (the language's Int is unbounded)"};
}
void both_natural(Int a, Int b, const char* op) {
  if (a < Int{0} || b < Int{0}) stop("E-TYPE-MISMATCH", Text{"`"} + op + "` does not apply to a negative Int");
}
std::vector<RuleName>& rule_names() {
  static std::vector<RuleName> names;
  return names;
}
}  // namespace

Int mod(Int a, Int b) {
  if (b == Int{0}) fail("DivByZero");
  if (b == Int{-1}) {
    if (a == Int::min()) overflow();
    return Int{0};
  }
  int128 m = a.value() % b.value();
  if (m != 0 && ((m < 0) != (b.value() < 0))) m += b.value();
  return Int::wide(m);
}

Int pow(Int base, Int exponent) {
  if (exponent < Int{0}) stop("E-TYPE-MISMATCH", "`int.pow` does not apply to a negative exponent");
  if (exponent > Int{4294967295LL}) overflow();
  auto left = static_cast<unsigned long long>(exponent.to_long());
  if (left == 0) return Int{1};
  Int result{1};
  while (left > 1) {
    if ((left & 1U) == 1U) result = result * base;
    left /= 2;
    base = base * base;
  }
  return result * base;
}

Int div(Int, Int) { stop("E-OPEN", "`/` on Int is an open choice: say how to round"); }

Int band(Int a, Int b) {
  both_natural(a, b, "int.band");
  return Int::wide(a.value() & b.value());
}
Int bor(Int a, Int b) {
  both_natural(a, b, "int.bor");
  return Int::wide(a.value() | b.value());
}
Int bxor(Int a, Int b) {
  both_natural(a, b, "int.bxor");
  return Int::wide(a.value() ^ b.value());
}
Int shl(Int a, Int b) {
  both_natural(a, b, "int.shl");
  if (b >= Int{128}) overflow();
  const auto by = static_cast<unsigned>(b.to_long());
  const auto shifted = static_cast<int128>(static_cast<uint128>(a.value()) << by);
  if ((shifted >> by) != a.value()) overflow();
  return Int::wide(shifted);
}
Int shr(Int a, Int b) {
  both_natural(a, b, "int.shr");
  if (b >= Int{128}) return Int{0};
  return Int::wide(a.value() >> static_cast<unsigned>(b.to_long()));
}
Int abs(Int a) {
  if (a == Int::min()) stop("E-TYPE-MISMATCH", "no `abs` of this Int");
  return a < Int{0} ? -a : a;
}
Int clamp(Int value, Int lo, Int hi) {
  if (lo > hi) stop("E-TYPE-MISMATCH", "`int.clamp` does not apply when lo is above hi");
  return value < lo ? lo : (value > hi ? hi : value);
}

std::string to_string(Int n) {
  if (n == Int{0}) return "0";
  const bool negative = n < Int{0};
  // The magnitude as unsigned, so the least Int has one too.
  uint128 left = negative ? ~static_cast<uint128>(n.value()) + 1 : static_cast<uint128>(n.value());
  std::string digits;
  while (left != 0) {
    digits.push_back(static_cast<char>('0' + static_cast<int>(left % 10)));
    left /= 10;
  }
  if (negative) digits.push_back('-');
  std::reverse(digits.begin(), digits.end());
  return digits;
}

Text show(Unit) { return "()"; }
Text show(bool value) { return value ? "true" : "false"; }
Text show(Int value) { return to_string(value); }
Text show(const Range& value) { return to_string(value.lo) + (value.inclusive ? "..=" : "..<") + to_string(value.hi); }

Text show(const Text& value) {
  static const char* const hex = "0123456789ABCDEF";
  Text out = "\"";
  for (std::size_t at = 0; at < value.size();) {
    const std::size_t from = at;
    const char32_t c = unicode::next(value, at);
    if (c == U'"') {
      out += "\\\"";
    } else if (c == U'\\') {
      out += "\\\\";
    } else if (c == U'\n') {
      out += "\\n";
    } else if (c == U'\t') {
      out += "\\t";
    } else if (c == U'{') {
      out += "\\{";
    } else if (unicode::is_control(c) || (unicode::is_white_space(c) && c != U' ')) {
      Text digits;
      for (char32_t left = c; left != 0; left /= 16) digits.insert(digits.begin(), hex[left % 16]);
      out += "\\u{" + (digits.empty() ? Text{"0"} : digits) + "}";
    } else {
      out.append(value, from, at - from);
    }
  }
  return out + "\"";
}

Text show(const Event& e) {
  const auto& names = rule_names();
  const auto index = static_cast<std::size_t>(e.rule);
  if (e.rule < 0 || index >= names.size()) return "<event>";
  const RuleName& rule = names[index];
  Text out = Text{rule.module} + "." + rule.name + "(";
  for (std::size_t i = 0; i < e.args.size(); ++i) {
    if (i > 0) out += ", ";
    if (i > 0 && i < rule.params.size()) out += Text{rule.params[i]} + ": ";
    out += show(e.args[i]);
  }
  return out + ")";
}

void name_rules(std::vector<RuleName> names) { rule_names() = std::move(names); }

Never missing() { throw Failure{"Missing"}; }
Never hole(const char* name) { throw Blocked{"F-BLOCKED", Text{"reached ?"} + name}; }
Never no_arm(const Text& shown) { throw Stop{"E-RUNTIME", "no arm matches " + shown}; }
Never foreign(const char* decl, const char* written) {
  throw Blocked{"B-FOREIGN", Text{"reached "} + decl + ", which is " + written + ": only a build with its library runs it"};
}
Never stop(const char* code, const Text& message) { throw Stop{code, message}; }

void claim(bool holds, const char* what) {
  if (!holds) throw Stop{"E-CLAIM", Text{what} + " breaks its declared type"};
}

void check(bool holds) {
  if (!holds) throw CheckFailed{"an assert is false"};
}

void tick() {
  thread_local unsigned long long turns_left = 20'000'000;
  if (turns_left == 0) throw Stop{"E-RUNTIME", "the evaluation budget ran out (a loop that does not end?)"};
  --turns_left;
}

void guard(bool holds, const char* error) {
  if (!holds) throw Failure{error};
}

List<Int> list(const Range& range) {
  const Int end = range.inclusive ? range.hi : (range.hi == Int::min() ? range.hi : range.hi - Int{1});
  List<Int> out;
  if (end < range.lo) return out;
  // The count is checked without leaving 128 bits.
  if (static_cast<uint128>(end.value()) - static_cast<uint128>(range.lo.value()) > 10'000'000) {
    stop("E-RUNTIME", "a range of more than ten million numbers");
  }
  for (Int n = range.lo; n <= end; n = n + Int{1}) {
    out.push_back(n);
    if (n == end) break;
  }
  return out;
}

}  // namespace rt
