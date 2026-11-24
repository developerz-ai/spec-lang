//! Product declarations as Rust items: types, fns, the world, refusals,
//! commands and rules. Each function returns token trees the printer
//! validates by parsing them as `syn` items.

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use spec_lang_ir::{FnId, RuleId, TypeDef, TypeId};

use super::layout::{Item, camel, ident};
use super::lower::{Cat, Ctx, EntryBind, R, entry_binding, static_text_fn};
use crate::front::typed::*;
use crate::front::typer::{Typer, alias_base};
use crate::front::{TFn, TRule};
use crate::provenance::Origin;

/// `#[doc = "..."]` lines.
pub fn docs(lines: &[String]) -> TokenStream {
    let lines = lines.iter().map(|l| Literal::string(&format!(" {l}")));
    quote!(#(#[doc = #lines])*)
}

/// The `@why` of a declaration, as doc lines.
fn why(info: &spec_lang_ir::DeclInfo) -> Vec<String> {
    info.why.iter().map(|w| format!("Why: {w}.")).collect()
}

/// The spec text of a declaration's first line, as written.
fn signature(ctx: &Ctx<'_, '_>, info: &spec_lang_ir::DeclInfo) -> String {
    let text = ctx.front.prog.snippet(info.span);
    let line = text.lines().find(|l| !l.trim_start().starts_with('@')).unwrap_or("").trim();
    format!("`{}`", line.trim_end_matches(['{', ' ', '=']).trim_end())
}

pub fn ty(ctx: &mut Ctx<'_, '_>, id: TypeId) -> R<TokenStream> {
    let decl = ctx.front.prog.ty(id);
    let name = ident(&decl.info.name);
    let mut doc = vec![signature(ctx, &decl.info)];
    doc.extend(why(&decl.info));
    match &decl.def {
        TypeDef::Sum(vs) => {
            if vs.iter().any(|v| !v.fields.is_empty()) {
                return Err(ctx.unsupported(decl.info.span, "a sum variant with fields"));
            }
            let doc = docs(&doc);
            let names: Vec<_> = vs.iter().map(|v| ident(&v.name)).collect();
            let texts: Vec<_> = vs.iter().map(|v| Literal::string(&v.name)).collect();
            let n = Literal::usize_unsuffixed(vs.len());
            Ok(quote! {
                #doc
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                pub enum #name { #(#names),* }

                impl #name {
                    /// Every variant, in declaration order.
                    pub const ALL: [Self; #n] = [#(Self::#names),*];

                    /// The variant's name as the spec writes it.
                    pub const fn name(self) -> &'static str {
                        match self { #(Self::#names => #texts),* }
                    }

                    pub fn from_name(name: &str) -> Option<Self> {
                        match name { #(#texts => Some(Self::#names),)* _ => None }
                    }
                }
            })
        }
        TypeDef::Record(fields) => {
            let doc = docs(&doc);
            let t = Typer::new(ctx.front.prog, ctx.front.decl_path(&decl.info));
            let mut out = Vec::new();
            for f in fields {
                let ft = t.ty(&f.ty, f.span)?;
                let fty = ctx.ty(&ft);
                let fname = ident(&f.name);
                out.push(quote!(pub #fname: #fty));
            }
            Ok(quote! {
                #doc
                #[derive(Clone, Debug, PartialEq, Eq)]
                pub struct #name { #(#out),* }
            })
        }
        TypeDef::Alias(_) => refined(ctx, id, doc),
        TypeDef::Rules(_) => Ok(quote!()),
    }
}

/// `type X = Base where claim`: a newtype whose only constructor checks the claim.
fn refined(ctx: &mut Ctx<'_, '_>, id: TypeId, mut doc: Vec<String>) -> R<TokenStream> {
    let decl = ctx.front.prog.ty(id);
    let name = ident(&decl.info.name);
    let Some((base, claim)) = alias_base(ctx.front.prog, id) else {
        return Err(ctx.unsupported(decl.info.span, "an alias of this type"));
    };
    let cond = match claim {
        Some(c) => {
            let mut t = Typer::new(ctx.front.prog, ctx.front.decl_path(&decl.info));
            t.bind("self", base.clone());
            let c = t.expr(&c.expr, Some(&T::Bool))?;
            ctx.reset_scope();
            let cat = if base == T::Text { Cat::Ref(1) } else { Cat::Val };
            ctx.bind("self", cat, None, usize::MAX);
            let v = ctx.expr(&c)?;
            let v = ctx.val(v);
            // `self` is a keyword in Rust: the claim's value is `value`.
            rename_self(v)
        }
        None => quote!(true),
    };
    let display = quote! {
        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
    match base {
        T::Int => {
            doc.push("Held in one `i64`: the store carries at most 2^53, and an argument beyond the word".into());
            doc.push("saturates to `i64::MAX`, which no world holds (a rule only looks such a number up).".into());
            let doc = docs(&doc);
            Ok(quote! {
                #doc
                #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
                pub struct #name(i64);

                impl #name {
                    /// The claim is checked here and nowhere else; `None` is a value outside it.
                    pub const fn new(value: i64) -> Option<Self> {
                        if #cond { Some(Self(value)) } else { None }
                    }

                    /// A value whose claim the compiler showed at build time; each call site
                    /// names the fact in `provenance.json`.
                    pub const fn from_proven(value: i64) -> Self {
                        Self(value)
                    }

                    pub const fn get(self) -> i64 {
                        self.0
                    }
                }

                #display
            })
        }
        T::Text if ctx.layout.on("text.inline") && ctx.front.facts.inline_text.contains_key(&id) => {
            let (n, licence) = ctx.front.facts.inline_text[&id].clone();
            doc.push(format!("Held inline, no heap: {licence}."));
            let doc = docs(&doc);
            let cap = Literal::usize_unsuffixed(n);
            Ok(quote! {
                #doc
                #[derive(Clone, Debug, PartialEq, Eq, Hash)]
                pub struct #name {
                    len: u8,
                    bytes: [u8; #cap],
                }

                impl #name {
                    /// The claim is checked here and nowhere else; `None` is a value outside it.
                    pub fn new(text: impl AsRef<str>) -> Option<Self> {
                        let value = text.as_ref();
                        if !Self::admits(value) {
                            return None;
                        }
                        let mut bytes = [0; #cap];
                        bytes.get_mut(..value.len())?.copy_from_slice(value.as_bytes());
                        Some(Self { len: u8::try_from(value.len()).ok()?, bytes })
                    }

                    /// Whether the text meets the claim, without building a value.
                    pub fn admits(value: &str) -> bool {
                        #cond
                    }

                    pub fn as_str(&self) -> &str {
                        // Only `new` fills the bytes, with text that met the claim: ASCII, never cut.
                        self.bytes.get(..usize::from(self.len)).and_then(|b| std::str::from_utf8(b).ok()).unwrap_or_default()
                    }
                }

                /// Ordered as its text is: by code point.
                impl Ord for #name {
                    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                        self.as_str().cmp(other.as_str())
                    }
                }

                impl PartialOrd for #name {
                    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                        Some(self.cmp(other))
                    }
                }

                impl std::fmt::Display for #name {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str(self.as_str())
                    }
                }
            })
        }
        T::Text => {
            let doc = docs(&doc);
            Ok(quote! {
                #doc
                #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
                pub struct #name(Box<str>);

                impl #name {
                    /// The claim is checked here and nowhere else; `None` is a value outside it.
                    pub fn new(text: impl Into<Box<str>> + AsRef<str>) -> Option<Self> {
                        if Self::admits(text.as_ref()) { Some(Self(text.into())) } else { None }
                    }

                    /// Whether the text meets the claim, without building a value.
                    pub fn admits(value: &str) -> bool {
                        #cond
                    }

                    pub fn as_str(&self) -> &str {
                        &self.0
                    }
                }

                #display
            })
        }
        _ => Err(ctx.unsupported(decl.info.span, "a refined type whose base is not Int or Text")),
    }
}

/// Replaces the identifier `self` (a claim's value) with `value`.
fn rename_self(ts: TokenStream) -> TokenStream {
    use proc_macro2::{Group, TokenTree};
    ts.into_iter()
        .map(|t| match t {
            TokenTree::Ident(i) if i == "self" || i == "r#self" => TokenTree::Ident(format_ident!("value")),
            TokenTree::Group(g) => {
                let mut n = Group::new(g.delimiter(), rename_self(g.stream()));
                n.set_span(g.span());
                TokenTree::Group(n)
            }
            t => t,
        })
        .collect()
}

/// Which fns can be `const fn`: bodies of literals, matches, fields and calls of const fns.
pub fn const_fns(ctx: &Ctx<'_, '_>) -> Vec<FnId> {
    let mut out: Vec<FnId> = Vec::new();
    loop {
        let before = out.len();
        for f in &ctx.front.fns {
            if out.contains(&f.id) {
                continue;
            }
            let ret_ok = ctx.copy(&f.result) || static_text_fn(f);
            if ret_ok && f.body.iter().all(|s| matches!(&s.kind, S::Return(Some(v)) if const_expr(ctx, v, &out))) {
                out.push(f.id);
            }
        }
        if out.len() == before {
            return out;
        }
    }
}

fn const_expr(ctx: &Ctx<'_, '_>, e: &TExpr, consts: &[FnId]) -> bool {
    let here = match &e.kind {
        K::Int(_) | K::Bool(_) | K::Text(_) | K::Variant { .. } | K::Local(_) | K::Tuple(_) => true,
        K::Field(x, _) => ctx.copy(&e.ty) && !matches!(x.ty, T::Text),
        K::Widen(x) => matches!(x.ty, T::Named(_)) && e.ty == T::Int,
        K::Match { scrutinee, .. } => ctx.copy(&scrutinee.ty),
        K::CallFn { f, .. } => consts.contains(f),
        K::Bin(op, ..) => !matches!(op, Bin::Implies),
        K::Compare(..) | K::Not(_) => ctx.copy(&T::Bool),
        _ => false,
    };
    let mut ok = here;
    crate::front::facts::children(e, &mut |c| ok &= const_expr(ctx, c, consts));
    ok
}

pub fn func(ctx: &mut Ctx<'_, '_>, f: &TFn, is_const: bool) -> R<TokenStream> {
    let decl = ctx.front.prog.func(f.id);
    ctx.decl = ctx.front.decl_path(&decl.info);
    let name = ident(&decl.info.name);
    let mut doc = vec![signature(ctx, &decl.info)];
    doc.extend(why(&decl.info));
    let doc = docs(&doc);
    ctx.reset_scope();
    ctx.fallible = false;
    let mut params = Vec::new();
    for p in &f.params {
        let pt = ctx.param_ty(&p.ty);
        let cat = ctx.param_cat(&p.ty);
        ctx.bind(&p.name, cat, None, usize::MAX);
        let pn = ident(&p.name);
        params.push(quote!(#pn: #pt));
    }
    let ret = if static_text_fn(f) {
        quote!(-> &'static str)
    } else if f.result == T::Unit {
        quote!()
    } else {
        let r = ctx.ty(&f.result);
        quote!(-> #r)
    };
    let body = ctx.body(&f.body, false)?;
    let constness = if is_const { quote!(const) } else { quote!() };
    Ok(quote! {
        #doc
        pub #constness fn #name(#(#params),*) #ret { #body }
    })
}

/// The module's states, as one value a rule takes and changes.
pub fn world(ctx: &mut Ctx<'_, '_>) -> R<TokenStream> {
    let mut fields = Vec::new();
    let mut inits = Vec::new();
    ctx.reset_scope();
    let states = ctx.front.states.iter().map(|s| (s.id, s.ty.clone(), s.init.clone())).collect::<Vec<_>>();
    for (id, ty, init) in states {
        let decl = ctx.front.prog.state(id);
        let name = ident(&decl.info.name);
        let t = ctx.ty(&ty);
        let doc = docs(&[signature(ctx, &decl.info)]);
        fields.push(quote!(#doc pub #name: #t));
        let v = match init {
            Some(e) => {
                let v = ctx.expr(&e)?;
                ctx.owned(v)?
            }
            None => quote!(Default::default()),
        };
        inits.push(quote!(#name: #v));
    }
    let world = ident("World");
    Ok(quote! {
        /// Every state of the module, as one value: a rule takes it and changes it.
        /// The fields are open so a test can install any start world.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct #world { #(#fields),* }

        impl Default for #world {
            /// The initial world.
            fn default() -> Self {
                Self { #(#inits),* }
            }
        }
    })
}

/// The errors the app's guards declare.
pub fn refusal(ctx: &mut Ctx<'_, '_>) -> TokenStream {
    let names: Vec<_> = ctx.front.errors.iter().map(|e| ident(e)).collect();
    let texts: Vec<_> = ctx.front.errors.iter().map(|e| Literal::string(e)).collect();
    quote! {
        /// The errors the rules' guards declare. A refused rule has changed nothing and sent nothing.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Refusal { #(#names),* }

        impl Refusal {
            /// The error's name as the spec writes it.
            pub const fn name(self) -> &'static str {
                match self { #(Self::#names => #texts),* }
            }
        }

        impl std::fmt::Display for Refusal {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.name())
            }
        }

        impl std::error::Error for Refusal {}
    }
}

/// The events of the app's rules, and the dispatch that runs one.
pub fn command(ctx: &mut Ctx<'_, '_>) -> R<TokenStream> {
    let mut variants = Vec::new();
    let mut writes = Vec::new();
    let mut arms = Vec::new();
    let world = ctx.item(Item::World, "World");
    let out = ctx.runtime("out::Out");
    let refusal = ctx.item(Item::Refusal, "Refusal");
    #[allow(clippy::type_complexity)]
    let rules: Vec<(RuleId, Vec<(String, T)>, bool, bool, bool)> = ctx
        .front
        .rules
        .iter()
        .zip(&ctx.front.facts.rules)
        .map(|(r, f)| {
            (
                r.id,
                r.params.iter().map(|p| (p.name.clone(), p.ty.clone())).collect(),
                !f.read_only(),
                !r.guards.is_empty(),
                r.result != T::Unit,
            )
        })
        .collect();
    for (id, params, write, fallible, replies) in rules {
        let rname = ctx.front.prog.rule(id).info.name.clone();
        let v = format_ident!("{}", camel(&rname));
        let f = ctx.item(Item::Rule(id), &rname);
        let names: Vec<_> = params.iter().map(|(n, _)| ident(n)).collect();
        let tys: Vec<_> = params.iter().map(|(_, t)| ctx.ty(t)).collect();
        let pat = if params.is_empty() { quote!(Self::#v) } else { quote!(Self::#v { .. }) };
        let bind = if params.is_empty() { quote!(Command::#v) } else { quote!(Command::#v { #(#names),* }) };
        if params.is_empty() {
            variants.push(quote!(#v));
        } else {
            variants.push(quote!(#v { #(#names: #tys),* }));
        }
        if !write {
            writes.push(pat);
        }
        let index = ctx.front.rules.iter().position(|x| x.id == id).unwrap_or(0);
        let w = match (view_names(ctx, index), write) {
            (Some((view, _)), _) => {
                let v = ctx.item(Item::Rule(id), &view);
                quote!(&#v::of(world))
            }
            (None, true) => quote!(world),
            (None, false) => quote!(&*world),
        };
        let call = quote!(#f(#w, out #(, #names)*));
        arms.push(match (fallible, replies) {
            (true, true) => quote!(#bind => { #call?; }),
            (true, false) => quote!(#bind => #call?,),
            (false, true) => quote!(#bind => { #call; }),
            (false, false) => quote!(#bind => #call,),
        });
    }
    let writes = if writes.is_empty() { quote!(true) } else { quote!(!matches!(self, #(#writes)|*)) };
    Ok(quote! {
        /// One call of a rule of the app: the rule's name and its parameters, which are the event.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum Command { #(#variants),* }

        impl Command {
            /// Whether the rule writes the world: from its body's assignments, which `@uses` declares.
            pub const fn writes(&self) -> bool {
                #writes
            }
        }

        /// Runs the rule the command names. Replies have no reader on a command line and stop here.
        pub fn apply(world: &mut #world, out: &mut #out, command: Command) -> Result<(), #refusal> {
            match command { #(#arms)* }
            Ok(())
        }
    })
}

/// One rule: guards in order, then the body, as one function over the world.
pub fn rule(ctx: &mut Ctx<'_, '_>, index: usize, r: &TRule) -> R<TokenStream> {
    let decl = ctx.front.prog.rule(r.id);
    let path = ctx.front.decl_path(&decl.info);
    ctx.decl = path.clone();
    let facts = &ctx.front.facts.rules[index];
    let read_only = ctx.layout.read_only(ctx.front, index);
    // A strategy switched off leaves the plain translation; the guard still shows the key present.
    let entry = facts.entry.clone();
    let entry_on =
        entry.as_ref().is_some_and(|e| ctx.layout.on(if e.removes { "entry.remove" } else { "entry.lookup" }));
    ctx.guarded = entry.as_ref().map(|e| (e.state, e.key.clone()));
    let entry = if entry_on { entry } else { None };
    let retain = facts.retain.clone().filter(|_| ctx.layout.on("filter-remove.retain"));
    ctx.writes_world = !read_only;
    let name = ident(&decl.info.name);
    let mut doc = vec![signature(ctx, &decl.info)];
    doc.extend(why(&decl.info));
    if read_only {
        doc.push("Reads the world and never writes it: its body assigns no state (`@uses` agrees).".into());
    }
    let doc = docs(&doc);
    ctx.reset_scope();
    ctx.fallible = !r.guards.is_empty();
    ctx.retain = retain.clone();
    let extra: Vec<&TExpr> = r.guards.iter().map(|g| &g.cond).collect();
    ctx.count_uses(&r.body, &extra);
    let world_t = ctx.item(Item::World, "World");
    let out_t = ctx.runtime("out::Out");
    let uses_out = mentions_out(&r.body);
    let view = projected_view(ctx, index, r)?;
    let world_p = match (&view, read_only) {
        (Some((_, name)), _) => quote!(world: &#name),
        (None, true) => quote!(world: &#world_t),
        (None, false) => quote!(world: &mut #world_t),
    };
    let out_p = if uses_out { quote!(out: &mut #out_t) } else { quote!(_out: &mut #out_t) };
    let mut params = vec![world_p, out_p];
    for p in &r.params {
        let t = ctx.ty(&p.ty);
        let cat = if ctx.copy(&p.ty) { Cat::Val } else { Cat::Place { movable: true } };
        let uses = count_param(&r.body, &extra, &p.name);
        ctx.bind(&p.name, cat, None, uses);
        let pn = ident(&p.name);
        params.push(quote!(#pn: #t));
    }
    let reply = if r.result == T::Unit { quote!(()) } else { ctx.ty(&r.result) };
    let ret = match (ctx.fallible, r.result == T::Unit) {
        (true, _) => {
            let refusal = ctx.item(Item::Refusal, "Refusal");
            quote!(-> Result<#reply, #refusal>)
        }
        (false, true) => quote!(),
        (false, false) => quote!(-> #reply),
    };
    let mut guards = Vec::new();
    ctx.entry = None;
    for (i, g) in r.guards.iter().enumerate() {
        let role = format!("guard[{i}]");
        let refusal = ctx.item(Item::Refusal, "Refusal");
        let err = ident(&g.error);
        let at = ctx.front.at(g.span);
        if i == 0
            && let Some(e) = &entry
        {
            let state_name = ctx.front.prog.state(e.state).info.name.clone();
            let st = ident(&state_name);
            let k = ident(&e.key);
            let strategy = if e.removes { "entry.remove" } else { "entry.lookup" };
            let licence = if e.removes {
                format!(
                    "guard[0] is `{} in {state_name}` and the body's only use of {state_name} is `{state_name}.remove({})`",
                    e.key, e.key
                )
            } else {
                format!(
                    "guard[0] is `{} in {state_name}` and every later read or write of {state_name} is `{state_name}[{}]`",
                    e.key, e.key
                )
            };
            let origin = Origin::new(&path, &at, &role).strategy(strategy, licence);
            let anchor = ctx.anchor(g.span, &role, origin);
            if e.removes {
                guards.push(quote!(#anchor world.#st.remove(&#k).ok_or(#refusal::#err)?;));
            } else {
                let b = entry_binding(ctx.front, e.state, &r.params);
                let get = if e.mutable { quote!(get_mut) } else { quote!(get) };
                guards.push(quote!(#anchor let #b = world.#st.#get(&#k).ok_or(#refusal::#err)?;));
                ctx.entry = Some(EntryBind { state: e.state, key: e.key.clone(), binding: b });
            }
            continue;
        }
        let origin = Origin::new(&path, &at, &role);
        let anchor = ctx.anchor(g.span, &role, origin);
        let cond = ctx.negate(&g.cond)?;
        guards.push(quote!(#anchor if #cond { return Err(#refusal::#err); }));
    }
    if let Some(rt) = &retain {
        let state_name = ctx.front.prog.state(rt.state).info.name.clone();
        let span = r.body.get(rt.at).map(|s| s.span);
        if let Some(span) = span {
            let at = ctx.front.at(span);
            let origin = Origin::new(&path, &at, "rule body").strategy(
                "filter-remove.retain",
                format!(
                    "{state_name} is keyed by a field of its values (every write is `{state_name}[k] = R(k, ...)`), and `{}` is used after the loop only for its size",
                    rt.name
                ),
            );
            let anchor = ctx.anchor(span, "body: retain", origin);
            guards.push(anchor);
        }
    }
    let body = ctx.body(&r.body, true)?;
    ctx.entry = None;
    ctx.retain = None;
    let view = view.map(|(ts, _)| ts);
    Ok(quote! {
        #view

        #doc
        pub fn #name(#(#params),*) #ret {
            #(#guards)*
            #body
        }
    })
}

/// `StatsView`, `StatsTask` for rule `stats`: the part of the world a projected rule reads.
pub fn view_names(ctx: &Ctx<'_, '_>, index: usize) -> Option<(String, String)> {
    if !ctx.layout.on("store.read.projection") {
        return None;
    }
    let p = ctx.front.facts.rules.get(index)?.projection.as_ref()?;
    let rule = camel(&ctx.front.prog.rule(ctx.front.rules[index].id).info.name);
    let T::Map(_, v) = &ctx.front.states.iter().find(|s| s.id == p.state)?.ty else { return None };
    let T::Named(rec) = **v else { return None };
    Some((format!("{rule}View"), format!("{rule}{}", ctx.front.prog.ty(rec).info.name)))
}

/// The view types of a projected rule, and the view's name.
fn projected_view(ctx: &mut Ctx<'_, '_>, index: usize, r: &TRule) -> R<Option<(TokenStream, TokenStream)>> {
    let Some(p) = ctx.front.facts.rules[index].projection.clone() else { return Ok(None) };
    let Some((view, value)) = view_names(ctx, index) else { return Ok(None) };
    let (view, value) = (ident(&view), ident(&value));
    let world_t = ctx.item(Item::World, "World");
    let rname = ctx.front.prog.rule(r.id).info.name.clone();
    let mut fields = Vec::new();
    let mut copies = Vec::new();
    let states: Vec<(spec_lang_ir::StateId, T)> = ctx.front.states.iter().map(|s| (s.id, s.ty.clone())).collect();
    let mut value_fields = Vec::new();
    let mut value_copies = Vec::new();
    for (id, ty) in states {
        let n = ident(&ctx.front.prog.state(id).info.name);
        if id == p.state {
            let T::Map(k, v) = &ty else { continue };
            let kt = ctx.ty(k);
            let m = ctx.runtime("lang::Entries");
            fields.push(quote!(pub #n: #m<#kt, #value>));
            let T::Named(rec) = **v else { continue };
            let all = crate::front::record_fields(ctx.front.prog, rec).unwrap_or(&[]);
            let t = Typer::new(ctx.front.prog, String::new());
            for i in &p.fields {
                let Some(f) = all.get(*i) else { continue };
                let ft = t.ty(&f.ty, f.span)?;
                let ftt = ctx.ty(&ft);
                let fname = ident(&f.name);
                value_fields.push(quote!(pub #fname: #ftt));
                value_copies.push(quote!(#fname: value.#fname));
            }
            let entries = ctx.runtime("lang::Entries");
            copies.push(quote!(#n: #entries::of(&world.#n, |value| #value { #(#value_copies),* })));
        } else {
            let t = ctx.ty(&ty);
            fields.push(quote!(pub #n: #t));
            let c = if ctx.copy(&ty) { quote!(world.#n) } else { quote!(world.#n.clone()) };
            copies.push(quote!(#n: #c));
        }
    }
    let doc = docs(&[
        format!("The part of the world rule `{rname}` reads. The store fills it without building the rest:"),
        format!("{}.", p.licence),
    ]);
    let vdoc = docs(&[format!("The fields of a value rule `{rname}` reads.")]);
    Ok(Some((
        quote! {
            #doc
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct #view { #(#fields),* }

            #vdoc
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub struct #value { #(#value_fields),* }

            impl #view {
                /// The view of a whole world, for a caller that has one (tests, `apply`).
                pub fn of(world: &#world_t) -> Self {
                    Self { #(#copies),* }
                }
            }

            impl Default for #view {
                /// The view of the initial world.
                fn default() -> Self {
                    Self::of(&#world_t::default())
                }
            }
        },
        quote!(#view),
    )))
}

fn mentions_out(body: &[TStmt]) -> bool {
    let mut hit = false;
    for s in body {
        crate::front::facts::stmt_exprs(s, &mut |e| hit |= has_out(e));
    }
    hit
}

fn has_out(e: &TExpr) -> bool {
    if matches!(e.kind, K::OutLine(_)) {
        return true;
    }
    let mut hit = false;
    crate::front::facts::children(e, &mut |c| hit |= has_out(c));
    hit
}

fn count_param(body: &[TStmt], extra: &[&TExpr], name: &str) -> usize {
    fn count(e: &TExpr, name: &str, n: &mut usize) {
        if matches!(&e.kind, K::Local(x) if x == name) {
            *n += 1;
        }
        crate::front::facts::children(e, &mut |c| count(c, name, n));
    }
    let mut n = 0;
    for s in body {
        crate::front::facts::stmt_exprs(s, &mut |e| count(e, name, &mut n));
    }
    for e in extra {
        count(e, name, &mut n);
    }
    n
}
