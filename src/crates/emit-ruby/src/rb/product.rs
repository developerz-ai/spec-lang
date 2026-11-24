//! The generated product files: one per declaration, plus the app's wiring.

use super::ast::*;
use super::file::RFile;
use super::lower::*;
use emit_ts::sem::app::{Access, App, Codec, Func, Rule, Shape};
use emit_ts::sem::infer::{BodyKind, Infer, Named, R};
use emit_ts::sem::names::{screaming, snake};
use emit_ts::sem::prov::{Entry, slg};
use emit_ts::sem::tree::*;
use spec_lang_ir::{DeclInfo, StateId, TypeId};
use spec_lang_syntax::Span;
use std::collections::BTreeMap;

pub const COMMANDS: &str = "lib/app/commands.rb";
pub const STORE: &str = "lib/app/store.rb";
pub const CLI: &str = "lib/app/cli.rb";
pub const RT_CLI: &str = "lib/runtime/cli.rb";
pub const RT_COMMAND: &str = "lib/runtime/cli/command.rb";
pub const RT_CODEC: &str = "lib/runtime/store/codec.rb";

#[derive(Debug)]
pub struct Gen<'a, 'p> {
    pub app: &'a App<'p>,
    pub files: BTreeMap<String, RFile>,
}

fn why(info: &DeclInfo) -> Option<Stmt> {
    info.why.as_ref().map(|w| Stmt::Comment(format!("@why {}", w.replace('\n', " "))))
}

impl<'a, 'p> Gen<'a, 'p> {
    pub fn new(app: &'a App<'p>) -> Self {
        Gen { app, files: BTreeMap::new() }
    }

    pub fn entry(&self, info: &DeclInfo, span: Span, role: &str, strategy: &str, fact: &str) -> Entry {
        Entry {
            decl: self.app.p.qualified(info),
            slg: slg(self.app.p, info, span),
            role: role.to_string(),
            strategy: strategy.to_string(),
            fact: fact.to_string(),
        }
    }

    /// The file at `path` in `namespace`, noting that `kind name` is one of its sources.
    pub fn file(&mut self, path: &str, namespace: &[String], kind: &str, info: &DeclInfo) -> &mut RFile {
        let source = format!("{kind} {} ({})", info.name, slg(self.app.p, info, info.name_span));
        let ns: Vec<&str> = namespace.iter().map(String::as_str).collect();
        let f = self.files.entry(path.to_string()).or_insert_with(|| RFile::new(path, &ns));
        if !f.sources.contains(&source) {
            f.sources.push(source);
        }
        f
    }

    fn decl_file(&mut self, path: &str, kind: &str, info: &DeclInfo) -> &mut RFile {
        let ns = module_parts(self.app, info.module);
        self.file(path, &ns, kind, info)
    }

    fn app_file(&mut self, path: &str, kind: &str, info: &DeclInfo) -> &mut RFile {
        self.file(path, &["App".to_string()], kind, info)
    }

    pub fn product(&mut self) -> R<()> {
        let app = self.app;
        for m in &app.p.modules {
            if let Some(root) = m.name.split('.').next()
                && (root == "app" || root == "runtime")
            {
                return app.types_of().unsupported(
                    app.decl.info.span,
                    format!("a module under `{root}`: the Ruby backend keeps that name for itself"),
                );
            }
        }
        for id in &app.types {
            self.type_decl(*id)?;
        }
        for f in &app.fns {
            self.func(f)?;
        }
        if let Some(c) = app.p.consts.first() {
            return app.types_of().unsupported(c.info.span, "a `const` declaration");
        }
        self.world()?;
        self.refusals();
        for r in &app.rules {
            self.rule(r)?;
        }
        self.commands()?;
        self.store()?;
        self.wiring();
        Ok(())
    }

    /// The statements that check a value against its type's claims, if it has any.
    fn checks(lo: &mut Lower<'_, '_>, ty: &Type, value: Expr) -> R<Vec<Stmt>> {
        Ok(match ty {
            Type::Named(id) => match lo.inf.named(*id)? {
                Named::Sum(_) | Named::Alias { claim: Some(_), .. } => {
                    let t = lo.type_ref(*id);
                    vec![Stmt::Expr(lo.rt("claim", vec![t, value]))]
                }
                _ => vec![],
            },
            Type::Set(inner) | Type::List(inner) => {
                let each = Self::checks(lo, inner, ident("item"))?;
                if each.is_empty() {
                    vec![]
                } else {
                    vec![Stmt::Expr(call_block(value, "each", vec![], &["item"], each))]
                }
            }
            _ => vec![],
        })
    }

    fn needs_freeze(lo: &Lower<'_, '_>, ty: &Type) -> bool {
        matches!(lo.inf.base(ty), Type::Text | Type::Set(_) | Type::List(_) | Type::Map(..) | Type::Tuple(_))
    }

    fn type_decl(&mut self, id: TypeId) -> R<()> {
        let app = self.app;
        let decl = app.p.ty(id);
        let info = &decl.info;
        let path = type_file(app, id);
        let name = info.name.clone();
        let qualified = app.p.qualified(info);
        let mut inf = Infer::new(app.p, &app.world, qualified.clone(), BodyKind::Fn);
        match inf.named(id)? {
            Named::Sum(variants) => {
                let entry = self.entry(
                    info,
                    info.span,
                    "type",
                    "sum.symbols: a sum with no data is the symbols of its variant names",
                    "no variant carries data",
                );
                let f = self.decl_file(&path, "type", info);
                let anchor = f.anchor(entry);
                f.items.push(anchor);
                f.items.extend(why(info));
                let all = call(Expr::Array(variants.iter().map(|v| sym(&v.name)).collect()), "freeze", vec![]);
                let valid = Stmt::Def {
                    singleton: true,
                    name: "valid?".into(),
                    params: vec![Param::Req("value".into())],
                    body: vec![Stmt::Expr(call(konst("ALL"), "include?", vec![ident("value")]))],
                };
                f.items.push(Stmt::Module {
                    name: name.clone(),
                    body: vec![
                        Stmt::ConstAssign("NAME".into(), text(&name)),
                        Stmt::ConstAssign("ALL".into(), all),
                        Stmt::Blank,
                        valid,
                    ],
                });
            }
            Named::Record(fields) => {
                let entry = self.entry(
                    info,
                    info.span,
                    "type",
                    "record.data: a frozen `Data` whose constructor applies the defaults and checks every claim",
                    "fields, defaults and claims as declared",
                );
                let f = self.decl_file(&path, "type", info);
                let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
                let mut params = Vec::new();
                let mut body = Vec::new();
                let mut supers = Vec::new();
                for field in fields {
                    let fty = lo.inf.ty(&field.ty, field.span)?;
                    let n = lo.local(&field.name);
                    params.push(match &field.default {
                        Some(d) => {
                            let d = lo.inf.expr(d, Some(&fty))?;
                            Param::KeyOpt(n.clone(), lo.expr(&d)?)
                        }
                        None => Param::Key(n.clone()),
                    });
                    body.extend(Self::checks(&mut lo, &fty, ident(&n))?);
                    let value =
                        if Self::needs_freeze(&lo, &fty) { lo.rt("frozen", vec![ident(&n)]) } else { ident(&n) };
                    supers.push(Arg::Key(field.name.clone(), value));
                }
                body.push(Stmt::Super(supers));
                let init = Stmt::Def { singleton: false, name: "initialize".into(), params, body };
                let anchor = f.anchor(entry);
                f.items.push(anchor);
                f.items.extend(why(info));
                f.items.push(Stmt::DataDefine {
                    name,
                    fields: fields.iter().map(|fd| fd.name.clone()).collect(),
                    body: vec![init],
                });
            }
            Named::Alias { base, claim } => {
                let entry = match claim {
                    Some(_) => self.entry(info, info.span, "type", "claim.module: the base value itself; `valid?` is the claim, checked at every edge and by every constructor that holds one", "`where` claim on the alias"),
                    None => self.entry(info, info.span, "type", "alias: another name for its base", "no `where` claim"),
                };
                let holds = match claim {
                    Some(claim) => {
                        inf.bind("self", base.clone());
                        Some(inf.expr(&claim.expr, Some(&Type::Bool))?)
                    }
                    None => None,
                };
                let f = self.decl_file(&path, "type", info);
                let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
                // `self` is the value in a claim; in Ruby it is the module, so the value gets a name.
                lo.rename("self", "value");
                let is_base = match lo.inf.base(&base) {
                    Type::Int => call(ident("value"), "is_a?", vec![konst("Integer")]),
                    Type::Text => {
                        lo.f.need(TEXT);
                        call(konst("Runtime::Text"), "text?", vec![ident("value")])
                    }
                    Type::Bool => {
                        bin("||", bin("==", ident("value"), Expr::True), bin("==", ident("value"), Expr::False))
                    }
                    _ => return lo.inf.unsupported(info.span, "a named alias of a type that is not Int, Text or Bool"),
                };
                let check = match &holds {
                    Some(h) => bin("&&", is_base, lo.expr(h)?),
                    None => is_base,
                };
                let valid = Stmt::Def {
                    singleton: true,
                    name: "valid?".into(),
                    params: vec![Param::Req("value".into())],
                    body: vec![Stmt::Expr(check)],
                };
                let anchor = f.anchor(entry);
                f.items.push(anchor);
                f.items.extend(why(info));
                f.items.push(Stmt::Module {
                    name: name.clone(),
                    body: vec![Stmt::ConstAssign("NAME".into(), text(&name)), Stmt::Blank, valid],
                });
            }
        }
        Ok(())
    }

    fn func(&mut self, func: &Func) -> R<()> {
        let app = self.app;
        let decl = app.p.func(func.id);
        let info = &decl.info;
        let path = fn_file(app, func.id);
        let qualified = app.p.qualified(info);
        let table = match (&func.body.kind, func.params.as_slice()) {
            (TKind::Match { scrutinee, arms }, [p])
                if app.strategies.on("match.table")
                    && matches!(&scrutinee.kind, TKind::Local(n) if n == &p.name)
                    && arms.iter().all(|(pat, v)| {
                        matches!(pat, TPat::Variant { .. }) && matches!(v.kind, TKind::Text(_) | TKind::Int(_))
                    }) =>
            {
                Some(arms)
            }
            _ => None,
        };
        let entry = match table {
            Some(_) => self.entry(
                info,
                info.span,
                "fn",
                "match.table: one frozen lookup table, fetched by the variant",
                "the match maps every variant of a data-less sum to a constant",
            ),
            None => self.entry(
                info,
                info.span,
                "fn",
                "fn.direct: the body, expression for expression",
                "a `fn` writes nothing",
            ),
        };
        let f = self.decl_file(&path, "fn", info);
        let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
        let mut params = Vec::new();
        for p in &func.params {
            let n = lo.local(&p.name);
            params.push(match &p.default {
                Some(d) => Param::KeyOpt(n, lo.expr(d)?),
                None => Param::Req(n),
            });
        }
        let mut before = None;
        let body = match (table, &func.body.kind) {
            (Some(arms), _) => {
                let table_name = format!("{}_TABLE", screaming(&info.name));
                let mut rows = Vec::new();
                for (pat, value) in arms {
                    if let TPat::Variant { ty, tag } = pat
                        && let Named::Sum(variants) = lo.inf.named(*ty)?
                    {
                        rows.push((sym(&variants[*tag as usize].name), lo.expr(value)?));
                    }
                }
                before = Some(Stmt::ConstAssign(table_name.clone(), call(Expr::Hash(rows), "freeze", vec![])));
                vec![Stmt::Expr(call(konst(&table_name), "fetch", vec![ident(&lo.local(&func.params[0].name))]))]
            }
            (None, TKind::Block(stmts)) => lo.stmts(stmts)?,
            (None, _) => vec![Stmt::Expr(lo.expr(&func.body)?)],
        };
        let anchor = f.anchor(entry);
        f.items.push(anchor);
        f.items.extend(why(info));
        if let Some(table) = before {
            f.items.push(table);
            f.items.push(Stmt::Blank);
        }
        f.items.push(Stmt::Def { singleton: true, name: info.name.clone(), params, body });
        Ok(())
    }

    fn initial(lo: &mut Lower<'_, '_>, id: StateId) -> R<Expr> {
        let s = lo.app.p.state(id);
        let ty = lo.inf.ty(&s.ty, s.info.span)?;
        match &s.init {
            Some(init) => {
                let mut inf = Infer::new(lo.app.p, &lo.app.world, lo.app.p.qualified(&s.info), BodyKind::Fn);
                let init = inf.expr(init, Some(&ty))?;
                lo.expr(&init)
            }
            None => match lo.inf.base(&ty) {
                Type::Map(..) => Ok(call(Expr::Hash(vec![]), "freeze", vec![])),
                Type::Set(_) => Ok(call(Expr::SetLit(vec![]), "freeze", vec![])),
                Type::List(_) => Ok(call(Expr::Array(vec![]), "freeze", vec![])),
                Type::Option(_) => Ok(Expr::Nil),
                _ => lo.inf.unsupported(s.info.span, "a state with no initial value that is not a collection"),
            },
        }
    }

    fn world(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let mut entries = Vec::new();
        for id in &app.world {
            let s = app.p.state(*id);
            entries.push(self.entry(
                &s.info,
                s.info.span,
                "state",
                "world.immutable: one field of an immutable World; a rule returns a new one",
                "a module owns its state; rules are atomic",
            ));
            self.app_file(WORLD, "state", &s.info);
        }
        let f = self.app_file(WORLD, "app", info);
        let mut lo = Lower::new(app, f, Mode::Pure, &app.p.qualified(info));
        let mut firsts = Vec::new();
        let mut params = Vec::new();
        let mut body = Vec::new();
        let mut supers = Vec::new();
        for id in &app.world {
            let s = app.p.state(*id);
            let name = s.info.name.clone();
            let ty = lo.inf.ty(&s.ty, s.info.span)?;
            firsts.push(Arg::Key(name.clone(), Self::initial(&mut lo, *id)?));
            let local = lo.local(&name);
            params.push(Param::Key(local.clone()));
            body.extend(Self::checks(&mut lo, &ty, ident(&local))?);
            let value = if Self::needs_freeze(&lo, &ty) { lo.rt("frozen", vec![ident(&local)]) } else { ident(&local) };
            supers.push(Arg::Key(name, value));
        }
        body.push(Stmt::Super(supers));
        let initial = Stmt::Def {
            singleton: true,
            name: "initial".into(),
            params: vec![],
            body: vec![Stmt::Expr(Expr::Call { recv: None, name: "new".into(), args: firsts, block: None })],
        };
        let init = Stmt::Def { singleton: false, name: "initialize".into(), params, body };
        if let Some(first) = entries.into_iter().next() {
            let anchor = f.anchor(first);
            f.items.push(anchor);
        }
        f.items.push(Stmt::DataDefine {
            name: "World".into(),
            fields: app.world.iter().map(|id| state_name(app, *id)).collect(),
            body: vec![initial, Stmt::Blank, init],
        });
        Ok(())
    }

    fn refusals(&mut self) {
        let app = self.app;
        let info = &app.decl.info;
        let entry = self.entry(
            info,
            info.span,
            "refusals",
            "refusal.exception: a refusal is an exception named for the error, raised before any new world exists",
            "every `@guard ... else Name` and `fail Name` of the app's rules",
        );
        let f = self.app_file(REFUSALS, "app", info);
        f.need(VALUES);
        let anchor = f.anchor(entry);
        f.items.push(anchor);
        for name in &app.refusals {
            f.items.push(Stmt::Class { name: name.clone(), superclass: Some(konst("Runtime::Refusal")), body: vec![] });
        }
    }

    fn rule(&mut self, rule: &Rule) -> R<()> {
        let app = self.app;
        let decl = app.p.rule(rule.id);
        let info = &decl.info;
        let path = rule_file(app, rule.id);
        let qualified = app.p.qualified(info);
        let written: Vec<String> = rule.writes.iter().map(|s| app.p.state(*s).info.name.clone()).collect();
        let uses = format!("@uses({})", decl.uses.join(", "));
        let top = match rule.access {
            Access::Read => self.entry(info, info.span, "rule", "rule.pure-read: a pure function that returns the world it was given; shared lock, the store is never written and its writer never loaded", &format!("the body assigns no state; {uses}")),
            Access::Write => self.entry(info, info.span, "rule", "rule.pure-write: a pure function from a world to a new world; exclusive lock, atomic replace", &format!("the body assigns {}; {uses}", written.join(", "))),
        };
        let guard_entries: Vec<Entry> = rule
            .guards
            .iter()
            .enumerate()
            .map(|(i, g)| {
                self.entry(
                    info,
                    g.span,
                    &format!("guard[{i}]"),
                    "guard.raise: checked in written order before any draft exists, so a refusal changes nothing",
                    "guards run first, top to bottom",
                )
            })
            .collect();
        let take = app.strategies.on("draft.take-or-copy");
        let draft_entry = if take {
            self.entry(info, decl.body.span, "body.draft", "draft.take-or-copy: the body writes a draft of each state it assigns; a map the store read for this one call is the draft itself, any other caller's is copied", &format!("write set {{{}}}; a rule is atomic and the process commits once", written.join(", ")))
        } else {
            self.entry(
                info,
                decl.body.span,
                "body.draft",
                "draft.copy: the body runs on a copy of each state it assigns",
                &format!("write set {{{}}}", written.join(", ")),
            )
        };
        let stmt_entries: Vec<Entry> = rule
            .body
            .iter()
            .enumerate()
            .map(|(i, s)| {
                self.entry(info, s.span, &format!("body.stmt[{i}]"), "stmt.direct", "the statement as written")
            })
            .collect();
        let end_entry = self.entry(
            info,
            decl.body.span,
            "body.commit",
            "commit.value: the new world, the lines to print after the commit, and the reply",
            "`out.line` is sent after commit",
        );

        let f = self.decl_file(&path, "rule", info);
        f.need(WORLD);
        let top_anchor = f.anchor(top);
        let mut lo = Lower::new(app, f, Mode::Guard, &qualified);
        let mut params = vec![Param::Req("world".into())];
        for p in &rule.params {
            let n = lo.local(&p.name);
            params.push(match &p.default {
                Some(d) => Param::KeyOpt(n, lo.expr(d)?),
                None => Param::Key(n),
            });
        }
        let mut body = Vec::new();
        for (g, entry) in rule.guards.iter().zip(guard_entries) {
            let cond = lo.expr(&g.cond)?;
            let error = lo.refusal(&g.error);
            body.push(lo.f.anchor(entry));
            body.push(Stmt::Modifier { stmt: Box::new(Stmt::Raise(error)), keyword: "unless", cond });
        }
        let mut drafts = Vec::new();
        for s in &rule.writes {
            let state = app.p.state(*s);
            let ty = lo.inf.ty(&state.ty, state.info.span)?;
            let name = state_name(app, *s);
            let from = call(ident("world"), &name, vec![]);
            let value = match lo.inf.base(&ty) {
                Type::Map(..) if take => lo.rt("draft", vec![from]),
                Type::Map(..) => call(from, "dup", vec![]),
                _ => from,
            };
            drafts.push(Stmt::Assign(lo.local_state(&name), value));
        }
        if rule.emits_lines {
            drafts.push(Stmt::Assign("lines".into(), Expr::Array(vec![])));
        }
        if !drafts.is_empty() {
            body.push(lo.f.anchor(draft_entry));
            body.extend(drafts);
        }
        lo.mode = Mode::Rule { drafts: rule.writes.clone(), lines: rule.emits_lines };
        for (s, entry) in rule.body.iter().zip(stmt_entries) {
            let stmt = lo.stmt(s)?;
            body.push(lo.f.anchor(entry));
            body.push(stmt);
        }
        if !matches!(rule.body.last().map(|s| &s.kind), Some(TStmtKind::Return(_) | TStmtKind::Fail(_))) {
            let end = lo.outcome(None)?;
            body.push(lo.f.anchor(end_entry));
            body.push(Stmt::Expr(end));
        }
        f.items.push(top_anchor);
        f.items.extend(why(info));
        f.items.push(Stmt::Def { singleton: true, name: info.name.clone(), params, body });
        Ok(())
    }

    fn commands(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let qualified = app.p.qualified(info);
        let cli = || konst("Runtime::CLI");
        let mut arg_types: Vec<TypeId> = Vec::new();
        for r in &app.rules {
            for p in &r.cli {
                if let Some(id) = p.arg
                    && !arg_types.contains(&id)
                {
                    arg_types.push(id);
                }
            }
        }
        for id in &arg_types {
            let tinfo = &app.p.ty(*id).info;
            let entry = self.entry(tinfo, tinfo.span, "cli argument type", "arg.parse-then-claim: the text is parsed, then the claim is checked at the edge; outside it is a usage error", "the parameter's declared type");
            let f = self.app_file(COMMANDS, "app", info);
            f.need(RT_COMMAND);
            let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
            let t = lo.type_ref(*id);
            let value = match lo.inf.named(*id)? {
                Named::Sum(_) => call(cli(), "sum_type", vec![text(&tinfo.name), child(t, "ALL")]),
                Named::Alias { claim: Some(_), .. } => {
                    let helper = if lo.inf.base(&Type::Named(*id)) == Type::Int { "int_type" } else { "text_type" };
                    call(cli(), helper, vec![t])
                }
                _ => return lo.inf.unsupported(tinfo.span, "a command-line argument of a type with no claim"),
            };
            let anchor = f.anchor(entry);
            f.items.push(anchor);
            f.items.push(Stmt::ConstAssign(format!("{}_ARG", screaming(&tinfo.name)), value));
        }
        let mut rows = Vec::new();
        for r in &app.rules {
            let rinfo = &app.p.rule(r.id).info;
            let (lock, fact) = match r.access {
                Access::Read => ("read", "the rule assigns no state"),
                Access::Write => ("write", "the rule assigns state"),
            };
            let entry = self.entry(rinfo, rinfo.span, "cli table row", &format!("cli.table: parameters become arguments by the derivation table of the cli vocabulary; lock mode `{lock}`; the rule's file is loaded only when the command runs"), fact);
            let f = self.app_file(COMMANDS, "app", info);
            let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
            let mut positionals = Vec::new();
            let mut flags = Vec::new();
            let mut named = Vec::new();
            for (c, p) in r.cli.iter().zip(&r.params) {
                let arg = c.arg.map(|id| konst(&format!("{}_ARG", screaming(&app.p.ty(id).info.name))));
                let n = sym(&c.name);
                match (c.shape, arg) {
                    (Shape::Positional, Some(a)) => positionals.push(call(cli(), "positional", vec![n, a])),
                    (Shape::Switch, _) => flags.push(call(cli(), "switch", vec![n])),
                    (Shape::Once, Some(a)) => {
                        let default = match (&p.default, c.optional) {
                            (Some(d), false) => lo.expr(d)?,
                            _ => Expr::Nil,
                        };
                        flags.push(call(cli(), "once", vec![n, a, default]));
                    }
                    (Shape::Repeat, Some(a)) => flags.push(call(cli(), "set", vec![n, a])),
                    _ => return lo.inf.unsupported(c.span, "a value flag without an argument type"),
                }
                named.push((lo.local(&p.name), call(ident("args"), "fetch", vec![sym(&c.name)])));
            }
            // The rule is named, not required: the lambda runs after `load` has been required.
            let module = konst(&module_path(app, rinfo.module));
            let mut all = vec![Arg::Pos(ident("world"))];
            all.extend(named.into_iter().map(|(n, v)| Arg::Key(n, v)));
            let params = if r.params.is_empty() {
                vec![Param::Req("world".into()), Param::Req("_args".into())]
            } else {
                vec![Param::Req("world".into()), Param::Req("args".into())]
            };
            let rule_lambda = Expr::Lambda(params, Box::new(call_args(module, &rinfo.name, all)));
            let relative = format!("../{}/{}", module_dir(app, rinfo.module), snake(&rinfo.name));
            let load = call(konst("File"), "expand_path", vec![text(&relative), ident("__dir__")]);
            let row = call_args(
                konst("Runtime::CLI::Command"),
                "new",
                vec![
                    key("name", text(&r.command)),
                    key("access", sym(lock)),
                    key("load", load),
                    key("rule", rule_lambda),
                    key("positionals", Expr::Array(positionals)),
                    key("flags", Expr::Array(flags)),
                ],
            );
            let row_name = format!("{}_COMMAND", screaming(&r.name));
            let anchor = f.anchor(entry);
            f.items.push(anchor);
            f.items.push(Stmt::ConstAssign(row_name.clone(), row));
            rows.push(konst(&row_name));
        }
        let entry = self.entry(
            info,
            info.span,
            "cli table",
            "cli.order: commands in the order of the app's rules, which is the order of --help",
            "app rules list",
        );
        let f = self.app_file(COMMANDS, "app", info);
        let anchor = f.anchor(entry);
        f.items.push(anchor);
        f.items.push(Stmt::ConstAssign("COMMANDS".into(), call(Expr::Array(rows), "freeze", vec![])));
        Ok(())
    }

    fn codec(lo: &mut Lower<'_, '_>, codec: &Codec, records: &mut Vec<(TypeId, Expr)>) -> R<Expr> {
        let c = || konst("Runtime::Store::Codec");
        let typed = |lo: &mut Lower<'_, '_>, named: &Option<TypeId>| match named {
            Some(id) if matches!(lo.inf.named(*id), Ok(Named::Alias { claim: Some(_), .. })) => vec![lo.type_ref(*id)],
            _ => vec![],
        };
        Ok(match codec {
            Codec::Int { named } => call(c(), "int", typed(lo, named)),
            Codec::Text { named } => call(c(), "text", typed(lo, named)),
            Codec::Bool => call(c(), "bool", vec![]),
            Codec::Sum(id) => call(c(), "variant", vec![child(lo.type_ref(*id), "ALL")]),
            Codec::Set(of) => call(c(), "set", vec![Self::codec(lo, of, records)?]),
            Codec::Record(id, fields) => {
                let name = lo.app.p.ty(*id).info.name.clone();
                if !records.iter().any(|(seen, _)| seen == id) {
                    let mut codecs = Vec::new();
                    for (field, fc) in fields {
                        codecs.push((text(field), Self::codec(lo, fc, records)?));
                    }
                    let get = Expr::Lambda(
                        vec![Param::Req("value".into())],
                        Box::new(Expr::Hash(
                            fields.iter().map(|(n, _)| (text(n), call(ident("value"), n, vec![]))).collect(),
                        )),
                    );
                    let made = call_args(
                        lo.type_ref(*id),
                        "new",
                        fields
                            .iter()
                            .map(|(n, _)| Arg::Key(n.clone(), call(ident("v"), "fetch", vec![text(n)])))
                            .collect(),
                    );
                    let make = Expr::Lambda(vec![Param::Req("v".into())], Box::new(made));
                    let value = call_args(
                        c(),
                        "record",
                        vec![Arg::Pos(Expr::Hash(codecs)), key("get", get), key("make", make)],
                    );
                    records.push((*id, value));
                }
                konst(&format!("{}_CODEC", screaming(&name)))
            }
            Codec::Keyed { key: k, of, .. } => {
                let inner = Self::codec(lo, of, records)?;
                call_block(c(), "keyed", vec![inner], &["value"], vec![Stmt::Expr(call(ident("value"), k, vec![]))])
            }
        })
    }

    fn store(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let qualified = app.p.qualified(info);
        let mut laws = Vec::new();
        for law in &app.laws {
            let Some((w, body)) = &law.invariant else { continue };
            let linfo = &app.p.law(law.id).info;
            let entry = self.entry(
                linfo,
                linfo.span,
                "store law",
                "store.law-on-read: a store that breaks a law of every state is refused as unreadable",
                "the law is `t.states.all(w => p)` with no other parameter",
            );
            let f = self.app_file(STORE, "law", linfo);
            let mut lo = Lower::new(app, f, Mode::Pure, &app.p.qualified(linfo));
            let param = lo.local(w);
            let body = lo.expr(body)?;
            let name = format!("{}_LAW", screaming(&linfo.name));
            let anchor = f.anchor(entry);
            f.items.push(anchor);
            f.items.extend(why(linfo));
            f.items.push(Stmt::ConstAssign(name.clone(), Expr::Lambda(vec![Param::Req(param)], Box::new(body))));
            laws.push(konst(&name));
        }
        let mut records = Vec::new();
        let mut fields = Vec::new();
        let mut gets = Vec::new();
        let mut makes = Vec::new();
        {
            let f = self.app_file(STORE, "app", info);
            f.need(RT_CODEC);
            f.need(WORLD);
            let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
            for field in &app.store {
                let name = state_name(app, field.state);
                fields.push((text(&name), Self::codec(&mut lo, &field.codec, &mut records)?));
                gets.push((text(&name), call(ident("world"), &name, vec![])));
                makes.push((field.state, Arg::Key(name.clone(), call(ident("v"), "fetch", vec![text(&name)]))));
            }
        }
        for (id, value) in records {
            let tinfo = &app.p.ty(id).info;
            let entry = self.entry(
                tinfo,
                tinfo.span,
                "store codec",
                "store.record: an object with exactly the record's fields, in declaration order",
                "the record's fields",
            );
            let f = self.app_file(STORE, "app", info);
            let anchor = f.anchor(entry);
            f.items.push(anchor);
            f.items.push(Stmt::ConstAssign(format!("{}_CODEC", screaming(&tinfo.name)), value));
        }
        let fast = app.strategies.on("store.canonical-fast-path");
        let schema = "store.schema: version, then scalar states, then collections, each in declaration order; a Map of records keyed by their first field is an array";
        let entry = if fast {
            self.entry(info, info.span, "store schema", &format!("{schema}. store.canonical-fast-path: the bytes this program writes are scanned straight into values, anything else goes to the strict reader"), "`@store(file(...))` and the types of the app's states; the canonical bytes are fixed and this program is their writer")
        } else {
            self.entry(info, info.span, "store schema", schema, "`@store(file(...))` and the types of the app's states")
        };
        let f = self.app_file(STORE, "app", info);
        makes.sort_by_key(|(id, _)| *id);
        let make = Expr::Lambda(
            vec![Param::Req("v".into())],
            Box::new(call_args(konst("App::World"), "new", makes.into_iter().map(|(_, a)| a).collect())),
        );
        let get = Expr::Lambda(vec![Param::Req("world".into())], Box::new(Expr::Hash(gets)));
        let codec = call_args(
            konst("Runtime::Store::Codec"),
            "store",
            vec![
                Arg::Pos(Expr::Int(1)),
                Arg::Pos(Expr::Hash(fields)),
                key("get", get),
                key("make", make),
                key("laws", Expr::Array(laws)),
                key("speculate", if fast { Expr::True } else { Expr::False }),
            ],
        );
        let anchor = f.anchor(entry);
        f.items.push(anchor);
        f.items.push(Stmt::ConstAssign("STORE_CODEC".into(), codec));
        Ok(())
    }

    fn wiring(&mut self) {
        let app = self.app;
        let info = &app.decl.info;
        let entry = self.entry(info, info.span, "app", "app.lazy: the app is its name, its command table and its store; the store's codec and the world are loaded only when a command runs", "`@bin`, `@store` and the rules list; help and usage errors need neither");
        let f = self.app_file(CLI, "app", info);
        f.need(RT_CLI);
        f.need(COMMANDS);
        let store = Expr::Call {
            recv: None,
            name: "lambda".into(),
            args: vec![],
            block: Some(Block {
                params: vec![],
                body: vec![
                    Stmt::Expr(Expr::Call {
                        recv: None,
                        name: "require_relative".into(),
                        args: vec![Arg::Pos(text("store"))],
                        block: None,
                    }),
                    Stmt::Expr(Expr::Array(vec![konst("STORE_CODEC"), call(konst("World"), "initial", vec![])])),
                ],
            }),
        };
        let value = call_args(
            konst("Runtime::CLI"),
            "app",
            vec![
                key("bin", text(&app.bin)),
                key("commands", konst("COMMANDS")),
                key("store_file", text(&app.store_file)),
                key("store_env", text(&app.store_env)),
                key("store", store),
            ],
        );
        let anchor = f.anchor(entry);
        f.items.push(anchor);
        f.items.extend(why(info));
        f.items.push(Stmt::ConstAssign("APP".into(), value));
    }
}
