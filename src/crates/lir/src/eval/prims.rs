//! Every primitive, as `crate::prim` defines it.

use super::machine::{Env, Halt, Halted, Machine, mismatch, runtime};
use super::value::Value;
use crate::node::Expr;
use crate::prim::Prim;
use spec_lang_diag::Diag;
use spec_lang_syntax::Span;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

fn overflow<'l>(span: Span) -> Halt<'l> {
    runtime(
        span,
        "an Int beyond the evaluator's 128-bit range (the language's Int is unbounded; this evaluator is not)",
    )
}

fn arity<'l>(span: Span, p: Prim, want: &str) -> Halt<'l> {
    Diag::error("E-TYPE-ARITY", span, format!("`{}` takes {want}", p.name())).into()
}

/// Unicode `White_Space` (`char::is_whitespace`: 25 code points).
fn trim(t: &str) -> &str {
    t.trim_matches(char::is_whitespace)
}

impl<'l> Machine<'l> {
    pub(crate) fn prim(&mut self, env: &mut Env<'l>, op: Prim, args: &'l [Expr], span: Span) -> Halted<'l, Value<'l>> {
        let vals = self.eval_all(env, args)?;
        self.apply_prim(op, vals, span)
    }

    fn holds(&mut self, f: &Value<'l>, x: Value<'l>, span: Span) -> Halted<'l, bool> {
        match self.apply(f, vec![x], span)? {
            Value::Bool(b) => Ok(b),
            _ => Err(mismatch(span, "a predicate gave no Bool")),
        }
    }

    pub(crate) fn apply_prim(&mut self, op: Prim, a: Vec<Value<'l>>, span: Span) -> Halted<'l, Value<'l>> {
        use Value as V;
        let int = |r: Option<i128>| r.map(V::Int).ok_or_else(|| overflow(span));
        Ok(match (op, a.as_slice()) {
            (Prim::Eq, [x, y]) => V::Bool(x == y),
            (Prim::Ne, [x, y]) => V::Bool(x != y),
            (Prim::Not, [V::Bool(b)]) => V::Bool(!b),
            (Prim::IntAdd, [V::Int(x), V::Int(y)]) => int(x.checked_add(*y))?,
            (Prim::IntSub, [V::Int(x), V::Int(y)]) => int(x.checked_sub(*y))?,
            (Prim::IntMul, [V::Int(x), V::Int(y)]) => int(x.checked_mul(*y))?,
            (Prim::IntNeg, [V::Int(x)]) => int(x.checked_neg())?,
            (Prim::IntMod, [V::Int(_), V::Int(0)]) => return Err(Halt::fail("DivByZero")),
            (Prim::IntMod, [V::Int(x), V::Int(y)]) => {
                int(x.checked_rem_euclid(*y).map(|m| if *y < 0 && m != 0 { m + y } else { m }))?
            }
            (Prim::IntPow, [V::Int(x), V::Int(y)]) if *y >= 0 => {
                int(u32::try_from(*y).ok().and_then(|e| x.checked_pow(e)))?
            }
            (Prim::IntDiv, [V::Int(_), V::Int(_)]) => {
                return Err(Diag::error("E-OPEN", span, "`/` on Int is an open choice: say how to round")
                    .with_alternative("rounds down", Vec::new())
                    .with_alternative("truncates toward zero", Vec::new())
                    .with_note("write `a.div(b, floor)` or `a.div(b, trunc)`")
                    .into());
            }
            (Prim::IntBand | Prim::IntBor | Prim::IntBxor | Prim::IntShl | Prim::IntShr, [V::Int(x), V::Int(y)])
                if *x >= 0 && *y >= 0 =>
            {
                match op {
                    Prim::IntBand => V::Int(x & y),
                    Prim::IntBor => V::Int(x | y),
                    Prim::IntBxor => V::Int(x ^ y),
                    Prim::IntShl => {
                        int(u32::try_from(*y).ok().and_then(|s| x.checked_shl(s)).filter(|v| v >> y == *x))?
                    }
                    _ => V::Int(u32::try_from(*y).ok().and_then(|s| x.checked_shr(s)).unwrap_or(0)),
                }
            }
            (Prim::IntLt, [V::Int(x), V::Int(y)]) => V::Bool(x < y),
            (Prim::IntLe, [V::Int(x), V::Int(y)]) => V::Bool(x <= y),
            (Prim::IntGt, [V::Int(x), V::Int(y)]) => V::Bool(x > y),
            (Prim::IntGe, [V::Int(x), V::Int(y)]) => V::Bool(x >= y),
            (Prim::IntAbs, [V::Int(x)]) => {
                x.checked_abs().map(V::Int).ok_or_else(|| mismatch(span, "no `abs` of this Int"))?
            }
            (Prim::IntMin, [V::Int(x), V::Int(y)]) => V::Int(*x.min(y)),
            (Prim::IntMax, [V::Int(x), V::Int(y)]) => V::Int(*x.max(y)),
            (Prim::IntClamp, [V::Int(x), V::Int(lo), V::Int(hi)]) if lo <= hi => V::Int((*x).clamp(*lo, *hi)),
            (Prim::TextLt, [V::Text(x), V::Text(y)]) => V::Bool(x < y),
            (Prim::TextLe, [V::Text(x), V::Text(y)]) => V::Bool(x <= y),
            (Prim::TextGt, [V::Text(x), V::Text(y)]) => V::Bool(x > y),
            (Prim::TextGe, [V::Text(x), V::Text(y)]) => V::Bool(x >= y),
            (Prim::TextSize, [V::Text(t)]) => V::Int(t.chars().count() as i128),
            (Prim::TextBytes, [V::Text(t)]) => V::list(t.bytes().map(|b| V::Int(b.into())).collect()),
            (Prim::TextTrim, [V::Text(t)]) => V::text(trim(t)),
            (Prim::TextLower, [V::Text(t)]) => V::text(&t.to_lowercase()),
            (Prim::TextStartsWith, [V::Text(t), V::Text(x)]) => V::Bool(t.starts_with(&**x)),
            (Prim::TextContains, [V::Text(t), V::Text(x)]) => V::Bool(t.contains(&**x)),
            (Prim::TextSplit, [V::Text(t), V::Text(sep)]) if !sep.is_empty() => {
                V::list(t.split(&**sep).map(V::text).collect())
            }
            (Prim::TextSplit, _) => return Err(arity(span, op, "one non-empty Text")),
            (Prim::TextReplace, [V::Text(t), V::Text(old), V::Text(new)]) if !old.is_empty() => {
                V::text(&t.replace(&**old, new))
            }
            (Prim::TextReplace, _) => return Err(arity(span, op, "two Text, the first not empty")),
            (Prim::TextTake | Prim::TextDrop, [V::Text(t), V::Int(n)]) => {
                let n = usize::try_from((*n).max(0)).unwrap_or(usize::MAX);
                let s: String =
                    if op == Prim::TextTake { t.chars().take(n).collect() } else { t.chars().skip(n).collect() };
                V::text(&s)
            }
            (Prim::ListSize, [V::List(xs)]) => V::Int(xs.len() as i128),
            (Prim::ListFirst, [V::List(xs)]) => xs.first().cloned().ok_or_else(Halt::missing)?,
            (Prim::ListLast, [V::List(xs)]) => xs.last().cloned().ok_or_else(Halt::missing)?,
            (Prim::ListGet, [V::List(xs), V::Int(i)]) => {
                self.option(usize::try_from(*i).ok().and_then(|i| xs.get(i)).cloned())
            }
            (Prim::ListIndex, [V::List(xs), V::Int(i)]) => {
                usize::try_from(*i).ok().and_then(|i| xs.get(i)).cloned().ok_or_else(Halt::missing)?
            }
            (Prim::ListFilter, [V::List(xs), f]) => {
                let mut out = Vec::new();
                for x in xs.iter() {
                    if self.holds(f, x.clone(), span)? {
                        out.push(x.clone());
                    }
                }
                V::list(out)
            }
            (Prim::ListMap, [V::List(xs), f]) => {
                let mut out = Vec::with_capacity(xs.len());
                for x in xs.iter() {
                    out.push(self.apply(f, vec![x.clone()], span)?);
                }
                V::list(out)
            }
            (Prim::ListFlatMap, [V::List(xs), f]) => {
                let mut out = Vec::new();
                for x in xs.iter() {
                    match self.apply(f, vec![x.clone()], span)? {
                        V::List(ys) => out.extend(ys.iter().cloned()),
                        _ => return Err(mismatch(span, "`flat_map` needs a function giving a List")),
                    }
                }
                V::list(out)
            }
            (Prim::ListFold, [V::List(xs), start, f]) => {
                let mut acc = start.clone();
                for x in xs.iter() {
                    acc = self.apply(f, vec![acc, x.clone()], span)?;
                }
                acc
            }
            (Prim::ListAll | Prim::ListAny, [V::List(xs), f]) => {
                let want = op == Prim::ListAny;
                for x in xs.iter() {
                    if self.holds(f, x.clone(), span)? == want {
                        return Ok(V::Bool(want));
                    }
                }
                V::Bool(!want)
            }
            (Prim::ListFind | Prim::ListPosition, [V::List(xs), f]) => {
                for (i, x) in xs.iter().enumerate() {
                    if self.holds(f, x.clone(), span)? {
                        let found = if op == Prim::ListFind { x.clone() } else { V::Int(i as i128) };
                        return Ok(self.some(found));
                    }
                }
                self.none()
            }
            (Prim::ListSum | Prim::ListSumBy, [V::List(xs), rest @ ..]) => {
                let mut total: i128 = 0;
                for x in xs.iter() {
                    let x = match rest.first() {
                        Some(f) => self.apply(f, vec![x.clone()], span)?,
                        None => x.clone(),
                    };
                    let V::Int(n) = x else { return Err(mismatch(span, "`sum` adds Int")) };
                    total = total.checked_add(n).ok_or_else(|| overflow(span))?;
                }
                V::Int(total)
            }
            (Prim::ListMin | Prim::ListMax | Prim::ListMinBy | Prim::ListMaxBy, [V::List(xs), rest @ ..]) => {
                let min = matches!(op, Prim::ListMin | Prim::ListMinBy);
                let mut best: Option<(V, V)> = None;
                for x in xs.iter() {
                    let k = match rest.first() {
                        Some(f) => self.apply(f, vec![x.clone()], span)?,
                        None => x.clone(),
                    };
                    let better = match &best {
                        None => true,
                        Some((bk, _)) => {
                            if min {
                                k < *bk
                            } else {
                                k > *bk
                            }
                        }
                    };
                    if better {
                        best = Some((k, x.clone()));
                    }
                }
                best.map(|(_, x)| x).ok_or_else(Halt::missing)?
            }
            (Prim::ListContains, [V::List(xs), x]) => V::Bool(xs.contains(x)),
            (Prim::ListDistinct, [V::List(xs)]) => {
                let mut seen = BTreeSet::new();
                V::list(xs.iter().filter(|x| seen.insert((*x).clone())).cloned().collect())
            }
            (Prim::ListSort | Prim::ListSortBy, [V::List(xs), rest @ ..]) => {
                let (key, desc) = match rest {
                    [V::Bool(d)] => (None, *d),
                    [f, V::Bool(d)] => (Some(f), *d),
                    _ => return Err(arity(span, op, "a key function and `desc: Bool`")),
                };
                let mut keyed = Vec::with_capacity(xs.len());
                for x in xs.iter() {
                    let k = match key {
                        Some(f) => self.apply(f, vec![x.clone()], span)?,
                        None => x.clone(),
                    };
                    keyed.push((k, x.clone()));
                }
                keyed.sort_by(|a, b| if desc { b.0.cmp(&a.0) } else { a.0.cmp(&b.0) });
                V::list(keyed.into_iter().map(|(_, x)| x).collect())
            }
            (Prim::ListReversed, [V::List(xs)]) => V::list(xs.iter().rev().cloned().collect()),
            (Prim::ListTake | Prim::ListDrop, [V::List(xs), V::Int(n)]) if *n >= 0 => {
                let n = usize::try_from(*n).unwrap_or(usize::MAX);
                if op == Prim::ListTake {
                    V::list(xs.iter().take(n).cloned().collect())
                } else {
                    V::list(xs.iter().skip(n).cloned().collect())
                }
            }
            (Prim::ListTake | Prim::ListDrop, _) => return Err(arity(span, op, "one Int, at least 0")),
            (Prim::ListZip, [V::List(xs), V::List(ys)]) => {
                if xs.len() != ys.len() {
                    return Err(Halt::fail("Uneven"));
                }
                V::list(xs.iter().zip(ys.iter()).map(|(a, b)| V::Tuple(Rc::new(vec![a.clone(), b.clone()]))).collect())
            }
            (Prim::ListGroupBy, [V::List(xs), f]) => {
                let mut groups: BTreeMap<V, Vec<V>> = BTreeMap::new();
                for x in xs.iter() {
                    let k = self.apply(f, vec![x.clone()], span)?;
                    groups.entry(k).or_default().push(x.clone());
                }
                V::Map(Rc::new(groups.into_iter().map(|(k, v)| (k, V::list(v))).collect()))
            }
            (Prim::ListJoin, [V::List(xs), V::Text(sep)]) => {
                let mut parts = Vec::with_capacity(xs.len());
                for x in xs.iter() {
                    let V::Text(t) = x else { return Err(mismatch(span, "`join` is on List[Text]")) };
                    parts.push(t.to_string());
                }
                V::text(&parts.join(sep))
            }
            (Prim::ListConcat, [V::List(x), V::List(y)]) => {
                let mut v = (**x).clone();
                v.extend(y.iter().cloned());
                V::list(v)
            }
            (Prim::ListPush, [V::List(xs), x]) => {
                let mut xs = xs.clone();
                Rc::make_mut(&mut xs).push(x.clone());
                V::List(xs)
            }
            (Prim::ListInsert, [V::List(xs), V::Int(at), x]) => {
                let at = usize::try_from(*at).ok().filter(|i| *i <= xs.len()).ok_or_else(Halt::missing)?;
                let mut xs = xs.clone();
                Rc::make_mut(&mut xs).insert(at, x.clone());
                V::List(xs)
            }
            (Prim::ListRemove, [V::List(xs), V::Int(at)]) => {
                let at = usize::try_from(*at).ok().filter(|i| *i < xs.len()).ok_or_else(Halt::missing)?;
                let mut xs = xs.clone();
                Rc::make_mut(&mut xs).remove(at);
                V::List(xs)
            }
            (Prim::SetList, [V::Set(s)]) => V::list(s.iter().cloned().collect()),
            (Prim::SetOfList, [V::List(xs)]) => V::Set(Rc::new(xs.iter().cloned().collect())),
            (Prim::SetSize, [V::Set(s)]) => V::Int(s.len() as i128),
            (Prim::SetContains, [V::Set(s), x]) => V::Bool(s.contains(x)),
            (Prim::SetUnion, [V::Set(x), V::Set(y)]) => V::Set(Rc::new(x.union(y).cloned().collect())),
            (Prim::SetDifference, [V::Set(x), V::Set(y)]) => V::Set(Rc::new(x.difference(y).cloned().collect())),
            (Prim::SetInsert, [V::Set(s), x]) => {
                let mut s = s.clone();
                Rc::make_mut(&mut s).insert(x.clone());
                V::Set(s)
            }
            (Prim::SetRemove, [V::Set(s), x]) => {
                let mut s = s.clone();
                Rc::make_mut(&mut s).remove(x);
                V::Set(s)
            }
            (Prim::MapSize, [V::Map(m)]) => V::Int(m.len() as i128),
            (Prim::MapKeys, [V::Map(m)]) => V::list(m.keys().cloned().collect()),
            (Prim::MapValues, [V::Map(m)]) => V::list(m.values().cloned().collect()),
            (Prim::MapGet, [V::Map(m), k]) => self.option(m.get(k).cloned()),
            (Prim::MapIndex, [V::Map(m), k]) => m.get(k).cloned().ok_or_else(Halt::missing)?,
            (Prim::MapHas, [V::Map(m), k]) => V::Bool(m.contains_key(k)),
            (Prim::MapRemove, [V::Map(m), k]) => {
                let mut m = m.clone();
                Rc::make_mut(&mut m).remove(k);
                V::Map(m)
            }
            (Prim::RangeList, [V::Range(lo, hi, incl)]) => {
                let end = if *incl { *hi } else { hi.saturating_sub(1) };
                if end.saturating_sub(*lo) > 10_000_000 {
                    return Err(runtime(span, "a range of more than ten million numbers"));
                }
                V::list((*lo..=end).map(V::Int).collect())
            }
            (Prim::RangeHas, [V::Range(lo, hi, incl), V::Int(n)]) => {
                V::Bool(*n >= *lo && if *incl { *n <= *hi } else { *n < *hi })
            }
            (Prim::TraceSteps, [V::Trace(t)]) => V::list(t.steps.clone()),
            (Prim::TraceStates, [V::Trace(t)]) => V::list(t.states.clone()),
            (Prim::StepEvent, [V::Step(s)]) => s.event.clone(),
            (Prim::StepReply, [V::Step(s)]) => s.reply.clone(),
            (Prim::StepBefore, [V::Step(s)]) => s.before.clone(),
            (Prim::StepAfter, [V::Step(s)]) => s.after.clone(),
            (op, _) => return Err(mismatch(span, format!("`{}` does not apply to these values", op.name()))),
        })
    }
}
