//! Libraries: finding `libs/<name>/`, reading its `vocab.slg` and
//! `pretend.slg` into [`Library`] and [`Handler`] data, and checking a note
//! or trigger written in a program against what its library declares.
//!
//! Nothing here knows a library by name: every note, trigger, handler and app
//! shape comes from the files.

use super::Lowerer;
use super::declare::flatten_bar;
use super::expr::{Body, Cx, R};
use crate::error::IrError;
use crate::expr::{Block, Callee, Effect, Expr, ExprKind};
use crate::ids::*;
use crate::library::*;
use crate::program::Param;
use spec_lang_syntax::ast::{self, BinOp as AB, ExprKind as AK, Tail, UnOp as AU};
use spec_lang_syntax::{SourceFile, SourceMap, Span, parse_file};
use std::collections::BTreeMap;

/// A library's files, parsed.
pub(crate) struct LibSource {
    pub(crate) name: String,
    pub(crate) dir: String,
    pub(crate) vocab: ast::File,
    pub(crate) pretend: Option<ast::File>,
    /// The module name of its `contract.slg`, when that declares anything.
    pub(crate) contract: Option<String>,
}

/// Every library the files `use`, transitively, in the order a library's own
/// `use` items need: a library comes after those it names. A `use` of one
/// segment that names no module of the project is a library; inside a
/// library, any `use` is. A name the search path does not have is left for
/// `use` resolution to report. Contracts that declare something are added to
/// `files` as modules.
pub(crate) fn find_libraries(
    files: &mut Vec<(String, String, ast::File)>,
    map: &mut SourceMap,
    libs: &LibPath,
) -> Result<Vec<LibSource>, Vec<IrError>> {
    let modules: Vec<String> = files.iter().map(|(n, _, _)| n.clone()).collect();
    let mut wanted: Vec<String> = Vec::new();
    for (_, _, f) in files.iter() {
        for u in &f.uses {
            let name = u.path.dotted();
            if u.path.segments.len() == 1 && !modules.contains(&name) && !wanted.contains(&name) {
                wanted.push(name);
            }
        }
    }
    let mut out: Vec<LibSource> = Vec::new();
    let mut errors = Vec::new();
    let mut visiting: Vec<String> = Vec::new();
    for name in wanted {
        load_one(&name, libs, map, files, &mut out, &mut visiting, &mut errors);
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

#[allow(clippy::too_many_arguments)]
fn load_one(
    name: &str,
    libs: &LibPath,
    map: &mut SourceMap,
    files: &mut Vec<(String, String, ast::File)>,
    out: &mut Vec<LibSource>,
    visiting: &mut Vec<String>,
    errors: &mut Vec<IrError>,
) {
    if out.iter().any(|l| l.name == name) || visiting.iter().any(|v| v == name) {
        return;
    }
    let Some(dir) = libs.find(name) else { return };
    visiting.push(name.to_string());
    let mut parse = |file: &str, required: bool| -> Option<ast::File> {
        let path = dir.join(file);
        if !required && !path.is_file() {
            return None;
        }
        let text = match crate::load::read(&path) {
            Ok(t) => t,
            Err(f) => {
                errors.extend(f.errors);
                return None;
            }
        };
        let id = map.add(SourceFile::new(path.display().to_string(), text.clone(), 1));
        match parse_file(id, &text) {
            Ok(f) => Some(f),
            Err(d) => {
                errors.push(IrError::syntax(&d));
                None
            }
        }
    };
    let vocab = parse("vocab.slg", true);
    let pretend = parse("pretend.slg", false);
    let contract = parse("contract.slg", false);
    let deps: Vec<String> = [&vocab, &contract]
        .into_iter()
        .flatten()
        .flat_map(|f| f.uses.iter().map(|u| u.path.segments[0].name.clone()))
        .collect();
    for d in deps {
        load_one(&d, libs, map, files, out, visiting, errors);
    }
    visiting.retain(|v| v != name);
    let Some(vocab) = vocab else { return };
    let contract = contract.filter(|c| !c.decls.is_empty()).map(|c| {
        files.push((name.to_string(), dir.join("contract.slg").display().to_string(), c));
        name.to_string()
    });
    out.push(LibSource { name: name.to_string(), dir: dir.display().to_string(), vocab, pretend, contract });
}

fn form(span: Span, msg: impl Into<String>) -> IrError {
    IrError::error("E-LIB-FORM", span, msg)
}

/// The names in `@allows(a, b)`.
fn allows_of(decl: &ast::Decl) -> R<Vec<String>> {
    let mut out = Vec::new();
    for n in decl.notes.iter().filter(|n| n.path.dotted() == "allows") {
        for a in n.args.iter().flatten() {
            out.push(a.value.as_name().map(str::to_string).ok_or_else(|| form(a.span, "`@allows` takes kind names"))?);
        }
    }
    Ok(out)
}

/// `@uses(a.write, b.read)` as written.
fn labels_of(decl: &ast::Decl, sources: &SourceMap) -> Vec<String> {
    decl.notes
        .iter()
        .filter(|n| n.path.dotted() == "uses")
        .flat_map(|n| n.args.iter().flatten())
        .map(|a| sources.slice(a.value.span).to_string())
        .collect()
}

/// Notes a library declaration may carry, beyond its own meaning.
fn only_notes(decl: &ast::Decl, allowed: &[&str]) -> R<()> {
    match decl.notes.iter().find(|n| !allowed.contains(&n.path.dotted().as_str())) {
        Some(n) => Err(form(n.span, format!("`@{}` has no meaning on this declaration of a library", n.path.dotted()))),
        None => Ok(()),
    }
}

/// `1Hz` as (1, Some("Hz")); `5` as (5, None).
fn split_num(text: &str) -> Option<(i128, Option<String>)> {
    let digits_end = text.find(|c: char| !(c.is_ascii_digit() || c == '_')).unwrap_or(text.len());
    let value = super::expr::parse_int(&text[..digits_end])?;
    let unit = &text[digits_end..];
    if unit.is_empty() {
        Some((value, None))
    } else if unit.chars().all(|c| c.is_alphabetic()) {
        Some((value, Some(unit.to_string())))
    } else {
        None
    }
}

/// A whole number, possibly negative, possibly with a unit.
fn num_of(e: &ast::Expr) -> Option<(i128, Option<String>)> {
    match &e.kind {
        AK::Num(s) => split_num(s),
        AK::Unary(AU::Neg, x) => num_of(x).map(|(v, u)| (-v, u)),
        _ => None,
    }
}

impl Lowerer {
    /// Register every library: its handlers, notes, app shape and pretend.
    pub(super) fn register_libraries(&mut self, sources: Vec<LibSource>) {
        for src in sources {
            if let Err(e) = self.register_library(&src) {
                self.err(e);
            }
        }
    }

    fn register_library(&mut self, src: &LibSource) -> R<()> {
        let mut lib = Library {
            name: src.name.clone(),
            dir: src.dir.clone(),
            notes: Vec::new(),
            handlers: Vec::new(),
            app: None,
            contract: src.contract.as_ref().and_then(|c| self.prog.find_module(c)).map(|m| m.id),
        };
        if let Some(m) = lib.contract {
            self.prog.modules[m.index()].library = Some(src.name.clone());
        }
        let mut types: BTreeMap<String, NoteTy> = BTreeMap::new();
        for d in src.vocab.decls.iter().filter(|d| d.kind.dotted() == "type") {
            only_notes(d, &["why"])?;
            let ty = self.choice_type(d)?;
            types.insert(d.name.name.clone(), ty);
        }
        for d in &src.vocab.decls {
            match d.kind.dotted().as_str() {
                "type" => {}
                "note" => {
                    only_notes(d, &["why", "allows"])?;
                    let note = self.note_sig(&src.name, d, &types)?;
                    if lib.note(&note.name).is_some() {
                        return Err(form(d.name.span, format!("`{}` is declared twice", note.name)));
                    }
                    lib.notes.push(note);
                }
                "handler" => {
                    let h = self.handler_decl(&src.name, d)?;
                    if lib.handlers.iter().any(|x| self.prog.handler(*x).name == h.name) {
                        return Err(form(d.name.span, format!("handler `{}` is declared twice", h.name)));
                    }
                    let id = HandlerId(next_id(self.prog.handlers.len()));
                    for (i, op) in h.ops.iter().enumerate() {
                        let names = op.params.iter().flatten().map(|p| (p.name.clone(), false)).collect();
                        self.sigs.insert(Callee::Effect(Effect { handler: id, op: next_id(i) }), names);
                    }
                    self.prog.handlers.push(h);
                    lib.handlers.push(id);
                }
                "app" => {
                    if lib.app.is_some() {
                        return Err(form(d.name.span, "a library declares at most one app shape"));
                    }
                    lib.app = Some(self.app_shape(&lib, d)?);
                }
                other => {
                    return Err(form(
                        d.kind.span,
                        format!("a vocabulary declares notes, handlers, types and an app shape, not a `{other}`"),
                    ));
                }
            }
        }
        for u in &src.vocab.uses {
            lib.handlers.push(self.offered_handler(u)?);
        }
        if let Some(p) = &src.pretend {
            self.pretends(&lib, p)?;
        }
        self.prog.libraries.push(lib);
        Ok(())
    }

    /// `use cli.out;` in a vocabulary: offer that library's handler.
    fn offered_handler(&self, u: &ast::Use) -> R<HandlerId> {
        let [lib, handler] = u.path.segments.as_slice() else {
            return Err(form(u.span, "a vocabulary's `use` offers another library's handler: `use cli.out;`"));
        };
        let found = self.prog.library(&lib.name).and_then(|l| {
            l.handlers
                .iter()
                .copied()
                .find(|h| self.prog.handler(*h).name == handler.name && { self.prog.handler(*h).library == lib.name })
        });
        found.ok_or_else(|| {
            IrError::error(
                "E-NAME-UNKNOWN",
                u.span,
                format!("library `{}` has no handler `{}`", lib.name, handler.name),
            )
        })
    }

    /// `type Store = file(path: Text);`: a sum for note parameters.
    fn choice_type(&self, d: &ast::Decl) -> R<NoteTy> {
        let (None, Tail::Body(ast::Body::Value(ast::Value::Expr(body)))) = (&d.params, &d.tail) else {
            return Err(form(d.span, "a vocabulary's type is a sum of variants: `type Store = file(path: Text);`"));
        };
        let mut alts = Vec::new();
        flatten_bar(body, &mut alts);
        let mut variants = Vec::new();
        for alt in alts {
            let (name, params) = match &alt.kind {
                AK::Name(n) => (n.clone(), Vec::new()),
                AK::Call(c, args) if c.as_name().is_some() => {
                    let mut params = Vec::new();
                    for a in args {
                        let n = a.name.as_ref().ok_or_else(|| form(a.span, "a variant's field is `name: Type`"))?;
                        params.push((n.name.clone(), self.note_ty(&a.value, &BTreeMap::new())?));
                    }
                    (c.as_name().unwrap_or_default().to_string(), params)
                }
                _ => return Err(form(alt.span, "a variant is a name, or a name with fields")),
            };
            variants.push(NoteVariant { name, params });
        }
        Ok(NoteTy::Choice { name: d.name.name.clone(), variants })
    }

    /// A note parameter's type: `Text`, `Int`, a range, or a type of the vocabulary.
    fn note_ty(&self, e: &ast::Expr, types: &BTreeMap<String, NoteTy>) -> R<NoteTy> {
        match &e.kind {
            AK::Name(n) if n == "Text" => Ok(NoteTy::Text),
            AK::Name(n) if n == "Int" => Ok(NoteTy::Int),
            AK::Name(n) if types.contains_key(n) => Ok(types[n].clone()),
            AK::Binary(AB::RangeIncl, lo, hi) => match (num_of(lo), num_of(hi)) {
                (Some((lo, u1)), Some((hi, u2))) if u1 == u2 && lo <= hi => Ok(NoteTy::Range { lo, hi, unit: u1 }),
                _ => Err(form(e.span, "a range of whole numbers with one unit: `1..=10`, `1Hz..=1000Hz`")),
            },
            _ => Err(IrError::unsupported(
                e.span,
                format!("the note parameter type `{}`: only Text, Int, a range or a type of the vocabulary", {
                    self.prog.snippet(e.span)
                }),
            )),
        }
    }

    fn note_sig(&self, library: &str, d: &ast::Decl, types: &BTreeMap<String, NoteTy>) -> R<NoteSig> {
        if !matches!(d.tail, Tail::Semi) {
            return Err(form(d.span, "a library's note has no body: its runtime gives it meaning"));
        }
        let allows = allows_of(d)?;
        if allows.is_empty() {
            return Err(form(d.name.span, "a note says where it may stand: `@allows(app)`, `@allows(on)`"));
        }
        let mut params = Vec::new();
        for p in d.params.iter().flatten() {
            if p.default.is_some() || !p.notes.is_empty() {
                return Err(form(p.span, "a note parameter is `name: Type`"));
            }
            let ty = p.ty.as_ref().ok_or_else(|| form(p.span, "a note parameter needs a type"))?;
            params.push(NoteParam { name: p.name.name.clone(), ty: self.note_ty(ty, types)? });
        }
        Ok(NoteSig { library: library.to_string(), name: d.name.name.clone(), allows, params, span: d.name.span })
    }

    /// `handler out { rule line(text: Text); test text -> Text; }`, or `handler screen;`.
    fn handler_decl(&mut self, library: &str, d: &ast::Decl) -> R<Handler> {
        only_notes(d, &["why", "uses"])?;
        let labels = labels_of(d, &self.prog.sources);
        let children: &[ast::Decl] = match &d.tail {
            Tail::Semi => &[],
            Tail::Children(c) => c,
            Tail::Body(_) => {
                return Err(form(d.span, "a vocabulary's handler declares operations; its pretend is in pretend.slg"));
            }
        };
        let mut ops = Vec::new();
        for c in children {
            only_notes(c, &["why", "uses"])?;
            let kind = match c.kind.dotted().as_str() {
                "rule" => OpKind::Rule,
                "fn" => OpKind::Fn,
                "test" => OpKind::Test,
                other => return Err(form(c.kind.span, format!("an operation is a rule, fn or test, not a `{other}`"))),
            };
            if !matches!(c.tail, Tail::Semi) {
                return Err(form(c.span, "an operation has no body here: the pretend and the runtime give one"));
            }
            let uses = labels_of(c, &self.prog.sources);
            if let Some(u) = uses.iter().find(|u| !labels.contains(u)) {
                return Err(form(c.name.span, format!("`{u}` is not an effect of handler `{}`", d.name.name)));
            }
            let params = match &c.params {
                None => None,
                Some(ps) => {
                    let mut out = Vec::new();
                    for p in ps {
                        let ty = p.ty.as_ref().ok_or_else(|| form(p.span, "an operation's parameter needs a type"))?;
                        let ty = self.lower_ty(ModuleId(0), ty)?;
                        out.push(Param { name: p.name.name.clone(), ty, default: None, span: p.span });
                    }
                    Some(out)
                }
            };
            let result = match &c.result {
                Some(r) => Some(self.lower_ty(ModuleId(0), &r.ty)?),
                None => None,
            };
            ops.push(HandlerOp { name: c.name.name.clone(), kind, params, result, uses, span: c.name.span });
        }
        Ok(Handler {
            name: d.name.name.clone(),
            library: library.to_string(),
            labels,
            ops,
            pretend: None,
            span: d.name.span,
        })
    }

    /// `app Game(start: rule, @on(tick) tick: rule, ...)`.
    fn app_shape(&self, lib: &Library, d: &ast::Decl) -> R<AppShape> {
        only_notes(d, &["why"])?;
        let mut params = Vec::new();
        for p in d.params.iter().flatten() {
            let kind = match p.ty.as_ref().and_then(|t| t.as_name()) {
                Some(k @ ("rule" | "fn")) => k.to_string(),
                _ => return Err(form(p.span, "an app shape's parameter names a kind: `start: rule`, `input: fn`")),
            };
            let mut trigger = None;
            for n in &p.notes {
                let t = match (n.path.dotted().as_str(), n.args.as_deref()) {
                    ("on", Some([a])) => a.value.as_name(),
                    _ => None,
                };
                let t = t.filter(|t| lib.note(t).is_some_and(|s| s.allows.iter().any(|a| a == "on")));
                let t =
                    t.ok_or_else(|| form(n.span, "a shape parameter may carry `@on(<trigger of this library>)`"))?;
                if kind != "rule" {
                    return Err(form(n.span, "only a rule carries a trigger"));
                }
                trigger = Some(t.to_string());
            }
            params.push(ShapeParam { name: p.name.name.clone(), kind, trigger });
        }
        Ok(AppShape { library: lib.name.clone(), name: d.name.name.clone(), params })
    }

    /// `handler PretendOut(sent: mut List[Text] = []) = { out.line(text) -> sent.push(text); ... }`.
    fn pretends(&mut self, lib: &Library, file: &ast::File) -> R<()> {
        if let Some(u) = file.uses.first() {
            return Err(form(u.span, "a pretend uses nothing: it implements its own library's handlers"));
        }
        for d in &file.decls {
            if d.kind.dotted() != "handler" {
                return Err(form(d.kind.span, "pretend.slg declares handlers only"));
            }
            only_notes(d, &["why"])?;
            let Tail::Body(ast::Body::Value(ast::Value::Block(block))) = &d.tail else {
                return Err(form(d.span, "a pretend handler's body is `{ op -> reply; ... }`"));
            };
            let mut cx = Cx::new(ModuleId(0), Body::Fn);
            let mut state = Vec::new();
            for p in d.params.iter().flatten() {
                let ty = match p.ty.as_ref().map(|t| &t.kind) {
                    Some(AK::Unary(AU::Mut, inner)) => self.lower_ty(ModuleId(0), inner)?,
                    Some(_) | None => {
                        return Err(form(p.span, "a pretend's state is `name: mut Type = start`"));
                    }
                };
                let start = p.default.as_ref().ok_or_else(|| form(p.span, "a pretend's state needs a start value"))?;
                let default = Some(self.lower_expr(&mut Cx::new(ModuleId(0), Body::Value), start)?);
                self.bind_local(&mut cx, &p.name.name, true, p.name.span)?;
                state.push(Param { name: p.name.name.clone(), ty, default, span: p.span });
            }
            let mut target: Option<HandlerId> = None;
            let mut arms: BTreeMap<usize, PretendArm> = BTreeMap::new();
            for s in &block.stmts {
                let ast::StmtKind::Expr(ast::Expr { kind: AK::Arrow(lhs, rhs), .. }) = &s.kind else {
                    return Err(form(s.span, "a pretend arm is `handler.op(params) -> reply;`"));
                };
                let (callee, params) = match &lhs.kind {
                    AK::Call(c, args) => (&**c, Some(args.as_slice())),
                    _ => (&**lhs, None),
                };
                let AK::Field(h, op) = &callee.kind else {
                    return Err(form(lhs.span, "a pretend arm names `handler.op`"));
                };
                let hid = h.as_name().and_then(|h| {
                    lib.handlers
                        .iter()
                        .copied()
                        .find(|id| self.prog.handler(*id).name == h && self.prog.handler(*id).library == lib.name)
                });
                let hid = hid.ok_or_else(|| form(h.span, "a pretend implements a handler of its own library"))?;
                if target.is_some_and(|t| t != hid) {
                    return Err(form(h.span, "one pretend handler implements one handler"));
                }
                target = Some(hid);
                let handler = self.prog.handler(hid);
                let i = handler.ops.iter().position(|o| o.name == op.name).ok_or_else(|| {
                    form(op.span, format!("handler `{}` has no operation `{}`", handler.name, op.name))
                })?;
                let (kind, arity) = (handler.ops[i].kind, handler.ops[i].params.as_ref().map(Vec::len));
                let names: Vec<String> = match params {
                    Some(ps) => ps
                        .iter()
                        .map(|a| a.value.as_name().map(str::to_string).filter(|_| a.name.is_none()))
                        .collect::<Option<_>>()
                        .ok_or_else(|| form(lhs.span, "a pretend arm binds names: `out.line(text)`"))?,
                    None => Vec::new(),
                };
                if arity != params.map(|_| names.len()) {
                    return Err(form(lhs.span, format!("`{}` binds {} parameter(s)", op.name, arity.unwrap_or(0))));
                }
                let mark = cx.mark();
                cx.body = if kind == OpKind::Rule { Body::Rule } else { Body::Fn };
                let body = names
                    .iter()
                    .try_for_each(|n| self.bind_local(&mut cx, n, false, lhs.span))
                    .and_then(|()| self.arm_body(&mut cx, kind, rhs));
                cx.pop_to(mark);
                arms.insert(i, PretendArm { params: names, body: body?, span: s.span });
            }
            let Some(hid) = target else { return Err(form(d.span, "a pretend handler with no arm")) };
            let n = self.prog.handler(hid).ops.len();
            let arms = (0..n).map(|i| arms.remove(&i)).collect();
            let pretend = Pretend { name: d.name.name.clone(), state, arms, span: d.name.span };
            if self.prog.handlers[hid.index()].pretend.replace(pretend).is_some() {
                return Err(form(d.name.span, "a handler has one pretend"));
            }
        }
        Ok(())
    }

    /// A `rule` operation's reply is a statement (`sent.push(text)`); any other's is a value.
    fn arm_body(&mut self, cx: &mut Cx, kind: OpKind, rhs: &ast::Expr) -> R<Expr> {
        if kind != OpKind::Rule {
            return self.lower_expr(cx, rhs);
        }
        let stmt = ast::Stmt { kind: ast::StmtKind::Expr(rhs.clone()), span: rhs.span };
        let s = self.lower_stmt(cx, &stmt)?;
        Ok(Expr::new(ExprKind::Block(Block { stmts: vec![s], span: rhs.span }), rhs.span))
    }
}

/// What a module may use from its libraries.
impl Lowerer {
    /// `out.line` (called) or `out.text` (read): an operation of a library's
    /// handler, when `recv` is a name nothing else binds. Tests read any
    /// loaded handler's test operations; any other use needs a `use` of a
    /// library that offers the handler.
    pub(crate) fn effect_of(
        &self,
        cx: &Cx,
        recv: &ast::Expr,
        op: &ast::Ident,
        called: bool,
        span: Span,
    ) -> R<Option<Effect>> {
        let AK::Name(h) = &recv.kind else { return Ok(None) };
        if !matches!(self.resolve(cx, h), Err(super::scope::LookupError::Unknown)) {
            return Ok(None);
        }
        let visible = self.visible_handler(cx.module, h);
        let Some(hid) = visible.or_else(|| self.loaded_handler(h)) else { return Ok(None) };
        let handler = self.prog.handler(hid);
        let written = format!("{h}.{}", op.name);
        let Some(i) = handler.ops.iter().position(|o| o.name == op.name) else {
            return Err(IrError::error(
                "E-NAME-UNKNOWN",
                span,
                format!("handler `{h}` has no operation `{}`", op.name),
            ));
        };
        let o = &handler.ops[i];
        if o.params.is_some() != called {
            let msg =
                if called { format!("`{written}` is read, not called") } else { format!("write `{written}(...)`") };
            return Err(IrError::error("E-NAME-KIND", span, msg));
        }
        match o.kind {
            OpKind::Rule if cx.body != Body::Rule => {
                return Err(IrError::error(
                    "E-EFFECT-KIND",
                    span,
                    format!("`{written}` is an effect: only a rule sends it"),
                ));
            }
            OpKind::Test if cx.body != Body::Test => {
                return Err(IrError::error(
                    "E-EFFECT-KIND",
                    span,
                    format!("`{written}` is a test handler: only tests read it"),
                ));
            }
            _ => {}
        }
        if o.kind != OpKind::Test && visible.is_none() {
            let lib = &handler.library;
            return Err(IrError::error(
                "E-NAME-UNKNOWN",
                span,
                format!("`{written}` comes from the `{lib}` library: add `use {lib};`"),
            )
            .with_note(format!("the module has no `use {lib};`")));
        }
        Ok(Some(Effect { handler: hid, op: next_id(i) }))
    }

    pub(crate) fn module_libraries(&self, m: ModuleId) -> Vec<&Library> {
        self.prog.module(m).vocabularies.iter().filter_map(|v| self.prog.library(v)).collect()
    }

    /// The note or trigger `name` of the module's libraries. Two libraries
    /// with one name is `E-NOTE-AMBIG`.
    pub(crate) fn library_note(&self, m: ModuleId, name: &str, span: Span) -> R<Option<NoteSig>> {
        let found: Vec<&NoteSig> = self.module_libraries(m).into_iter().filter_map(|l| l.note(name)).collect();
        match found.as_slice() {
            [] => Ok(None),
            [one] => Ok(Some((*one).clone())),
            many => Err(IrError::error(
                "E-NOTE-AMBIG",
                span,
                format!(
                    "`{name}` is declared by {}: use one of them in this module",
                    many.iter().map(|n| format!("`{}`", n.library)).collect::<Vec<_>>().join(" and ")
                ),
            )),
        }
    }

    /// A hint for a note or trigger no used library declares: the loaded ones that do.
    pub(crate) fn note_hint(&self, name: &str) -> String {
        let from: Vec<String> = self
            .prog
            .libraries
            .iter()
            .filter(|l| l.note(name).is_some())
            .map(|l| format!("`use {};`", l.name))
            .collect();
        if from.is_empty() { String::new() } else { format!(" (it comes from {})", from.join(" or ")) }
    }

    /// The handler `name` the module may call, from its libraries.
    pub(crate) fn visible_handler(&self, m: ModuleId, name: &str) -> Option<HandlerId> {
        self.module_libraries(m)
            .into_iter()
            .flat_map(|l| l.handlers.iter().copied())
            .find(|h| self.prog.handler(*h).name == name)
    }

    /// Any loaded handler named `name`.
    pub(crate) fn loaded_handler(&self, name: &str) -> Option<HandlerId> {
        (0..self.prog.handlers.len()).map(|i| HandlerId(next_id(i))).find(|h| self.prog.handler(*h).name == name)
    }

    /// Arguments of a library note checked against its signature, in parameter order.
    pub(crate) fn note_use(&self, sig: &NoteSig, args: &[ast::Arg], span: Span) -> R<NoteUse> {
        let form = || {
            IrError::error("E-NOTE-ARG", span, format!("`{}`", sig.form()))
                .with_expected(sig.form(), self.prog.snippet(span).to_string())
        };
        let args = bind_args(&sig.params.iter().map(|p| (p.name.clone(), p.ty.clone())).collect::<Vec<_>>(), args)
            .ok_or_else(form)?;
        Ok(NoteUse { library: sig.library.clone(), name: sig.name.clone(), args, span })
    }
}

/// Bind arguments to `(name, type)` parameters, positional then named; every
/// parameter is given once and each value has its type.
fn bind_args(params: &[(String, NoteTy)], args: &[ast::Arg]) -> Option<Vec<NoteArg>> {
    let mut slots: Vec<Option<NoteValue>> = vec![None; params.len()];
    for (i, a) in args.iter().enumerate() {
        let at = match &a.name {
            None => i,
            Some(n) => params.iter().position(|(p, _)| *p == n.name)?,
        };
        let (_, ty) = params.get(at)?;
        if slots[at].is_some() {
            return None;
        }
        slots[at] = Some(note_value(ty, &a.value)?);
    }
    params.iter().zip(slots).map(|((p, _), v)| v.map(|value| NoteArg { param: p.clone(), value })).collect()
}

fn note_value(ty: &NoteTy, e: &ast::Expr) -> Option<NoteValue> {
    match ty {
        NoteTy::Text => match &e.kind {
            AK::Text(t) if t.prefix.is_none() => Some(NoteValue::Text(t.raw.clone())),
            _ => None,
        },
        NoteTy::Int => match num_of(e)? {
            (v, None) => Some(NoteValue::Int(v)),
            _ => None,
        },
        NoteTy::Range { lo, hi, unit } => {
            let (v, u) = num_of(e)?;
            if u != *unit || v < *lo || v > *hi {
                return None;
            }
            Some(match u {
                Some(unit) => NoteValue::Num { value: v, unit },
                None => NoteValue::Int(v),
            })
        }
        NoteTy::Choice { variants, .. } => {
            let (name, args): (&str, &[ast::Arg]) = match &e.kind {
                AK::Name(n) => (n, &[]),
                AK::Call(c, args) => (c.as_name()?, args),
                _ => return None,
            };
            let v = variants.iter().find(|v| v.name == name)?;
            if v.params.is_empty() && matches!(e.kind, AK::Call(..)) {
                return None;
            }
            Some(NoteValue::Variant { name: name.to_string(), args: bind_args(&v.params, args)? })
        }
    }
}

/// Where a library was looked for, for a `use` that found nothing.
pub(crate) fn searched(libs: &LibPath) -> String {
    libs.dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
}
