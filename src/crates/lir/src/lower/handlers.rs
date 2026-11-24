//! Library handlers and their pretends.

use super::{Lowerer, R, Typer, decls};
use crate::program::*;
use crate::ty::LTy;
use spec_lang_ir as ir;

pub(super) fn lower_handler(low: &mut Lowerer<'_>, id: ir::HandlerId) -> R<Handler> {
    let prog = low.prog;
    let h = prog.handler(id);
    let decl = format!("handler {}.{}", h.library, h.name);
    let mut ops = Vec::new();
    for o in &h.ops {
        let params = match &o.params {
            Some(ps) => {
                let mut t = Typer::new(low, decl.clone());
                Some(decls::params(&mut t, ps)?)
            }
            None => None,
        };
        let kind = match o.kind {
            ir::OpKind::Rule => OpKind::Rule,
            ir::OpKind::Fn => OpKind::Fn,
            ir::OpKind::Test => OpKind::Test,
        };
        let result = o.result.as_ref().map_or(LTy::Unit, |t| low.erase(t));
        ops.push(HandlerOp { name: o.name.clone(), kind, params, result, uses: o.uses.clone() });
    }
    let pretend = match &h.pretend {
        None => None,
        Some(p) => {
            let mut t = Typer::new(low, format!("{decl}: pretend {}", p.name));
            let state = decls::params(&mut t, &p.state)?;
            let mut arms = Vec::new();
            for (i, arm) in p.arms.iter().enumerate() {
                let Some(arm) = arm else {
                    arms.push(None);
                    continue;
                };
                let op = &ops[i];
                let tys: Vec<LTy> = op.params.iter().flatten().map(|p| p.ty.clone()).collect();
                let mark = t.mark();
                let params: Vec<(String, LTy)> = arm.params.iter().cloned().zip(tys).collect();
                for (n, ty) in &params {
                    t.bind(n, ty.clone());
                }
                let body = if op.kind == OpKind::Rule { t.expr(&arm.body) } else { t.expr_as(&arm.body, &op.result) };
                t.reset(mark);
                let mut body = body?;
                t.finish_expr(&mut body);
                arms.push(Some(PretendArm { params, body }));
            }
            Some(Pretend { name: p.name.clone(), state, arms })
        }
    };
    Ok(Handler { name: h.name.clone(), library: h.library.clone(), labels: h.labels.clone(), ops, pretend })
}
