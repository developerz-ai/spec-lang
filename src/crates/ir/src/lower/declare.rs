//! Passes 2 to 4: declare names, resolve `use`, classify `type` bodies.

use super::Lowerer;
use super::scope::Binding;
use crate::error::IrError;
use crate::expr::{Expr, ExprKind};
use crate::ids::*;
use crate::program::*;
use crate::ty::Ty;
use spec_lang_syntax::ast::{self, BinOp as AstBin, ExprKind as AK, Tail};
use spec_lang_syntax::{FileId, Span};

/// The built-in module 0, `core`, holding `Option` and `Result`.
pub(super) fn add_core(prog: &mut Program) {
    let span = Span::new(FileId(u32::MAX), 0, 0);
    prog.modules.push(Module {
        id: ModuleId(0),
        name: "core".into(),
        path: String::new(),
        uses: Vec::new(),
        vocabularies: Vec::new(),
        decls: vec![DeclRef::Type(TypeId(0)), DeclRef::Type(TypeId(1))],
        library: None,
    });
    let info = DeclInfo { module: ModuleId(0), name: "Option".into(), span, name_span: span, is_pub: true, why: None };
    let some = Variant {
        name: "Some".into(),
        fields: vec![Field { name: "value".into(), ty: Ty::Unit, default: None, span }],
        span,
    };
    let none = Variant { name: "none".into(), fields: Vec::new(), span };
    prog.types.push(TypeDecl { info: info.clone(), def: TypeDef::Sum(vec![some, none]) });
    prog.option_type = TypeId(0);
    let field = |name: &str| Field { name: name.into(), ty: Ty::Unit, default: None, span };
    let ok = Variant { name: "Ok".into(), fields: vec![field("value")], span };
    let err = Variant { name: "Err".into(), fields: vec![field("error")], span };
    prog.types.push(TypeDecl { info: DeclInfo { name: "Result".into(), ..info }, def: TypeDef::Sum(vec![ok, err]) });
    prog.result_type = TypeId(1);
}

/// The text of a `@why("...")` note, if the declaration has one.
fn why_of(decl: &ast::Decl) -> Option<String> {
    decl.notes.iter().find(|n| n.path.dotted() == "why").and_then(|n| {
        let arg = n.args.as_ref()?.first()?;
        match &arg.value.kind {
            AK::Text(t) if t.prefix.is_none() => Some(t.raw.clone()),
            _ => None,
        }
    })
}

impl Lowerer {
    fn placeholder(span: Span) -> Expr {
        Expr::new(ExprKind::Unit, span)
    }

    /// Pass 2: a table entry per declaration and its name in the module scope.
    pub(super) fn declare_module(&mut self, m: ModuleId, file: &ast::File) {
        for decl in &file.decls {
            if let Some(d) = self.declare(m, decl) {
                let name = decl.name.name.clone();
                if self.scopes.own[m.index()].insert(name.clone(), Binding::Decl(d)).is_some() {
                    self.err(IrError::error("E-NAME-AMBIG", decl.name.span, format!("{name} is declared twice")));
                }
                self.prog.modules[m.index()].decls.push(d);
                self.pending.push((d, decl.clone()));
            }
        }
    }

    fn declare(&mut self, m: ModuleId, decl: &ast::Decl) -> Option<DeclRef> {
        let kind = decl.kind.dotted();
        let info = DeclInfo {
            module: m,
            name: decl.name.name.clone(),
            span: decl.span,
            name_span: decl.name.span,
            is_pub: decl.is_pub,
            why: why_of(decl),
        };
        if let Tail::Children(_) = decl.tail {
            self.err(IrError::unsupported(decl.span, format!("a {kind} with nested declarations")));
            return None;
        }
        if decl.generics.is_some() {
            self.err(IrError::unsupported(decl.span, "generic parameters"));
            return None;
        }
        let p = Self::placeholder(decl.span);
        let sig: Vec<(String, bool)> =
            decl.params.iter().flatten().map(|p| (p.name.name.clone(), p.default.is_some())).collect();
        let r = match kind.as_str() {
            "type" => {
                self.prog.types.push(TypeDecl { info, def: TypeDef::Alias(Ty::Unit) });
                DeclRef::Type(TypeId(next_id(self.prog.types.len() - 1)))
            }
            "const" => {
                self.prog.consts.push(ConstDecl { info, ty: None, value: p });
                DeclRef::Const(ConstId(next_id(self.prog.consts.len() - 1)))
            }
            "state" => {
                self.prog.states.push(StateDecl { info, ty: Ty::Unit, init: None });
                DeclRef::State(StateId(next_id(self.prog.states.len() - 1)))
            }
            "fn" => {
                self.prog.fns.push(FnDecl {
                    info,
                    params: Vec::new(),
                    result: None,
                    body: p,
                    cases: Vec::new(),
                    uses: Vec::new(),
                    foreign: None,
                });
                DeclRef::Fn(FnId(next_id(self.prog.fns.len() - 1)))
            }
            "rule" => {
                self.prog.rules.push(RuleDecl {
                    info,
                    params: Vec::new(),
                    result: None,
                    guards: Vec::new(),
                    body: p,
                    uses: Vec::new(),
                    triggers: Vec::new(),
                    foreign: None,
                });
                DeclRef::Rule(RuleId(next_id(self.prog.rules.len() - 1)))
            }
            "test" => {
                let body = crate::expr::Block { stmts: Vec::new(), span: decl.span };
                self.prog.tests.push(TestDecl { info, params: Vec::new(), given: Vec::new(), cases: Vec::new(), body });
                DeclRef::Test(TestId(next_id(self.prog.tests.len() - 1)))
            }
            "law" => {
                self.prog.laws.push(LawDecl { info, params: Vec::new(), body: p });
                DeclRef::Law(LawId(next_id(self.prog.laws.len() - 1)))
            }
            "app" => {
                self.prog.apps.push(AppDecl {
                    info,
                    rules: Vec::new(),
                    target: None,
                    notes: Vec::new(),
                    shape: None,
                    fields: Vec::new(),
                });
                DeclRef::App(AppId(next_id(self.prog.apps.len() - 1)))
            }
            "task" | "event" | "flow" | "note" | "kind" | "handler" => {
                self.err(IrError::unsupported(decl.kind.span, format!("kind `{kind}`")));
                return None;
            }
            _ => {
                self.err(IrError::error(
                    "E-KIND-UNKNOWN",
                    decl.kind.span,
                    format!("`{kind}` is neither a core kind nor a kind of a used library"),
                ));
                return None;
            }
        };
        let callee = match r {
            DeclRef::Fn(f) => Some(crate::expr::Callee::Fn(f)),
            DeclRef::Rule(x) => Some(crate::expr::Callee::Rule(x)),
            DeclRef::Type(t) if decl.params.is_some() => Some(crate::expr::Callee::Record(t)),
            _ => None,
        };
        if let Some(c) = callee {
            self.sigs.insert(c, sig);
        }
        Some(r)
    }

    /// Pass 3: bind every `use` of a module.
    pub(super) fn resolve_uses(&mut self, m: ModuleId, file: &ast::File) {
        for u in &file.uses {
            let dotted = u.path.dotted();
            // A library's contract is a module too: `use raylib;` binds it and the library.
            let library = (u.path.segments.len() == 1).then(|| self.prog.library(&dotted)).flatten();
            let item = if let Some(target) = self.prog.find_module(&dotted).map(|t| t.id) {
                if library.is_some() {
                    self.prog.modules[m.index()].vocabularies.push(dotted.clone());
                }
                let alias = u.path.segments.last().map(|s| s.name.clone()).unwrap_or_default();
                UseItem::Module { alias, module: target, span: u.span }
            } else if let Some((target, name)) = self.split_decl_path(&u.path) {
                match self.scopes.member(target, &name) {
                    Some(Binding::Decl(d)) => {
                        if let Err(e) = self.check_visible(Binding::Decl(d), u.span) {
                            self.err(e);
                            continue;
                        }
                        UseItem::Decl { name, decl: d, span: u.span }
                    }
                    _ => {
                        self.err(IrError::error(
                            "E-NAME-UNKNOWN",
                            u.span,
                            format!("module {} declares no {name}", self.prog.module(target).name),
                        ));
                        continue;
                    }
                }
            } else if library.is_some() {
                UseItem::Vocabulary { name: dotted, span: u.span }
            } else {
                self.err(
                    IrError::error(
                        "E-NAME-UNKNOWN",
                        u.span,
                        format!("`use {dotted}` names no module and no library on the search path"),
                    )
                    .with_note(format!(
                        "looked for `{dotted}/vocab.slg` in: {}",
                        super::library::searched(&self.lib_path)
                    )),
                );
                continue;
            };
            let bound = match &item {
                UseItem::Module { alias, module, .. } => Some((alias.clone(), Binding::Module(*module))),
                UseItem::Decl { name, decl, .. } => Some((name.clone(), Binding::Decl(*decl))),
                UseItem::Vocabulary { name, .. } => {
                    self.prog.modules[m.index()].vocabularies.push(name.clone());
                    None
                }
            };
            if let Some((name, b)) = bound
                && self.scopes.imported[m.index()].insert(name.clone(), b).is_some()
            {
                self.err(IrError::error("E-NAME-AMBIG", u.span, format!("{name} is bound by two `use` items")));
            }
            self.prog.modules[m.index()].uses.push(item);
        }
    }

    fn split_decl_path(&self, path: &ast::Path) -> Option<(ModuleId, String)> {
        let (last, init) = path.segments.split_last()?;
        let module = init.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(".");
        let m = self.prog.find_module(&module)?;
        Some((m.id, last.name.clone()))
    }

    fn module_deps(&self, m: &Module) -> Vec<ModuleId> {
        m.uses
            .iter()
            .filter_map(|u| match u {
                UseItem::Module { module, .. } => Some(*module),
                UseItem::Decl { decl, .. } => Some(self.decl_module(*decl)),
                UseItem::Vocabulary { .. } => None,
            })
            .collect()
    }

    pub(crate) fn decl_module(&self, d: DeclRef) -> ModuleId {
        self.decl_info(d).module
    }

    pub(crate) fn decl_info(&self, d: DeclRef) -> &DeclInfo {
        let p = &self.prog;
        match d {
            DeclRef::Type(i) => &p.ty(i).info,
            DeclRef::Const(i) => &p.constant(i).info,
            DeclRef::State(i) => &p.state(i).info,
            DeclRef::Fn(i) => &p.func(i).info,
            DeclRef::Rule(i) => &p.rule(i).info,
            DeclRef::Test(i) => &p.test(i).info,
            DeclRef::Law(i) => &p.law(i).info,
            DeclRef::App(i) => &p.app(i).info,
        }
    }

    /// `use` has no cycles (`E-USE-CYCLE`).
    pub(super) fn check_use_cycles(&mut self) {
        let n = self.prog.modules.len();
        let deps: Vec<Vec<ModuleId>> = self.prog.modules.iter().map(|m| self.module_deps(m)).collect();
        // 0 unvisited, 1 on the stack, 2 done.
        let mut mark = vec![0u8; n];
        for start in 0..n {
            if mark[start] != 0 {
                continue;
            }
            let mut stack = vec![(start, 0usize)];
            mark[start] = 1;
            while let Some((node, i)) = stack.pop() {
                if let Some(next) = deps[node].get(i).map(|d| d.index()) {
                    stack.push((node, i + 1));
                    if mark[next] == 1 {
                        let name = &self.prog.modules[next].name;
                        let span = self.use_span(node, next);
                        let msg = format!("module {name} is part of a `use` cycle; an event breaks one");
                        match span {
                            Some(s) => self.errors.push(IrError::error("E-USE-CYCLE", s, msg)),
                            None => self.errors.push(IrError::load(msg)),
                        }
                        return;
                    }
                    if mark[next] == 0 {
                        mark[next] = 1;
                        stack.push((next, 0));
                    }
                } else {
                    mark[node] = 2;
                }
            }
        }
    }

    fn use_span(&self, from: usize, to: usize) -> Option<Span> {
        self.prog.modules[from].uses.iter().find_map(|u| match u {
            UseItem::Module { module, span, .. } if module.index() == to => Some(*span),
            UseItem::Decl { decl, span, .. } if self.decl_module(*decl).index() == to => Some(*span),
            _ => None,
        })
    }

    /// Pass 4: what a `type` declaration is. Sum variants enter the scope here.
    pub(super) fn classify_type(&mut self, t: TypeId, decl: &ast::Decl) {
        let m = self.prog.ty(t).info.module;
        let body = match (&decl.params, &decl.tail) {
            (Some(_), Tail::Semi) => {
                self.prog.types[t.index()].def = TypeDef::Record(Vec::new());
                return;
            }
            (None, Tail::Body(ast::Body::Value(ast::Value::Expr(e)))) => e,
            _ => {
                self.err(IrError::unsupported(decl.span, "this form of `type`"));
                return;
            }
        };
        let mut alts = Vec::new();
        flatten_bar(body, &mut alts);
        if alts.len() == 1 {
            return; // an alias, lowered in pass 5
        }
        if alts.iter().all(|a| matches!(a.kind, AK::Field(..))) {
            self.prog.types[t.index()].def = TypeDef::Rules(Vec::new());
            return;
        }
        let mut variants = Vec::new();
        for alt in alts {
            let (name, span, sig) = match &alt.kind {
                AK::Name(n) => (n.clone(), alt.span, Vec::new()),
                AK::Call(callee, args) => match callee.as_name() {
                    Some(n) => (
                        n.to_string(),
                        callee.span,
                        args.iter()
                            .map(|a| (a.name.as_ref().map(|i| i.name.clone()).unwrap_or_default(), false))
                            .collect(),
                    ),
                    None => {
                        self.err(IrError::unsupported(alt.span, "this alternative of a sum type"));
                        return;
                    }
                },
                _ => {
                    self.err(IrError::unsupported(alt.span, "an alternative that is neither a variant nor a rule"));
                    return;
                }
            };
            if let Ok(Binding::Decl(DeclRef::Type(_))) = self.scopes.lookup(m, &name) {
                self.err(IrError::unsupported(span, format!("a union of existing types ({name} is a type)")));
                return;
            }
            let tag = next_id(variants.len());
            self.sigs.insert(crate::expr::Callee::Variant(t, tag), sig);
            if self.scopes.own[m.index()].insert(name.clone(), Binding::Variant(t, tag)).is_some() {
                self.err(IrError::error("E-NAME-AMBIG", span, format!("{name} is declared twice")));
            }
            variants.push(Variant { name, fields: Vec::new(), span });
        }
        self.prog.types[t.index()].def = TypeDef::Sum(variants);
    }
}

impl Lowerer {
    /// `use a.Status;` also binds `Status`'s variants (`Open`, `Done`).
    pub(super) fn import_variants(&mut self) {
        for m in 0..self.prog.modules.len() {
            let types: Vec<(String, TypeId, Span)> = self.prog.modules[m]
                .uses
                .iter()
                .filter_map(|u| match u {
                    UseItem::Decl { name, decl: DeclRef::Type(t), span } => Some((name.clone(), *t, *span)),
                    _ => None,
                })
                .collect();
            for (_, t, span) in types {
                let TypeDef::Sum(vs) = &self.prog.ty(t).def else { continue };
                let names: Vec<String> = vs.iter().map(|v| v.name.clone()).collect();
                for (tag, name) in names.into_iter().enumerate() {
                    let b = Binding::Variant(t, next_id(tag));
                    if let Some(old) = self.scopes.imported[m].insert(name.clone(), b)
                        && old != b
                    {
                        self.err(IrError::error("E-NAME-AMBIG", span, format!("{name} is bound by two `use` items")));
                    }
                }
            }
        }
    }
}

/// The alternatives of `a | b | c`.
pub(super) fn flatten_bar<'e>(e: &'e ast::Expr, out: &mut Vec<&'e ast::Expr>) {
    match &e.kind {
        AK::Binary(AstBin::Bar, l, r) => {
            flatten_bar(l, out);
            flatten_bar(r, out);
        }
        _ => out.push(e),
    }
}
