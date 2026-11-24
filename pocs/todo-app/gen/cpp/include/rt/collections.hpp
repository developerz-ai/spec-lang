// The C++ printer's runtime, the same in every project `spec-lang build` prints. Do not edit.
// The primitives on List, Set, Map and ranges, each with the one meaning the language gives it.
// Set and Map iterate ascending in the value order; every sort, min, max, distinct and group_by uses it.
#pragma once

#include <algorithm>
#include <functional>
#include <type_traits>

#include "rt/value.hpp"

namespace rt {

template <class R, class... A>
Text show(const std::function<R(A...)>&) {
  return "<function>";
}

// ---- List ----

// What reading one item gives: a reference, or the value itself for Bool (a `std::vector<bool>` has no bool to refer to).
template <class T>
using Item = std::conditional_t<std::same_as<T, bool>, bool, const T&>;

template <class T>
Int size(const List<T>& xs) {
  return Int::of_size(xs.size());
}
template <class T>
Item<T> first(const List<T>& xs) {
  if (xs.empty()) missing();
  return xs.front();
}
template <class T>
Item<T> last(const List<T>& xs) {
  if (xs.empty()) missing();
  return xs.back();
}
template <class T>
Option<T> get(const List<T>& xs, Int index) {
  const auto i = index.index(xs.size());
  return i < xs.size() ? Option<T>{xs[i]} : Option<T>{};
}
// `xs[i]`: outside fails Missing.
template <class T>
Item<T> at(const List<T>& xs, Int index) {
  const auto i = index.index(xs.size());
  if (i == xs.size()) missing();
  return xs[i];
}
template <class T>
typename List<T>::reference at(List<T>& xs, Int index) {
  const auto i = index.index(xs.size());
  if (i == xs.size()) missing();
  return xs[i];
}
template <class T, class F>
List<T> filter(const List<T>& xs, F&& keep) {
  List<T> out;
  for (const auto& x : xs) {
    if (keep(x)) out.push_back(x);
  }
  return out;
}
template <class T, class F>
auto map(const List<T>& xs, F&& f) -> List<std::decay_t<std::invoke_result_t<F, const T&>>> {
  List<std::decay_t<std::invoke_result_t<F, const T&>>> out;
  out.reserve(xs.size());
  for (const auto& x : xs) out.push_back(f(x));
  return out;
}
template <class T, class F>
auto flat_map(const List<T>& xs, F&& f) -> std::decay_t<std::invoke_result_t<F, const T&>> {
  std::decay_t<std::invoke_result_t<F, const T&>> out;
  for (const auto& x : xs) {
    auto part = f(x);
    out.insert(out.end(), part.begin(), part.end());
  }
  return out;
}
template <class T, class A, class F>
A fold(const List<T>& xs, A start, F&& f) {
  for (const auto& x : xs) start = f(start, x);
  return start;
}
template <class T, class F>
bool all(const List<T>& xs, F&& holds) {
  for (const auto& x : xs) {
    if (!holds(x)) return false;
  }
  return true;
}
template <class T, class F>
bool any(const List<T>& xs, F&& holds) {
  for (const auto& x : xs) {
    if (holds(x)) return true;
  }
  return false;
}
template <class T, class F>
Option<T> find(const List<T>& xs, F&& holds) {
  for (const auto& x : xs) {
    if (holds(x)) return Option<T>{x};
  }
  return {};
}
template <class T, class F>
Option<Int> position(const List<T>& xs, F&& holds) {
  for (std::size_t i = 0; i < xs.size(); ++i) {
    if (holds(xs[i])) return Option<Int>{Int::of_size(i)};
  }
  return {};
}
inline Int sum(const List<Int>& xs) {
  Int total;
  for (const auto x : xs) total = total + x;
  return total;
}
template <class T, class F>
Int sum_by(const List<T>& xs, F&& f) {
  Int total;
  for (const auto& x : xs) total = total + f(x);
  return total;
}

namespace detail {
// The index of the item whose key is least (`sign` 1) or greatest (`sign` -1), the earliest of ties.
template <class T, class F>
std::size_t best(const List<T>& xs, F&& key, int sign) {
  if (xs.empty()) missing();
  std::size_t best = 0;
  auto best_key = key(xs[0]);
  for (std::size_t i = 1; i < xs.size(); ++i) {
    auto k = key(xs[i]);
    if (sign > 0 ? k < best_key : k > best_key) {
      best = i;
      best_key = std::move(k);
    }
  }
  return best;
}
}  // namespace detail

template <class T>
Item<T> min(const List<T>& xs) {
  return xs[detail::best(xs, [](Item<T> x) -> T { return x; }, 1)];
}
template <class T>
Item<T> max(const List<T>& xs) {
  return xs[detail::best(xs, [](Item<T> x) -> T { return x; }, -1)];
}
template <class T, class F>
Item<T> min_by(const List<T>& xs, F&& key) {
  return xs[detail::best(xs, key, 1)];
}
template <class T, class F>
Item<T> max_by(const List<T>& xs, F&& key) {
  return xs[detail::best(xs, key, -1)];
}
template <class T>
bool contains(const List<T>& xs, const std::type_identity_t<T>& x) {
  return std::find(xs.begin(), xs.end(), x) != xs.end();
}
template <class T>
List<T> distinct(const List<T>& xs) {
  List<T> out;
  Set<T> seen;
  for (const auto& x : xs) {
    if (seen.insert(x).second) out.push_back(x);
  }
  return out;
}
// Stable; `descending` reverses the order, not the ties.
template <class T, class F>
List<T> sort_by(const List<T>& xs, F&& key, bool descending = false) {
  using K = std::decay_t<std::invoke_result_t<F, const T&>>;
  std::vector<std::pair<K, std::size_t>> keyed;
  keyed.reserve(xs.size());
  for (std::size_t i = 0; i < xs.size(); ++i) keyed.emplace_back(key(xs[i]), i);
  std::stable_sort(keyed.begin(), keyed.end(), [descending](const auto& a, const auto& b) {
    return descending ? b.first < a.first : a.first < b.first;
  });
  List<T> out;
  out.reserve(xs.size());
  for (const auto& [k, i] : keyed) out.push_back(xs[i]);
  return out;
}
template <class T>
List<T> sort(const List<T>& xs, bool descending = false) {
  List<T> out = xs;
  std::stable_sort(out.begin(), out.end(), [descending](const T& a, const T& b) { return descending ? b < a : a < b; });
  return out;
}
template <class T>
List<T> reversed(const List<T>& xs) {
  return List<T>(xs.rbegin(), xs.rend());
}
// The first `count` items; a negative count stops E-TYPE-ARITY.
template <class T>
List<T> take(const List<T>& xs, Int count) {
  if (count < Int{0}) stop("E-TYPE-ARITY", "`list.take` takes one Int, at least 0");
  const auto n = std::min(count.index(xs.size()), xs.size());
  return List<T>(xs.begin(), xs.begin() + static_cast<std::ptrdiff_t>(n));
}
template <class T>
List<T> drop(const List<T>& xs, Int count) {
  if (count < Int{0}) stop("E-TYPE-ARITY", "`list.drop` takes one Int, at least 0");
  const auto n = std::min(count.index(xs.size()), xs.size());
  return List<T>(xs.begin() + static_cast<std::ptrdiff_t>(n), xs.end());
}
// Pairs by position; unequal sizes fail Uneven.
template <class T, class U>
List<std::tuple<T, U>> zip(const List<T>& xs, const List<U>& ys) {
  if (xs.size() != ys.size()) fail("Uneven");
  List<std::tuple<T, U>> out;
  out.reserve(xs.size());
  for (std::size_t i = 0; i < xs.size(); ++i) out.emplace_back(xs[i], ys[i]);
  return out;
}
template <class T, class F>
auto group_by(const List<T>& xs, F&& key) -> Map<std::decay_t<std::invoke_result_t<F, const T&>>, List<T>> {
  Map<std::decay_t<std::invoke_result_t<F, const T&>>, List<T>> out;
  for (const auto& x : xs) out[key(x)].push_back(x);
  return out;
}
template <class T>
List<T> concat(const List<T>& a, const List<T>& b) {
  List<T> out = a;
  out.insert(out.end(), b.begin(), b.end());
  return out;
}
template <class T>
List<T> pushed(List<T> xs, std::type_identity_t<T> x) {
  xs.push_back(std::move(x));
  return xs;
}
template <class T>
void push(List<T>& xs, std::type_identity_t<T> x) {
  xs.push_back(std::move(x));
}
// `x` put before `index` (0..=size); outside fails Missing.
template <class T>
void insert_at(List<T>& xs, Int index, std::type_identity_t<T> x) {
  const auto i = index.index(xs.size() + 1);
  if (i > xs.size()) missing();
  xs.insert(xs.begin() + static_cast<std::ptrdiff_t>(i), std::move(x));
}
template <class T>
List<T> inserted_at(List<T> xs, Int index, std::type_identity_t<T> x) {
  insert_at(xs, index, std::move(x));
  return xs;
}
// The item at `index` removed; outside fails Missing.
template <class T>
void remove_at(List<T>& xs, Int index) {
  const auto i = index.index(xs.size());
  if (i == xs.size()) missing();
  xs.erase(xs.begin() + static_cast<std::ptrdiff_t>(i));
}
template <class T>
List<T> removed_at(List<T> xs, Int index) {
  remove_at(xs, index);
  return xs;
}

// ---- Set ----

template <class T>
Int size(const Set<T>& s) {
  return Int::of_size(s.size());
}
template <class T>
List<T> list(const Set<T>& s) {
  return List<T>(s.begin(), s.end());
}
template <class T>
Set<T> set_of(const List<T>& xs) {
  return Set<T>(xs.begin(), xs.end());
}
template <class T>
bool contains(const Set<T>& s, const std::type_identity_t<T>& x) {
  return s.contains(x);
}
template <class T>
Set<T> set_union(Set<T> a, const Set<T>& b) {
  a.insert(b.begin(), b.end());
  return a;
}
template <class T>
Set<T> difference(Set<T> a, const Set<T>& b) {
  for (const auto& x : b) a.erase(x);
  return a;
}
template <class T>
Set<T> with(Set<T> s, std::type_identity_t<T> x) {
  s.insert(std::move(x));
  return s;
}
template <class T>
Set<T> without(Set<T> s, const std::type_identity_t<T>& x) {
  s.erase(x);
  return s;
}

// ---- Map ----

template <class K, class V>
Int size(const Map<K, V>& m) {
  return Int::of_size(m.size());
}
template <class K, class V>
List<K> keys(const Map<K, V>& m) {
  List<K> out;
  out.reserve(m.size());
  for (const auto& [k, v] : m) out.push_back(k);
  return out;
}
template <class K, class V>
List<V> values(const Map<K, V>& m) {
  List<V> out;
  out.reserve(m.size());
  for (const auto& [k, v] : m) out.push_back(v);
  return out;
}
template <class K, class V>
Option<V> get(const Map<K, V>& m, const std::type_identity_t<K>& key) {
  const auto it = m.find(key);
  return it == m.end() ? Option<V>{} : Option<V>{it->second};
}
// `m[k]`: absent fails Missing.
template <class K, class V>
const V& at(const Map<K, V>& m, const std::type_identity_t<K>& key) {
  const auto it = m.find(key);
  if (it == m.end()) missing();
  return it->second;
}
template <class K, class V>
V& at(Map<K, V>& m, const std::type_identity_t<K>& key) {
  const auto it = m.find(key);
  if (it == m.end()) missing();
  return it->second;
}
template <class K, class V>
bool contains(const Map<K, V>& m, const std::type_identity_t<K>& key) {
  return m.contains(key);
}
template <class K, class V>
Map<K, V> without(Map<K, V> m, const std::type_identity_t<K>& key) {
  m.erase(key);
  return m;
}

// `Map(k -> v, ...)`: entries left to right; a later equal key replaces an earlier one.
template <class M>
M map_of(std::initializer_list<typename M::value_type> entries) {
  M out;
  for (const auto& [key, value] : entries) out.insert_or_assign(key, value);
  return out;
}

// ---- ranges ----

// The numbers of the range ascending; more than ten million stops E-RUNTIME.
List<Int> list(const Range& range);
inline bool contains(const Range& range, Int n) {
  return range.has(n);
}

}  // namespace rt
