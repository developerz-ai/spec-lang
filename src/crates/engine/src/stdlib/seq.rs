//! List, Set and range methods over their elements in order.

use super::{Args, Seq, wrong_args};
use crate::display::show;
use crate::error::{Halt, Halted};
use crate::interp::Interp;
use crate::value::Value;
use spec_lang_diag::Diag;
use spec_lang_syntax::Span;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

impl Interp<'_> {
    fn rebuild(&self, kind: Seq, items: Vec<Value>) -> Value {
        match kind {
            Seq::Set => Value::Set(Rc::new(items.into_iter().collect::<BTreeSet<_>>())),
            Seq::List | Seq::Range => Value::list(items),
        }
    }

    /// Apply a predicate and require a Bool.
    fn holds(&mut self, f: &Value, args: Vec<Value>, span: Span) -> Halted<bool> {
        let r = self.apply(f, args, span)?;
        self.truth(r, span)
    }

    fn truth(&self, v: Value, span: Span) -> Halted<bool> {
        match v {
            Value::Bool(b) => Ok(b),
            other => Err(self.not_bool(&other, span)),
        }
    }

    pub(crate) fn seq_method(
        &mut self,
        kind: Seq,
        items: Vec<Value>,
        name: &str,
        args: Args,
        span: Span,
    ) -> Halted<Option<Value>> {
        let positional: Vec<&Value> = args.iter().filter(|(n, _)| n.is_none()).map(|(_, v)| v).collect();
        let named = |key: &str| args.iter().find(|(n, _)| n.as_deref() == Some(key)).map(|(_, v)| v);
        if name != "sort" && positional.len() != args.len() {
            return Err(wrong_args(span, name, "positional arguments"));
        }
        let f = positional.first().copied().cloned();
        let one_fn = |what: &str| {
            f.clone().filter(|v| matches!(v, Value::Func(_))).ok_or_else(|| wrong_args(span, what, "one function"))
        };
        let v = match name {
            "filter" => {
                let f = one_fn(name)?;
                let mut out = Vec::new();
                for x in items {
                    if self.holds(&f, vec![x.clone()], span)? {
                        out.push(x);
                    }
                }
                self.rebuild(kind, out)
            }
            "map" => {
                let f = one_fn(name)?;
                let mut out = Vec::new();
                for x in items {
                    out.push(self.apply(&f, vec![x], span)?);
                }
                self.rebuild(kind, out)
            }
            "flat_map" => {
                let f = one_fn(name)?;
                let mut out = Vec::new();
                for x in items {
                    match self.apply(&f, vec![x], span)? {
                        Value::List(ys) => out.extend(ys.iter().cloned()),
                        other => {
                            return Err(Diag::error(
                                "E-TYPE-MISMATCH",
                                span,
                                "`flat_map` needs a function giving a List",
                            )
                            .with_expected("List", show(self.prog, &other))
                            .into());
                        }
                    }
                }
                Value::list(out)
            }
            "fold" => {
                let [start, f] = positional.as_slice() else {
                    return Err(wrong_args(span, name, "a start value and a function"));
                };
                let mut acc = (*start).clone();
                for x in items {
                    acc = self.apply(f, vec![acc, x], span)?;
                }
                acc
            }
            "all" | "any" => {
                let f = one_fn(name)?;
                let want = name == "any";
                for x in items {
                    if self.holds(&f, vec![x], span)? == want {
                        return Ok(Some(Value::Bool(want)));
                    }
                }
                Value::Bool(!want)
            }
            "find" | "position" => {
                let f = one_fn(name)?;
                for (i, x) in items.into_iter().enumerate() {
                    if self.holds(&f, vec![x.clone()], span)? {
                        let found = if name == "find" { x } else { Value::Int(i as i128) };
                        return Ok(Some(self.option(Some(found))));
                    }
                }
                self.option(None)
            }
            "sum" => {
                let mut total: i128 = 0;
                for x in items {
                    let x = match &f {
                        Some(f) => self.apply(f, vec![x], span)?,
                        None => x,
                    };
                    match x {
                        Value::Int(n) => {
                            total = total
                                .checked_add(n)
                                .ok_or_else(|| super::super::eval::runtime(span, "an Int beyond 128 bits"))?
                        }
                        other => {
                            return Err(Diag::error("E-TYPE-MISMATCH", span, "`sum` adds Int")
                                .with_expected("Int", show(self.prog, &other))
                                .into());
                        }
                    }
                }
                Value::Int(total)
            }
            "min" | "max" => {
                let mut best: Option<(Value, Value)> = None;
                for x in items {
                    let k = match &f {
                        Some(f) => self.apply(f, vec![x.clone()], span)?,
                        None => x.clone(),
                    };
                    let better = match &best {
                        None => true,
                        Some((bk, _)) => {
                            if name == "min" {
                                k < *bk
                            } else {
                                k > *bk
                            }
                        }
                    };
                    if better {
                        best = Some((k, x));
                    }
                }
                best.map(|(_, x)| x).ok_or_else(Halt::missing)?
            }
            "contains" => match positional.as_slice() {
                [x] => Value::Bool(items.contains(x)),
                _ => return Err(wrong_args(span, name, "one value")),
            },
            "distinct" => {
                let mut seen = BTreeSet::new();
                Value::list(items.into_iter().filter(|x| seen.insert(x.clone())).collect())
            }
            "sort" => {
                let key = named("key").or(positional.first().copied()).cloned();
                let desc = match named("desc") {
                    None | Some(Value::Bool(false)) => false,
                    Some(Value::Bool(true)) => true,
                    Some(_) => return Err(wrong_args(span, name, "`desc: Bool`")),
                };
                let mut keyed = Vec::new();
                for x in items {
                    let k = match &key {
                        Some(f) => self.apply(f, vec![x.clone()], span)?,
                        None => x.clone(),
                    };
                    self.need_comparable(&k, span)?;
                    keyed.push((k, x));
                }
                keyed.sort_by(|a, b| if desc { b.0.cmp(&a.0) } else { a.0.cmp(&b.0) });
                Value::list(keyed.into_iter().map(|(_, x)| x).collect())
            }
            "reversed" if args.is_empty() => Value::list(items.into_iter().rev().collect()),
            "take" | "drop" => match positional.as_slice() {
                [Value::Int(n)] if *n >= 0 => {
                    let n = usize::try_from(*n).unwrap_or(usize::MAX);
                    let out: Vec<Value> = if name == "take" {
                        items.into_iter().take(n).collect()
                    } else {
                        items.into_iter().skip(n).collect()
                    };
                    self.rebuild(if kind == Seq::Set { Seq::List } else { kind }, out)
                }
                _ => return Err(wrong_args(span, name, "one Int, at least 0")),
            },
            "zip" => match positional.as_slice() {
                [Value::List(ys)] if ys.len() == items.len() => Value::list(
                    items.into_iter().zip(ys.iter().cloned()).map(|(a, b)| Value::Tuple(Rc::new(vec![a, b]))).collect(),
                ),
                [Value::List(_)] => return Err(Halt::fail("Uneven")),
                _ => return Err(wrong_args(span, name, "one List")),
            },
            "group_by" => {
                let f = one_fn(name)?;
                let mut groups: BTreeMap<Value, Vec<Value>> = BTreeMap::new();
                for x in items {
                    let k = self.apply(&f, vec![x.clone()], span)?;
                    self.need_comparable(&k, span)?;
                    groups.entry(k).or_default().push(x);
                }
                Value::Map(Rc::new(groups.into_iter().map(|(k, v)| (k, Value::list(v))).collect()))
            }
            "get" if kind == Seq::List => match positional.as_slice() {
                [Value::Int(i)] => self.option(usize::try_from(*i).ok().and_then(|i| items.get(i)).cloned()),
                _ => return Err(wrong_args(span, name, "one Int")),
            },
            "join" if kind == Seq::List => match positional.as_slice() {
                [Value::Text(sep)] => {
                    let mut parts = Vec::new();
                    for x in items {
                        match x {
                            Value::Text(t) => parts.push(t.to_string()),
                            other => {
                                return Err(Diag::error("E-TYPE-MISMATCH", span, "`join` is on List[Text]")
                                    .with_expected("Text", show(self.prog, &other))
                                    .into());
                            }
                        }
                    }
                    Value::text(&parts.join(sep))
                }
                _ => return Err(wrong_args(span, name, "one Text")),
            },
            _ => return Ok(None),
        };
        Ok(Some(v))
    }
}
