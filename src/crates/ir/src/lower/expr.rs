//! Lowering expressions: name resolution, calls with bound arguments,
//! implicit `it` lambdas.

use super::Lowerer;
use super::scope::{Binding, LookupError};
use crate::error::IrError;
use crate::expr::*;
use crate::ids::*;
use crate::program::{ERR_TAG, NONE_TAG, OK_TAG, SOME_TAG};
use spec_lang_syntax::Span;
use spec_lang_syntax::ast::{self, BinOp as AB, ExprKind as AK, UnOp as AU};

pub(crate) type R<T> = Result<T, IrError>;

/// What kind of body is being lowered: decides what it may write and call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Body {
    Fn,
    Rule,
    Test,
    Law,
    /// A `where` claim on `self`.
    Claim,
    /// A constant, default, state initial value, `@given` or `@cases` value.
    Value,
}

#[derive(Debug)]
struct Local {
    name: String,
    mutable: bool,
}

/// The lowering context of one body.
#[derive(Debug)]
pub(crate) struct Cx {
    pub(crate) module: ModuleId,
    pub(crate) body: Body,
    locals: Vec<Local>,
}

impl Cx {
    pub(crate) fn new(module: ModuleId, body: Body) -> Self {
        Cx { module, body, locals: Vec::new() }
    }

    pub(crate) fn mark(&self) -> usize {
        self.locals.len()
    }

    pub(crate) fn pop_to(&mut self, mark: usize) {
        self.locals.truncate(mark);
    }

    fn local(&self, name: &str) -> Option<&Local> {
        self.locals.iter().rev().find(|l| l.name == name)
    }

    pub(crate) fn is_local(&self, name: &str) -> bool {
        self.local(name).is_some()
    }
}

/// Core names that are not declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Core {
    True,
    False,
    None,
    Some,
    Ok,
    Err,
    Set,
    Map,
}

fn core(name: &str) -> Option<Core> {
    Some(match name {
        "true" => Core::True,
        "false" => Core::False,
        "none" => Core::None,
        "Some" => Core::Some,
        "Ok" => Core::Ok,
        "Err" => Core::Err,
        "Set" => Core::Set,
        "Map" => Core::Map,
        _ => return None,
    })
}

/// What a bare name in an expression is.
pub(crate) enum Named {
    Local,
    Bind(Binding),
    Core(Core),
}

/// Library methods whose argument is a function: an argument that mentions
/// `it` becomes `it => argument` (card: "`it` for one unnested argument").
pub(crate) const FN_TAKING: &[&str] =
    &["filter", "map", "flat_map", "fold", "sum", "min", "max", "all", "any", "find", "position", "sort", "group_by"];

/// The core free names (`docs/idea/28-standard-library.md`, How to read 3).
/// Not reserved: a parameter may take one, and then its body cannot call
/// the free one (card, Expressions).
pub(crate) const FREE_NAMES: &[&str] = &["now", "try_call", "all", "race", "timeout", "old"];

/// Methods that change a place: statements only.
pub(crate) const MUTATING: &[&str] = &["push", "insert", "remove"];

impl Lowerer {
    pub(crate) fn resolve(&self, cx: &Cx, name: &str) -> Result<Named, LookupError> {
        if cx.is_local(name) {
            return Ok(Named::Local);
        }
        match self.scopes.lookup(cx.module, name) {
            Ok(b) => Ok(Named::Bind(b)),
            Err(LookupError::Unknown) => core(name).map(Named::Core).ok_or(LookupError::Unknown),
            Err(e) => Err(e),
        }
    }

    /// The constructor a core variant name stands for.
    pub(crate) fn core_variant(&self, k: Core) -> Callee {
        match k {
            Core::Ok => Callee::Variant(self.prog.result_type, OK_TAG),
            Core::Err => Callee::Variant(self.prog.result_type, ERR_TAG),
            _ => Callee::Variant(self.prog.option_type, SOME_TAG),
        }
    }

    /// Bind a local, refusing to shadow a local or a module-level name.
    pub(crate) fn bind_local(&self, cx: &mut Cx, name: &str, mutable: bool, span: Span) -> R<()> {
        if name != "_" && (cx.is_local(name) || self.scopes.lookup(cx.module, name).is_ok()) {
            return Err(IrError::error(
                "E-NAME-SHADOW",
                span,
                format!("`{name}` is already a name here; nothing shadows, so pick another"),
            ));
        }
        cx.locals.push(Local { name: name.to_string(), mutable });
        Ok(())
    }

    pub(crate) fn unknown_name(&self, cx: &Cx, name: &str, span: Span) -> IrError {
        let mut d = IrError::error("E-NAME-UNKNOWN", span, format!("no `{name}` in scope"));
        if let Some(best) = self.nearest_name(cx, name) {
            d = d.with_edit(span, best.clone()).with_note(format!("did you mean `{best}`?"));
        }
        d
    }

    fn nearest_name(&self, cx: &Cx, name: &str) -> Option<String> {
        let own = self.scopes.own.get(cx.module.index()).into_iter().flat_map(|s| s.keys());
        let imported = self.scopes.imported.get(cx.module.index()).into_iter().flat_map(|s| s.keys());
        let locals = cx.locals.iter().map(|l| &l.name);
        own.chain(imported)
            .chain(locals)
            .map(|c| (edit_distance(c, name), c))
            .filter(|(d, c)| *d <= 2 && *d < c.len())
            .min()
            .map(|(_, c)| c.clone())
    }

    /// The module a `use`-bound name denotes, if `e` is one.
    pub(crate) fn module_of(&self, cx: &Cx, e: &ast::Expr) -> Option<ModuleId> {
        match &e.kind {
            AK::Name(n) => match self.resolve(cx, n) {
                Ok(Named::Bind(Binding::Module(m))) => Some(m),
                _ => None,
            },
            _ => None,
        }
    }

    /// A member `m.name` of a module.
    pub(crate) fn member(&self, m: ModuleId, name: &str, span: Span) -> R<Binding> {
        let b = self.scopes.member(m, name).ok_or_else(|| {
            IrError::error("E-NAME-UNKNOWN", span, format!("module {} declares no `{name}`", self.prog.module(m).name))
        })?;
        self.check_visible(b, span)?;
        Ok(b)
    }

    /// Another module's name must be `pub` (`docs/idea/30-type-rules.md` section 2).
    /// Tests (named by `@given`) and states (read by tests and laws, written
    /// only by their own rules) need no `pub`; a sum's variants follow their type.
    pub(crate) fn check_visible(&self, b: Binding, span: Span) -> R<()> {
        let d = match b {
            Binding::Decl(d @ (DeclRef::Type(_) | DeclRef::Const(_) | DeclRef::Fn(_) | DeclRef::Rule(_))) => d,
            Binding::Variant(t, _) => DeclRef::Type(t),
            _ => return Ok(()),
        };
        let info = self.decl_info(d);
        if info.is_pub {
            return Ok(());
        }
        Err(IrError::error(
            "E-NAME-PRIVATE",
            span,
            format!("`{}` is not `pub` in {}", info.name, self.prog.module(info.module).name),
        )
        .with_note(format!(
            "mark it `pub` at {}, or keep its use inside its module",
            self.prog.location(info.name_span)
        )))
    }

    /// A name or `module.member` that denotes a binding, if `e` is one.
    pub(crate) fn binding_of(&self, cx: &Cx, e: &ast::Expr) -> Option<R<Binding>> {
        match &e.kind {
            AK::Name(n) => match self.resolve(cx, n) {
                Ok(Named::Bind(b)) => Some(Ok(b)),
                _ => None,
            },
            AK::Field(recv, name) => self.module_of(cx, recv).map(|m| self.member(m, &name.name, name.span)),
            _ => None,
        }
    }

    pub(crate) fn lower_expr(&mut self, cx: &mut Cx, e: &ast::Expr) -> R<Expr> {
        let span = e.span;
        let kind = match &e.kind {
            AK::Name(n) => return self.lower_name(cx, n, span),
            AK::Num(s) => ExprKind::Int(parse_int(s).ok_or_else(|| {
                IrError::unsupported(span, format!("the number `{s}`: only whole numbers without a unit"))
            })?),
            AK::Text(t) => return self.lower_text(cx, t),
            AK::Hole(i) => ExprKind::Hole(i.name.clone()),
            AK::Group { args, trailing_comma } => {
                if args.is_empty() {
                    ExprKind::Unit
                } else if args.iter().any(|a| a.name.is_some()) {
                    return Err(IrError::unsupported(span, "a group of named values"));
                } else if args.len() == 1 && !trailing_comma {
                    return self.lower_expr(cx, &args[0].value);
                } else {
                    ExprKind::Tuple(self.lower_positional(cx, args)?)
                }
            }
            AK::List(args) => ExprKind::List(self.lower_positional(cx, args)?),
            AK::Field(recv, name) => {
                if let Some(m) = self.module_of(cx, recv) {
                    let b = self.member(m, &name.name, name.span)?;
                    return self.lower_binding(cx, b, &name.name, span);
                }
                if let Some(effect) = self.effect_of(cx, recv, name, false, span)? {
                    return Ok(Expr::new(ExprKind::EffectRead(effect), span));
                }
                ExprKind::Prop { recv: Box::new(self.lower_expr(cx, recv)?), name: name.name.clone() }
            }
            AK::Call(callee, args) => return self.lower_call(cx, callee, args, span),
            AK::Index(recv, args) => {
                let [arg] = args.as_slice() else {
                    return Err(IrError::unsupported(span, "an index with several arguments"));
                };
                if arg.name.is_some() {
                    return Err(IrError::unsupported(span, "a named index"));
                }
                ExprKind::Index {
                    recv: Box::new(self.lower_expr(cx, recv)?),
                    key: Box::new(self.lower_expr(cx, &arg.value)?),
                }
            }
            AK::Unary(op, x) => match op {
                AU::Neg | AU::Pos | AU::Not => ExprKind::Unary(*op, Box::new(self.lower_expr(cx, x)?)),
                AU::Own | AU::Ref | AU::Mut => {
                    return Err(IrError::unsupported(span, "`own`, `ref` and `mut` (machine altitude)"));
                }
            },
            AK::Binary(op, l, r) => {
                let bop = match op {
                    AB::Add => BinOp::Add,
                    AB::Sub => BinOp::Sub,
                    AB::Mul => BinOp::Mul,
                    AB::Div => BinOp::Div,
                    AB::Mod => BinOp::Mod,
                    AB::Pow => BinOp::Pow,
                    AB::Band => BinOp::Band,
                    AB::Bor => BinOp::Bor,
                    AB::Bxor => BinOp::Bxor,
                    AB::Shl => BinOp::Shl,
                    AB::Shr => BinOp::Shr,
                    AB::And => BinOp::And,
                    AB::Or => BinOp::Or,
                    AB::Implies => BinOp::Implies,
                    AB::RangeIncl | AB::RangeExcl => {
                        let lo = Box::new(self.lower_expr(cx, l)?);
                        let hi = Box::new(self.lower_expr(cx, r)?);
                        return Ok(Expr::new(ExprKind::Range { lo, hi, inclusive: *op == AB::RangeIncl }, span));
                    }
                    AB::Bar => {
                        return Err(IrError::unsupported(span, "`|` in an expression (it forms sums and rule sets)"));
                    }
                    AB::Where => return Err(IrError::unsupported(span, "`where` outside a type")),
                    AB::PlusMinus => return Err(IrError::unsupported(span, "`±`")),
                };
                ExprKind::Binary(bop, Box::new(self.lower_expr(cx, l)?), Box::new(self.lower_expr(cx, r)?))
            }
            AK::Sup(x, n) => ExprKind::Binary(
                BinOp::Pow,
                Box::new(self.lower_expr(cx, x)?),
                Box::new(Expr::new(ExprKind::Int(i128::from(*n)), span)),
            ),
            AK::Compare(first, rest) => {
                let first = Box::new(self.lower_expr(cx, first)?);
                let mut out = Vec::new();
                for (op, x) in rest {
                    out.push((*op, self.lower_expr(cx, x)?));
                }
                ExprKind::Compare(first, out)
            }
            AK::Lambda(params, body) => ExprKind::Lambda(self.lower_lambda(cx, params, body, span)?),
            AK::Arrow(..) => {
                return Err(IrError::unsupported(
                    span,
                    "`a -> b` outside a test step, `@cases`, `@given` or `Map(...)`",
                ));
            }
            AK::Else(v, f) => ExprKind::Else {
                unwrap_option: !matches!(v.kind, AK::Index(..)),
                value: Box::new(self.lower_expr(cx, v)?),
                fallback: Box::new(self.lower_expr(cx, f)?),
            },
            AK::Cond(c, a, b) => ExprKind::If {
                cond: Box::new(self.lower_expr(cx, c)?),
                then: Box::new(self.lower_expr(cx, a)?),
                els: Box::new(self.lower_expr(cx, b)?),
            },
            AK::Fail(x) => ExprKind::Fail(self.lower_failure(cx, x)?),
        };
        Ok(Expr::new(kind, span))
    }

    pub(crate) fn lower_positional(&mut self, cx: &mut Cx, args: &[ast::Arg]) -> R<Vec<Expr>> {
        let mut out = Vec::new();
        for a in args {
            if let Some(n) = &a.name {
                return Err(IrError::error("E-TYPE-ARITY", n.span, format!("`{}:` names no parameter here", n.name)));
            }
            out.push(self.lower_expr(cx, &a.value)?);
        }
        Ok(out)
    }

    fn lower_name(&mut self, cx: &mut Cx, n: &str, span: Span) -> R<Expr> {
        match self.resolve(cx, n) {
            Ok(Named::Local) => Ok(Expr::new(ExprKind::Local(n.to_string()), span)),
            Ok(Named::Bind(b)) => self.lower_binding(cx, b, n, span),
            Ok(Named::Core(Core::True)) => Ok(Expr::new(ExprKind::Bool(true), span)),
            Ok(Named::Core(Core::False)) => Ok(Expr::new(ExprKind::Bool(false), span)),
            Ok(Named::Core(Core::None)) => {
                Ok(Expr::new(ExprKind::Variant { ty: self.prog.option_type, tag: NONE_TAG }, span))
            }
            Ok(Named::Core(_)) => {
                Err(IrError::error("E-NAME-KIND", span, format!("`{n}` is called, not used as a value")))
            }
            Err(LookupError::Ambiguous) => {
                Err(IrError::error("E-NAME-AMBIG", span, format!("`{n}` is both declared here and bound by `use`")))
            }
            Err(LookupError::Unknown) => Err(self.unknown_name(cx, n, span)),
        }
    }

    fn lower_binding(&mut self, cx: &mut Cx, b: Binding, n: &str, span: Span) -> R<Expr> {
        let kind = match b {
            Binding::Decl(DeclRef::Const(c)) => ExprKind::Const(c),
            Binding::Decl(DeclRef::State(s)) => {
                if cx.body == Body::Claim {
                    return Err(IrError::error(
                        "E-EFFECT-KIND",
                        span,
                        format!("a claim reads no state, and `{n}` is one"),
                    ));
                }
                ExprKind::State(s)
            }
            Binding::Variant(t, tag) => {
                if self.sigs.get(&Callee::Variant(t, tag)).is_some_and(|s| !s.is_empty()) {
                    return Err(IrError::error(
                        "E-NAME-KIND",
                        span,
                        format!("variant `{n}` has data: write `{n}(...)`"),
                    ));
                }
                ExprKind::Variant { ty: t, tag }
            }
            Binding::Decl(DeclRef::Fn(_) | DeclRef::Rule(_)) => {
                return Err(IrError::error(
                    "E-NAME-KIND",
                    span,
                    format!("`{n}` is called, not used as a value: write `{n}(...)`"),
                ));
            }
            Binding::Decl(_) | Binding::Module(_) => {
                return Err(IrError::error("E-NAME-KIND", span, format!("`{n}` is not a value")));
            }
        };
        let _ = cx;
        Ok(Expr::new(kind, span))
    }

    fn lower_call(&mut self, cx: &mut Cx, callee: &ast::Expr, args: &[ast::Arg], span: Span) -> R<Expr> {
        if let AK::Name(n) = &callee.kind {
            match self.resolve(cx, n) {
                Ok(Named::Local) if FREE_NAMES.contains(&n.as_str()) => {
                    return Err(IrError::error(
                        "E-NAME-SHADOW",
                        callee.span,
                        format!("`{n}` is a local here, so this body cannot call the free `{n}`"),
                    )
                    .with_note("rename the local, or move the call out of this body"));
                }
                Ok(Named::Local) => {
                    let f = Box::new(self.lower_expr(cx, callee)?);
                    let args = self.lower_positional(cx, args)?;
                    return Ok(Expr::new(ExprKind::CallValue { callee: f, args }, span));
                }
                Ok(Named::Core(Core::Set)) => {
                    return Ok(Expr::new(ExprKind::SetOf(self.lower_positional(cx, args)?), span));
                }
                Ok(Named::Core(Core::Map)) => return self.lower_map(cx, args, span),
                Ok(Named::Core(k @ (Core::Some | Core::Ok | Core::Err))) => {
                    return self.bound_call(cx, self.core_variant(k), args, span);
                }
                _ => {}
            }
        }
        if let Some(b) = self.binding_of(cx, callee) {
            let c = match b? {
                Binding::Decl(DeclRef::Fn(f)) => Callee::Fn(f),
                Binding::Decl(DeclRef::Rule(r)) => Callee::Rule(r),
                Binding::Decl(DeclRef::Type(t)) if self.sigs.contains_key(&Callee::Record(t)) => Callee::Record(t),
                Binding::Variant(t, tag) => Callee::Variant(t, tag),
                _ => return Err(IrError::error("E-NAME-KIND", callee.span, "this cannot be called")),
            };
            return self.bound_call(cx, c, args, span);
        }
        if let AK::Name(n) = &callee.kind {
            return Err(self.unknown_name(cx, n, callee.span));
        }
        let AK::Field(recv, name) = &callee.kind else {
            return Err(IrError::unsupported(span, "calling the value of an expression"));
        };
        if let Some(effect) = self.effect_of(cx, recv, name, true, span)? {
            return self.bound_call(cx, Callee::Effect(effect), args, span);
        }
        match name.name.as_str() {
            "is" => {
                let [arg] = args else {
                    return Err(IrError::error("E-TYPE-ARITY", span, "`.is` takes one pattern"));
                };
                let value = Box::new(self.lower_expr(cx, recv)?);
                let mark = cx.mark();
                let mut binds = Vec::new();
                let pattern = self.lower_pattern(cx, &arg.value, &mut binds)?;
                cx.pop_to(mark);
                if !binds.is_empty() {
                    return Err(IrError::unsupported(arg.span, "a pattern in `.is(...)` that binds names"));
                }
                Ok(Expr::new(ExprKind::Is { value, pattern: Box::new(pattern) }, span))
            }
            "with" => {
                let recv = Box::new(self.lower_expr(cx, recv)?);
                let mut fields = Vec::new();
                for a in args {
                    let Some(n) = &a.name else {
                        return Err(IrError::error(
                            "E-TYPE-ARITY",
                            a.span,
                            "`.with` takes named fields: `x.with(f: v)`",
                        ));
                    };
                    fields.push((n.name.clone(), self.lower_expr(cx, &a.value)?));
                }
                Ok(Expr::new(ExprKind::With { recv, fields }, span))
            }
            m if MUTATING.contains(&m) => Err(IrError::error(
                "E-PLACE-ASSIGN",
                span,
                format!("`{m}` changes a place and returns nothing: use it as a statement"),
            )),
            m => {
                let recv = Box::new(self.lower_expr(cx, recv)?);
                let takes_fn = FN_TAKING.contains(&m);
                let mut out = Vec::new();
                for a in args {
                    let value = if takes_fn && mentions_it(&a.value) {
                        self.implicit_lambda(cx, &a.value)?
                    } else {
                        self.lower_expr(cx, &a.value)?
                    };
                    out.push(Arg { name: a.name.as_ref().map(|n| n.name.clone()), value });
                }
                Ok(Expr::new(ExprKind::Method { recv, name: m.to_string(), args: out }, span))
            }
        }
    }

    fn lower_map(&mut self, cx: &mut Cx, args: &[ast::Arg], span: Span) -> R<Expr> {
        let mut out = Vec::new();
        for a in args {
            let AK::Arrow(k, v) = &a.value.kind else {
                return Err(IrError::error("E-TYPE-MISMATCH", a.span, "`Map(...)` takes `key -> value` pairs")
                    .with_expected("key -> value", self.prog.snippet(a.span).to_string()));
            };
            out.push((self.lower_expr(cx, k)?, self.lower_expr(cx, v)?));
        }
        Ok(Expr::new(ExprKind::MapOf(out), span))
    }

    pub(crate) fn bound_call(&mut self, cx: &mut Cx, c: Callee, args: &[ast::Arg], span: Span) -> R<Expr> {
        if let Callee::Rule(_) = c
            && !matches!(cx.body, Body::Rule | Body::Test)
        {
            return Err(IrError::error("E-EFFECT-KIND", span, "a rule is called only from a rule or a test"));
        }
        let args = self.bind_args(cx, c, args, span)?;
        Ok(Expr::new(ExprKind::Call { callee: c, args }, span))
    }

    /// Arguments in parameter order: positional first, then by name; a
    /// missing parameter takes its default or is an error.
    pub(crate) fn bind_args(&mut self, cx: &mut Cx, c: Callee, args: &[ast::Arg], span: Span) -> R<Vec<Option<Expr>>> {
        let sig = self.sigs.get(&c).cloned().unwrap_or_default();
        self.bind_with(cx, &sig, args, span)
    }

    /// [`Self::bind_args`] against an explicit parameter list.
    pub(crate) fn bind_with(
        &mut self,
        cx: &mut Cx,
        sig: &[(String, bool)],
        args: &[ast::Arg],
        span: Span,
    ) -> R<Vec<Option<Expr>>> {
        let mut slots: Vec<Option<Expr>> = (0..sig.len()).map(|_| None).collect();
        let mut pos = 0;
        let mut named = false;
        for a in args {
            let idx = match &a.name {
                None => {
                    if named {
                        return Err(IrError::error("E-TYPE-ARITY", a.span, "a positional argument after a named one"));
                    }
                    pos += 1;
                    if pos > sig.len() {
                        return Err(IrError::error(
                            "E-TYPE-ARITY",
                            a.span,
                            format!("too many arguments: {} expected", sig.len()),
                        )
                        .with_expected(format!("{} arguments", sig.len()), format!("{}", args.len())));
                    }
                    pos - 1
                }
                Some(n) => {
                    named = true;
                    sig.iter().position(|(p, _)| *p == n.name).ok_or_else(|| {
                        let names: Vec<_> = sig.iter().map(|(p, _)| p.as_str()).collect();
                        IrError::error("E-TYPE-ARITY", n.span, format!("no parameter `{}`", n.name))
                            .with_expected(format!("one of {}", names.join(", ")), n.name.clone())
                    })?
                }
            };
            if slots[idx].is_some() {
                return Err(IrError::error("E-TYPE-ARITY", a.span, format!("`{}` is given twice", sig[idx].0)));
            }
            slots[idx] = Some(self.lower_expr(cx, &a.value)?);
        }
        for (slot, (name, has_default)) in slots.iter().zip(sig) {
            if slot.is_none() && !has_default {
                return Err(IrError::error("E-TYPE-ARITY", span, format!("missing argument `{name}`")));
            }
        }
        Ok(slots)
    }

    fn implicit_lambda(&mut self, cx: &mut Cx, e: &ast::Expr) -> R<Expr> {
        let mark = cx.mark();
        if cx.is_local("it") {
            return Err(IrError::error(
                "E-NAME-SHADOW",
                e.span,
                "`it` inside an argument that already has `it`: name one with `x => ...`",
            ));
        }
        cx.locals.push(Local { name: "it".into(), mutable: false });
        let body = self.lower_expr(cx, e);
        cx.pop_to(mark);
        let id = self.push_lambda(Lambda {
            params: vec!["it".into()],
            body: LambdaBody::Expr(body?),
            module: cx.module,
            span: e.span,
        });
        Ok(Expr::new(ExprKind::Lambda(id), e.span))
    }

    fn lower_lambda(&mut self, cx: &mut Cx, params: &ast::Expr, body: &ast::LambdaBody, span: Span) -> R<LambdaId> {
        let names: Vec<(String, Span)> = match &params.kind {
            AK::Name(n) => vec![(n.clone(), params.span)],
            AK::Group { args, .. } if args.iter().all(|a| a.name.is_none() && a.value.as_name().is_some()) => {
                args.iter().map(|a| (a.value.as_name().unwrap_or_default().to_string(), a.span)).collect()
            }
            _ => return Err(IrError::unsupported(params.span, "lambda parameters other than names")),
        };
        let mark = cx.mark();
        let lowered = (|| {
            for (n, s) in &names {
                self.bind_local(cx, n, false, *s)?;
            }
            match body {
                ast::LambdaBody::Expr(e) => Ok(LambdaBody::Expr(self.lower_expr(cx, e)?)),
                ast::LambdaBody::Block(b) => Ok(LambdaBody::Block(self.lower_block(cx, b)?)),
            }
        })();
        cx.pop_to(mark);
        Ok(self.push_lambda(Lambda {
            params: names.into_iter().map(|(n, _)| n).collect(),
            body: lowered?,
            module: cx.module,
            span,
        }))
    }

    fn push_lambda(&mut self, l: Lambda) -> LambdaId {
        self.prog.lambdas.push(l);
        LambdaId(next_id(self.prog.lambdas.len() - 1))
    }

    /// `fail Name` or `fail Name(data)`.
    pub(crate) fn lower_failure(&mut self, cx: &mut Cx, x: &ast::Expr) -> R<Failure> {
        match &x.kind {
            AK::Name(n) => Ok(Failure { name: n.clone(), args: Vec::new() }),
            AK::Call(c, args) => match c.as_name() {
                Some(n) => Ok(Failure { name: n.to_string(), args: self.lower_positional(cx, args)? }),
                None => Err(IrError::unsupported(x.span, "this error form: write `fail Name` or `fail Name(data)`")),
            },
            _ => Err(IrError::unsupported(x.span, "this error form: write `fail Name` or `fail Name(data)`")),
        }
    }

    pub(crate) fn push_local(&self, cx: &mut Cx, name: &str, mutable: bool) {
        cx.locals.push(Local { name: name.to_string(), mutable });
    }

    pub(crate) fn local_mutable(&self, cx: &Cx, name: &str) -> Option<bool> {
        cx.local(name).map(|l| l.mutable)
    }
}

/// Whether `it` occurs in `e` outside explicit lambdas and outside the
/// arguments of nested function-taking calls (which own their own `it`).
pub(crate) fn mentions_it(e: &ast::Expr) -> bool {
    match &e.kind {
        AK::Name(n) => n == "it",
        AK::Lambda(..) | AK::Num(_) | AK::Text(_) | AK::Hole(_) => false,
        AK::Call(callee, args) => {
            if let AK::Field(recv, m) = &callee.kind
                && FN_TAKING.contains(&m.name.as_str())
            {
                return mentions_it(recv);
            }
            mentions_it(callee) || args.iter().any(|a| mentions_it(&a.value))
        }
        AK::Group { args, .. } | AK::List(args) => args.iter().any(|a| mentions_it(&a.value)),
        AK::Field(r, _) => mentions_it(r),
        AK::Index(r, args) => mentions_it(r) || args.iter().any(|a| mentions_it(&a.value)),
        AK::Unary(_, x) | AK::Sup(x, _) | AK::Fail(x) => mentions_it(x),
        AK::Binary(_, l, r) | AK::Arrow(l, r) | AK::Else(l, r) => mentions_it(l) || mentions_it(r),
        AK::Compare(f, rest) => mentions_it(f) || rest.iter().any(|(_, x)| mentions_it(x)),
        AK::Cond(a, b, c) => mentions_it(a) || mentions_it(b) || mentions_it(c),
    }
}

/// `123`, `1_000`, `0xff`: whole numbers without a unit.
pub(crate) fn parse_int(s: &str) -> Option<i128> {
    let clean: String = s.chars().filter(|c| *c != '_').collect();
    if let Some(hex) = clean.strip_prefix("0x") {
        return i128::from_str_radix(hex, 16).ok();
    }
    if clean.chars().all(|c| c.is_ascii_digit()) { clean.parse().ok() } else { None }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != *cb);
            cur.push(sub.min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}
