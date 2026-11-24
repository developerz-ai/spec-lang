//! The typed tree to swc expressions and statements.
//!
//! Representation, fixed for the backend: `Int` is a `bigint`, a `Size` a
//! `number`, `Text` a `string`, a sum with no data its variant name, a record
//! a frozen object, `Option[T]` is `T | undefined`, a `Set` a frozen sorted
//! array, a `Map` a JS `Map` kept in ascending key order. A rule is a pure
//! function `(w, args) -> Outcome`: guards first against `w`, then the body
//! on a draft of the states it writes.

use super::build as b;
use super::file::File;
use crate::sem::app::App;
use crate::sem::infer::{Infer, Named, R};
use crate::sem::names::{camel, screaming};
use crate::sem::tree::*;
use spec_lang_ir::{BinOp, CmpOp, FnId, ModuleId, RuleId, StateId, TypeId};
use std::collections::BTreeSet;
use swc_ecma_ast::{BinaryOp, Stmt, TsKeywordTypeKind};

pub const VALUES: &str = "src/runtime/values.ts";
pub const TEXT: &str = "src/runtime/text.ts";
pub const OUTCOME_RT: &str = "src/runtime/outcome.ts";
pub const WORLD: &str = "src/app/world.ts";
pub const OUTCOME: &str = "src/app/outcome.ts";

/// Names the generated code uses itself; a local with one of them gets a `_`.
const RESERVED: &[&str] = &[
    "w",
    "b",
    "lines",
    "fc",
    "prefix",
    "events",
    "expect",
    "test",
    "undefined",
    "at",
    "put",
    "update",
    "equals",
    "compare",
    "sortBy",
    "setOf",
    "mapOf",
    "size",
    "trim",
    "bytes",
    "isText",
    "everyByte",
    "someByte",
    "draft",
    "commit",
    "refuse",
    "claimed",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
    "let",
    "static",
    "await",
    "arguments",
    "eval",
    "interface",
    "package",
    "private",
    "protected",
    "public",
    "implements",
];

pub fn module_dir(app: &App<'_>, m: ModuleId) -> String {
    app.p.module(m).name.replace('.', "/")
}

pub fn type_file(app: &App<'_>, id: TypeId) -> String {
    let info = &app.p.ty(id).info;
    format!("src/{}/{}.ts", module_dir(app, info.module), crate::sem::names::kebab(&info.name))
}

pub fn fn_file(app: &App<'_>, id: FnId) -> String {
    let info = &app.p.func(id).info;
    format!("src/{}/{}.ts", module_dir(app, info.module), crate::sem::names::kebab(&info.name))
}

pub fn rule_file(app: &App<'_>, id: RuleId) -> String {
    let info = &app.p.rule(id).info;
    format!("src/{}/{}.ts", module_dir(app, info.module), crate::sem::names::kebab(&info.name))
}

/// The constant holding a sum's variants.
pub fn variants_name(type_name: &str) -> String {
    format!("{}_VARIANTS", screaming(type_name))
}

pub fn state_name(app: &App<'_>, id: StateId) -> String {
    camel(&app.p.state(id).info.name)
}

#[derive(Debug)]
pub enum Mode {
    /// A `fn`, a claim, a law: no state but what a world value gives.
    Pure,
    /// A rule's guards: state is read from `w`.
    Guard,
    /// A rule's body: written states are local drafts.
    Rule { drafts: BTreeSet<StateId>, lines: bool },
    /// A test: state is `b.world`, rules are called through `b`.
    Test,
}

#[derive(Debug)]
pub struct Lower<'a, 'p> {
    pub app: &'a App<'p>,
    pub inf: Infer<'a>,
    pub f: &'a mut File,
    pub mode: Mode,
    renames: Vec<(String, String)>,
}

impl<'a, 'p> Lower<'a, 'p> {
    pub fn new(app: &'a App<'p>, f: &'a mut File, mode: Mode, decl: &str) -> Self {
        let mut inf = app.types_of();
        inf.decl = decl.to_string();
        Lower { app, inf, f, mode, renames: Vec::new() }
    }

    pub fn local(&self, name: &str) -> String {
        if let Some((_, to)) = self.renames.iter().rev().find(|(from, _)| from == name) {
            return to.clone();
        }
        let n = camel(name);
        if RESERVED.contains(&n.as_str()) { format!("{n}_") } else { n }
    }

    fn rt(&mut self, name: &str) -> b::E {
        self.f.value(VALUES, name)
    }

    /// The TypeScript type of a value type.
    pub fn ty(&mut self, t: &Type) -> R<b::T> {
        Ok(match t {
            Type::Unit => b::keyword(TsKeywordTypeKind::TsUndefinedKeyword),
            Type::Bool => b::keyword(TsKeywordTypeKind::TsBooleanKeyword),
            Type::Int => b::keyword(TsKeywordTypeKind::TsBigIntKeyword),
            Type::Size => b::keyword(TsKeywordTypeKind::TsNumberKeyword),
            Type::Text => b::keyword(TsKeywordTypeKind::TsStringKeyword),
            Type::Named(id) => {
                let file = type_file(self.app, *id);
                let name = self.app.p.ty(*id).info.name.clone();
                self.f.type_ref(&file, &name, vec![])
            }
            Type::Option(t) => b::t_union(vec![self.ty(t)?, b::keyword(TsKeywordTypeKind::TsUndefinedKeyword)]),
            Type::List(t) | Type::Set(t) => b::t_list(self.ty(t)?),
            Type::Map(k, v) => b::t_ref("ReadonlyMap", vec![self.ty(k)?, self.ty(v)?]),
            Type::Tuple(ts) => b::t_tuple(ts.iter().map(|t| self.ty(t)).collect::<R<_>>()?),
            Type::World => self.f.type_ref(WORLD, "World", vec![]),
            Type::Trace(_) | Type::Step(_) | Type::Event(_) | Type::Reply => {
                return self.inf.unsupported(self.app.decl.info.span, "a trace type outside a law");
            }
        })
    }

    /// True when `===` is structural equality for the type.
    fn primitive(&self, t: &Type) -> bool {
        match self.inf.base(t) {
            Type::Unit | Type::Bool | Type::Int | Type::Size | Type::Text => true,
            Type::Named(id) => matches!(self.inf.named(id), Ok(Named::Sum(_))),
            Type::Option(t) => self.primitive(&t),
            _ => false,
        }
    }

    fn state(&mut self, id: StateId, e: &TExpr) -> R<b::E> {
        let name = state_name(self.app, id);
        match &self.mode {
            Mode::Guard => Ok(b::member(b::name("w"), &name)),
            Mode::Rule { drafts, .. } => {
                Ok(if drafts.contains(&id) { b::name(&name) } else { b::member(b::name("w"), &name) })
            }
            Mode::Test => Ok(b::member(b::member(b::name("b"), "world"), &name)),
            Mode::Pure => self.inf.unsupported(e.span, "a `fn` that reads state"),
        }
    }

    /// Call arguments: trailing defaults are left out, a default in the middle is `undefined`.
    fn args(&mut self, args: &[Option<TExpr>]) -> R<Vec<b::E>> {
        let keep = args.iter().rposition(|a| a.is_some()).map_or(0, |i| i + 1);
        args[..keep]
            .iter()
            .map(|a| match a {
                Some(a) => self.expr(a),
                None => Ok(b::undefined()),
            })
            .collect()
    }

    pub fn fn_ref(&mut self, id: FnId) -> b::E {
        let file = fn_file(self.app, id);
        let name = camel(&self.app.p.func(id).info.name);
        self.f.value(&file, &name)
    }

    pub fn rule_ref(&mut self, id: RuleId) -> b::E {
        let file = rule_file(self.app, id);
        let name = camel(&self.app.p.rule(id).info.name);
        self.f.value(&file, &name)
    }

    /// The value that holds a claimed type's `parse` and `of`.
    pub fn claimed_ref(&mut self, id: TypeId) -> b::E {
        let file = type_file(self.app, id);
        let name = self.app.p.ty(id).info.name.clone();
        self.f.value(&file, &name)
    }

    pub fn variants_ref(&mut self, id: TypeId) -> b::E {
        let file = type_file(self.app, id);
        let name = variants_name(&self.app.p.ty(id).info.name);
        self.f.value(&file, &name)
    }

    fn variant_name(&self, ty: TypeId, tag: u32) -> R<String> {
        match self.inf.named(ty)? {
            Named::Sum(variants) => Ok(variants.get(tag as usize).map(|v| v.name.clone()).unwrap_or_default()),
            _ => self.inf.unsupported(self.app.p.ty(ty).info.span, "a variant of a type that is not a sum"),
        }
    }

    /// One rule call in a test: `b.call((w) => rule(w, args))`, or `b.refused(...)`.
    pub fn rule_call(&mut self, e: &TExpr, how: &str) -> R<b::E> {
        let TKind::CallRule { rule, args } = &e.kind else {
            return self.inf.unsupported(e.span, "a step that is not a rule call");
        };
        let mut call_args = vec![b::name("w")];
        call_args.extend(self.args(args)?);
        let callee = self.rule_ref(*rule);
        Ok(b::method(b::name("b"), how, vec![b::arrow(vec![b::bare("w")], b::call(callee, call_args))]))
    }

    pub fn expr(&mut self, e: &TExpr) -> R<b::E> {
        Ok(match &e.kind {
            TKind::Unit => b::undefined(),
            TKind::Bool(v) => b::boolean(*v),
            TKind::Int(i) => {
                if e.ty == Type::Size {
                    b::num(*i as f64)
                } else {
                    b::big(*i)
                }
            }
            TKind::Text(s) => b::text(s),
            TKind::Interp(parts) => {
                let mut quasis = vec![String::new()];
                let mut exprs = Vec::new();
                for p in parts {
                    match p {
                        TPart::Lit(s) => {
                            if let Some(last) = quasis.last_mut() {
                                last.push_str(s);
                            }
                        }
                        TPart::Expr(x) => {
                            exprs.push(self.expr(x)?);
                            quasis.push(String::new());
                        }
                    }
                }
                b::template(quasis, exprs)
            }
            TKind::Local(n) => b::name(&self.local(n)),
            TKind::Const(id) => {
                let info = &self.app.p.constant(*id).info;
                let file =
                    format!("src/{}/{}.ts", module_dir(self.app, info.module), crate::sem::names::kebab(&info.name));
                self.f.value(&file, &screaming(&info.name))
            }
            TKind::State(id) => self.state(*id, e)?,
            TKind::Variant { ty, tag } => b::text(&self.variant_name(*ty, *tag)?),
            TKind::Nothing => b::undefined(),
            TKind::Just(x) => self.expr(x)?,
            TKind::CallFn { func, args } => {
                let args = self.args(args)?;
                b::call(self.fn_ref(*func), args)
            }
            TKind::CallRule { .. } => match self.mode {
                Mode::Test => self.rule_call(e, "call")?,
                _ => return self.inf.unsupported(e.span, "a rule called outside a test"),
            },
            TKind::Record { ty, args } => {
                let args = self.args(args)?;
                let file = type_file(self.app, *ty);
                let name = format!("make{}", self.app.p.ty(*ty).info.name);
                b::call(self.f.value(&file, &name), args)
            }
            TKind::Field { recv, name } => b::member(self.expr(recv)?, &camel(name)),
            TKind::Core { op, recv, args } => self.core(*op, recv, args, e)?,
            TKind::Index { recv, key } => {
                let (m, k) = (self.expr(recv)?, self.expr(key)?);
                b::call(self.rt("at"), vec![m, k])
            }
            TKind::With { recv, fields } => {
                let r = self.expr(recv)?;
                let patch = fields.iter().map(|(n, v)| Ok((camel(n), self.expr(v)?))).collect::<R<Vec<_>>>()?;
                b::call(self.rt("update"), vec![r, b::object(patch)])
            }
            TKind::Not(x) => b::not(self.expr(x)?),
            TKind::Neg(x) => b::neg(self.expr(x)?),
            TKind::Binary(op, x, y) => {
                let (x, y) = (self.expr(x)?, self.expr(y)?);
                match op {
                    BinOp::And => b::bin(BinaryOp::LogicalAnd, x, y),
                    BinOp::Or => b::bin(BinaryOp::LogicalOr, x, y),
                    BinOp::Implies => b::bin(BinaryOp::LogicalOr, b::not(x), y),
                    BinOp::Add => b::bin(BinaryOp::Add, x, y),
                    BinOp::Sub => b::bin(BinaryOp::Sub, x, y),
                    BinOp::Mul => b::bin(BinaryOp::Mul, x, y),
                    _ => return self.inf.unsupported(e.span, "this operator"),
                }
            }
            TKind::Compare(op, x, y) => self.compare(*op, x, y)?,
            TKind::Tuple(xs) | TKind::List(xs) => b::array(xs.iter().map(|x| self.expr(x)).collect::<R<_>>()?),
            TKind::SetOf(xs) => {
                let items = b::array(xs.iter().map(|x| self.expr(x)).collect::<R<_>>()?);
                if xs.is_empty() { items } else { b::call(self.rt("setOf"), vec![items]) }
            }
            TKind::MapOf(pairs) => {
                let entries = pairs
                    .iter()
                    .map(|(k, v)| Ok(b::array(vec![self.expr(k)?, self.expr(v)?])))
                    .collect::<R<Vec<_>>>()?;
                let (Type::Map(k, v), true) = (self.inf.base(&e.ty), pairs.is_empty()) else {
                    let call = self.rt("mapOf");
                    return Ok(b::call(call, vec![b::array(entries)]));
                };
                b::new("Map", vec![self.ty(&k)?, self.ty(&v)?], vec![])
            }
            TKind::Lambda { params, body } => {
                let pats = params.iter().map(|(n, _)| b::bare(&self.local(n))).collect();
                b::arrow(pats, self.expr(body)?)
            }
            TKind::If { cond, then, els } => b::cond(self.expr(cond)?, self.expr(then)?, self.expr(els)?),
            TKind::Match { scrutinee, arms } => self.match_chain(scrutinee, arms)?,
            TKind::Claim(x) => {
                let Type::Named(id) = &e.ty else { return self.inf.unsupported(e.span, "a claim on an unnamed type") };
                let value = self.expr(x)?;
                b::method(self.claimed_ref(*id), "of", vec![value])
            }
            TKind::Widen(x) => b::call(b::name("BigInt"), vec![self.expr(x)?]),
            TKind::OutText => b::member(b::name("b"), "outText"),
            TKind::Trace { recv, prop } => {
                let r = self.expr(recv)?;
                match prop {
                    TraceProp::States => b::member(r, "states"),
                    TraceProp::Steps => b::member(r, "steps"),
                    TraceProp::Before => b::member(r, "before"),
                    TraceProp::After => b::member(r, "after"),
                    TraceProp::Event => b::member(r, "event"),
                    TraceProp::Reply => b::member(r, "reply"),
                    TraceProp::State(id) => b::member(r, &state_name(self.app, *id)),
                    TraceProp::Arg(n) => b::member(r, &camel(n)),
                }
            }
            TKind::IsRule { event, rule } => {
                let name = self.app.p.rule(*rule).info.name.clone();
                b::bin(BinaryOp::EqEqEq, b::member(self.expr(event)?, "rule"), b::text(&name))
            }
            TKind::IsRefused { reply, error } => {
                let r = self.expr(reply)?;
                match error {
                    None => b::not(b::member(r, "ok")),
                    Some(name) => {
                        let again = self.expr(reply)?;
                        b::bin(
                            BinaryOp::LogicalAnd,
                            b::not(b::member(r, "ok")),
                            b::bin(BinaryOp::EqEqEq, b::member(again, "refusal"), b::text(name)),
                        )
                    }
                }
            }
            TKind::Block(_) => return self.inf.unsupported(e.span, "a block used as a value inside an expression"),
        })
    }

    fn core(&mut self, op: CoreOp, recv: &TExpr, args: &[TExpr], e: &TExpr) -> R<b::E> {
        let base = self.inf.base(&recv.ty);
        // Strategy sort.identity: a Set iterates ascending, so sorting it by its own elements changes nothing.
        if self.app.strategies.on("sort.identity")
            && let (CoreOp::Sort, Type::Set(_), [TExpr { kind: TKind::Lambda { params, body }, .. }]) =
                (op, &base, args)
            && matches!((&body.kind, params.as_slice()), (TKind::Local(n), [(p, _)]) if n == p)
        {
            return self.expr(recv);
        }
        // Strategy bytes.no-array: `t.bytes.all(p)` reads the text's bytes without building the list.
        if self.app.strategies.on("bytes.no-array")
            && let (CoreOp::All | CoreOp::Any, TKind::Core { op: CoreOp::TextBytes, recv: text, .. }, [f]) =
                (op, &recv.kind, args)
        {
            let (t, f) = (self.expr(text)?, self.expr(f)?);
            let helper = if op == CoreOp::All { "everyByte" } else { "someByte" };
            return Ok(b::call(self.f.value(TEXT, helper), vec![t, f]));
        }
        let r = self.expr(recv)?;
        let a: Vec<b::E> = args.iter().map(|a| self.expr(a)).collect::<R<_>>()?;
        let mut a = a.into_iter();
        let mut first = || a.next().unwrap_or_else(b::undefined);
        Ok(match op {
            CoreOp::TextSize => b::call(self.f.value(TEXT, "size"), vec![r]),
            CoreOp::TextTrim => b::call(self.f.value(TEXT, "trim"), vec![r]),
            CoreOp::TextBytes => b::call(self.f.value(TEXT, "bytes"), vec![r]),
            // A text is well-formed, so matching UTF-16 units matches code points.
            CoreOp::TextContains => b::method(r, "includes", vec![first()]),
            CoreOp::TextStartsWith => b::method(r, "startsWith", vec![first()]),
            CoreOp::Size => b::member(r, if matches!(base, Type::Map(..)) { "size" } else { "length" }),
            // A Set is held as a sorted array, which is its list.
            CoreOp::ToList => r,
            CoreOp::Keys => b::spread_array(b::method(r, "keys", vec![])),
            CoreOp::Values => b::spread_array(b::method(r, "values", vec![])),
            CoreOp::Get => b::method(r, "get", vec![first()]),
            CoreOp::Filter => b::method(r, "filter", vec![first()]),
            CoreOp::Map => b::method(r, "map", vec![first()]),
            CoreOp::All => b::method(r, "every", vec![first()]),
            CoreOp::Any => b::method(r, "some", vec![first()]),
            CoreOp::Sort => b::call(self.rt("sortBy"), vec![r, first()]),
            CoreOp::Join => b::method(r, "join", vec![first()]),
            CoreOp::In => match base {
                Type::Map(..) => b::method(r, "has", vec![first()]),
                Type::List(t) | Type::Set(t) if self.primitive(&t) => b::method(r, "includes", vec![first()]),
                _ => return self.inf.unsupported(e.span, "`in` on a collection of records or tuples"),
            },
        })
    }

    fn compare(&mut self, op: CmpOp, x: &TExpr, y: &TExpr) -> R<b::E> {
        let primitive = self.primitive(&x.ty);
        let numeric = matches!(self.inf.base(&x.ty), Type::Int | Type::Size);
        let (a, c) = (self.expr(x)?, self.expr(y)?);
        let ordered = |this: &mut Self, op: BinaryOp, a, c| {
            if numeric { b::bin(op, a, c) } else { b::bin(op, b::call(this.rt("compare"), vec![a, c]), b::num(0.0)) }
        };
        Ok(match op {
            CmpOp::Eq if primitive => b::bin(BinaryOp::EqEqEq, a, c),
            CmpOp::Ne if primitive => b::bin(BinaryOp::NotEqEq, a, c),
            CmpOp::Eq => b::call(self.rt("equals"), vec![a, c]),
            CmpOp::Ne => b::not(b::call(self.rt("equals"), vec![a, c])),
            CmpOp::Lt => ordered(self, BinaryOp::Lt, a, c),
            CmpOp::Le => ordered(self, BinaryOp::LtEq, a, c),
            CmpOp::Gt => ordered(self, BinaryOp::Gt, a, c),
            CmpOp::Ge => ordered(self, BinaryOp::GtEq, a, c),
            CmpOp::In => return self.inf.unsupported(x.span, "`in` here"),
        })
    }

    /// A match as a chain of conditionals; the last arm needs no test, the match being exhaustive.
    fn match_chain(&mut self, scrutinee: &TExpr, arms: &[(TPat, TExpr)]) -> R<b::E> {
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
                TPat::Wild => b::boolean(true),
                TPat::Variant { ty, tag } => b::bin(BinaryOp::EqEqEq, s, b::text(&self.variant_name(*ty, *tag)?)),
                TPat::Nothing => b::bin(BinaryOp::EqEqEq, s, b::undefined()),
                TPat::Just(_) => b::bin(BinaryOp::NotEqEq, s, b::undefined()),
            };
            self.bind_pat(pat, scrutinee);
            let value = self.expr(value);
            self.renames.truncate(depth);
            out = b::cond(test, value?, out);
        }
        Ok(out)
    }

    /// `Some(x)` binds `x` to the scrutinee itself: the test before it has narrowed its type.
    fn bind_pat(&mut self, pat: &TPat, scrutinee: &TExpr) {
        if let (TPat::Just(Some(name)), TKind::Local(s)) = (pat, &scrutinee.kind) {
            let to = self.local(s);
            self.renames.push((name.clone(), to));
        }
    }

    /// The world a rule commits: its drafts where it wrote, `w` elsewhere.
    fn committed(&mut self) -> b::E {
        let Mode::Rule { drafts, .. } = &self.mode else { return b::name("w") };
        if drafts.is_empty() {
            return b::name("w");
        }
        let fields = self
            .app
            .world
            .iter()
            .map(|id| {
                let name = state_name(self.app, *id);
                let value = if drafts.contains(id) { b::name(&name) } else { b::member(b::name("w"), &name) };
                (name, value)
            })
            .collect();
        b::object(fields)
    }

    fn commit(&mut self, reply: Option<&TExpr>) -> R<Stmt> {
        let reply = match reply {
            Some(r) => self.expr(r)?,
            None => b::undefined(),
        };
        let lines = match &self.mode {
            Mode::Rule { lines: true, .. } => b::name("lines"),
            _ => b::array(vec![]),
        };
        let world = self.committed();
        let commit = self.f.value(OUTCOME_RT, "commit");
        Ok(b::ret(Some(b::call(commit, vec![world, lines, reply]))))
    }

    /// The `return` that ends a rule whose body ran off its end.
    pub fn commit_end(&mut self) -> R<Stmt> {
        self.commit(None)
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
            TStmtKind::Let { name, mutable, value } => {
                let v = self.expr(value)?;
                let n = self.local(name);
                if *mutable { b::let_(&n, None, v) } else { b::const_(&n, None, v) }
            }
            TStmtKind::SetLocal { name, value } => b::assign(&self.local(name), self.expr(value)?),
            TStmtKind::SetState { state, value } if in_rule => {
                b::assign(&state_name(self.app, *state), self.expr(value)?)
            }
            TStmtKind::PutEntry { state, key, value } if in_rule => {
                let (k, v) = (self.expr(key)?, self.expr(value)?);
                b::expr_stmt(b::call(self.rt("put"), vec![b::name(&state_name(self.app, *state)), k, v]))
            }
            TStmtKind::SetEntryField { state, key, field, value } if in_rule => {
                let map = state_name(self.app, *state);
                let (k, k2, v) = (self.expr(key)?, self.expr(key)?, self.expr(value)?);
                let old = b::call(self.rt("at"), vec![b::name(&map), k2]);
                let new = b::call(self.rt("update"), vec![old, b::object(vec![(camel(field), v)])]);
                b::expr_stmt(b::call(self.rt("put"), vec![b::name(&map), k, new]))
            }
            TStmtKind::RemoveEntry { state, key } if in_rule => {
                b::expr_stmt(b::method(b::name(&state_name(self.app, *state)), "delete", vec![self.expr(key)?]))
            }
            TStmtKind::OutLine(text) if in_rule => {
                b::expr_stmt(b::method(b::name("lines"), "push", vec![self.expr(text)?]))
            }
            TStmtKind::Fail(name) if in_rule => {
                b::ret(Some(b::call(self.f.value(OUTCOME_RT, "refuse"), vec![b::text(name)])))
            }
            TStmtKind::Return(value) if in_rule => self.commit(value.as_ref())?,
            TStmtKind::Return(None) => b::ret(None),
            TStmtKind::Return(Some(v)) if !in_test => b::ret(Some(self.expr(v)?)),
            TStmtKind::Expr(e) if in_test && matches!(e.kind, TKind::CallRule { .. }) => {
                b::expr_stmt(self.rule_call(e, "call")?)
            }
            TStmtKind::Expr(e) => b::expr_stmt(self.expr(e)?),
            TStmtKind::For { var, iter, body } => {
                let it = self.expr(iter)?;
                b::for_of(&self.local(var), it, self.stmts(body)?)
            }
            TStmtKind::While { cond, body } => b::while_(self.expr(cond)?, self.stmts(body)?),
            TStmtKind::If { cond, then, els } => {
                let c = self.expr(cond)?;
                let t = self.stmts(then)?;
                let e = match els {
                    Some(e) => Some(self.stmts(e)?),
                    None => None,
                };
                b::if_(c, t, e)
            }
            TStmtKind::ExpectValue { call, value } if in_test => {
                let got = self.rule_call(call, "call")?;
                let want = self.expr(value)?;
                b::expr_stmt(b::method(b::call(self.f.value("bun:test", "expect"), vec![got]), "toEqual", vec![want]))
            }
            TStmtKind::ExpectFail { call, error } if in_test => {
                let got = self.rule_call(call, "refused")?;
                b::expr_stmt(b::method(
                    b::call(self.f.value("bun:test", "expect"), vec![got]),
                    "toBe",
                    vec![b::text(error)],
                ))
            }
            TStmtKind::Assert(e) if in_test => {
                let expect = self.f.value("bun:test", "expect");
                match &e.kind {
                    TKind::Compare(CmpOp::Eq, x, y) => {
                        let (x, y) = (self.expr(x)?, self.expr(y)?);
                        b::expr_stmt(b::method(b::call(expect, vec![x]), "toEqual", vec![y]))
                    }
                    _ => b::expr_stmt(b::method(b::call(expect, vec![self.expr(e)?]), "toBe", vec![b::boolean(true)])),
                }
            }
            _ => return self.inf.unsupported(s.span, "this statement here"),
        })
    }
}
