//! Lowering patterns. A name already in scope compares; a new one binds.

use super::Lowerer;
use super::expr::{Core, Cx, Named, R};
use super::scope::Binding;
use crate::error::IrError;
use crate::expr::*;
use crate::ids::DeclRef;
use crate::program::NONE_TAG;
use spec_lang_syntax::ast::{self, ExprKind as AK, UnOp as AU};

impl Lowerer {
    /// Lower a pattern; names it binds are pushed on `cx` and listed in `binds`.
    pub(crate) fn lower_pattern(&mut self, cx: &mut Cx, e: &ast::Expr, binds: &mut Vec<String>) -> R<Pattern> {
        match &e.kind {
            AK::Name(n) if n == "_" => Ok(Pattern::Wild),
            AK::Name(n) => match self.resolve(cx, n) {
                Ok(Named::Local) => Ok(Pattern::Value(Expr::new(ExprKind::Local(n.clone()), e.span))),
                Ok(Named::Bind(Binding::Variant(ty, tag))) => Ok(Pattern::Variant { ty, tag, args: Vec::new() }),
                Ok(Named::Bind(Binding::Decl(DeclRef::Const(_)))) => Ok(Pattern::Value(self.lower_expr(cx, e)?)),
                Ok(Named::Core(Core::True | Core::False)) => Ok(Pattern::Value(self.lower_expr(cx, e)?)),
                Ok(Named::Core(Core::None)) => {
                    Ok(Pattern::Variant { ty: self.prog.option_type, tag: NONE_TAG, args: Vec::new() })
                }
                Ok(_) => Err(IrError::error("E-NAME-KIND", e.span, format!("`{n}` cannot be a pattern"))),
                Err(_) => {
                    self.push_local(cx, n, false);
                    binds.push(n.clone());
                    Ok(Pattern::Bind(n.clone()))
                }
            },
            AK::Num(_) | AK::Text(_) => Ok(Pattern::Value(self.lower_expr(cx, e)?)),
            AK::Unary(AU::Neg, x) if matches!(x.kind, AK::Num(_)) => Ok(Pattern::Value(self.lower_expr(cx, e)?)),
            AK::Group { args, .. } if args.len() == 1 && args[0].name.is_none() => {
                self.lower_pattern(cx, &args[0].value, binds)
            }
            AK::Group { args, .. } => {
                let mut out = Vec::new();
                for a in args {
                    out.push(self.lower_pattern(cx, &a.value, binds)?);
                }
                Ok(Pattern::Tuple(out))
            }
            AK::Fail(x) => Err(self.fail_pattern(e, x.span)),
            AK::Call(callee, args) => {
                if callee.as_name() == Some("fail") && !cx.is_local("fail") {
                    return Err(self.fail_pattern(e, callee.span));
                }
                let target = match &callee.kind {
                    AK::Name(n) => match self.resolve(cx, n) {
                        Ok(Named::Core(k @ (Core::Some | Core::Ok | Core::Err))) => match self.core_variant(k) {
                            Callee::Variant(t, tag) => Some(Ok(Binding::Variant(t, tag))),
                            _ => None,
                        },
                        _ => self.binding_of(cx, callee),
                    },
                    _ => self.binding_of(cx, callee),
                };
                match target {
                    Some(Ok(Binding::Variant(ty, tag))) => {
                        Ok(Pattern::Variant { ty, tag, args: self.pat_args(cx, args, binds)? })
                    }
                    Some(Ok(Binding::Decl(DeclRef::Rule(rule)))) => {
                        Ok(Pattern::Call { rule, args: self.pat_args(cx, args, binds)? })
                    }
                    Some(Err(d)) => Err(d),
                    _ => Err(IrError::unsupported(
                        e.span,
                        "this pattern: a variant, a rule call, `fail(...)`, a literal or a name",
                    )),
                }
            }
            _ => Err(IrError::unsupported(
                e.span,
                "this pattern: a variant, a rule call, `fail(...)`, a literal or a name",
            )),
        }
    }

    fn pat_args(&mut self, cx: &mut Cx, args: &[ast::Arg], binds: &mut Vec<String>) -> R<Vec<PatArg>> {
        let mut out = Vec::new();
        for a in args {
            out.push(PatArg {
                name: a.name.as_ref().map(|n| n.name.clone()),
                pattern: self.lower_pattern(cx, &a.value, binds)?,
            });
        }
        Ok(out)
    }

    /// `fail(p)` is not a pattern: a reply is `Ok(v)` or `Err(e)` (card, Traces).
    fn fail_pattern(&self, e: &ast::Expr, fail_word: spec_lang_syntax::Span) -> IrError {
        let text = self.prog.snippet(e.span);
        let fixed = text.strip_prefix("fail").map(|rest| format!("Err{rest}")).unwrap_or_else(|| "Err(_)".into());
        IrError::error("E-NAME-KIND", fail_word, "`fail(...)` is not a pattern: a refused reply is `Err(_)`")
            .with_edit(e.span, fixed)
    }
}
