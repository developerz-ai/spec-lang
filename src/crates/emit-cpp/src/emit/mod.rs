//! From LIR nodes to C++ text. [`Gen`] knows the program's declarations and
//! their C++ names; [`Fx`] prints one body, keeping its locals and whether it
//! reached for the machine.

mod conform;
mod expr;
mod pattern;
mod prims;
mod stmt;

use crate::facts::Facts;
use crate::names::{ident, namespace, snake};
use spec_lang_diag::Diag;
use spec_lang_ir::{ConstId, FnId, RuleId, StateId, TypeId};
use spec_lang_lir::{DTy, LTy, Lir, TypeDef};
use spec_lang_syntax::Span;
use std::cell::RefCell;
use std::collections::BTreeSet;

pub use expr::int_literal;
pub use stmt::Ret;

/// A span for text that comes from no one place of the source.
pub fn no_span() -> Span {
    Span::new(spec_lang_syntax::FileId(0), 0, 0)
}

/// C++ text with the precedence of its outermost operator: 16 a primary or
/// postfix expression, down to 1 for a comma.
#[derive(Clone, Debug)]
pub struct Code {
    pub s: String,
    pub p: u8,
}

impl Code {
    pub fn new(s: impl Into<String>, p: u8) -> Code {
        Code { s: s.into(), p }
    }
    pub fn atom(s: impl Into<String>) -> Code {
        Code::new(s, 16)
    }
    /// The text, in parentheses when it binds looser than `min`.
    pub fn at(&self, min: u8) -> String {
        if self.p >= min { self.s.clone() } else { format!("({})", self.s) }
    }
}

/// The program-wide part of printing.
#[derive(Debug)]
pub struct Gen<'a> {
    pub lir: &'a Lir,
    pub facts: Facts,
    pub errors: RefCell<Vec<Diag>>,
    /// Names of fns, rules and constants: a local with one of them is renamed, so a call still finds its function.
    taken: BTreeSet<String>,
    /// Per state: its field in `World`.
    state_fields: Vec<String>,
    /// Fields held in an `rt::Box`, as (type, variant or `u32::MAX`, field): those through which a type holds itself.
    boxed: BTreeSet<(u32, u32, u32)>,
}

impl<'a> Gen<'a> {
    pub fn new(lir: &'a Lir) -> Gen<'a> {
        let mut taken = BTreeSet::new();
        taken.extend(lir.fns.iter().map(|f| ident(&f.info.name)));
        taken.extend(lir.rules.iter().map(|f| ident(&f.info.name)));
        taken.extend(lir.consts.iter().map(|f| ident(&f.info.name)));
        taken.extend(lir.modules.iter().filter_map(|m| m.name.split('.').next().map(ident)));
        let state_fields = lir
            .states
            .iter()
            .map(|s| {
                let shared = lir.states.iter().filter(|o| o.info.name == s.info.name).count() > 1;
                if shared { crate::names::flat(&s.info.qualified) } else { ident(&s.info.name) }
            })
            .collect();
        let mut boxed = BTreeSet::new();
        for (i, t) in lir.types.iter().enumerate() {
            let id = TypeId(i as u32);
            let groups: Vec<(u32, &[spec_lang_lir::Field])> = match &t.def {
                TypeDef::Record { fields } => vec![(u32::MAX, fields.as_slice())],
                TypeDef::Sum { variants } if !t.builtin => {
                    variants.iter().enumerate().map(|(v, x)| (v as u32, x.fields.as_slice())).collect()
                }
                _ => Vec::new(),
            };
            for (tag, fields) in groups {
                for (f, field) in fields.iter().enumerate() {
                    if holds_by_value(lir, &field.check, id, &mut Vec::new()) {
                        boxed.insert((id.0, tag, f as u32));
                    }
                }
            }
        }
        Gen { lir, facts: Facts::of(lir), errors: RefCell::new(Vec::new()), taken, state_fields, boxed }
    }

    /// Whether a field is kept in an `rt::Box`.
    pub fn is_boxed(&self, t: TypeId, tag: Option<u32>, field: u32) -> bool {
        self.boxed.contains(&(t.0, tag.unwrap_or(u32::MAX), field))
    }

    /// `base.field`, through its box when it has one.
    pub fn field(&self, base: &str, t: TypeId, tag: Option<u32>, index: u32) -> String {
        let name = self.fields(t, tag).get(index as usize).map(|f| ident(&f.name)).unwrap_or_default();
        if self.is_boxed(t, tag, index) { format!("(*{base}.{name})") } else { format!("{base}.{name}") }
    }

    pub fn unsupported(&self, span: Span, what: impl std::fmt::Display) {
        self.errors.borrow_mut().push(Diag::unsupported(span, format!("the C++ printer does not print {what}")));
    }

    pub fn module_ns(&self, module: spec_lang_ir::ModuleId) -> String {
        namespace(&self.lir.modules[module.index()].name)
    }

    /// A sum whose variants all have no fields: an `enum class`.
    pub fn is_enum(&self, t: TypeId) -> bool {
        let d = self.lir.ty(t);
        !d.builtin && matches!(&d.def, TypeDef::Sum { variants } if variants.iter().all(|v| v.fields.is_empty()))
    }

    pub fn type_name(&self, t: TypeId) -> String {
        let d = self.lir.ty(t);
        format!("{}::{}", self.module_ns(d.info.module), ident(&d.info.name))
    }

    pub fn variant_name(&self, t: TypeId, tag: u32) -> String {
        let name = match &self.lir.ty(t).def {
            TypeDef::Sum { variants } => variants.get(tag as usize).map(|v| ident(&v.name)).unwrap_or_default(),
            _ => String::new(),
        };
        format!("{}::{name}", self.type_name(t))
    }

    /// The function that says whether a value conforms to a declared type with claims.
    pub fn valid_name(&self, t: TypeId) -> String {
        let d = self.lir.ty(t);
        let mut name = format!("valid_{}", snake(&d.info.name));
        if self.taken.contains(&name) {
            name.push('_');
        }
        format!("{}::{name}", self.module_ns(d.info.module))
    }

    pub fn fn_name(&self, f: FnId) -> String {
        let d = self.lir.func(f);
        format!("{}::{}", self.module_ns(d.info.module), ident(&d.info.name))
    }

    pub fn rule_name(&self, r: RuleId) -> String {
        let d = self.lir.rule(r);
        format!("{}::{}", self.module_ns(d.info.module), ident(&d.info.name))
    }

    /// The rule's number, as generated code names it.
    pub fn rule_const(&self, r: RuleId) -> String {
        format!("program::rule::{}", crate::names::flat(&self.lir.rule(r).info.qualified))
    }

    /// A constant whose value cannot fail is an `inline const`; any other is a function with a static.
    pub fn const_is_plain(&self, c: ConstId) -> bool {
        let fx = self.facts.effects(&self.lir.constant(c).value);
        !fx.can_fail() && !fx.hard && !fx.reads_state && !fx.writes_state && !self.facts.needs_ctx(&self.lir.constant(c).value)
    }

    pub fn const_ref(&self, c: ConstId) -> String {
        let d = self.lir.constant(c);
        let name = format!("{}::{}", self.module_ns(d.info.module), ident(&d.info.name));
        if self.const_is_plain(c) { name } else { format!("{name}()") }
    }

    pub fn state_field(&self, s: StateId) -> &str {
        &self.state_fields[s.index()]
    }

    pub fn ty(&self, t: &LTy) -> String {
        match t {
            LTy::Unit => "rt::Unit".into(),
            LTy::Bool => "bool".into(),
            LTy::Int => "rt::Int".into(),
            LTy::Text => "rt::Text".into(),
            LTy::Named(id) => self.type_name(*id),
            LTy::Option(t) => format!("rt::Option<{}>", self.ty(t)),
            LTy::List(t) => format!("rt::List<{}>", self.ty(t)),
            LTy::Set(t) => format!("rt::Set<{}>", self.ty(t)),
            LTy::Map(k, v) => format!("rt::Map<{}, {}>", self.ty(k), self.ty(v)),
            LTy::Tuple(ts) => format!("std::tuple<{}>", ts.iter().map(|t| self.ty(t)).collect::<Vec<_>>().join(", ")),
            LTy::Range => "rt::Range".into(),
            LTy::Func(ps, r) => {
                format!("std::function<{}({})>", self.ty(r), ps.iter().map(|p| self.param_ty(p)).collect::<Vec<_>>().join(", "))
            }
            LTy::Event(_) => "rt::Event".into(),
            LTy::Reply(_) => "rt::Reply".into(),
            LTy::Failure => "rt::Failure".into(),
            LTy::Step(_) => "rt::Step<program::World>".into(),
            LTy::World => "program::World".into(),
            LTy::Trace(_) => "rt::Trace<program::World>".into(),
            LTy::Never | LTy::Var(_) => "rt::Never".into(),
        }
    }

    /// A declared type by the names the program gave: an alias is its `using`.
    pub fn dty(&self, d: &DTy) -> String {
        match d {
            DTy::Unit => "rt::Unit".into(),
            DTy::Bool => "bool".into(),
            DTy::Int | DTy::Range { .. } => "rt::Int".into(),
            DTy::Text => "rt::Text".into(),
            DTy::Named(id) => match &self.lir.ty(*id).def {
                TypeDef::RuleSet { .. } => "rt::Event".into(),
                _ => self.type_name(*id),
            },
            DTy::Option(t) => format!("rt::Option<{}>", self.dty(t)),
            DTy::List(t) => format!("rt::List<{}>", self.dty(t)),
            DTy::Set(t) => format!("rt::Set<{}>", self.dty(t)),
            DTy::Map(k, v) => format!("rt::Map<{}, {}>", self.dty(k), self.dty(v)),
            DTy::Tuple(ts) => format!("std::tuple<{}>", ts.iter().map(|t| self.dty(t)).collect::<Vec<_>>().join(", ")),
            DTy::Where(b, _) => self.dty(b),
            DTy::Trace(_) => "rt::Trace<program::World>".into(),
        }
    }

    /// Small values travel by value, the rest by reference to const.
    pub fn by_value(&self, t: &LTy) -> bool {
        match t {
            LTy::Unit | LTy::Bool | LTy::Int | LTy::Range => true,
            LTy::Named(id) => self.is_enum(*id),
            _ => false,
        }
    }

    pub fn param_ty(&self, t: &LTy) -> String {
        if self.by_value(t) { self.ty(t) } else { format!("const {}&", self.ty(t)) }
    }

    /// A parameter's type as declared (`const Title&`), passed as its erased type asks.
    pub fn param_dty(&self, t: &LTy, d: &DTy) -> String {
        let name = if matches!(t, LTy::Func(..) | LTy::Step(_) | LTy::Event(_) | LTy::Reply(_) | LTy::World) {
            self.ty(t)
        } else {
            self.dty(d)
        };
        if self.by_value(t) { name } else { format!("const {name}&") }
    }

    /// The first parameter from which every later one has a default C++ can state: a closed expression
    /// that needs no machine. Arguments before it are always written at the call.
    pub fn first_cpp_default(&self, params: &[spec_lang_lir::Param]) -> usize {
        params
            .iter()
            .rposition(|p| !p.default.as_ref().is_some_and(|d| closed(d) && !self.facts.needs_ctx(d)))
            .map_or(0, |i| i + 1)
    }

    pub fn fields(&self, t: TypeId, tag: Option<u32>) -> &'a [spec_lang_lir::Field] {
        match (&self.lir.ty(t).def, tag) {
            (TypeDef::Record { fields }, None) => fields,
            (TypeDef::Sum { variants }, Some(tag)) => variants.get(tag as usize).map(|v| v.fields.as_slice()).unwrap_or(&[]),
            _ => &[],
        }
    }
}

/// One body being printed.
#[derive(Debug)]
pub struct Fx<'g, 'a> {
    pub g: &'g Gen<'a>,
    /// Source name to C++ text, innermost scope last.
    scopes: Vec<Vec<(String, String)>>,
    used: BTreeSet<String>,
    /// How this body names the machine, when it has one.
    ctx: Option<String>,
    pub ctx_used: bool,
    /// C++ names of locals the printed body reads.
    read: BTreeSet<String>,
    counter: u32,
    /// Where the body is declared, for a diagnostic.
    pub span: Span,
}

impl<'g, 'a> Fx<'g, 'a> {
    pub fn new(g: &'g Gen<'a>, ctx: Option<&str>, span: Span) -> Fx<'g, 'a> {
        Fx {
            g,
            scopes: vec![Vec::new()],
            used: BTreeSet::new(),
            ctx: ctx.map(str::to_string),
            ctx_used: false,
            read: BTreeSet::new(),
            counter: 0,
            span,
        }
    }

    /// A mark before a declaration, which [`Fx::resolve`] turns into `[[maybe_unused]] ` when nothing read the name.
    pub fn mark(&self, cpp: &str) -> String {
        format!("\u{1}{cpp}\u{2}")
    }

    /// The printed body with every mark settled.
    pub fn resolve(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(at) = rest.find('\u{1}') {
            out.push_str(&rest[..at]);
            let after = &rest[at + 1..];
            let end = after.find('\u{2}').unwrap_or(after.len());
            if !self.read.contains(&after[..end]) {
                out.push_str("[[maybe_unused]] ");
            }
            rest = after.get(end + 1..).unwrap_or("");
        }
        out.push_str(rest);
        out
    }

    /// A C++ name this body may not give a local: one its surroundings already use.
    pub fn reserve(&mut self, name: &str) {
        self.used.insert(name.to_string());
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// A fresh C++ name for a source local, visible until its scope ends.
    pub fn declare(&mut self, name: &str) -> String {
        let base = ident(name);
        let mut cpp = base.clone();
        let mut n = 1;
        while self.used.contains(&cpp) || self.g.taken.contains(&cpp) {
            n += 1;
            cpp = format!("{base}_{n}");
        }
        self.used.insert(cpp.clone());
        if let Some(s) = self.scopes.last_mut() {
            s.push((name.to_string(), cpp.clone()));
        }
        cpp
    }

    /// A source local that is some C++ expression (a claim's `self`, a pretend's state).
    pub fn bind(&mut self, name: &str, text: &str) {
        if let Some(s) = self.scopes.last_mut() {
            s.push((name.to_string(), text.to_string()));
        }
    }

    /// A name of the printer's own.
    pub fn fresh(&mut self, base: &str) -> String {
        loop {
            self.counter += 1;
            let name = if self.counter == 1 { base.to_string() } else { format!("{base}_{}", self.counter) };
            if !self.used.contains(&name) && !self.g.taken.contains(&name) {
                self.used.insert(name.clone());
                return name;
            }
        }
    }

    pub fn local(&mut self, name: &str) -> String {
        let cpp = self
            .scopes
            .iter()
            .rev()
            .flat_map(|s| s.iter().rev())
            .find(|(n, _)| n == name)
            .map(|(_, c)| c.clone())
            .unwrap_or_else(|| ident(name));
        self.read.insert(cpp.clone());
        cpp
    }

    /// The machine, where this body has one.
    pub fn ctx(&mut self, span: Span) -> String {
        self.ctx_used = true;
        match &self.ctx {
            Some(c) => c.clone(),
            None => {
                self.g.unsupported(span, "a state, a rule or a handler reached from a body that has no machine");
                "ctx".into()
            }
        }
    }
}

/// Whether an expression reads no local: a default that can stand anywhere.
pub fn closed(e: &spec_lang_lir::node::Expr) -> bool {
    let mut reads = false;
    spec_lang_lir::walk::exprs(e, &mut |x| reads |= matches!(&x.kind, spec_lang_lir::node::ExprKind::Local(_)));
    !reads
}

/// Whether a value of `d` holds a value of `target` in place: not through a List, a Set or a Map.
fn holds_by_value(lir: &Lir, d: &DTy, target: TypeId, seen: &mut Vec<TypeId>) -> bool {
    match d {
        DTy::Named(t) if *t == target => true,
        DTy::Named(t) => {
            if seen.contains(t) || lir.ty(*t).builtin {
                return false;
            }
            seen.push(*t);
            match &lir.ty(*t).def {
                TypeDef::Alias { of } => holds_by_value(lir, of, target, seen),
                TypeDef::Record { fields } => fields.iter().any(|f| holds_by_value(lir, &f.check, target, seen)),
                TypeDef::Sum { variants } => {
                    variants.iter().flat_map(|v| &v.fields).any(|f| holds_by_value(lir, &f.check, target, seen))
                }
                TypeDef::RuleSet { .. } => false,
            }
        }
        DTy::Option(t) | DTy::Where(t, _) => holds_by_value(lir, t, target, seen),
        DTy::Tuple(ts) => ts.iter().any(|t| holds_by_value(lir, t, target, seen)),
        _ => false,
    }
}
