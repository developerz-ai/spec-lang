//! `x.name` and `x.name(args)`: resolved by the receiver's type to a field,
//! a world's state, an event's argument, or a primitive.

use super::{R, Typer, id32};
use crate::node::*;
use crate::prim::Prim;
use crate::ty::LTy;
use spec_lang_diag::Diag;
use spec_lang_ir as ir;
use spec_lang_syntax::Span;

/// Which collection a sequence method was called on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Seq {
    List,
    Set,
    Range,
}

impl<'p> Typer<'_, 'p> {
    fn no_member(&self, ty: &LTy, name: &str, span: Span) -> Diag {
        Diag::error("E-TYPE-MISMATCH", span, format!("{} has no `{name}`", self.low.show_ty(&self.zonk(ty))))
            .with_decl(self.decl.clone())
    }

    fn arity(&self, span: Span, name: &str, want: &str) -> Diag {
        Diag::error("E-TYPE-ARITY", span, format!("`{name}` takes {want}")).with_decl(self.decl.clone())
    }

    pub(crate) fn prop(&mut self, recv: &'p ir::Expr, name: &str, span: Span) -> R<Expr> {
        let r = self.expr(recv)?;
        let ty = self.shallow(&r.ty);
        let prog = self.low.prog;
        let p = |op: Prim, ty: LTy, me: &Self, r: Expr| Ok(me.prim(op, vec![r], ty, span));
        match (&ty, name) {
            (LTy::Named(t), _) => match &prog.ty(*t).def {
                ir::TypeDef::Record(fs) => match fs.iter().position(|f| f.name == name) {
                    Some(i) => {
                        let fty = self.low.erase(&fs[i].ty);
                        let kind = ExprKind::Field { recv: Box::new(r), ty: *t, index: id32(i), name: name.into() };
                        Ok(Expr::new(kind, fty, span))
                    }
                    None => Err(self.no_member(&ty, name, span)),
                },
                ir::TypeDef::Sum(vs) if *t != prog.option_type && *t != prog.result_type => {
                    let mut fty = None;
                    let mut by_tag = Vec::new();
                    for v in vs {
                        let i = v.fields.iter().position(|f| f.name == name);
                        if let Some(i) = i {
                            let ft = self.low.erase(&v.fields[i].ty);
                            match &fty {
                                None => fty = Some(ft),
                                Some(prev) => {
                                    let prev = prev.clone();
                                    self.unify(&prev, &ft, span)?;
                                }
                            }
                        }
                        by_tag.push(i.map(id32));
                    }
                    let Some(fty) = fty else { return Err(self.no_member(&ty, name, span)) };
                    let kind = ExprKind::VariantField { recv: Box::new(r), ty: *t, name: name.into(), by_tag };
                    Ok(Expr::new(kind, fty, span))
                }
                _ => Err(self.no_member(&ty, name, span)),
            },
            (LTy::World, _) => {
                let ids: Vec<usize> =
                    prog.states.iter().enumerate().filter(|(_, s)| s.info.name == name).map(|(i, _)| i).collect();
                match ids.as_slice() {
                    [i] => {
                        let s = ir::StateId(id32(*i));
                        let sty = self.low.erase(&prog.state(s).ty);
                        Ok(Expr::new(ExprKind::WorldState { recv: Box::new(r), state: s }, sty, span))
                    }
                    [] => Err(self.no_member(&ty, name, span)),
                    _ => Err(self.unsupported(span, format!("several modules have a state `{name}`"))),
                }
            }
            (LTy::Event(rules), _) => {
                let mut aty: Option<LTy> = None;
                let mut by_rule = Vec::new();
                for rid in rules {
                    let ps = &prog.rule(*rid).params;
                    let i = ps.iter().position(|p| p.name == name);
                    if let Some(i) = i {
                        let t = self.low.erase(&ps[i].ty);
                        match &aty {
                            None => aty = Some(t),
                            Some(prev) => {
                                let prev = prev.clone();
                                self.unify(&prev, &t, span)?;
                            }
                        }
                    }
                    by_rule.push((*rid, i.map(id32)));
                }
                let Some(aty) = aty else { return Err(self.no_member(&ty, name, span)) };
                Ok(Expr::new(ExprKind::EventArg { recv: Box::new(r), name: name.into(), by_rule }, aty, span))
            }
            (LTy::Step(rs), "event") => p(Prim::StepEvent, LTy::Event(rs.clone()), self, r),
            (LTy::Step(rs), "reply") => p(Prim::StepReply, LTy::Reply(rs.clone()), self, r),
            (LTy::Step(_), "before") => p(Prim::StepBefore, LTy::World, self, r),
            (LTy::Step(_), "after") => p(Prim::StepAfter, LTy::World, self, r),
            (LTy::Step(_), "at") => Err(self.unsupported(span, "`at` of a step: the engine has no clock")),
            (LTy::Trace(rs), "steps") => p(Prim::TraceSteps, LTy::list(LTy::Step(rs.clone())), self, r),
            (LTy::Trace(_), "states") => p(Prim::TraceStates, LTy::list(LTy::World), self, r),
            (LTy::Text, "size") => p(Prim::TextSize, LTy::Int, self, r),
            (LTy::Text, "bytes") => p(Prim::TextBytes, LTy::list(LTy::Int), self, r),
            (LTy::List(_), "size") => p(Prim::ListSize, LTy::Int, self, r),
            (LTy::List(t), "first") => p(Prim::ListFirst, (**t).clone(), self, r),
            (LTy::List(t), "last") => p(Prim::ListLast, (**t).clone(), self, r),
            (LTy::Set(_), "size") => p(Prim::SetSize, LTy::Int, self, r),
            (LTy::Set(t), "list") => p(Prim::SetList, LTy::List(t.clone()), self, r),
            (LTy::Map(..), "size") => p(Prim::MapSize, LTy::Int, self, r),
            (LTy::Map(k, _), "keys") => p(Prim::MapKeys, LTy::List(k.clone()), self, r),
            (LTy::Map(_, v), "values") => p(Prim::MapValues, LTy::List(v.clone()), self, r),
            (LTy::Range, "list") => p(Prim::RangeList, LTy::list(LTy::Int), self, r),
            (LTy::Range, "size") => {
                let l = self.prim(Prim::RangeList, vec![r], LTy::list(LTy::Int), span);
                p(Prim::ListSize, LTy::Int, self, l)
            }
            (LTy::Var(_), _) => {
                Err(self.unsupported(span, format!("`.{name}` on a value whose type is not known here")))
            }
            _ => Err(self.no_member(&ty, name, span)),
        }
    }

    pub(crate) fn method(&mut self, recv: &'p ir::Expr, name: &str, args: &'p [ir::Arg], span: Span) -> R<Expr> {
        let r = self.expr(recv)?;
        let ty = self.shallow(&r.ty);
        if name != "sort" && args.iter().any(|a| a.name.is_some()) {
            return Err(self.arity(span, name, "positional arguments"));
        }
        let pos: Vec<&'p ir::Expr> = args.iter().map(|a| &a.value).collect();
        match ty.clone() {
            LTy::Text => self.text_method(r, name, &pos, span),
            LTy::Int => {
                let (op, n) = match name {
                    "abs" => (Prim::IntAbs, 0),
                    "min" => (Prim::IntMin, 1),
                    "max" => (Prim::IntMax, 1),
                    "clamp" => (Prim::IntClamp, 2),
                    _ => return Err(self.no_member(&ty, name, span)),
                };
                if pos.len() != n {
                    return Err(self.arity(span, name, &format!("{n} Int argument(s)")));
                }
                let mut xs = vec![r];
                for a in pos {
                    xs.push(self.expr_as(a, &LTy::Int)?);
                }
                Ok(self.prim(op, xs, LTy::Int, span))
            }
            LTy::Map(k, v) => match (name, pos.as_slice()) {
                ("get", [key]) => {
                    let key = self.expr_as(key, &k)?;
                    Ok(self.prim(Prim::MapGet, vec![r, key], LTy::Option(v), span))
                }
                _ => Err(self.no_member(&ty, name, span)),
            },
            LTy::List(t) => self.seq_method(Seq::List, r, *t, name, args, span),
            LTy::Set(t) => {
                let items = self.prim(Prim::SetList, vec![r], LTy::List(t.clone()), span);
                self.seq_method(Seq::Set, items, *t, name, args, span)
            }
            LTy::Range => {
                let items = self.prim(Prim::RangeList, vec![r], LTy::list(LTy::Int), span);
                self.seq_method(Seq::Range, items, LTy::Int, name, args, span)
            }
            LTy::Trace(_) if matches!(name, "eventually" | "within") => {
                Err(self.unsupported(span, format!("`t.{name}`: liveness over traces")))
            }
            LTy::Var(_) => {
                Err(self.unsupported(span, format!("`.{name}(..)` on a value whose type is not known here")))
            }
            _ => Err(self.no_member(&ty, name, span)),
        }
    }

    fn text_method(&mut self, r: Expr, name: &str, pos: &[&'p ir::Expr], span: Span) -> R<Expr> {
        let (op, arg_tys, ty): (Prim, &[LTy], LTy) = match name {
            "trim" => (Prim::TextTrim, &[], LTy::Text),
            "lower" => (Prim::TextLower, &[], LTy::Text),
            "starts_with" => (Prim::TextStartsWith, &[LTy::Text], LTy::Bool),
            "contains" => (Prim::TextContains, &[LTy::Text], LTy::Bool),
            "split" => (Prim::TextSplit, &[LTy::Text], LTy::list(LTy::Text)),
            "replace" => (Prim::TextReplace, &[LTy::Text, LTy::Text], LTy::Text),
            "take" => (Prim::TextTake, &[LTy::Int], LTy::Text),
            "drop" => (Prim::TextDrop, &[LTy::Int], LTy::Text),
            _ => return Err(self.no_member(&LTy::Text, name, span)),
        };
        if pos.len() != arg_tys.len() {
            return Err(self.arity(span, name, &format!("{} argument(s)", arg_tys.len())));
        }
        let mut xs = vec![r];
        for (a, t) in pos.iter().zip(arg_tys) {
            xs.push(self.expr_as(a, t)?);
        }
        Ok(self.prim(op, xs, ty, span))
    }

    /// A List, Set or range method over `items` (a `List[t]`).
    fn seq_method(&mut self, kind: Seq, items: Expr, t: LTy, name: &str, args: &'p [ir::Arg], span: Span) -> R<Expr> {
        let pos: Vec<&'p ir::Expr> = args.iter().filter(|a| a.name.is_none()).map(|a| &a.value).collect();
        let list_t = LTy::list(t.clone());
        // `filter` and `map` of a Set give a Set.
        let rebuild = |me: &Self, e: Expr| -> Expr {
            if kind == Seq::Set {
                let LTy::List(x) = e.ty.clone() else { return e };
                me.prim(Prim::SetOfList, vec![e], LTy::Set(x), span)
            } else {
                e
            }
        };
        let one = |me: &Self| -> R<&'p ir::Expr> {
            match pos.as_slice() {
                [x] => Ok(*x),
                _ => Err(me.arity(span, name, "one argument")),
            }
        };
        let none =
            |me: &Self| -> R<()> { if pos.is_empty() { Ok(()) } else { Err(me.arity(span, name, "no arguments")) } };
        let e = match name {
            "filter" => {
                let f = self.fn_arg(one(self)?, vec![t.clone()])?;
                self.unify_result(&f, &LTy::Bool, span)?;
                let x = self.prim(Prim::ListFilter, vec![items, f], list_t, span);
                rebuild(self, x)
            }
            "map" => {
                let f = self.fn_arg(one(self)?, vec![t.clone()])?;
                let u = self.result_of(&f);
                let x = self.prim(Prim::ListMap, vec![items, f], LTy::list(u), span);
                rebuild(self, x)
            }
            "flat_map" => {
                let f = self.fn_arg(one(self)?, vec![t.clone()])?;
                let u = self.fresh();
                self.unify_result(&f, &LTy::list(u.clone()), span)?;
                self.prim(Prim::ListFlatMap, vec![items, f], LTy::list(u), span)
            }
            "fold" => {
                let [start, f] = pos.as_slice() else {
                    return Err(self.arity(span, name, "a start value and a function"));
                };
                let start = self.expr(start)?;
                let acc = start.ty.clone();
                let f = self.fn_arg(f, vec![acc.clone(), t.clone()])?;
                self.unify_result(&f, &acc, span)?;
                self.prim(Prim::ListFold, vec![items, start, f], acc, span)
            }
            "all" | "any" => {
                let f = self.fn_arg(one(self)?, vec![t.clone()])?;
                self.unify_result(&f, &LTy::Bool, span)?;
                let op = if name == "all" { Prim::ListAll } else { Prim::ListAny };
                self.prim(op, vec![items, f], LTy::Bool, span)
            }
            "find" | "position" => {
                let f = self.fn_arg(one(self)?, vec![t.clone()])?;
                self.unify_result(&f, &LTy::Bool, span)?;
                if name == "find" {
                    self.prim(Prim::ListFind, vec![items, f], LTy::option(t), span)
                } else {
                    self.prim(Prim::ListPosition, vec![items, f], LTy::option(LTy::Int), span)
                }
            }
            "sum" => match pos.as_slice() {
                [] => {
                    self.unify(&t, &LTy::Int, span)?;
                    self.prim(Prim::ListSum, vec![items], LTy::Int, span)
                }
                [f] => {
                    let f = self.fn_arg(f, vec![t])?;
                    self.unify_result(&f, &LTy::Int, span)?;
                    self.prim(Prim::ListSumBy, vec![items, f], LTy::Int, span)
                }
                _ => return Err(self.arity(span, name, "at most one function")),
            },
            "min" | "max" => match pos.as_slice() {
                [] => {
                    let op = if name == "min" { Prim::ListMin } else { Prim::ListMax };
                    self.prim(op, vec![items], t, span)
                }
                [f] => {
                    let f = self.fn_arg(f, vec![t.clone()])?;
                    let op = if name == "min" { Prim::ListMinBy } else { Prim::ListMaxBy };
                    self.prim(op, vec![items, f], t, span)
                }
                _ => return Err(self.arity(span, name, "at most one function")),
            },
            "contains" => {
                let x = self.expr_as(one(self)?, &t)?;
                self.prim(Prim::ListContains, vec![items, x], LTy::Bool, span)
            }
            "distinct" => {
                none(self)?;
                self.prim(Prim::ListDistinct, vec![items], list_t, span)
            }
            "reversed" => {
                none(self)?;
                self.prim(Prim::ListReversed, vec![items], list_t, span)
            }
            "sort" => return self.sort(items, t, args, span),
            "take" | "drop" => {
                let n = self.expr_as(one(self)?, &LTy::Int)?;
                let op = if name == "take" { Prim::ListTake } else { Prim::ListDrop };
                self.prim(op, vec![items, n], list_t, span)
            }
            "zip" => {
                let ys = self.expr(one(self)?)?;
                let u = self.fresh();
                self.unify(&ys.ty, &LTy::list(u.clone()), span)?;
                self.prim(Prim::ListZip, vec![items, ys], LTy::list(LTy::Tuple(vec![t, u])), span)
            }
            "group_by" => {
                let f = self.fn_arg(one(self)?, vec![t.clone()])?;
                let k = self.result_of(&f);
                self.prim(Prim::ListGroupBy, vec![items, f], LTy::map(k, list_t), span)
            }
            "get" if kind == Seq::List => {
                let i = self.expr_as(one(self)?, &LTy::Int)?;
                self.prim(Prim::ListGet, vec![items, i], LTy::option(t), span)
            }
            "join" if kind == Seq::List => {
                self.unify(&t, &LTy::Text, span)?;
                let sep = self.expr_as(one(self)?, &LTy::Text)?;
                self.prim(Prim::ListJoin, vec![items, sep], LTy::Text, span)
            }
            _ => {
                let recv = match kind {
                    Seq::List => LTy::list(t),
                    Seq::Set => LTy::set(t),
                    Seq::Range => LTy::Range,
                };
                return Err(self.no_member(&recv, name, span));
            }
        };
        Ok(e)
    }

    /// `sort`, `sort(key)`, `sort(key: f, desc: true)`.
    fn sort(&mut self, items: Expr, t: LTy, args: &'p [ir::Arg], span: Span) -> R<Expr> {
        let mut key = None;
        let mut desc = None;
        for a in args {
            match a.name.as_deref() {
                None | Some("key") if key.is_none() => key = Some(&a.value),
                Some("desc") if desc.is_none() => desc = Some(&a.value),
                _ => return Err(self.arity(span, "sort", "a key function and `desc: Bool`")),
            }
        }
        let desc = match desc {
            Some(d) => self.expr_as(d, &LTy::Bool)?,
            None => Expr::new(ExprKind::Bool(false), LTy::Bool, span),
        };
        let ty = LTy::list(t.clone());
        Ok(match key {
            Some(k) => {
                let f = self.fn_arg(k, vec![t])?;
                let key = self.result_of(&f);
                if !self.zonk(&key).comparable() {
                    return Err(self.unsupported(span, "a sort key that is a function"));
                }
                self.prim(Prim::ListSortBy, vec![items, f, desc], ty, span)
            }
            None => self.prim(Prim::ListSort, vec![items, desc], ty, span),
        })
    }

    fn result_of(&mut self, f: &Expr) -> LTy {
        match self.shallow(&f.ty) {
            LTy::Func(_, r) => *r,
            _ => self.fresh(),
        }
    }

    fn unify_result(&mut self, f: &Expr, want: &LTy, span: Span) -> R<()> {
        let r = self.result_of(f);
        self.unify(&r, want, span)
    }
}
