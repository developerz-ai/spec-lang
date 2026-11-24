//! What a program takes from its libraries: an app's notes and shape,
//! triggers in `@on(...)`, effect labels in `@uses(...)`; and `foreign` bodies.

use super::Lowerer;
use super::expr::{Body, Cx, R};
use super::notes::note_args;
use super::scope::Binding;
use crate::error::IrError;
use crate::ids::*;
use crate::library::{AppField, NoteUse, Trigger};
use crate::program::*;
use spec_lang_syntax::ast::{self, ExprKind as AK};

impl Lowerer {
    /// `@uses(...)` as written. A label of a library's handler (`screen.write`)
    /// needs a `use` of a library that offers the handler.
    pub(super) fn uses_of(&self, m: ModuleId, decl: &ast::Decl) -> R<Vec<String>> {
        let mut uses = Vec::new();
        for (_, args) in note_args(&decl.notes, "uses") {
            for a in args {
                let label = self.prog.snippet(a.value.span).to_string();
                let owner = self.prog.handlers.iter().find(|h| h.labels.contains(&label));
                if let Some(h) = owner
                    && self.visible_handler(m, &h.name).is_none()
                {
                    return Err(IrError::error(
                        "E-NAME-UNKNOWN",
                        a.span,
                        format!("`{label}` comes from the `{}` library: add `use {};`", h.library, h.library),
                    ));
                }
                uses.push(label);
            }
        }
        Ok(uses)
    }

    /// `foreign <lang> "<symbol>"`, kept as written for the backends.
    pub(super) fn foreign(&self, lang: &ast::Ident, symbol: &ast::TextLit) -> R<Foreign> {
        let span = lang.span.to(symbol.span);
        if symbol.prefix.is_some() || symbol.raw.contains(['\\', '{']) {
            return Err(IrError::unsupported(span, "a foreign symbol with a prefix, an escape or interpolation"));
        }
        Ok(Foreign { lang: lang.name.clone(), symbol: symbol.raw.clone(), span })
    }

    /// Every `@on(trigger)`: a trigger of one of the module's libraries.
    pub(super) fn triggers_of(&self, m: ModuleId, decl: &ast::Decl) -> R<Vec<Trigger>> {
        let mut out = Vec::new();
        for (n, args) in note_args(&decl.notes, "on") {
            let [a] = args else {
                return Err(IrError::error("E-NOTE-ARG", n.span, "`@on` takes one trigger"));
            };
            let (name, targs): (&str, &[ast::Arg]) = match &a.value.kind {
                AK::Call(c, targs) if c.as_name().is_some() => (c.as_name().unwrap_or_default(), targs),
                AK::Name(x) => (x, &[]),
                _ => ("", &[]),
            };
            let sig = self.library_note(m, name, a.span)?.filter(|s| s.allows.iter().any(|k| k == "on"));
            let Some(sig) = sig else {
                return Err(IrError::unsupported(
                    a.span,
                    format!("the trigger `{}`{}", self.prog.snippet(a.span), self.note_hint(name)),
                ));
            };
            out.push(self.note_use(&sig, targs, a.span)?);
        }
        Ok(out)
    }

    /// Pass 5 for an app: `rules: [...]`, or the parameters of the shape of
    /// one of the module's libraries.
    pub(super) fn app_signature(&mut self, a: AppId, m: ModuleId, decl: &ast::Decl) -> R<()> {
        let shapes: Vec<_> = self.module_libraries(m).into_iter().filter_map(|l| l.app.clone()).collect();
        match shapes.as_slice() {
            [] => self.rules_app(a, m, decl),
            [shape] => self.shaped_app(a, m, decl, shape),
            many => Err(IrError::unsupported(
                decl.span,
                format!(
                    "an app in a module that uses {}, which each give an app shape",
                    many.iter().map(|s| format!("`{}`", s.library)).collect::<Vec<_>>().join(" and ")
                ),
            )),
        }
    }

    fn rules_app(&mut self, a: AppId, m: ModuleId, decl: &ast::Decl) -> R<()> {
        let cx = Cx::new(m, Body::Value);
        let mut rules = Vec::new();
        for p in decl.params.iter().flatten() {
            if p.name.name != "rules" {
                return Err(IrError::unsupported(p.span, format!("the app parameter `{}`", p.name.name)));
            }
            let Some(AK::List(items)) = p.ty.as_ref().map(|t| &t.kind) else {
                return Err(IrError::error(
                    "E-TYPE-MISMATCH",
                    p.span,
                    "`rules:` takes a list of rules: `rules: [a.b, c]`",
                ));
            };
            for it in items {
                match self.binding_of(&cx, &it.value) {
                    Some(Ok(Binding::Decl(DeclRef::Rule(r)))) => rules.push(r),
                    Some(Err(d)) => return Err(d),
                    _ => return Err(IrError::error("E-NAME-KIND", it.span, "not a rule")),
                }
            }
        }
        self.prog.apps[a.index()].rules = rules;
        Ok(())
    }

    fn shaped_app(&mut self, a: AppId, m: ModuleId, decl: &ast::Decl, shape: &crate::library::AppShape) -> R<()> {
        let mut cx = Cx::new(m, Body::Value);
        let names: Vec<&str> = shape.params.iter().map(|p| p.name.as_str()).collect();
        let mut found: Vec<Option<AppField>> = vec![None; names.len()];
        for p in decl.params.iter().flatten() {
            let Some(i) = names.iter().position(|n| *n == p.name.name) else {
                return Err(IrError::error(
                    "E-TYPE-ARITY",
                    p.span,
                    format!("a {} app has no parameter `{}`", shape.library, p.name.name),
                )
                .with_expected(names.join(", "), p.name.name.clone()));
            };
            if found[i].is_some() {
                return Err(IrError::error("E-TYPE-ARITY", p.span, format!("`{}` is given twice", p.name.name)));
            }
            let kind = shape.params[i].kind.as_str();
            let bad = || {
                IrError::error("E-NAME-KIND", p.span, format!("`{}` names a {kind}", p.name.name))
                    .with_expected(kind.to_string(), "something else")
            };
            let Some(value) = &p.ty else { return Err(bad()) };
            let decl_ref = match self.binding_of(&cx, value) {
                Some(Ok(Binding::Decl(d @ DeclRef::Rule(_)))) if kind == "rule" => d,
                Some(Ok(Binding::Decl(d @ DeclRef::Fn(_)))) if kind == "fn" => d,
                Some(Err(e)) => return Err(e),
                // A name that resolves to nothing says so, with its suggestion.
                None => match self.lower_expr(&mut cx, value) {
                    Err(e) if e.code == "E-NAME-UNKNOWN" => return Err(e),
                    _ => return Err(bad().with_expected(kind.to_string(), self.prog.snippet(value.span).to_string())),
                },
                _ => return Err(bad().with_expected(kind.to_string(), self.prog.snippet(value.span).to_string())),
            };
            found[i] = Some(AppField { name: p.name.name.clone(), decl: decl_ref, span: p.span });
        }
        let missing: Vec<&str> = names.iter().zip(&found).filter(|(_, f)| f.is_none()).map(|(n, _)| *n).collect();
        if !missing.is_empty() {
            return Err(IrError::error(
                "E-TYPE-ARITY",
                decl.name.span,
                format!("a {} app needs {}", shape.library, missing.join(", ")),
            )
            .with_expected(names.join(", "), format!("no {}", missing.join(", "))));
        }
        let app = &mut self.prog.apps[a.index()];
        app.fields = found.into_iter().flatten().collect();
        app.shape = Some(shape.library.clone());
        Ok(())
    }

    /// Pass 6 for an app: `@target`, then each library note, checked against its declaration.
    pub(super) fn app_notes(&mut self, a: AppId, decl: &ast::Decl) -> R<()> {
        let m = self.prog.app(a).info.module;
        let mut notes: Vec<NoteUse> = Vec::new();
        for n in &decl.notes {
            let args = n.args.as_deref().unwrap_or(&[]);
            let name = n.path.dotted();
            if name == "target" {
                match args {
                    [x] if x.value.as_name().is_some() => {
                        self.prog.apps[a.index()].target = x.value.as_name().map(str::to_string);
                    }
                    _ => return Err(IrError::error("E-NOTE-ARG", n.span, "`@target(name)` takes one backend name")),
                }
            } else if let Some(sig) = self.library_note(m, &name, n.span)? {
                notes.push(self.note_use(&sig, args, n.span)?);
            }
        }
        self.prog.apps[a.index()].notes = notes;
        Ok(())
    }

    /// Pass 7: a field whose shape parameter carries `@on(t)` names a rule with trigger `t`.
    pub(super) fn check_app_triggers(&mut self) {
        let mut errors = Vec::new();
        for app in &self.prog.apps {
            let Some(shape) = app.shape.as_deref().and_then(|l| self.prog.library(l)).and_then(|l| l.app.as_ref())
            else {
                continue;
            };
            for p in &shape.params {
                let (Some(t), Some(DeclRef::Rule(r))) = (&p.trigger, app.field(&p.name)) else { continue };
                let rule = self.prog.rule(r);
                if rule.triggers.iter().any(|x| x.name == *t && x.library == shape.library) {
                    continue;
                }
                errors.push(
                    IrError::error(
                        "E-APP-TRIGGER",
                        app.info.name_span,
                        format!(
                            "`{}: {}` names a rule without `@on({t}(..))`",
                            p.name,
                            self.prog.qualified(&rule.info)
                        ),
                    )
                    .in_decl(&self.prog.qualified(&app.info))
                    .with_at(p.name.clone())
                    .with_expected(format!("@on({t}(..))"), "no trigger"),
                );
            }
        }
        self.errors.extend(errors);
    }
}
