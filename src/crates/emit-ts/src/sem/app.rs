//! The plan of one app: everything both emitters derive before printing.
//!
//! Built once from the IR: the world (every state the app's rules can touch),
//! each rule typed and analysed (write set, lock mode), the command-line
//! parameter table and help text of the `cli` vocabulary, the store schema of
//! `@store`, the state laws a store must satisfy, and the typed tests and laws.

use super::infer::{BodyKind, Infer, Named, R};
use super::tree::*;
use spec_lang_diag::Diag;
use spec_lang_ir as ir;
use spec_lang_ir::{AppDecl, FnId, LawId, ModuleId, Program, RuleId, StateId, TestId, TypeId};
use spec_lang_syntax::Span;
use std::collections::BTreeSet;

/// Every optimisation an emitter may apply, with the fact that licenses it. Each can be switched off;
/// the plain translation is what remains. A backend ignores a name it has no use for.
pub const STRATEGIES: &[(&str, &str)] = &[
    (
        "draft.take-or-copy",
        "a rule is atomic and the process commits once, so a map the store read for one call is written in place",
    ),
    (
        "store.canonical-fast-path",
        "the store's canonical bytes are fixed by the backend and this program is their writer",
    ),
    ("bytes.no-array", "`t.bytes` feeds only `all` or `any`, so the list is never needed"),
    ("match.table", "a match maps every variant of a data-less sum to a constant"),
    ("sort.identity", "a Set iterates ascending, so sorting it by its own elements changes nothing"),
];

/// Which strategies are on. All of them, unless some were switched off.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Strategies {
    off: BTreeSet<String>,
}

impl Strategies {
    /// Every strategy but the named ones; an unknown name is an error, never ignored.
    pub fn without(names: &[&str]) -> Result<Strategies, String> {
        let mut off = BTreeSet::new();
        for n in names {
            if !STRATEGIES.iter().any(|(known, _)| known == n) {
                let known: Vec<&str> = STRATEGIES.iter().map(|(k, _)| *k).collect();
                return Err(format!("unknown strategy `{n}`; the strategies are: {}", known.join(", ")));
            }
            off.insert(n.to_string());
        }
        Ok(Strategies { off })
    }

    /// Only the plain translation.
    pub fn none() -> Strategies {
        Strategies { off: STRATEGIES.iter().map(|(k, _)| k.to_string()).collect() }
    }

    pub fn on(&self, name: &str) -> bool {
        !self.off.contains(name)
    }
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub default: Option<TExpr>,
    pub span: Span,
}

/// How a rule uses the store, from its write set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Writes no state: shared lock, the store is never rewritten.
    Read,
    Write,
}

/// How a parameter appears on the command line (`docs/cli-vocabulary.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// No default: `<name>`.
    Positional,
    /// `Bool = false`: `--name`.
    Switch,
    /// `Option[T] = none` or a plain default: `--name <value>`, at most once.
    Once,
    /// `Set[T]` with a default: `--name <value>`, repeatable.
    Repeat,
}

#[derive(Debug)]
pub struct CliParam {
    pub name: String,
    pub shape: Shape,
    /// The type of one argument's value; `None` for a switch.
    pub arg: Option<TypeId>,
    /// True when the parameter is an `Option`: no flag gives `none`.
    pub optional: bool,
    /// This parameter's part of the `--help` line.
    pub usage: String,
    pub span: Span,
}

#[derive(Debug)]
pub struct Guard {
    pub cond: TExpr,
    pub error: String,
    pub span: Span,
}

#[derive(Debug)]
pub struct Rule {
    pub id: RuleId,
    pub name: String,
    /// `@on(cli("name"))`.
    pub command: String,
    pub params: Vec<Param>,
    pub result: Type,
    pub guards: Vec<Guard>,
    pub body: Vec<TStmt>,
    /// States the body assigns, from the body itself; checked against `@uses`.
    pub writes: BTreeSet<StateId>,
    pub emits_lines: bool,
    pub access: Access,
    pub cli: Vec<CliParam>,
}

#[derive(Debug)]
pub struct Case {
    pub args: Vec<Option<TExpr>>,
    pub expect: TExpr,
    pub span: Span,
}

#[derive(Debug)]
pub struct Func {
    pub id: FnId,
    pub params: Vec<Param>,
    pub result: Type,
    pub body: TExpr,
    pub cases: Vec<Case>,
}

#[derive(Debug)]
pub enum Given {
    State { state: StateId, value: TExpr, span: Span },
    Test { test: TestId, span: Span },
}

#[derive(Debug)]
pub struct Test {
    pub id: TestId,
    pub params: Vec<Param>,
    pub given: Vec<Given>,
    pub body: Vec<TStmt>,
}

#[derive(Debug)]
pub struct Law {
    pub id: LawId,
    /// The trace parameter and the rules its events are drawn from.
    pub trace: String,
    pub rules: Vec<RuleId>,
    /// Every other parameter: generated from its type.
    pub params: Vec<Param>,
    pub body: TExpr,
    /// `t.states.all(w => p)` with no other parameter: `p` holds in every
    /// state, so a store must satisfy it. The lambda's parameter and body.
    pub invariant: Option<(String, TExpr)>,
}

/// How one value is held in the store file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Codec {
    /// A JSON integer, at most 2^53; `named` is the alias whose claim is checked.
    Int {
        named: Option<TypeId>,
    },
    Text {
        named: Option<TypeId>,
    },
    Bool,
    /// The variant's name as a string.
    Sum(TypeId),
    /// A sorted array without repeats.
    Set(Box<Codec>),
    /// An object with exactly the record's fields, in declaration order.
    Record(TypeId, Vec<(String, Codec)>),
    /// `Map[K, R]` where the record's first field is the key: an array of
    /// records, ascending by that field, no key twice.
    Keyed {
        record: TypeId,
        key: String,
        of: Box<Codec>,
    },
}

#[derive(Debug)]
pub struct StoreField {
    pub state: StateId,
    pub codec: Codec,
}

#[derive(Debug)]
pub struct App<'p> {
    pub p: &'p Program,
    pub decl: &'p AppDecl,
    pub strategies: Strategies,
    pub bin: String,
    /// `@store(file(...))`: the default path.
    pub store_file: String,
    /// The environment variable that replaces it: `<BIN>_STORE`.
    pub store_env: String,
    pub world: Vec<StateId>,
    pub rules: Vec<Rule>,
    /// Every error a rule can refuse with, in first-written order.
    pub refusals: Vec<String>,
    /// Declared types to emit, in declaration order.
    pub types: Vec<TypeId>,
    pub fns: Vec<Func>,
    pub tests: Vec<Test>,
    pub laws: Vec<Law>,
    /// The `--help` text, byte for byte.
    pub help: String,
    /// The store's fields in file order: scalars, then collections.
    pub store: Vec<StoreField>,
}

fn boxed(d: Diag) -> Box<Diag> {
    Box::new(d)
}

impl<'p> App<'p> {
    pub fn build(p: &'p Program, decl: &'p AppDecl) -> R<App<'p>> {
        App::build_with(p, decl, Strategies::default())
    }

    pub fn build_with(p: &'p Program, decl: &'p AppDecl, strategies: Strategies) -> R<App<'p>> {
        let path = p.qualified(&decl.info);
        let need = |what: &str| {
            boxed(
                Diag::error(
                    "E-NOTE-ARG",
                    decl.info.span,
                    format!("app `{}` needs {what} to be built as a command-line program", decl.info.name),
                )
                .with_decl(path.clone())
                .with_note(format!("fix: add {what} above the app")),
            )
        };
        let bin = decl.bin.clone().ok_or_else(|| need("@bin(\"name\")"))?;
        let Some(ir::Store::File(store_file)) = decl.store.clone() else { return Err(need("@store(file(\"path\"))")) };
        let store_env = format!("{}_STORE", bin.to_uppercase().replace('-', "_"));

        // The world: every state of a module that owns one of the app's rules.
        let owners: BTreeSet<ModuleId> = decl.rules.iter().map(|r| p.rule(*r).info.module).collect();
        let world: Vec<StateId> = (0..p.states.len())
            .map(|i| StateId(i as u32))
            .filter(|id| owners.contains(&p.state(*id).info.module))
            .collect();
        let mut names = BTreeSet::new();
        for id in &world {
            let info = &p.state(*id).info;
            if !names.insert(info.name.clone()) {
                return Err(boxed(
                    Diag::unsupported(
                        info.span,
                        format!("two states named `{}` in one app's world (emitters)", info.name),
                    )
                    .with_decl(p.qualified(info))
                    .with_note("fix: rename one of the states"),
                ));
            }
        }

        let mut app = App {
            p,
            decl,
            strategies,
            bin,
            store_file,
            store_env,
            world,
            rules: Vec::new(),
            refusals: Vec::new(),
            types: Vec::new(),
            fns: Vec::new(),
            tests: Vec::new(),
            laws: Vec::new(),
            help: String::new(),
            store: Vec::new(),
        };
        let world = app.world.clone();
        for id in &decl.rules {
            let rule = app.rule(&world, *id)?;
            for name in rule.guards.iter().map(|g| &g.error).chain(body_fails(&rule.body).iter()) {
                if !app.refusals.contains(name) {
                    app.refusals.push(name.clone());
                }
            }
            app.rules.push(rule);
        }
        for (i, t) in p.types.iter().enumerate() {
            let id = TypeId(i as u32);
            if id != p.option_type && t.info.module != ModuleId(0) && !matches!(t.def, ir::TypeDef::Rules(_)) {
                app.infer(&world, &t.info, BodyKind::Fn).named(id)?;
                app.types.push(id);
            }
        }
        for i in 0..p.fns.len() {
            let f = app.func(&world, FnId(i as u32))?;
            app.fns.push(f);
        }
        for i in 0..p.tests.len() {
            let t = app.test(&world, TestId(i as u32))?;
            app.tests.push(t);
        }
        for i in 0..p.laws.len() {
            let l = app.law(&world, LawId(i as u32))?;
            app.laws.push(l);
        }
        app.help = app.help_text();
        app.store = app.schema(&world)?;
        Ok(app)
    }

    fn infer<'a>(&self, world: &'a [StateId], info: &ir::DeclInfo, kind: BodyKind) -> Infer<'a>
    where
        'p: 'a,
    {
        Infer::new(self.p, world, self.p.qualified(info), kind)
    }

    fn params(&self, inf: &mut Infer<'_>, params: &[ir::Param]) -> R<Vec<Param>> {
        let mut out = Vec::new();
        for p in params {
            let ty = inf.ty(&p.ty, p.span)?;
            let default = match &p.default {
                Some(d) => Some(inf.expr(d, Some(&ty))?),
                None => None,
            };
            inf.bind(&p.name, ty.clone());
            out.push(Param { name: p.name.clone(), ty, default, span: p.span });
        }
        Ok(out)
    }

    fn rule(&self, world: &[StateId], id: RuleId) -> R<Rule> {
        let decl = self.p.rule(id);
        let path = self.p.qualified(&decl.info);
        let mut inf = self.infer(world, &decl.info, BodyKind::Rule);
        let params = self.params(&mut inf, &decl.params)?;
        let result = match &decl.result {
            Some(t) => inf.ty(t, decl.info.span)?,
            None => Type::Unit,
        };
        let mut guards = Vec::new();
        for g in &decl.guards {
            guards.push(Guard { cond: inf.expr(&g.cond, Some(&Type::Bool))?, error: g.error.clone(), span: g.span });
        }
        let body = match &decl.body.kind {
            ir::ExprKind::Block(b) => inf.block(b, &result)?,
            _ => return inf.unsupported(decl.body.span, "a rule whose body is not a block"),
        };
        let mut writes = BTreeSet::new();
        let mut emits_lines = false;
        scan(&body, &mut writes, &mut emits_lines);
        // `@uses` is what the author promised; the write set is what the body does.
        for s in &writes {
            let state = &self.p.state(*s).info.name;
            if !decl.uses.is_empty() && !decl.uses.iter().any(|u| u == &format!("{state}.write")) {
                return Err(boxed(
                    Diag::error(
                        "E-EFFECT-UNDECLARED",
                        decl.info.name_span,
                        format!("the body writes `{state}`, which `@uses` does not list"),
                    )
                    .with_decl(path.clone())
                    .with_expected(format!("@uses(..., {state}.write)"), format!("@uses({})", decl.uses.join(", ")))
                    .with_note("fix: add the effect to @uses; the lock mode of the rule is chosen from it"),
                ));
            }
        }
        let Some(command) = decl
            .triggers
            .iter()
            .map(|t| match t {
                ir::Trigger::Cli { command } => command.clone(),
            })
            .next()
        else {
            return Err(boxed(
                Diag::error(
                    "E-NOTE-ARG",
                    decl.info.name_span,
                    format!("rule `{}` is in the app but has no `@on(cli(...))`", decl.info.name),
                )
                .with_decl(path)
                .with_note("fix: add @on(cli(\"name\")) to the rule, or take it out of the app's rules"),
            ));
        };
        let mut cli = Vec::new();
        for p in &params {
            cli.push(self.cli_param(&inf, p)?);
        }
        let access = if writes.is_empty() { Access::Read } else { Access::Write };
        Ok(Rule {
            id,
            name: decl.info.name.clone(),
            command,
            params,
            result,
            guards,
            body,
            writes,
            emits_lines,
            access,
            cli,
        })
    }

    /// The derivation table of the `cli` vocabulary, one parameter.
    fn cli_param(&self, inf: &Infer<'_>, p: &Param) -> R<CliParam> {
        let arg_of = |t: &Type| -> R<TypeId> {
            if let Type::Named(id) = t {
                let ok = match inf.named(*id)? {
                    Named::Sum(_) => true,
                    Named::Alias { .. } => matches!(inf.base(t), Type::Int | Type::Text),
                    Named::Record(_) => false,
                };
                if ok {
                    return Ok(*id);
                }
            }
            Err(boxed(
                Diag::new(
                    spec_lang_diag::Severity::Error,
                    "E-OPEN",
                    format!("how `{}: {}` is written on a command line is not derived", p.name, inf.show(t)),
                )
                .with_span(p.span)
                .with_decl(inf.decl.clone())
                .with_expected("a named sum with no data, or a named `Int where` / `Text where` type", inf.show(t))
                .with_note("fix: give the parameter a named type the cli vocabulary derives an argument for"),
            ))
        };
        let hint = |id: TypeId| -> R<String> {
            Ok(match inf.named(id)? {
                Named::Sum(variants) => variants.iter().map(|v| v.name.to_lowercase()).collect::<Vec<_>>().join("|"),
                _ => format!("<{}>", self.p.ty(id).info.name.to_lowercase()),
            })
        };
        let name = p.name.clone();
        let mk = |shape, arg, optional, usage| {
            Ok(CliParam { name: name.clone(), shape, arg, optional, usage, span: p.span })
        };
        match (&p.default, inf.base(&p.ty)) {
            (None, _) => mk(Shape::Positional, Some(arg_of(&p.ty)?), false, format!("<{name}>")),
            (Some(d), Type::Bool) if d.kind == TKind::Bool(false) => {
                mk(Shape::Switch, None, false, format!("[--{name}]"))
            }
            (Some(d), Type::Option(t)) if d.kind == TKind::Nothing => {
                let arg = arg_of(&t)?;
                mk(Shape::Once, Some(arg), true, format!("[--{name} {}]", hint(arg)?))
            }
            (Some(_), Type::Set(t)) => {
                let arg = arg_of(&t)?;
                mk(Shape::Repeat, Some(arg), false, format!("[--{name} {}]...", hint(arg)?))
            }
            (Some(_), Type::Bool | Type::Option(_)) => inf.unsupported(
                p.span,
                "a Bool defaulting to true or an Option defaulting to a value, as a command-line flag",
            ),
            (Some(_), _) => {
                let arg = arg_of(&p.ty)?;
                mk(Shape::Once, Some(arg), false, format!("[--{name} {}]", hint(arg)?))
            }
        }
    }

    fn func(&self, world: &[StateId], id: FnId) -> R<Func> {
        let decl = self.p.func(id);
        let mut inf = self.infer(world, &decl.info, BodyKind::Fn);
        let params = self.params(&mut inf, &decl.params)?;
        let Some(result) = &decl.result else {
            return inf.unsupported(decl.info.name_span, "a fn without a written result type");
        };
        let result = inf.ty(result, decl.info.span)?;
        let body = inf.expr(&decl.body, Some(&result))?;
        let mut cases = Vec::new();
        for c in &decl.cases {
            let mut case_inf = self.infer(world, &decl.info, BodyKind::Fn);
            let mut args = Vec::new();
            for (param, arg) in params.iter().zip(&c.args) {
                args.push(match arg {
                    Some(a) => Some(case_inf.expr(a, Some(&param.ty))?),
                    None => None,
                });
            }
            let ir::Expect::Value(v) = &c.expect else {
                return inf.unsupported(c.span, "a case that expects a failure");
            };
            cases.push(Case { args, expect: case_inf.expr(v, Some(&result))?, span: c.span });
        }
        Ok(Func { id, params, result, body, cases })
    }

    fn test(&self, world: &[StateId], id: TestId) -> R<Test> {
        let decl = self.p.test(id);
        let mut inf = self.infer(world, &decl.info, BodyKind::Test);
        if !decl.cases.is_empty() {
            return inf.unsupported(decl.info.name_span, "`@cases` on a test");
        }
        let mut given = Vec::new();
        for g in &decl.given {
            given.push(match g {
                ir::Given::State { state, value, span } => {
                    if !world.contains(state) {
                        return inf.unsupported(*span, "a `@given` for a state outside the app's world");
                    }
                    let s = self.p.state(*state);
                    let ty = inf.ty(&s.ty, s.info.span)?;
                    Given::State { state: *state, value: inf.expr(value, Some(&ty))?, span: *span }
                }
                ir::Given::Test { test, span } => {
                    if !self.p.test(*test).params.is_empty() {
                        return inf.unsupported(*span, "a `@given` naming a test that has parameters");
                    }
                    Given::Test { test: *test, span: *span }
                }
            });
        }
        let params = self.params(&mut inf, &decl.params)?;
        let body = inf.block(&decl.body, &Type::Unit)?;
        Ok(Test { id, params, given, body })
    }

    fn law(&self, world: &[StateId], id: LawId) -> R<Law> {
        let decl = self.p.law(id);
        let mut inf = self.infer(world, &decl.info, BodyKind::Law);
        let Some((at, rules)) = decl.trace() else {
            return inf.unsupported(decl.info.name_span, "a law without a `Trace` parameter");
        };
        let trace = decl.params[at].name.clone();
        for r in rules {
            if !self.decl.rules.contains(r) {
                return inf.unsupported(decl.params[at].span, "a trace over a rule that is not in the app");
            }
        }
        inf.bind(&trace, Type::Trace(rules.to_vec()));
        let others: Vec<&ir::Param> =
            decl.params.iter().enumerate().filter(|(i, _)| *i != at).map(|(_, p)| p).collect();
        let mut params = Vec::new();
        for p in others {
            let ty = inf.ty(&p.ty, p.span)?;
            inf.bind(&p.name, ty.clone());
            params.push(Param { name: p.name.clone(), ty, default: None, span: p.span });
        }
        let body = inf.expr(&decl.body, Some(&Type::Bool))?;
        let invariant = match &body.kind {
            TKind::Core { op: CoreOp::All, recv, args } if params.is_empty() => match (&recv.kind, args.as_slice()) {
                (
                    TKind::Trace { prop: TraceProp::States, .. },
                    [TExpr { kind: TKind::Lambda { params: lp, body: lb }, .. }],
                ) => {
                    let mut uses_trace = false;
                    walk_expr(lb, &mut |e| uses_trace |= matches!(&e.kind, TKind::Local(n) if n == &trace));
                    match lp.as_slice() {
                        [(w, _)] if !uses_trace => Some((w.clone(), lb.as_ref().clone())),
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => None,
        };
        Ok(Law { id, trace, rules: rules.to_vec(), params, body, invariant })
    }

    fn help_text(&self) -> String {
        let mut out = format!("usage: {} <command> [args]\n\ncommands:\n", self.bin);
        for r in &self.rules {
            out.push_str("  ");
            out.push_str(&r.command);
            for p in &r.cli {
                out.push(' ');
                out.push_str(&p.usage);
            }
            out.push('\n');
        }
        out
    }

    /// The store schema: scalars first, then collections, each in declaration order.
    fn schema(&self, world: &[StateId]) -> R<Vec<StoreField>> {
        let mut scalars = Vec::new();
        let mut collections = Vec::new();
        for id in world {
            let s = self.p.state(*id);
            let inf = self.infer(world, &s.info, BodyKind::Fn);
            let ty = inf.ty(&s.ty, s.info.span)?;
            let codec = self.codec(&inf, &ty, s.info.span)?;
            let field = StoreField { state: *id, codec };
            if matches!(field.codec, Codec::Set(_) | Codec::Keyed { .. }) {
                collections.push(field)
            } else {
                scalars.push(field)
            }
        }
        scalars.extend(collections);
        Ok(scalars)
    }

    fn codec(&self, inf: &Infer<'_>, ty: &Type, span: Span) -> R<Codec> {
        let named = match ty {
            Type::Named(id) => Some(*id),
            _ => None,
        };
        Ok(match inf.base(ty) {
            Type::Int => Codec::Int { named },
            Type::Text => Codec::Text { named },
            Type::Bool => Codec::Bool,
            Type::Set(t) => {
                let inner = self.codec(inf, &t, span)?;
                if !matches!(inner, Codec::Int { .. } | Codec::Text { .. }) {
                    return inf.unsupported(span, "a stored Set of values that are not Ints or Texts");
                }
                Codec::Set(Box::new(inner))
            }
            Type::Named(id) => match inf.named(id)? {
                Named::Sum(_) => Codec::Sum(id),
                Named::Record(fields) => {
                    let mut out = Vec::new();
                    for f in fields {
                        let fty = inf.ty(&f.ty, f.span)?;
                        out.push((f.name.clone(), self.codec(inf, &fty, f.span)?));
                    }
                    Codec::Record(id, out)
                }
                Named::Alias { .. } => return inf.unsupported(span, "this stored type"),
            },
            Type::Map(k, v) => {
                let of = self.codec(inf, &v, span)?;
                let key = match (&of, v.as_ref()) {
                    (Codec::Record(_, fields), Type::Named(rec)) => match (fields.first(), inf.named(*rec)?) {
                        (Some((name, _)), Named::Record(decl))
                            if decl.first().map(|f| inf.ty(&f.ty, f.span)).transpose()?.as_ref()
                                == Some(k.as_ref()) =>
                        {
                            Some((*rec, name.clone()))
                        }
                        _ => None,
                    },
                    _ => None,
                };
                let Some((record, key)) = key else {
                    return Err(boxed(
                        Diag::new(
                            spec_lang_diag::Severity::Error,
                            "E-OPEN",
                            "how this Map is laid out in the store file is not derived",
                        )
                        .with_span(span)
                        .with_decl(inf.decl.clone())
                        .with_expected("Map[K, R] where R is a record whose first field has type K", inf.show(ty))
                        .with_note("fix: make the key the first field of the record the map holds"),
                    ));
                };
                Codec::Keyed { record, key, of: Box::new(of) }
            }
            _ => return inf.unsupported(span, format!("a stored {}", inf.show(ty))),
        })
    }

    /// A rule of this app by id.
    pub fn rule_of(&self, id: RuleId) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// The typed-tree helper for a declaration, for printers that need `base`, `named` or `show`.
    pub fn types_of(&self) -> Infer<'_> {
        Infer::new(self.p, &self.world, self.p.qualified(&self.decl.info), BodyKind::Fn)
    }
}

fn scan(stmts: &[TStmt], writes: &mut BTreeSet<StateId>, lines: &mut bool) {
    for s in stmts {
        match &s.kind {
            TStmtKind::SetState { state, .. }
            | TStmtKind::PutEntry { state, .. }
            | TStmtKind::SetEntryField { state, .. }
            | TStmtKind::RemoveEntry { state, .. } => {
                writes.insert(*state);
            }
            TStmtKind::OutLine(_) => *lines = true,
            TStmtKind::For { body, .. } | TStmtKind::While { body, .. } => scan(body, writes, lines),
            TStmtKind::If { then, els, .. } => {
                scan(then, writes, lines);
                if let Some(els) = els {
                    scan(els, writes, lines);
                }
            }
            _ => {}
        }
    }
}

fn body_fails(stmts: &[TStmt]) -> Vec<String> {
    let mut out = Vec::new();
    for s in stmts {
        match &s.kind {
            TStmtKind::Fail(name) => out.push(name.clone()),
            TStmtKind::For { body, .. } | TStmtKind::While { body, .. } => out.extend(body_fails(body)),
            TStmtKind::If { then, els, .. } => {
                out.extend(body_fails(then));
                if let Some(els) = els {
                    out.extend(body_fails(els));
                }
            }
            _ => {}
        }
    }
    out
}
