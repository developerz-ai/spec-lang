//! Lowering type expressions.

use super::Lowerer;
use super::declare::flatten_bar;
use super::expr::{Body, Cx, R};
use super::scope::Binding;
use crate::error::IrError;
use crate::expr::UnOp;
use crate::ids::*;
use crate::program::TypeDef;
use crate::ty::{Claim, Ty};
use spec_lang_syntax::ast::{self, BinOp as AB, ExprKind as AK};

impl Lowerer {
    pub(crate) fn lower_ty(&mut self, m: ModuleId, e: &ast::Expr) -> R<Ty> {
        match &e.kind {
            AK::Name(n) => match n.as_str() {
                "Int" => Ok(Ty::Int),
                "Bool" => Ok(Ty::Bool),
                "Text" => Ok(Ty::Text),
                _ => self.named_ty(m, e),
            },
            AK::Field(..) => self.named_ty(m, e),
            AK::Index(base, args) => {
                let name = base.as_name().unwrap_or("");
                if name == "Trace" {
                    let [arg] = args.as_slice() else {
                        return Err(IrError::error(
                            "E-TYPE-ARITY",
                            e.span,
                            "`Trace[...]` takes one rule set: `Trace[a | b]`",
                        ));
                    };
                    return Ok(Ty::Trace(self.rule_set(m, &arg.value)?));
                }
                let mut tys = Vec::new();
                for a in args {
                    tys.push(self.lower_ty(m, &a.value)?);
                }
                let arity = |n: usize| {
                    if tys.len() == n {
                        Ok(())
                    } else {
                        Err(IrError::error("E-TYPE-ARITY", e.span, format!("`{name}` takes {n} type argument(s)")))
                    }
                };
                let mut it = tys.clone().into_iter();
                let mut next = || Box::new(it.next().unwrap_or(Ty::Unit));
                match name {
                    "List" => arity(1).map(|()| Ty::List(next())),
                    "Set" => arity(1).map(|()| Ty::Set(next())),
                    "Option" => arity(1).map(|()| Ty::Option(next())),
                    "Map" => arity(2).map(|()| {
                        let k = next();
                        Ty::Map(k, next())
                    }),
                    _ => Err(IrError::unsupported(e.span, format!("the type `{}`", self.prog.snippet(e.span)))),
                }
            }
            AK::Group { args, .. } if args.is_empty() => Ok(Ty::Unit),
            AK::Group { args, trailing_comma } if args.len() == 1 && !trailing_comma => {
                self.lower_ty(m, &args[0].value)
            }
            AK::Group { args, .. } => {
                let mut tys = Vec::new();
                for a in args {
                    tys.push(self.lower_ty(m, &a.value)?);
                }
                Ok(Ty::Tuple(tys))
            }
            AK::Binary(AB::Where, base, claim) => {
                let base = self.lower_ty(m, base)?;
                let mut cx = Cx::new(m, Body::Claim);
                self.push_local(&mut cx, "self", false);
                let expr = self.lower_expr(&mut cx, claim)?;
                Ok(Ty::Where(Box::new(base), Box::new(Claim { expr, module: m })))
            }
            AK::Binary(op @ (AB::RangeIncl | AB::RangeExcl), lo, hi) => Ok(Ty::Range {
                lo: const_int(lo)
                    .ok_or_else(|| IrError::unsupported(lo.span, "a range type bound that is not a whole number"))?,
                hi: const_int(hi)
                    .ok_or_else(|| IrError::unsupported(hi.span, "a range type bound that is not a whole number"))?,
                inclusive: *op == AB::RangeIncl,
            }),
            _ => Err(IrError::unsupported(e.span, format!("the type `{}`", self.prog.snippet(e.span)))),
        }
    }

    fn named_ty(&mut self, m: ModuleId, e: &ast::Expr) -> R<Ty> {
        let cx = Cx::new(m, Body::Value);
        match self.binding_of(&cx, e) {
            Some(Ok(Binding::Decl(DeclRef::Type(t)))) => Ok(Ty::Named(t)),
            Some(Ok(_)) => {
                Err(IrError::error("E-NAME-KIND", e.span, format!("`{}` is not a type", self.prog.snippet(e.span))))
            }
            Some(Err(d)) => Err(d),
            None => match e.as_name() {
                Some(n) => Err(self.unknown_name(&cx, n, e.span)),
                None => Err(IrError::unsupported(e.span, "this type")),
            },
        }
    }

    /// `a | b.c | Change`: rules, and rule-set types, flattened.
    pub(crate) fn rule_set(&mut self, m: ModuleId, e: &ast::Expr) -> R<Vec<RuleId>> {
        let cx = Cx::new(m, Body::Value);
        let mut alts = Vec::new();
        flatten_bar(e, &mut alts);
        let mut out = Vec::new();
        for a in alts {
            match self.binding_of(&cx, a) {
                Some(Ok(Binding::Decl(DeclRef::Rule(r)))) => out.push(r),
                Some(Ok(Binding::Decl(DeclRef::Type(t)))) => match &self.prog.ty(t).def {
                    TypeDef::Rules(rs) => {
                        out.extend(rs.iter().copied());
                        self.prog.rule_set_uses.push(t);
                    }
                    _ => {
                        return Err(IrError::error(
                            "E-NAME-KIND",
                            a.span,
                            "a trace names rules or rule sets, not a type of values",
                        ));
                    }
                },
                Some(Ok(_)) => return Err(IrError::error("E-NAME-KIND", a.span, "not a rule")),
                Some(Err(d)) => return Err(d),
                None => match a.as_name() {
                    Some(n) => return Err(self.unknown_name(&cx, n, a.span)),
                    None => return Err(IrError::unsupported(a.span, "this member of a rule set")),
                },
            }
        }
        out.dedup();
        Ok(out)
    }
}

/// A whole-number literal, possibly negated.
fn const_int(e: &ast::Expr) -> Option<i128> {
    match &e.kind {
        AK::Num(s) => super::expr::parse_int(s),
        AK::Unary(UnOp::Neg, x) => const_int(x).map(|n| -n),
        _ => None,
    }
}
