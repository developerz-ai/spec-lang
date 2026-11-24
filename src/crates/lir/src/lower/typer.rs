//! Types of one declaration's expressions: unknowns, unification, locals.

use super::Lowerer;
use crate::ty::LTy;
use spec_lang_diag::Diag;
use spec_lang_syntax::Span;

pub(crate) type R<T> = Result<T, Diag>;

/// A lambda being lowered: the scope depth where it starts, and the outer
/// locals it reads.
#[derive(Debug)]
struct Frame {
    base: usize,
    captures: Vec<(String, LTy)>,
}

/// Lowers the expressions of one declaration.
#[derive(Debug)]
pub(crate) struct Typer<'l, 'p> {
    pub(crate) low: &'l mut Lowerer<'p>,
    pub(crate) decl: String,
    subst: Vec<Option<LTy>>,
    scopes: Vec<(String, LTy)>,
    frames: Vec<Frame>,
    /// What a `return` gives, innermost block last.
    pub(crate) returns: Vec<LTy>,
}

impl<'l, 'p> Typer<'l, 'p> {
    pub(crate) fn new(low: &'l mut Lowerer<'p>, decl: String) -> Self {
        Typer { low, decl, subst: Vec::new(), scopes: Vec::new(), frames: Vec::new(), returns: Vec::new() }
    }

    pub(crate) fn fresh(&mut self) -> LTy {
        self.subst.push(None);
        LTy::Var(u32::try_from(self.subst.len() - 1).unwrap_or(u32::MAX))
    }

    /// Follow solved unknowns at the top.
    pub(crate) fn shallow(&self, t: &LTy) -> LTy {
        let mut t = t.clone();
        while let LTy::Var(v) = t {
            match self.subst.get(v as usize).and_then(Clone::clone) {
                Some(x) => t = x,
                None => break,
            }
        }
        t
    }

    /// Replace every solved unknown, deeply.
    pub(crate) fn zonk(&self, t: &LTy) -> LTy {
        match self.shallow(t) {
            LTy::Option(x) => LTy::option(self.zonk(&x)),
            LTy::List(x) => LTy::list(self.zonk(&x)),
            LTy::Set(x) => LTy::set(self.zonk(&x)),
            LTy::Map(k, v) => LTy::map(self.zonk(&k), self.zonk(&v)),
            LTy::Tuple(ts) => LTy::Tuple(ts.iter().map(|x| self.zonk(x)).collect()),
            LTy::Func(ps, r) => LTy::func(ps.iter().map(|x| self.zonk(x)).collect(), self.zonk(&r)),
            other => other,
        }
    }

    /// Set an unknown that is still open.
    pub(crate) fn solve(&mut self, v: &LTy, t: LTy) {
        if let LTy::Var(i) = self.shallow(v) {
            self.subst[i as usize] = Some(t);
        }
    }

    pub(crate) fn mismatch(&self, span: Span, want: &LTy, found: &LTy) -> Diag {
        Diag::error("E-TYPE-MISMATCH", span, "a value is not of the type this place needs")
            .with_decl(self.decl.clone())
            .with_expected(self.low.show_ty(&self.zonk(want)), self.low.show_ty(&self.zonk(found)))
    }

    pub(crate) fn unsupported(&self, span: Span, what: impl std::fmt::Display) -> Diag {
        Diag::unsupported(span, what).with_decl(self.decl.clone())
    }

    fn occurs(&self, v: u32, t: &LTy) -> bool {
        match self.shallow(t) {
            LTy::Var(w) => v == w,
            LTy::Option(x) | LTy::List(x) | LTy::Set(x) => self.occurs(v, &x),
            LTy::Map(k, x) => self.occurs(v, &k) || self.occurs(v, &x),
            LTy::Tuple(ts) => ts.iter().any(|x| self.occurs(v, x)),
            LTy::Func(ps, r) => ps.iter().any(|x| self.occurs(v, x)) || self.occurs(v, &r),
            _ => false,
        }
    }

    /// Make two types one; `Never` fits anything.
    pub(crate) fn unify(&mut self, a: &LTy, b: &LTy, span: Span) -> R<()> {
        let (x, y) = (self.shallow(a), self.shallow(b));
        match (&x, &y) {
            (LTy::Never, _) | (_, LTy::Never) => Ok(()),
            (LTy::Var(v), LTy::Var(w)) if v == w => Ok(()),
            (LTy::Var(v), t) | (t, LTy::Var(v)) => {
                if self.occurs(*v, t) {
                    return Err(self.mismatch(span, &x, &y));
                }
                self.subst[*v as usize] = Some(t.clone());
                Ok(())
            }
            (LTy::Option(p), LTy::Option(q)) | (LTy::List(p), LTy::List(q)) | (LTy::Set(p), LTy::Set(q)) => {
                self.unify(p, q, span)
            }
            (LTy::Map(k1, v1), LTy::Map(k2, v2)) => {
                self.unify(k1, k2, span)?;
                self.unify(v1, v2, span)
            }
            (LTy::Tuple(p), LTy::Tuple(q)) if p.len() == q.len() => {
                for (s, t) in p.iter().zip(q) {
                    self.unify(s, t, span)?;
                }
                Ok(())
            }
            (LTy::Func(p1, r1), LTy::Func(p2, r2)) if p1.len() == p2.len() => {
                for (s, t) in p1.iter().zip(p2) {
                    self.unify(s, t, span)?;
                }
                self.unify(r1, r2, span)
            }
            (p, q) if p == q => Ok(()),
            _ => Err(self.mismatch(span, &x, &y)),
        }
    }

    /// Unify, and give the more informative of the two (not `Never`).
    pub(crate) fn join(&mut self, a: &LTy, b: &LTy, span: Span) -> R<LTy> {
        self.unify(a, b, span)?;
        Ok(if matches!(self.shallow(a), LTy::Never) { b.clone() } else { a.clone() })
    }

    // ---- locals ----

    pub(crate) fn mark(&self) -> usize {
        self.scopes.len()
    }

    pub(crate) fn reset(&mut self, mark: usize) {
        self.scopes.truncate(mark);
    }

    pub(crate) fn bind(&mut self, name: &str, ty: LTy) {
        self.scopes.push((name.to_string(), ty));
    }

    /// A local's type; a lambda that reads an outer local captures it.
    pub(crate) fn local(&mut self, name: &str) -> Option<LTy> {
        let i = self.scopes.iter().rposition(|(n, _)| n == name)?;
        let ty = self.scopes[i].1.clone();
        for f in self.frames.iter_mut().filter(|f| f.base > i) {
            if !f.captures.iter().any(|(n, _)| n == name) {
                f.captures.push((name.to_string(), ty.clone()));
            }
        }
        Some(ty)
    }

    pub(crate) fn open_lambda(&mut self) {
        self.frames.push(Frame { base: self.scopes.len(), captures: Vec::new() });
    }

    pub(crate) fn close_lambda(&mut self) -> Vec<(String, LTy)> {
        self.frames.pop().map(|f| f.captures).unwrap_or_default()
    }
}
