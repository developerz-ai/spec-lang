//! Defaults and constants are of their declared types, claims included.

use spec_lang_diag::Diag;
use spec_lang_engine::display::{show, show_ty};
use spec_lang_engine::{Halt, Interp};
use spec_lang_ir::*;

pub(crate) fn check_defaults(p: &Program) -> Vec<Diag> {
    let mut out = Vec::new();
    let Ok(mut it) = Interp::new(p) else {
        return out; // the test stage reports a world that cannot start
    };
    let mut one = |it: &mut Interp<'_>, decl: &DeclInfo, what: &str, e: &Expr, ty: &Ty| {
        let d = match it.eval_closed(e) {
            Ok(v) => match it.conform(&v, ty) {
                Ok(None) => None,
                Ok(Some(reason)) => Some(
                    Diag::error("E-TYPE-MISMATCH", e.span, format!("{what} is not a {}: {reason}", show_ty(p, ty)))
                        .with_expected(show_ty(p, ty), show(p, &v)),
                ),
                Err(h) => halt(h, e.span),
            },
            // A default that reads parameters cannot be evaluated alone; the test stage checks it on use.
            Err(Halt::Fatal(_)) if mentions_local(p, e) => None,
            Err(h) => halt(h, e.span),
        };
        if let Some(d) = d {
            out.push(d.with_decl(p.qualified(decl)));
        }
    };
    for t in &p.types {
        if let TypeDef::Record(fields) = &t.def {
            for f in fields {
                if let Some(d) = &f.default {
                    one(&mut it, &t.info, &format!("the default of field `{}`", f.name), d, &f.ty);
                }
            }
        }
    }
    let params = p
        .fns
        .iter()
        .map(|f| (&f.info, &f.params))
        .chain(p.rules.iter().map(|r| (&r.info, &r.params)))
        .chain(p.tests.iter().map(|t| (&t.info, &t.params)));
    for (info, ps) in params {
        for prm in ps.iter() {
            if let Some(d) = &prm.default {
                one(&mut it, info, &format!("the default of `{}`", prm.name), d, &prm.ty);
            }
        }
    }
    for c in &p.consts {
        if let Some(ty) = &c.ty {
            one(&mut it, &c.info, "the constant", &c.value, ty);
        }
    }
    out
}

/// A value that could not be computed; `None` when it reaches a `foreign`
/// body, which only a backend runs: the test stage reports each use blocked.
fn halt(h: Halt, span: spec_lang_syntax::Span) -> Option<Diag> {
    match h {
        Halt::Fatal(d) | Halt::Check(d) => Some(*d),
        Halt::Fail(f) => Some(Diag::error("E-TYPE-MISMATCH", span, format!("evaluating it failed with {}", f.name))),
        Halt::Blocked(d) if d.code == "B-FOREIGN" => None,
        Halt::Blocked(d) => Some(Diag::error("E-TYPE-MISMATCH", d.span.unwrap_or(span), format!("it {}", d.message))),
    }
}

fn mentions_local(p: &Program, e: &Expr) -> bool {
    let mut found = false;
    spec_lang_ir::visit::walk_expr(p, e, &mut |x| found |= matches!(x.kind, ExprKind::Local(_)));
    found
}
