//! The typed tree to the Ruby tree.
//!
//! Representation, fixed for the backend: `Int` is an `Integer`, `Text` a
//! frozen UTF-8 `String`, a sum with no data the symbol of its variant, a
//! record a `Data` whose constructor checks every claim, `Option[T]` is
//! `T` or `nil`, a `Set` a frozen `Set`, a `Map` a frozen `Hash` kept in
//! ascending key order. A rule is a module function
//! `rule(world, **params) -> Outcome`; a refusal is an exception raised
//! before any new world exists.

use super::ast::*;
use super::file::RFile;
use emit_ts::sem::app::App;
use emit_ts::sem::infer::{Infer, Named, R};
use emit_ts::sem::names::{pascal, snake};
use emit_ts::sem::tree::*;
use spec_lang_ir::{BinOp, CmpOp, FnId, ModuleId, RuleId, StateId, TypeId};
use std::collections::BTreeSet;

pub const VALUES: &str = "lib/runtime/values.rb";
pub const TEXT: &str = "lib/runtime/text.rb";
pub const WORLD: &str = "lib/app/world.rb";
pub const REFUSALS: &str = "lib/app/refusals.rb";

/// Ruby's own words and the names the generated code uses itself; a local with one of them gets a `_`.
const RESERVED: &[&str] = &[
    "world", "board", "lines", "alias", "and", "begin", "break", "case", "class", "def", "defined", "do", "else",
    "elsif", "end", "ensure", "false", "for", "if", "in", "module", "next", "nil", "not", "or", "redo", "rescue",
    "retry", "return", "self", "super", "then", "true", "undef", "unless", "until", "when", "while", "yield", "gen",
    "prefix", "events",
];

/// `todo.board` gives `Todo::Board`.
pub fn module_path(app: &App<'_>, m: ModuleId) -> String {
    app.p.module(m).name.split('.').map(pascal).collect::<Vec<_>>().join("::")
}

pub fn module_parts(app: &App<'_>, m: ModuleId) -> Vec<String> {
    app.p.module(m).name.split('.').map(pascal).collect()
}

pub fn module_dir(app: &App<'_>, m: ModuleId) -> String {
    app.p.module(m).name.replace('.', "/")
}

fn decl_file(app: &App<'_>, info: &spec_lang_ir::DeclInfo) -> String {
    format!("lib/{}/{}.rb", module_dir(app, info.module), snake(&info.name))
}

pub fn type_file(app: &App<'_>, id: TypeId) -> String {
    decl_file(app, &app.p.ty(id).info)
}

pub fn fn_file(app: &App<'_>, id: FnId) -> String {
    decl_file(app, &app.p.func(id).info)
}

pub fn rule_file(app: &App<'_>, id: RuleId) -> String {
    decl_file(app, &app.p.rule(id).info)
}

pub fn state_name(app: &App<'_>, id: StateId) -> String {
    app.p.state(id).info.name.clone()
}

/// `Path::NAME` under a constant.
pub fn child(e: Expr, name: &str) -> Expr {
    match e {
        Expr::Const(mut path) => {
            path.push(name.to_string());
            Expr::Const(path)
        }
        other => other,
    }
}

#[derive(Debug)]
pub enum Mode {
    Pure,
    Guard,
    Rule { drafts: BTreeSet<StateId>, lines: bool },
    Test,
}

#[derive(Debug)]
pub struct Lower<'a, 'p> {
    pub app: &'a App<'p>,
    pub inf: Infer<'a>,
    pub f: &'a mut RFile,
    pub mode: Mode,
    renames: Vec<(String, String)>,
}

impl<'a, 'p> Lower<'a, 'p> {
    pub fn new(app: &'a App<'p>, f: &'a mut RFile, mode: Mode, decl: &str) -> Self {
        let mut inf = app.types_of();
        inf.decl = decl.to_string();
        Lower { app, inf, f, mode, renames: Vec::new() }
    }

    /// Give a spec name another Ruby name for the rest of this lowering.
    pub fn rename(&mut self, from: &str, to: &str) {
        self.renames.push((from.to_string(), to.to_string()));
    }

    pub fn local(&self, name: &str) -> String {
        if let Some((_, to)) = self.renames.iter().rev().find(|(from, _)| from == name) {
            return to.clone();
        }
        if RESERVED.contains(&name) { format!("{name}_") } else { name.to_string() }
    }

    /// `Runtime.name(args)`.
    pub fn rt(&mut self, name: &str, args: Vec<Expr>) -> Expr {
        self.f.need(VALUES);
        call(konst("Runtime"), name, args)
    }

    /// The constant of a declared type: `Todo::Model::Task`.
    pub fn type_ref(&mut self, id: TypeId) -> Expr {
        let info = &self.app.p.ty(id).info;
        self.f.need(&type_file(self.app, id));
        konst(&format!("{}::{}", module_path(self.app, info.module), info.name))
    }

    /// The module a `fn` or `rule` is a function of.
    fn owner(&mut self, info: &spec_lang_ir::DeclInfo) -> Expr {
        self.f.need(&decl_file(self.app, info));
        konst(&module_path(self.app, info.module))
    }

    pub fn refusal(&mut self, name: &str) -> Expr {
        self.f.need(REFUSALS);
        konst(&format!("App::{name}"))
    }

    fn primitive_order(&self, t: &Type) -> bool {
        matches!(self.inf.base(t), Type::Int | Type::Size | Type::Text)
    }

    fn is_set(&self, t: &Type) -> bool {
        matches!(self.inf.base(t), Type::Set(_))
    }

    fn state(&mut self, id: StateId, e: &TExpr) -> R<Expr> {
        let name = state_name(self.app, id);
        match &self.mode {
            Mode::Guard => Ok(call(ident("world"), &name, vec![])),
            Mode::Rule { drafts, .. } => Ok(if drafts.contains(&id) {
                ident(&self.local_state(&name))
            } else {
                call(ident("world"), &name, vec![])
            }),
            Mode::Test => Ok(call(call(ident("board"), "world", vec![]), &name, vec![])),
            Mode::Pure => self.inf.unsupported(e.span, "a `fn` that reads state"),
        }
    }

    /// The local that drafts a state in a rule body.
    pub fn local_state(&self, name: &str) -> String {
        if RESERVED.contains(&name) { format!("{name}_") } else { name.to_string() }
    }

    /// Arguments of a `fn` call: positional where the parameter has no default, by keyword where it has.
    fn fn_args(&mut self, params: &[spec_lang_ir::Param], args: &[Option<TExpr>]) -> R<Vec<Arg>> {
        let mut out = Vec::new();
        for (p, a) in params.iter().zip(args) {
            if let Some(a) = a {
                let v = self.expr(a)?;
                out.push(if p.default.is_some() { Arg::Key(self.local(&p.name), v) } else { Arg::Pos(v) });
            }
        }
        Ok(out)
    }

    /// `Mod.rule(world, name: value, ...)`.
    pub fn rule_apply(&mut self, rule: RuleId, world: Expr, args: Vec<(String, Expr)>) -> Expr {
        let info = &self.app.p.rule(rule).info;
        let owner = self.owner(info);
        let mut all = vec![Arg::Pos(world)];
        all.extend(args.into_iter().map(|(n, v)| Arg::Key(n, v)));
        call_args(owner, &info.name, all)
    }

    /// One rule call in a test: `board.call { |world| Mod.rule(world, ...) }`, or `board.refused { ... }`.
    pub fn rule_call(&mut self, e: &TExpr, how: &str) -> R<Expr> {
        let TKind::CallRule { rule, args } = &e.kind else {
            return self.inf.unsupported(e.span, "a step that is not a rule call");
        };
        let decl = self.app.p.rule(*rule);
        let mut named = Vec::new();
        for (p, a) in decl.params.iter().zip(args) {
            if let Some(a) = a {
                named.push((self.local(&p.name), self.expr(a)?));
            }
        }
        let applied = self.rule_apply(*rule, ident("world"), named);
        Ok(call_block(ident("board"), how, vec![], &["world"], vec![Stmt::Expr(applied)]))
    }

    fn variant_name(&self, ty: TypeId, tag: u32) -> R<String> {
        match self.inf.named(ty)? {
            Named::Sum(variants) => Ok(variants.get(tag as usize).map(|v| v.name.clone()).unwrap_or_default()),
            _ => self.inf.unsupported(self.app.p.ty(ty).info.span, "a variant of a type that is not a sum"),
        }
    }

    /// `recv.method { |param| body }` from a one-parameter lambda.
    fn with_block(&mut self, recv: Expr, method: &str, lambda: Option<&TExpr>) -> R<Expr> {
        let Some(TExpr { kind: TKind::Lambda { params, body }, span, .. }) = lambda else {
            return self.inf.unsupported(self.app.decl.info.span, "a core-library call without its function");
        };
        let [(p, _)] = params.as_slice() else {
            return self.inf.unsupported(*span, "a lambda with more than one parameter");
        };
        let name = self.local(p);
        let body = self.expr(body)?;
        Ok(call_block(recv, method, vec![], &[&name], vec![Stmt::Expr(body)]))
    }

    pub fn expr(&mut self, e: &TExpr) -> R<Expr> {
        Ok(match &e.kind {
            TKind::Unit | TKind::Nothing => Expr::Nil,
            TKind::Bool(true) => Expr::True,
            TKind::Bool(false) => Expr::False,
            TKind::Int(i) => Expr::Int(*i),
            TKind::Text(s) => text(s),
            TKind::Interp(parts) => Expr::Interp(
                parts
                    .iter()
                    .map(|p| {
                        Ok(match p {
                            TPart::Lit(s) => Part::Lit(s.clone()),
                            TPart::Expr(x) => Part::Expr(self.expr(x)?),
                        })
                    })
                    .collect::<R<_>>()?,
            ),
            TKind::Local(n) => ident(&self.local(n)),
            TKind::Const(id) => {
                let info = &self.app.p.constant(*id).info;
                let owner = self.owner(info);
                match owner {
                    Expr::Const(mut path) => {
                        path.push(emit_ts::sem::names::screaming(&info.name));
                        Expr::Const(path)
                    }
                    other => other,
                }
            }
            TKind::State(id) => self.state(*id, e)?,
            TKind::Variant { ty, tag } => sym(&self.variant_name(*ty, *tag)?),
            TKind::Just(x) | TKind::Widen(x) => self.expr(x)?,
            TKind::CallFn { func, args } => {
                let decl = self.app.p.func(*func);
                let args = self.fn_args(&decl.params, args)?;
                let owner = self.owner(&decl.info);
                call_args(owner, &decl.info.name, args)
            }
            TKind::CallRule { .. } => match self.mode {
                Mode::Test => self.rule_call(e, "call")?,
                _ => return self.inf.unsupported(e.span, "a rule called outside a test"),
            },
            TKind::Record { ty, args } => {
                let Named::Record(fields) = self.inf.named(*ty)? else {
                    return self.inf.unsupported(e.span, "constructing a type that is not a record");
                };
                let mut named = Vec::new();
                for (f, a) in fields.iter().zip(args) {
                    if let Some(a) = a {
                        named.push(Arg::Key(self.local(&f.name), self.expr(a)?));
                    }
                }
                call_args(self.type_ref(*ty), "new", named)
            }
            TKind::Field { recv, name } => call(self.expr(recv)?, name, vec![]),
            TKind::Core { op, recv, args } => self.core(*op, recv, args, e)?,
            TKind::Index { recv, key } => call(self.expr(recv)?, "fetch", vec![self.expr(key)?]),
            TKind::With { recv, fields } => {
                let r = self.expr(recv)?;
                let named = fields.iter().map(|(n, v)| Ok(Arg::Key(n.clone(), self.expr(v)?))).collect::<R<_>>()?;
                call_args(r, "with", named)
            }
            TKind::Not(x) => not(self.expr(x)?),
            TKind::Neg(x) => Expr::Neg(Box::new(self.expr(x)?)),
            TKind::Binary(op, x, y) => {
                let (x, y) = (self.expr(x)?, self.expr(y)?);
                match op {
                    BinOp::And => bin("&&", x, y),
                    BinOp::Or => bin("||", x, y),
                    BinOp::Implies => bin("||", not(x), y),
                    BinOp::Add => bin("+", x, y),
                    BinOp::Sub => bin("-", x, y),
                    BinOp::Mul => bin("*", x, y),
                    _ => return self.inf.unsupported(e.span, "this operator"),
                }
            }
            TKind::Compare(op, x, y) => {
                let direct = self.primitive_order(&x.ty);
                let (a, c) = (self.expr(x)?, self.expr(y)?);
                let ordered = |op: &'static str, a, c| {
                    if direct { bin(op, a, c) } else { bin(op, bin("<=>", a, c), Expr::Int(0)) }
                };
                match op {
                    CmpOp::Eq => bin("==", a, c),
                    CmpOp::Ne => bin("!=", a, c),
                    CmpOp::Lt => ordered("<", a, c),
                    CmpOp::Le => ordered("<=", a, c),
                    CmpOp::Gt => ordered(">", a, c),
                    CmpOp::Ge => ordered(">=", a, c),
                    CmpOp::In => return self.inf.unsupported(x.span, "`in` here"),
                }
            }
            TKind::Tuple(xs) | TKind::List(xs) => Expr::Array(xs.iter().map(|x| self.expr(x)).collect::<R<_>>()?),
            TKind::SetOf(xs) => {
                call(Expr::SetLit(xs.iter().map(|x| self.expr(x)).collect::<R<_>>()?), "freeze", vec![])
            }
            TKind::MapOf(pairs) => {
                let hash = Expr::Hash(pairs.iter().map(|(k, v)| Ok((self.expr(k)?, self.expr(v)?))).collect::<R<_>>()?);
                if pairs.is_empty() { call(hash, "freeze", vec![]) } else { self.rt("map_of", vec![hash]) }
            }
            TKind::Lambda { .. } => return self.inf.unsupported(e.span, "a lambda outside a core-library call"),
            TKind::If { cond, then, els } => {
                Expr::Ternary(Box::new(self.expr(cond)?), Box::new(self.expr(then)?), Box::new(self.expr(els)?))
            }
            TKind::Match { scrutinee, arms } => self.match_chain(scrutinee, arms)?,
            TKind::Claim(x) => {
                let Type::Named(id) = &e.ty else { return self.inf.unsupported(e.span, "a claim on an unnamed type") };
                let value = self.expr(x)?;
                let ty = self.type_ref(*id);
                self.rt("claim", vec![ty, value])
            }
            TKind::OutText => call(ident("board"), "out_text", vec![]),
            TKind::Trace { recv, prop } => {
                let r = self.expr(recv)?;
                match prop {
                    TraceProp::States => call(r, "states", vec![]),
                    TraceProp::Steps => call(r, "steps", vec![]),
                    TraceProp::Before => call(r, "before", vec![]),
                    TraceProp::After => call(r, "after", vec![]),
                    TraceProp::Event => call(r, "event", vec![]),
                    TraceProp::Reply => call(r, "reply", vec![]),
                    TraceProp::State(id) => call(r, &state_name(self.app, *id), vec![]),
                    TraceProp::Arg(n) => call(r, "fetch", vec![sym(n)]),
                }
            }
            TKind::IsRule { event, rule } => {
                let name = self.app.p.rule(*rule).info.name.clone();
                bin("==", call(self.expr(event)?, "fetch", vec![sym("rule")]), sym(&name))
            }
            TKind::IsRefused { reply, error } => {
                self.f.need(VALUES);
                let refused = call(self.expr(reply)?, "is_a?", vec![konst("Runtime::Refusal")]);
                match error {
                    None => refused,
                    Some(name) => {
                        bin("&&", refused, bin("==", call(self.expr(reply)?, "error_name", vec![]), text(name)))
                    }
                }
            }
            TKind::Block(_) => return self.inf.unsupported(e.span, "a block used as a value inside an expression"),
        })
    }

    /// A Set iterates ascending; a Ruby Set in insertion order, so it is sorted where order shows.
    fn ordered(&mut self, recv: &TExpr) -> R<Expr> {
        let r = self.expr(recv)?;
        Ok(if self.is_set(&recv.ty) { call(r, "sort", vec![]) } else { r })
    }

    fn core(&mut self, op: CoreOp, recv: &TExpr, args: &[TExpr], e: &TExpr) -> R<Expr> {
        let base = self.inf.base(&recv.ty);
        Ok(match op {
            CoreOp::TextSize | CoreOp::Size => call(self.expr(recv)?, "size", vec![]),
            CoreOp::TextTrim => {
                self.f.need(TEXT);
                call(konst("Runtime::Text"), "trim", vec![self.expr(recv)?])
            }
            CoreOp::TextBytes => call(self.expr(recv)?, "bytes", vec![]),
            CoreOp::TextContains => call(self.expr(recv)?, "include?", vec![self.expr(&args[0])?]),
            CoreOp::TextStartsWith => call(self.expr(recv)?, "start_with?", vec![self.expr(&args[0])?]),
            CoreOp::ToList => self.ordered(recv)?,
            CoreOp::Keys => call(self.expr(recv)?, "keys", vec![]),
            CoreOp::Values => call(self.expr(recv)?, "values", vec![]),
            CoreOp::Get => Expr::Index(Box::new(self.expr(recv)?), Box::new(self.expr(&args[0])?)),
            CoreOp::Filter => {
                let r = self.ordered(recv)?;
                self.with_block(r, "select", args.first())?
            }
            CoreOp::Map => {
                let r = self.ordered(recv)?;
                self.with_block(r, "map", args.first())?
            }
            CoreOp::All => {
                let r = self.expr(recv)?;
                self.with_block(r, "all?", args.first())?
            }
            CoreOp::Any => {
                let r = self.expr(recv)?;
                self.with_block(r, "any?", args.first())?
            }
            CoreOp::Sort => {
                // Strategy sort.identity: sorting a Set by its own elements is its ascending order.
                if self.app.strategies.on("sort.identity")
                    && let (Type::Set(_), Some(TExpr { kind: TKind::Lambda { params, body }, .. })) =
                        (&base, args.first())
                    && matches!((&body.kind, params.as_slice()), (TKind::Local(n), [(p, _)]) if n == p)
                {
                    return self.ordered(recv);
                }
                let r = self.ordered(recv)?;
                self.f.need(VALUES);
                self.with_block(konst("Runtime"), "sort_by", args.first()).map(|c| match c {
                    Expr::Call { recv, name, block, .. } => Expr::Call { recv, name, args: vec![Arg::Pos(r)], block },
                    other => other,
                })?
            }
            CoreOp::Join => {
                let sep = self.expr(&args[0])?;
                let r = self.expr(recv)?;
                if sep == text("") { call(r, "join", vec![]) } else { call(r, "join", vec![sep]) }
            }
            CoreOp::In => {
                let item = self.expr(&args[0])?;
                let r = self.expr(recv)?;
                match base {
                    Type::Map(..) => call(r, "key?", vec![item]),
                    Type::List(_) | Type::Set(_) => call(r, "include?", vec![item]),
                    _ => return self.inf.unsupported(e.span, "`in` on a value that is not a collection"),
                }
            }
        })
    }

    fn match_chain(&mut self, scrutinee: &TExpr, arms: &[(TPat, TExpr)]) -> R<Expr> {
        let Some(((last_pat, last), rest)) = arms.split_last() else {
            return self.inf.unsupported(scrutinee.span, "a match with no arms");
        };
        let depth = self.renames.len();
        self.bind_pat(last_pat, scrutinee);
        let out = self.expr(last);
        self.renames.truncate(depth);
        let mut out = out?;
        for (pat, value) in rest.iter().rev() {
            let s = self.expr(scrutinee)?;
            let test = match pat {
                TPat::Wild => Expr::True,
                TPat::Variant { ty, tag } => bin("==", s, sym(&self.variant_name(*ty, *tag)?)),
                TPat::Nothing => call(s, "nil?", vec![]),
                TPat::Just(_) => not(call(s, "nil?", vec![])),
            };
            self.bind_pat(pat, scrutinee);
            let value = self.expr(value);
            self.renames.truncate(depth);
            out = Expr::Ternary(Box::new(test), Box::new(value?), Box::new(out));
        }
        Ok(out)
    }

    fn bind_pat(&mut self, pat: &TPat, scrutinee: &TExpr) {
        if let (TPat::Just(Some(name)), TKind::Local(s)) = (pat, &scrutinee.kind) {
            let to = self.local(s);
            self.renames.push((name.clone(), to));
        }
    }

    /// `Runtime::Outcome.new(world: ..., lines: ..., reply: ...)`: the drafts where the rule wrote.
    pub fn outcome(&mut self, reply: Option<&TExpr>) -> R<Expr> {
        let reply = match reply {
            Some(r) => self.expr(r)?,
            None => Expr::Nil,
        };
        let (world, lines) = match &self.mode {
            Mode::Rule { drafts, lines } => {
                let world = if drafts.is_empty() {
                    ident("world")
                } else {
                    let mut changed = Vec::new();
                    for id in drafts {
                        let s = self.app.p.state(*id);
                        let local = ident(&self.local_state(&s.info.name));
                        let is_map =
                            matches!(self.inf.ty(&s.ty, s.info.span).map(|t| self.inf.base(&t)), Ok(Type::Map(..)));
                        changed.push(Arg::Key(
                            s.info.name.clone(),
                            if is_map { call(local, "freeze", vec![]) } else { local },
                        ));
                    }
                    call_args(ident("world"), "with", changed)
                };
                (world, if *lines { ident("lines") } else { Expr::Array(vec![]) })
            }
            _ => (ident("world"), Expr::Array(vec![])),
        };
        self.f.need(VALUES);
        Ok(call_args(
            konst("Runtime::Outcome"),
            "new",
            vec![key("world", world), key("lines", lines), key("reply", reply)],
        ))
    }

    pub fn stmts(&mut self, stmts: &[TStmt]) -> R<Vec<Stmt>> {
        let depth = self.renames.len();
        let out = stmts.iter().map(|s| self.stmt(s)).collect();
        self.renames.truncate(depth);
        out
    }

    pub fn stmt(&mut self, s: &TStmt) -> R<Stmt> {
        let in_rule = matches!(self.mode, Mode::Rule { .. });
        let in_test = matches!(self.mode, Mode::Test);
        Ok(match &s.kind {
            TStmtKind::Let { name, value, .. } | TStmtKind::SetLocal { name, value } => {
                Stmt::Assign(self.local(name), self.expr(value)?)
            }
            TStmtKind::SetState { state, value } if in_rule => {
                Stmt::Assign(self.local_state(&state_name(self.app, *state)), self.expr(value)?)
            }
            TStmtKind::PutEntry { state, key, value } if in_rule => {
                let map = ident(&self.local_state(&state_name(self.app, *state)));
                let (k, v) = (self.expr(key)?, self.expr(value)?);
                Stmt::Expr(self.rt("put", vec![map, k, v]))
            }
            TStmtKind::SetEntryField { state, key, field, value } if in_rule => {
                let map = ident(&self.local_state(&state_name(self.app, *state)));
                let (k, k2, v) = (self.expr(key)?, self.expr(key)?, self.expr(value)?);
                let new = call_args(call(map.clone(), "fetch", vec![k2]), "with", vec![Arg::Key(field.clone(), v)]);
                Stmt::Expr(self.rt("put", vec![map, k, new]))
            }
            TStmtKind::RemoveEntry { state, key } if in_rule => Stmt::Expr(call(
                ident(&self.local_state(&state_name(self.app, *state))),
                "delete",
                vec![self.expr(key)?],
            )),
            TStmtKind::OutLine(line) if in_rule => Stmt::Expr(bin("<<", ident("lines"), self.expr(line)?)),
            TStmtKind::Fail(name) if in_rule => Stmt::Raise(self.refusal(name)),
            TStmtKind::Return(value) if in_rule => Stmt::Return(Some(self.outcome(value.as_ref())?)),
            TStmtKind::Return(None) => Stmt::Return(None),
            TStmtKind::Return(Some(v)) if !in_test => Stmt::Return(Some(self.expr(v)?)),
            TStmtKind::Expr(e) if in_test && matches!(e.kind, TKind::CallRule { .. }) => {
                Stmt::Expr(self.rule_call(e, "call")?)
            }
            TStmtKind::Expr(e) => Stmt::Expr(self.expr(e)?),
            TStmtKind::For { var, iter, body } => {
                let it = self.ordered(iter)?;
                let name = self.local(var);
                let body = self.stmts(body)?;
                Stmt::Expr(Expr::Call {
                    recv: Some(Box::new(it)),
                    name: "each".into(),
                    args: vec![],
                    block: Some(Block { params: vec![name], body }),
                })
            }
            TStmtKind::While { cond, body } => Stmt::While { cond: self.expr(cond)?, body: self.stmts(body)? },
            TStmtKind::If { cond, then, els } => {
                let cond = self.expr(cond)?;
                let then = self.stmts(then)?;
                let els = match els {
                    Some(e) => Some(self.stmts(e)?),
                    None => None,
                };
                Stmt::If { cond, then, els }
            }
            TStmtKind::ExpectValue { call: c, value } if in_test => {
                let got = self.rule_call(c, "call")?;
                let want = self.expr(value)?;
                Stmt::Expr(assert_equal(want, got))
            }
            TStmtKind::ExpectFail { call: c, error } if in_test => {
                let got = self.rule_call(c, "refused")?;
                Stmt::Expr(assert_equal(text(error), got))
            }
            TStmtKind::Assert(e) if in_test => match &e.kind {
                TKind::Compare(CmpOp::Eq, x, y) => {
                    let (x, y) = (self.expr(x)?, self.expr(y)?);
                    Stmt::Expr(assert_equal(y, x))
                }
                _ => Stmt::Expr(Expr::Call {
                    recv: None,
                    name: "assert".into(),
                    args: vec![Arg::Pos(self.expr(e)?)],
                    block: None,
                }),
            },
            _ => return self.inf.unsupported(s.span, "this statement here"),
        })
    }
}

/// `assert_equal(want, got)`; `assert_nil` when nothing is wanted, as minitest asks.
pub fn assert_equal(want: Expr, got: Expr) -> Expr {
    if want == Expr::Nil {
        return Expr::Call { recv: None, name: "assert_nil".into(), args: vec![Arg::Pos(got)], block: None };
    }
    Expr::Call { recv: None, name: "assert_equal".into(), args: vec![Arg::Pos(want), Arg::Pos(got)], block: None }
}
