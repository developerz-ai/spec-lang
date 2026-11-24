//! Patterns to explicit tests and bindings, in the engine's order: depth
//! first, left to right.

use super::{R, Typer, id32};
use crate::node::*;
use crate::ty::LTy;
use spec_lang_diag::Diag;
use spec_lang_ir as ir;
use spec_lang_ir::Pattern as P;
use spec_lang_syntax::Span;

impl<'p> Typer<'_, 'p> {
    /// Compile `p` against a value of type `ty`; bindings are added to scope.
    pub(crate) fn pattern(&mut self, p: &'p ir::Pattern, ty: &LTy, span: Span) -> R<Pat> {
        let mut pat = Pat::default();
        self.pat_at(p, ty, &[], span, &mut pat)?;
        Ok(pat)
    }

    fn pat_at(&mut self, p: &'p ir::Pattern, ty: &LTy, path: &[PathStep], span: Span, out: &mut Pat) -> R<()> {
        let prog = self.low.prog;
        let at = |s: PathStep| -> Vec<PathStep> {
            let mut v = path.to_vec();
            v.push(s);
            v
        };
        match p {
            P::Wild => Ok(()),
            P::Bind(n) => {
                self.bind(n, ty.clone());
                out.steps.push(PatStep::Bind { name: n.clone(), path: path.to_vec(), ty: ty.clone() });
                Ok(())
            }
            P::Value(e) => {
                let v = self.expr_as(e, ty)?;
                out.steps.push(PatStep::Equals { path: path.to_vec(), value: v });
                Ok(())
            }
            P::Variant { ty: t, tag, args } => {
                // The fields of this variant, each with its type at this use.
                let fields: Vec<(String, LTy)> = if *t == prog.option_type {
                    let inner = self.fresh();
                    self.unify(ty, &LTy::option(inner.clone()), span)?;
                    if *tag == ir::SOME_TAG { vec![("value".into(), inner)] } else { Vec::new() }
                } else if *t == prog.result_type {
                    let LTy::Reply(rules) = self.shallow(ty) else {
                        return Err(
                            self.unsupported(span, "`Ok(..)` or `Err(..)` on a value that is not a trace reply")
                        );
                    };
                    if *tag == ir::OK_TAG && args.iter().all(|a| matches!(a.pattern, P::Wild)) {
                        // Nothing reads the payload: the rules' results need not agree.
                        vec![("value".into(), LTy::Never)]
                    } else if *tag == ir::OK_TAG {
                        let payload = self.fresh();
                        for r in &rules {
                            let rt = self.low.rule_result(*r)?;
                            self.unify(&payload, &rt, span)?;
                        }
                        vec![("value".into(), payload)]
                    } else {
                        vec![("error".into(), LTy::Failure)]
                    }
                } else {
                    self.unify(ty, &LTy::Named(*t), span)?;
                    let ir::TypeDef::Sum(vs) = &prog.ty(*t).def else {
                        return Err(self.unsupported(span, "a variant pattern on a type that is not a sum"));
                    };
                    let Some(v) = vs.get(*tag as usize) else {
                        return Err(self.unsupported(span, "no such variant"));
                    };
                    v.fields.iter().map(|f| (f.name.clone(), self.low.erase(&f.ty))).collect()
                };
                out.steps.push(PatStep::Tag { path: path.to_vec(), ty: *t, tag: *tag });
                self.pat_args(args, &fields, span, out, |i| at(PathStep::VariantField { ty: *t, tag: *tag, index: i }))
            }
            P::Tuple(ps) => {
                let items: Vec<LTy> = match self.shallow(ty) {
                    LTy::Tuple(ts) => ts,
                    _ => {
                        let ts: Vec<LTy> = ps.iter().map(|_| self.fresh()).collect();
                        self.unify(ty, &LTy::Tuple(ts.clone()), span)?;
                        ts
                    }
                };
                if items.len() != ps.len() {
                    out.steps.push(PatStep::Never);
                    return Ok(());
                }
                for (i, (q, t)) in ps.iter().zip(items).enumerate() {
                    self.pat_at(q, &t, &at(PathStep::TupleItem(id32(i))), span, out)?;
                }
                Ok(())
            }
            P::Call { rule, args } => {
                let LTy::Event(_) = self.shallow(ty) else {
                    return Err(Diag::error("E-TYPE-MISMATCH", span, "a call pattern matches a trace event")
                        .with_decl(self.decl.clone()));
                };
                let fields: Vec<(String, LTy)> =
                    prog.rule(*rule).params.iter().map(|p| (p.name.clone(), self.low.erase(&p.ty))).collect();
                out.steps.push(PatStep::Event { path: path.to_vec(), rule: *rule });
                self.pat_args(args, &fields, span, out, |i| at(PathStep::EventArg { rule: *rule, index: i }))
            }
        }
    }

    /// Positional arguments match fields in order, named ones by name; a
    /// field the variant lacks never matches.
    fn pat_args(
        &mut self,
        args: &'p [ir::PatArg],
        fields: &[(String, LTy)],
        span: Span,
        out: &mut Pat,
        at: impl Fn(u32) -> Vec<PathStep>,
    ) -> R<()> {
        for (i, a) in args.iter().enumerate() {
            let idx = match &a.name {
                Some(n) => fields.iter().position(|(f, _)| f == n),
                None => Some(i).filter(|i| *i < fields.len()),
            };
            let Some(idx) = idx else {
                out.steps.push(PatStep::Never);
                return Ok(());
            };
            let path = at(id32(idx));
            self.pat_at(&a.pattern, &fields[idx].1, &path, span, out)?;
        }
        Ok(())
    }
}
