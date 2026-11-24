//! `List`, `Set` and `Map` primitives that can fail or take a function. A function argument may fail; the first
//! failure wins and stops the walk.

use super::{Int, Opt, R, fail, int, missing, stop};
use std::collections::{BTreeMap, BTreeSet};

fn index(xs: usize, i: Int) -> Option<usize> {
    usize::try_from(i).ok().filter(|i| *i < xs)
}

pub fn first<T: Clone>(xs: &[T]) -> R<T> {
    xs.first().cloned().ok_or_else(missing)
}

pub fn last<T: Clone>(xs: &[T]) -> R<T> {
    xs.last().cloned().ok_or_else(missing)
}

pub fn get<T: Clone>(xs: &[T], i: Int) -> Opt<T> {
    index(xs.len(), i).map(|i| xs[i].clone()).into()
}

/// `xs[i]`.
pub fn at<T: Clone>(xs: &[T], i: Int) -> R<T> {
    index(xs.len(), i).map(|i| xs[i].clone()).ok_or_else(missing)
}

pub fn filter<T: Clone>(xs: &[T], mut keep: impl FnMut(T) -> R<bool>) -> R<Vec<T>> {
    let mut out = Vec::new();
    for x in xs {
        if keep(x.clone())? {
            out.push(x.clone());
        }
    }
    Ok(out)
}

pub fn map<T: Clone, U>(xs: &[T], f: impl FnMut(T) -> R<U>) -> R<Vec<U>> {
    xs.iter().cloned().map(f).collect()
}

pub fn flat_map<T: Clone, U>(xs: &[T], mut f: impl FnMut(T) -> R<Vec<U>>) -> R<Vec<U>> {
    let mut out = Vec::new();
    for x in xs {
        out.extend(f(x.clone())?);
    }
    Ok(out)
}

pub fn fold<T: Clone, A>(xs: &[T], start: A, mut f: impl FnMut(A, T) -> R<A>) -> R<A> {
    let mut acc = start;
    for x in xs {
        acc = f(acc, x.clone())?;
    }
    Ok(acc)
}

pub fn all<T: Clone>(xs: &[T], mut p: impl FnMut(T) -> R<bool>) -> R<bool> {
    for x in xs {
        if !p(x.clone())? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn any<T: Clone>(xs: &[T], mut p: impl FnMut(T) -> R<bool>) -> R<bool> {
    for x in xs {
        if p(x.clone())? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn find<T: Clone>(xs: &[T], mut p: impl FnMut(T) -> R<bool>) -> R<Opt<T>> {
    for x in xs {
        if p(x.clone())? {
            return Ok(Opt::Some(x.clone()));
        }
    }
    Ok(Opt::None)
}

pub fn position<T: Clone>(xs: &[T], mut p: impl FnMut(T) -> R<bool>) -> R<Opt<Int>> {
    for (i, x) in xs.iter().enumerate() {
        if p(x.clone())? {
            return Ok(Opt::Some(int::count(i)));
        }
    }
    Ok(Opt::None)
}

pub fn sum(xs: &[Int]) -> R<Int> {
    xs.iter().try_fold(0, |a, x| int::add(a, *x))
}

pub fn sum_by<T: Clone>(xs: &[T], mut f: impl FnMut(T) -> R<Int>) -> R<Int> {
    let mut total = 0;
    for x in xs {
        total = int::add(total, f(x.clone())?)?;
    }
    Ok(total)
}

/// The item with the least (`least`) or greatest key; the earliest of ties; empty fails `Missing`.
fn best_by<T: Clone, K: Ord>(xs: &[T], mut key: impl FnMut(T) -> R<K>, least: bool) -> R<T> {
    let mut best: Option<(K, &T)> = None;
    for x in xs {
        let k = key(x.clone())?;
        let better = match &best {
            None => true,
            Some((b, _)) => {
                if least {
                    k < *b
                } else {
                    k > *b
                }
            }
        };
        if better {
            best = Some((k, x));
        }
    }
    best.map(|(_, x)| x.clone()).ok_or_else(missing)
}

pub fn min<T: Clone + Ord>(xs: &[T]) -> R<T> {
    best_by(xs, Ok, true)
}

pub fn max<T: Clone + Ord>(xs: &[T]) -> R<T> {
    best_by(xs, Ok, false)
}

pub fn min_by<T: Clone, K: Ord>(xs: &[T], key: impl FnMut(T) -> R<K>) -> R<T> {
    best_by(xs, key, true)
}

pub fn max_by<T: Clone, K: Ord>(xs: &[T], key: impl FnMut(T) -> R<K>) -> R<T> {
    best_by(xs, key, false)
}

/// The earliest of each group of equal items, in order.
pub fn distinct<T: Clone + Ord>(xs: &[T]) -> Vec<T> {
    let mut seen = BTreeSet::new();
    xs.iter().filter(|x| seen.insert((*x).clone())).cloned().collect()
}

/// Stable, by the value order; `desc` reverses the order, not the ties.
pub fn sort<T: Clone + Ord>(xs: &[T], desc: bool) -> Vec<T> {
    let mut out = xs.to_vec();
    out.sort_by(|a, b| if desc { b.cmp(a) } else { a.cmp(b) });
    out
}

pub fn sort_by<T: Clone, K: Ord>(xs: &[T], mut key: impl FnMut(T) -> R<K>, desc: bool) -> R<Vec<T>> {
    let mut keyed = Vec::with_capacity(xs.len());
    for x in xs {
        keyed.push((key(x.clone())?, x.clone()));
    }
    keyed.sort_by(|a, b| if desc { b.0.cmp(&a.0) } else { a.0.cmp(&b.0) });
    Ok(keyed.into_iter().map(|(_, x)| x).collect())
}

pub fn reversed<T: Clone>(xs: &[T]) -> Vec<T> {
    xs.iter().rev().cloned().collect()
}

fn count_arg(n: Int, op: &str) -> R<usize> {
    if n < 0 {
        return Err(stop("E-TYPE-ARITY", format!("`{op}` takes one Int, at least 0")));
    }
    Ok(usize::try_from(n).unwrap_or(usize::MAX))
}

pub fn take<T: Clone>(xs: &[T], n: Int) -> R<Vec<T>> {
    Ok(xs.iter().take(count_arg(n, "take")?).cloned().collect())
}

pub fn drop<T: Clone>(xs: &[T], n: Int) -> R<Vec<T>> {
    Ok(xs.iter().skip(count_arg(n, "drop")?).cloned().collect())
}

/// Pairs by position; unequal sizes fail `Uneven`.
pub fn zip<T: Clone, U: Clone>(xs: &[T], ys: &[U]) -> R<Vec<(T, U)>> {
    if xs.len() != ys.len() {
        return Err(fail("Uneven", Vec::new()));
    }
    Ok(xs.iter().cloned().zip(ys.iter().cloned()).collect())
}

pub fn group_by<T: Clone, K: Ord>(xs: &[T], mut key: impl FnMut(T) -> R<K>) -> R<BTreeMap<K, Vec<T>>> {
    let mut groups: BTreeMap<K, Vec<T>> = BTreeMap::new();
    for x in xs {
        groups.entry(key(x.clone())?).or_default().push(x.clone());
    }
    Ok(groups)
}

pub fn concat<T: Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let mut out = a.to_vec();
    out.extend_from_slice(b);
    out
}

pub fn push<T: Clone>(xs: &[T], x: T) -> Vec<T> {
    let mut out = xs.to_vec();
    out.push(x);
    out
}

/// `x` put before index `at` (0..=size); outside fails `Missing`.
pub fn insert<T: Clone>(xs: &[T], at: Int, x: T) -> R<Vec<T>> {
    let at = usize::try_from(at).ok().filter(|i| *i <= xs.len()).ok_or_else(missing)?;
    let mut out = xs.to_vec();
    out.insert(at, x);
    Ok(out)
}

pub fn remove<T: Clone>(xs: &[T], at: Int) -> R<Vec<T>> {
    let at = index(xs.len(), at).ok_or_else(missing)?;
    let mut out = xs.to_vec();
    out.remove(at);
    Ok(out)
}

pub fn set_insert<T: Clone + Ord>(s: &BTreeSet<T>, x: T) -> BTreeSet<T> {
    let mut out = s.clone();
    out.insert(x);
    out
}

pub fn set_remove<T: Clone + Ord>(s: &BTreeSet<T>, x: &T) -> BTreeSet<T> {
    let mut out = s.clone();
    out.remove(x);
    out
}

pub fn union<T: Clone + Ord>(a: &BTreeSet<T>, b: &BTreeSet<T>) -> BTreeSet<T> {
    a.union(b).cloned().collect()
}

pub fn difference<T: Clone + Ord>(a: &BTreeSet<T>, b: &BTreeSet<T>) -> BTreeSet<T> {
    a.difference(b).cloned().collect()
}

pub fn map_get<K: Ord, V: Clone>(m: &BTreeMap<K, V>, k: &K) -> Opt<V> {
    m.get(k).cloned().into()
}

/// `m[k]`: absent fails `Missing`.
pub fn map_at<K: Ord, V: Clone>(m: &BTreeMap<K, V>, k: &K) -> R<V> {
    m.get(k).cloned().ok_or_else(missing)
}

pub fn map_remove<K: Ord + Clone, V: Clone>(m: &BTreeMap<K, V>, k: &K) -> BTreeMap<K, V> {
    let mut out = m.clone();
    out.remove(k);
    out
}
