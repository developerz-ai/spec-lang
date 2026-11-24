//! The generated product files: one per declaration, plus the app's wiring.

use super::build as b;
use super::file::File;
use super::lower::*;
use crate::sem::app::{Access, App, Codec, Func, Rule, Shape};
use crate::sem::infer::{BodyKind, Infer, Named, R};
use crate::sem::names::{camel, screaming};
use crate::sem::prov::{Entry, slg};
use crate::sem::tree::*;
use spec_lang_ir::{DeclInfo, StateId, TypeId};
use spec_lang_syntax::Span;
use std::collections::BTreeMap;
use swc_ecma_ast::{Stmt, TsKeywordTypeKind};

pub const COMMANDS: &str = "src/app/commands.ts";
pub const STORE: &str = "src/app/store.ts";
pub const APP: &str = "src/app/app.ts";
pub const MAIN: &str = "src/main.ts";

/// Every generated file of one project, by path.
#[derive(Debug)]
pub struct Gen<'a, 'p> {
    pub app: &'a App<'p>,
    pub files: BTreeMap<String, File>,
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

    /// The file at `path`, noting that `kind name` is one of its sources.
    pub fn file(&mut self, path: &str, kind: &str, info: &DeclInfo) -> &mut File {
        let source = format!("{kind} {} ({})", info.name, slg(self.app.p, info, info.name_span));
        let f = self.files.entry(path.to_string()).or_insert_with(|| File::new(path));
        if !f.sources.contains(&source) {
            f.sources.push(source);
        }
        f
    }

    pub fn product(&mut self) -> R<()> {
        let app = self.app;
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
        self.outcome()?;
        for r in &app.rules {
            self.rule(r)?;
        }
        self.commands()?;
        self.store()?;
        self.wiring()
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
                    "sum.strings: a sum with no data is its variant names",
                    "no variant carries data",
                );
                let f = self.file(&path, "type", info);
                let consts = variants_name(&name);
                let names = variants.iter().map(|v| b::text(&v.name)).collect();
                let frozen = b::method(b::name("Object"), "freeze", vec![b::as_const(b::array(names))]);
                let at = f.anchor(entry, info.why.as_deref());
                f.items.push(b::export(b::const_decl(&consts, None, frozen), at));
                f.items.push(b::export(b::type_alias(&name, vec![], b::t_element_of(&consts)), swc_common::DUMMY_SP));
            }
            Named::Record(fields) => {
                let entry = self.entry(
                    info,
                    info.span,
                    "type",
                    "record.frozen: an interface and a constructor that applies the defaults",
                    "fields and defaults as declared",
                );
                let f = self.file(&path, "type", info);
                let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
                let mut props = Vec::new();
                let mut params = Vec::new();
                for field in fields {
                    let fty = lo.inf.ty(&field.ty, field.span)?;
                    let t = lo.ty(&fty)?;
                    let n = lo.local(&field.name);
                    props.push((camel(&field.name), lo.ty(&fty)?));
                    params.push(match &field.default {
                        Some(d) => {
                            let d = lo.inf.expr(d, Some(&fty))?;
                            b::param_default(&n, t, lo.expr(&d)?)
                        }
                        None => b::param(&n, t),
                    });
                }
                let literal =
                    b::object(fields.iter().map(|fd| (camel(&fd.name), b::name(&lo.local(&fd.name)))).collect());
                let body = vec![b::ret(Some(b::method(b::name("Object"), "freeze", vec![literal])))];
                let at = f.anchor(entry, info.why.as_deref());
                f.items.push(b::export(b::interface(&name, props), at));
                f.items.push(b::export(
                    b::function(&format!("make{name}"), params, Some(b::t_ref(&name, vec![])), body),
                    swc_common::DUMMY_SP,
                ));
            }
            Named::Alias { base, claim: None } => {
                let entry = self.entry(info, info.span, "type", "alias: another name for its base", "no `where` claim");
                let f = self.file(&path, "type", info);
                let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
                let t = lo.ty(&base)?;
                let at = f.anchor(entry, info.why.as_deref());
                f.items.push(b::export(b::type_alias(&name, vec![], t), at));
            }
            Named::Alias { base, claim: Some(claim) } => {
                let entry = self.entry(
                    info,
                    info.span,
                    "type",
                    "claim.brand: a branded base type; `parse` checks the claim at an edge, `of` monitors it inside",
                    "`where` claim on the alias",
                );
                inf.bind("self", base.clone());
                let holds = inf.expr(&claim.expr, Some(&Type::Bool))?;
                let f = self.file(&path, "type", info);
                let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
                let base_ty = lo.ty(&base)?;
                let base_ty2 = lo.ty(&base)?;
                let mut check = lo.expr(&holds)?;
                if lo.inf.base(&base) == Type::Text {
                    // A Text is well-formed Unicode; a JS string need not be.
                    let is_text = lo.f.value(TEXT, "isText");
                    check = b::bin(swc_ecma_ast::BinaryOp::LogicalAnd, b::call(is_text, vec![b::name("self")]), check);
                }
                let brand = lo.f.type_ref(VALUES, "Brand", vec![base_ty, b::t_str(&name)]);
                let claimed = lo.f.value(VALUES, "claimed");
                let value = b::call_t(
                    claimed,
                    vec![base_ty2, b::t_ref(&name, vec![])],
                    vec![b::text(&name), b::arrow(vec![b::bare("self")], check)],
                );
                let at = f.anchor(entry, info.why.as_deref());
                f.items.push(b::export(b::type_alias(&name, vec![], brand), at));
                f.items.push(b::export(b::const_decl(&name, None, value), swc_common::DUMMY_SP));
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
        let name = camel(&info.name);
        // Strategy match.table: a match of a sum to constants is one frozen lookup table.
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
                "match.table: one frozen lookup table, indexed by the variant",
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
        let f = self.file(&path, "fn", info);
        let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
        let mut params = Vec::new();
        for p in &func.params {
            let t = lo.ty(&p.ty)?;
            let n = lo.local(&p.name);
            params.push(match &p.default {
                Some(d) => b::param_default(&n, t, lo.expr(d)?),
                None => b::param(&n, t),
            });
        }
        let result = lo.ty(&func.result)?;
        let mut before = Vec::new();
        let body: Vec<Stmt> = match (table, &func.body.kind) {
            (Some(arms), _) => {
                let table_name = format!("{}_TABLE", screaming(&info.name));
                let mut rows = Vec::new();
                for (pat, value) in arms {
                    if let TPat::Variant { ty, tag } = pat
                        && let Named::Sum(variants) = lo.inf.named(*ty)?
                    {
                        rows.push((variants[*tag as usize].name.clone(), lo.expr(value)?));
                    }
                }
                let key = lo.ty(&func.params[0].ty)?;
                let table_ty = b::t_ref("Readonly", vec![b::t_ref("Record", vec![key, lo.ty(&func.result)?])]);
                before.push(b::const_decl(
                    &table_name,
                    Some(table_ty),
                    b::method(b::name("Object"), "freeze", vec![b::object(rows)]),
                ));
                vec![b::ret(Some(b::index(b::name(&table_name), b::name(&lo.local(&func.params[0].name)))))]
            }
            (None, TKind::Block(stmts)) => lo.stmts(stmts)?,
            (None, _) => vec![b::ret(Some(lo.expr(&func.body)?))],
        };
        let at = f.anchor(entry, info.why.as_deref());
        if let Some(table) = before.pop() {
            f.items.push(b::local(table, at));
            f.items.push(b::export(b::function(&name, params, Some(result), body), swc_common::DUMMY_SP));
        } else {
            f.items.push(b::export(b::function(&name, params, Some(result), body), at));
        }
        Ok(())
    }

    /// The first value of a state: its initial expression, or empty.
    fn initial(lo: &mut Lower<'_, '_>, id: StateId) -> R<b::E> {
        let s = lo.app.p.state(id);
        let ty = lo.inf.ty(&s.ty, s.info.span)?;
        match &s.init {
            Some(init) => {
                let mut inf = Infer::new(lo.app.p, &lo.app.world, lo.app.p.qualified(&s.info), BodyKind::Fn);
                let init = inf.expr(init, Some(&ty))?;
                lo.expr(&init)
            }
            None => match lo.inf.base(&ty) {
                Type::Map(k, v) => Ok(b::new("Map", vec![lo.ty(&k)?, lo.ty(&v)?], vec![])),
                Type::Set(_) | Type::List(_) => Ok(b::array(vec![])),
                Type::Option(_) => Ok(b::undefined()),
                _ => lo.inf.unsupported(s.info.span, "a state with no initial value that is not a collection"),
            },
        }
    }

    fn world(&mut self) -> R<()> {
        let app = self.app;
        let mut fields = Vec::new();
        let mut firsts = Vec::new();
        let mut anchors = Vec::new();
        for id in &app.world {
            let s = app.p.state(*id);
            let entry = self.entry(
                &s.info,
                s.info.span,
                "state",
                "world.immutable: one field of an immutable World; a rule returns a new one",
                "a module owns its state; rules are atomic",
            );
            let f = self.file(WORLD, "state", &s.info);
            let mut lo = Lower::new(app, f, Mode::Pure, &app.p.qualified(&s.info));
            let ty = lo.inf.ty(&s.ty, s.info.span)?;
            fields.push((state_name(app, *id), lo.ty(&ty)?));
            firsts.push((state_name(app, *id), Self::initial(&mut lo, *id)?));
            anchors.push(entry);
        }
        let info = &app.decl.info;
        let f = self.file(WORLD, "app", info);
        let first = anchors.first().cloned();
        let at = match first {
            Some(e) => f.anchor(e, None),
            None => swc_common::DUMMY_SP,
        };
        f.items.push(b::export(b::interface("World", fields), at));
        let empty = b::method(b::name("Object"), "freeze", vec![b::object(firsts)]);
        f.items.push(b::export(
            b::const_decl("EMPTY_WORLD", Some(b::t_ref("World", vec![])), empty),
            swc_common::DUMMY_SP,
        ));
        Ok(())
    }

    fn outcome(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let entry = self.entry(
            info,
            info.span,
            "refusals",
            "refusal.value: a refusal is a value holding the error's name, never an exception",
            "every `@guard ... else Name` and `fail Name` of the app's rules",
        );
        let f = self.file(OUTCOME, "app", info);
        let names = app.refusals.iter().map(|n| b::t_str(n)).collect::<Vec<_>>();
        let refusal = if names.is_empty() { b::keyword(TsKeywordTypeKind::TsNeverKeyword) } else { b::t_union(names) };
        let at = f.anchor(entry, None);
        f.items.push(b::export(b::type_alias("Refusal", vec![], refusal), at));
        let world = f.type_ref(WORLD, "World", vec![]);
        let outcome =
            f.type_ref(OUTCOME_RT, "RuleOutcome", vec![world, b::t_ref("Refusal", vec![]), b::t_ref("R", vec![])]);
        let default = b::keyword(TsKeywordTypeKind::TsUndefinedKeyword);
        f.items.push(b::export(b::type_alias("Outcome", vec![("R", Some(default))], outcome), swc_common::DUMMY_SP));
        Ok(())
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
            Access::Read => self.entry(info, info.span, "rule", "rule.pure-read: a pure function that returns the world it was given; shared lock, the store is never written", &format!("the body assigns no state; {uses}")),
            Access::Write => self.entry(info, info.span, "rule", "rule.pure-write: a pure function from a world to a new world; exclusive lock, atomic replace", &format!("the body assigns {}; {uses}", written.join(", "))),
        };
        let guard_entries: Vec<Entry> = rule
            .guards
            .iter()
            .enumerate()
            .map(|(i, g)| self.entry(info, g.span, &format!("guard[{i}]"), "guard.early-return: checked in written order before any draft exists, so a refusal changes nothing", "guards run first, top to bottom"))
            .collect();
        let take = app.strategies.on("draft.take-or-copy");
        let draft_entry = if take {
            self.entry(info, decl.body.span, "body.draft", "draft.take-or-copy: the body writes a draft of each state it assigns; a map the store session read for this one call is the draft itself, any other caller's is copied", &format!("write set {{{}}}; a rule is atomic and the process commits once", written.join(", ")))
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

        let f = self.file(&path, "rule", info);
        let top_at = f.anchor(top, info.why.as_deref());
        let mut lo = Lower::new(app, f, Mode::Guard, &qualified);
        let mut params = vec![b::param("w", lo.f.type_ref(WORLD, "World", vec![]))];
        for p in &rule.params {
            let t = lo.ty(&p.ty)?;
            let n = lo.local(&p.name);
            params.push(match &p.default {
                Some(d) => b::param_default(&n, t, lo.expr(d)?),
                None => b::param(&n, t),
            });
        }
        let result = if rule.result == Type::Unit { vec![] } else { vec![lo.ty(&rule.result)?] };
        let outcome = lo.f.type_ref(OUTCOME, "Outcome", result);

        let mut body = Vec::new();
        for (g, entry) in rule.guards.iter().zip(guard_entries) {
            let cond = lo.expr(&g.cond)?;
            let refuse = lo.f.value(OUTCOME_RT, "refuse");
            let at = lo.f.anchor(entry, None);
            body.push(b::at(b::if_return(b::not(cond), Some(b::call(refuse, vec![b::text(&g.error)]))), at));
        }
        let mut drafts = Vec::new();
        for s in &rule.writes {
            let state = app.p.state(*s);
            let ty = lo.inf.ty(&state.ty, state.info.span)?;
            let name = state_name(app, *s);
            let from = b::member(b::name("w"), &name);
            drafts.push(match lo.inf.base(&ty) {
                Type::Map(..) if take => b::const_(&name, None, b::call(lo.f.value(VALUES, "draft"), vec![from])),
                Type::Map(..) => b::const_(&name, None, b::new("Map", vec![], vec![from])),
                _ => b::let_(&name, None, from),
            });
        }
        if rule.emits_lines {
            let ty = b::t_ref("Array", vec![b::keyword(TsKeywordTypeKind::TsStringKeyword)]);
            drafts.push(b::const_("lines", Some(ty), b::array(vec![])));
        }
        if let Some(first) = drafts.first_mut() {
            let at = lo.f.anchor(draft_entry, None);
            *first = b::at(first.clone(), at);
        }
        body.extend(drafts);
        lo.mode = Mode::Rule { drafts: rule.writes.clone(), lines: rule.emits_lines };
        for (s, entry) in rule.body.iter().zip(stmt_entries) {
            let stmt = lo.stmt(s)?;
            let at = lo.f.anchor(entry, None);
            body.push(b::at(stmt, at));
        }
        if !matches!(rule.body.last().map(|s| &s.kind), Some(TStmtKind::Return(_) | TStmtKind::Fail(_))) {
            let end = lo.commit_end()?;
            let at = lo.f.anchor(end_entry, None);
            body.push(b::at(end, at));
        }
        f.items.push(b::export(b::function(&camel(&info.name), params, Some(outcome), body), top_at));
        Ok(())
    }

    fn commands(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let qualified = app.p.qualified(info);
        const ARGS: &str = "src/runtime/cli/arg-types.ts";
        const PARAMS: &str = "src/runtime/cli/params.ts";
        const COMMAND: &str = "src/runtime/cli/command.ts";
        // One argument type per named type that appears on the command line, in first-use order.
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
            let f = self.file(COMMANDS, "app", info);
            let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
            let name = b::text(&tinfo.name);
            let value = match lo.inf.named(*id)? {
                Named::Sum(_) => b::call(lo.f.value(ARGS, "sumArg"), vec![name, lo.variants_ref(*id)]),
                Named::Alias { claim: Some(_), .. } => {
                    let helper = if lo.inf.base(&Type::Named(*id)) == Type::Int { "intArg" } else { "textArg" };
                    let parse = b::member(lo.claimed_ref(*id), "parse");
                    b::call(lo.f.value(ARGS, helper), vec![name, parse])
                }
                _ => return lo.inf.unsupported(tinfo.span, "a command-line argument of a type with no claim"),
            };
            let at = f.anchor(entry, None);
            f.items.push(b::local(b::const_decl(&screaming(&tinfo.name), None, value), at));
        }
        let mut rows = Vec::new();
        for r in &app.rules {
            let rinfo = &app.p.rule(r.id).info;
            let (lock, fact) = match r.access {
                Access::Read => ("read", "the rule assigns no state"),
                Access::Write => ("write", "the rule assigns state"),
            };
            let entry = self.entry(rinfo, rinfo.span, "cli table row", &format!("cli.table: parameters become arguments by the derivation table of the cli vocabulary; lock mode `{lock}`"), fact);
            let f = self.file(COMMANDS, "app", info);
            let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
            let mut params = Vec::new();
            let mut names = Vec::new();
            let mut call_args = vec![b::name("w")];
            for (cli, p) in r.cli.iter().zip(&r.params) {
                let arg = cli.arg.map(|id| b::name(&screaming(&app.p.ty(id).info.name)));
                let n = b::text(&cli.name);
                let local = lo.local(&cli.name);
                let with_arg = |n, arg: Option<b::E>| vec![n, arg.unwrap_or_else(b::undefined)];
                params.push(match cli.shape {
                    Shape::Positional => b::call(lo.f.value(PARAMS, "positional"), with_arg(n, arg)),
                    Shape::Switch => b::call(lo.f.value(PARAMS, "switchFlag"), vec![n]),
                    Shape::Once if cli.optional => b::call(lo.f.value(PARAMS, "option"), with_arg(n, arg)),
                    Shape::Once => {
                        let default = match &p.default {
                            Some(d) => lo.expr(d)?,
                            None => b::undefined(),
                        };
                        let mut a = with_arg(n, arg);
                        a.push(default);
                        b::call(lo.f.value(PARAMS, "defaulted"), a)
                    }
                    Shape::Repeat => b::call(lo.f.value(PARAMS, "repeated"), with_arg(n, arg)),
                });
                call_args.push(if cli.shape == Shape::Repeat {
                    b::call(lo.f.value(VALUES, "setOf"), vec![b::name(&local)])
                } else {
                    b::name(&local)
                });
                names.push(local);
            }
            let rule_fn = lo.rule_ref(r.id);
            let mut invoke_params = vec![b::param("w", lo.f.type_ref(WORLD, "World", vec![]))];
            if !names.is_empty() {
                invoke_params.push(b::array_pat(&names));
            }
            let invoke = b::arrow(invoke_params, b::call(rule_fn, call_args));
            let world = lo.f.type_ref(WORLD, "World", vec![]);
            let command = lo.f.value(COMMAND, "command");
            let row =
                b::call_t(command, vec![world], vec![b::text(&r.command), b::text(lock), b::array(params), invoke]);
            let row_name = format!("{}_COMMAND", screaming(&r.name));
            let at = f.anchor(entry, None);
            // `command<World>` alone would leave its second type parameter uninferred, so the row is typed by its use.
            f.items.push(b::local(b::const_decl(&row_name, None, strip_type_args(row)), at));
            rows.push(b::name(&row_name));
        }
        let entry = self.entry(
            info,
            info.span,
            "cli table",
            "cli.order: commands in the order of the app's rules, which is the order of --help",
            "app rules list",
        );
        let f = self.file(COMMANDS, "app", info);
        let world = f.type_ref(WORLD, "World", vec![]);
        let command_ty = f.type_ref(COMMAND, "Command", vec![world]);
        let at = f.anchor(entry, None);
        let table = b::method(b::name("Object"), "freeze", vec![b::array(rows)]);
        f.items.push(b::export(b::const_decl("COMMANDS", Some(b::t_list(command_ty)), table), at));
        Ok(())
    }

    fn codec(lo: &mut Lower<'_, '_>, codec: &Codec, records: &mut Vec<(TypeId, b::E)>) -> R<b::E> {
        const CODEC: &str = "src/runtime/store/codec.ts";
        let parse_of = |lo: &mut Lower<'_, '_>, named: &Option<TypeId>, x: &str| match named {
            Some(id) if matches!(lo.inf.named(*id), Ok(Named::Alias { claim: Some(_), .. })) => {
                b::member(lo.claimed_ref(*id), "parse")
            }
            _ => b::arrow(vec![b::bare(x)], b::name(x)),
        };
        Ok(match codec {
            Codec::Int { named } => {
                let parse = parse_of(lo, named, "n");
                b::call(lo.f.value(CODEC, "int"), vec![parse])
            }
            Codec::Text { named } => {
                let parse = parse_of(lo, named, "s");
                b::call(lo.f.value(CODEC, "text"), vec![parse])
            }
            Codec::Bool => lo.f.value(CODEC, "bool"),
            Codec::Sum(id) => {
                let variants = lo.variants_ref(*id);
                b::call(lo.f.value(CODEC, "variant"), vec![variants])
            }
            Codec::Set(of) => {
                let inner = Self::codec(lo, of, records)?;
                b::call(lo.f.value(CODEC, "set"), vec![inner])
            }
            Codec::Record(id, fields) => {
                let name = lo.app.p.ty(*id).info.name.clone();
                let const_name = format!("{}Codec", camel(&name));
                if !records.iter().any(|(seen, _)| seen == id) {
                    let mut codecs = Vec::new();
                    for (field, c) in fields {
                        codecs.push((field.clone(), Self::codec(lo, c, records)?));
                    }
                    let record_ty = lo.ty(&Type::Named(*id))?;
                    let get = b::arrow(
                        vec![b::param("value", record_ty)],
                        b::object(
                            fields.iter().map(|(n, _)| (n.clone(), b::member(b::name("value"), &camel(n)))).collect(),
                        ),
                    );
                    let file = type_file(lo.app, *id);
                    let make_fn = lo.f.value(&file, &format!("make{name}"));
                    let make = b::arrow(
                        vec![b::bare("v")],
                        b::call(make_fn, fields.iter().map(|(n, _)| b::member(b::name("v"), n)).collect()),
                    );
                    let value = b::call(lo.f.value(CODEC, "record"), vec![b::object(codecs), get, make]);
                    records.push((*id, value));
                }
                b::name(&const_name)
            }
            Codec::Keyed { key, of, .. } => {
                let inner = Self::codec(lo, of, records)?;
                let by = b::arrow(vec![b::bare("value")], b::member(b::name("value"), &camel(key)));
                b::call(lo.f.value(CODEC, "keyed"), vec![inner, by])
            }
        })
    }

    fn store(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let qualified = app.p.qualified(info);
        const CODEC: &str = "src/runtime/store/codec.ts";
        const SESSION: &str = "src/runtime/store/session.ts";
        let mut law_names = Vec::new();
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
            let f = self.file(STORE, "law", linfo);
            let mut lo = Lower::new(app, f, Mode::Pure, &app.p.qualified(linfo));
            let world = lo.f.type_ref(WORLD, "World", vec![]);
            let param = b::param(&lo.local(w), world);
            let body = vec![b::ret(Some(lo.expr(body)?))];
            let name = camel(&linfo.name);
            let at = f.anchor(entry, linfo.why.as_deref());
            f.items.push(b::local(
                b::function(&name, vec![param], Some(b::keyword(TsKeywordTypeKind::TsBooleanKeyword)), body),
                at,
            ));
            law_names.push(b::name(&name));
        }
        let mut records = Vec::new();
        let mut fields = Vec::new();
        let mut gets = Vec::new();
        let mut makes = Vec::new();
        {
            let f = self.file(STORE, "app", info);
            let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
            for field in &app.store {
                let key = app.p.state(field.state).info.name.clone();
                let state = state_name(app, field.state);
                fields.push((key.clone(), Self::codec(&mut lo, &field.codec, &mut records)?));
                gets.push((key.clone(), b::member(b::name("w"), &state)));
                makes.push((field.state, state, b::member(b::name("v"), &key)));
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
            let f = self.file(STORE, "app", info);
            let at = f.anchor(entry, None);
            f.items.push(b::local(b::const_decl(&format!("{}Codec", camel(&tinfo.name)), None, value), at));
        }
        let fast = app.strategies.on("store.canonical-fast-path");
        let schema = "store.schema: version, then scalar states, then collections, each in declaration order; a Map of records keyed by their first field is an array";
        let entry = if fast {
            self.entry(info, info.span, "store schema", &format!("{schema}. store.canonical-fast-path: the bytes this program writes are read by one regular expression per record, anything else by the strict reader"), "`@store(file(...))` and the types of the app's states; the canonical bytes are fixed and this program is their writer")
        } else {
            self.entry(info, info.span, "store schema", schema, "`@store(file(...))` and the types of the app's states")
        };
        let f = self.file(STORE, "app", info);
        let world = f.type_ref(WORLD, "World", vec![]);
        let world2 = f.type_ref(WORLD, "World", vec![]);
        // The world is built in declaration order, whatever order the file holds its fields in.
        makes.sort_by_key(|(id, _, _)| *id);
        let make = b::arrow(vec![b::bare("v")], b::object(makes.into_iter().map(|(_, n, v)| (n, v)).collect()));
        let get = b::arrow(vec![b::param("w", world)], b::object(gets));
        let codec = b::call(
            f.value(CODEC, "store"),
            vec![b::big(1), b::object(fields), get, make, b::array(law_names), b::boolean(fast)],
        );
        let initial = f.value(WORLD, "EMPTY_WORLD");
        let store_ty = f.type_ref(SESSION, "Store", vec![world2]);
        let value = b::method(
            b::name("Object"),
            "freeze",
            vec![b::object(vec![("initial".into(), initial), ("codec".into(), codec)])],
        );
        let at = f.anchor(entry, None);
        f.items.push(b::export(b::const_decl("STORE", Some(store_ty), value), at));
        Ok(())
    }

    fn wiring(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let entry = self.entry(
            info,
            info.span,
            "app",
            "app.table: the app is its name, its command table and its store",
            "`@bin`, `@store` and the rules list",
        );
        let f = self.file(APP, "app", info);
        let commands = f.value(COMMANDS, "COMMANDS");
        let store = f.value(STORE, "STORE");
        let make = f.value("src/runtime/cli/app.ts", "app");
        let value =
            b::call(make, vec![b::text(&app.bin), commands, store, b::text(&app.store_file), b::text(&app.store_env)]);
        let at = f.anchor(entry, info.why.as_deref());
        f.items.push(b::export(b::const_decl("APP", None, value), at));

        let entry = self.entry(
            info,
            info.span,
            "entry point",
            "main.sync: argv and the environment in, fds 1 and 2 and an exit code out",
            "a rule never waits on the outside world",
        );
        let f = self.file(MAIN, "app", info);
        let app_value = f.value(APP, "APP");
        let main = f.value("src/runtime/cli/run.ts", "main");
        let at = f.anchor(entry, None);
        f.items.push(swc_ecma_ast::ModuleItem::Stmt(b::at(b::expr_stmt(b::call(main, vec![app_value])), at)));
        Ok(())
    }
}

// The builders pass expressions boxed, as swc holds them.
#[allow(clippy::boxed_local)]
fn strip_type_args(e: b::E) -> b::E {
    match *e {
        swc_ecma_ast::Expr::Call(mut c) => {
            c.type_args = None;
            Box::new(swc_ecma_ast::Expr::Call(c))
        }
        other => Box::new(other),
    }
}
