//! Typed expressions and statements to Rust token trees.
//!
//! Every lowered expression carries a value category ([`Cat`]): an owned
//! value, a reference of some depth, a place (an lvalue that may be moved
//! from when it is a local's last use), an `Option<&T>`, or an iterator. The
//! category decides each borrow, move, deref and clone, so the output reads
//! like hand-written Rust instead of cloning by default.

use std::collections::{BTreeSet, HashMap};

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use spec_lang_diag::Diag;
use spec_lang_ir::{CmpOp, StateId, TypeDef};
use spec_lang_syntax::Span;

use super::layout::{Item, Layout, camel, ident, is_copy, snake};
use crate::front::Front;
use crate::front::facts::Retain;
use crate::front::typed::*;
use crate::provenance::Origin;

/// How a lowered expression holds its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cat {
    /// An owned value or a `Copy` value.
    Val,
    /// An expression of type `&...&T` with this many `&`.
    Ref(u8),
    /// An lvalue of type `T`; `movable` when it is the last use of an owned local.
    Place { movable: bool },
    /// An `Option<&T>`.
    OptRef,
    /// An iterator whose items have this category ([`Cat::Val`] or [`Cat::Ref`]).
    Iter(Item1),
    /// A local that holds the count of a list nothing else reads (the `retain` strategy).
    Count,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item1 {
    Val,
    Ref(u8),
}

impl Item1 {
    fn cat(self) -> Cat {
        match self {
            Item1::Val => Cat::Val,
            Item1::Ref(d) => Cat::Ref(d),
        }
    }

    fn deeper(self) -> Cat {
        match self {
            Item1::Val => Cat::Ref(1),
            Item1::Ref(d) => Cat::Ref(d + 1),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ex {
    pub ts: TokenStream,
    pub cat: Cat,
    pub ty: T,
}

#[derive(Clone, Debug)]
struct Local {
    cat: Cat,
    /// For a local list: the category of its elements.
    elem: Option<Item1>,
    /// Uses of the local left, counting this one.
    uses: usize,
}

/// Product code refuses what tests may do (a claim checked when it runs, a map read that may panic).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Product,
    Test,
}

/// The entry strategy in force: `m[k]` is `binding`.
#[derive(Clone, Debug)]
pub struct EntryBind {
    pub state: StateId,
    pub key: String,
    pub binding: proc_macro2::Ident,
}

#[allow(missing_debug_implementations)]
pub struct Ctx<'a, 'p> {
    pub front: &'a Front<'p>,
    pub layout: &'a Layout,
    /// Module and file being written.
    pub here: (String, String),
    /// `crate` inside the library, the package name in integration tests.
    pub root: String,
    pub uses: BTreeSet<String>,
    locals: Vec<HashMap<String, Local>>,
    pub mode: Mode,
    pub entry: Option<EntryBind>,
    pub retain: Option<Retain>,
    /// The function returns `Result`: wrap `return` values in `Ok`.
    pub fallible: bool,
    /// Loops and closures entered: a local used inside one is never moved.
    depth: usize,
    /// Anchors placed in this item: `__slg!(n)` becomes `// slg: ...`.
    pub anchors: Vec<(String, Origin)>,
    /// Event parameters read by laws: `name` -> type.
    pub event_args: BTreeSet<String>,
    pub decl: String,
    /// Uses of each local in the body being lowered ([`Ctx::count_uses`]).
    uses_after: HashMap<String, usize>,
    /// Guard 0 shows `key in state`: `state[key]` cannot be `Missing` (any strategy).
    pub guarded: Option<(StateId, String)>,
    /// The rule writes the world: a list read from it is copied before a write.
    pub writes_world: bool,
    /// A parameter of an inlined fn -> the caller's local it stands for.
    aliases: HashMap<String, String>,
    /// Lambda parameters that range over a map's keys, with that map.
    keys_of: Vec<(String, TExpr)>,
    /// The same, walking entries: `m[k]` is the local `value`.
    entries_of: Vec<(String, TExpr)>,
}

pub type R<X> = Result<X, Diag>;

impl<'a, 'p> Ctx<'a, 'p> {
    pub fn new(front: &'a Front<'p>, layout: &'a Layout, here: (String, String), root: &str, mode: Mode) -> Self {
        Ctx {
            front,
            layout,
            here,
            root: root.to_string(),
            uses: BTreeSet::new(),
            locals: vec![HashMap::new()],
            mode,
            entry: None,
            retain: None,
            fallible: false,
            depth: 0,
            anchors: Vec::new(),
            event_args: BTreeSet::new(),
            decl: String::new(),
            uses_after: HashMap::new(),
            guarded: None,
            writes_world: false,
            aliases: HashMap::new(),
            keys_of: Vec::new(),
            entries_of: Vec::new(),
        }
    }

    pub fn unsupported(&self, span: Span, what: impl std::fmt::Display) -> Diag {
        Diag::unsupported(span, what).with_decl(self.decl.clone())
    }

    /// A readable anchor: `// slg: <file>:<line> <role>`, placed as a marker statement.
    pub fn anchor(&mut self, span: Span, role: &str, origin: Origin) -> TokenStream {
        let l = self.front.prog.location(span);
        self.anchors.push((format!("// slg: {}:{} {role}", l.path, l.line), origin));
        let n = Literal::usize_unsuffixed(self.anchors.len() - 1);
        quote!(__slg!(#n);)
    }

    // ---- scope ----

    pub fn reset_scope(&mut self) {
        self.locals = vec![HashMap::new()];
        self.depth = 0;
    }

    pub fn bind(&mut self, name: &str, cat: Cat, elem: Option<Item1>, uses: usize) {
        if let Some(s) = self.locals.last_mut() {
            s.insert(name.to_string(), Local { cat, elem, uses });
        }
    }

    fn push(&mut self) {
        self.locals.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.locals.pop();
    }

    fn local_mut(&mut self, name: &str) -> Option<&mut Local> {
        self.locals.iter_mut().rev().find_map(|s| s.get_mut(name))
    }

    // ---- names ----

    /// A declaration's name, imported into the file being written.
    pub fn item(&mut self, item: Item, name: &str) -> TokenStream {
        let id = ident(name);
        let Some((module, file)) = self.layout.place.get(&item).cloned() else {
            return quote!(#id);
        };
        if self.root == "crate" && module == self.here.0 {
            if file != self.here.1 && self.layout.is_dir(&module) {
                self.uses.insert(format!("super::{name}"));
            }
        } else {
            self.uses.insert(format!("{}::{module}::{name}", self.root));
        }
        quote!(#id)
    }

    pub fn ty_name(&mut self, id: spec_lang_ir::TypeId) -> TokenStream {
        let name = self.front.prog.ty(id).info.name.clone();
        self.item(Item::Type(id), &name)
    }

    /// A path of the runtime (`lang`, `out`, `store`, `cli`): imported from the crate root.
    pub fn runtime(&mut self, path: &str) -> TokenStream {
        let last = path.rsplit("::").next().unwrap_or(path);
        self.uses.insert(format!("{}::{path}", self.root));
        let id = ident(last);
        quote!(#id)
    }

    pub fn std(&mut self, path: &str) -> TokenStream {
        let last = path.rsplit("::").next().unwrap_or(path);
        self.uses.insert(format!("std::{path}"));
        let id = ident(last);
        quote!(#id)
    }

    // ---- types ----

    /// The owned Rust type.
    pub fn ty(&mut self, t: &T) -> TokenStream {
        match t {
            T::Unit => quote!(()),
            T::Bool => quote!(bool),
            T::Int => quote!(i64),
            T::Byte => quote!(u8),
            T::Text => quote!(String),
            T::Named(id) => self.ty_name(*id),
            T::Option(x) => {
                let x = self.ty(x);
                quote!(Option<#x>)
            }
            T::List(x) => {
                let x = self.ty(x);
                quote!(Vec<#x>)
            }
            T::Set(x) => {
                let s = self.std("collections::BTreeSet");
                let x = self.ty(x);
                quote!(#s<#x>)
            }
            T::Map(k, v) => {
                let m = self.std("collections::BTreeMap");
                let k = self.ty(k);
                let v = self.ty(v);
                quote!(#m<#k, #v>)
            }
            T::Tuple(ts) => {
                let ts: Vec<TokenStream> = ts.iter().map(|x| self.ty(x)).collect();
                quote!((#(#ts),*))
            }
            T::World => self.item(Item::World, "World"),
            T::Reply | T::Event(_) | T::Step(_) | T::Trace(_) => quote!(()),
        }
    }

    /// The type of a fn parameter: `Copy` by value, text as `&str`, the rest borrowed.
    pub fn param_ty(&mut self, t: &T) -> TokenStream {
        if is_copy(self.front, t) {
            return self.ty(t);
        }
        match t {
            T::Text => quote!(&str),
            T::Option(x) => {
                let x = self.ty(x);
                quote!(Option<&#x>)
            }
            _ => {
                let x = self.ty(t);
                quote!(&#x)
            }
        }
    }

    /// The category a fn parameter has inside the body.
    pub fn param_cat(&self, t: &T) -> Cat {
        if is_copy(self.front, t) {
            Cat::Val
        } else if matches!(t, T::Option(_)) {
            Cat::OptRef
        } else {
            Cat::Ref(1)
        }
    }

    pub fn copy(&self, t: &T) -> bool {
        is_copy(self.front, t)
    }

    // ---- category conversions ----

    /// A `Copy` value.
    pub fn val(&self, e: Ex) -> TokenStream {
        let ts = e.ts;
        match e.cat {
            Cat::Ref(d) => {
                let stars = (0..d).map(|_| quote!(*));
                quote!(#(#stars)* #ts)
            }
            _ => ts,
        }
    }

    /// A `&T`.
    pub fn refr(&self, e: Ex) -> TokenStream {
        let ts = e.ts;
        match e.cat {
            Cat::Ref(1) | Cat::OptRef => ts,
            Cat::Ref(d) => {
                let stars = (1..d).map(|_| quote!(*));
                quote!(#(#stars)* #ts)
            }
            _ => quote!(&#ts),
        }
    }

    /// An owned value, moved when it can be, cloned when it must be.
    pub fn owned(&mut self, e: Ex) -> R<TokenStream> {
        if self.copy(&e.ty) {
            return Ok(self.val(e));
        }
        let text = e.ty == T::Text;
        Ok(match e.cat {
            Cat::Val | Cat::Place { movable: true } => e.ts,
            Cat::Place { movable: false } => {
                let ts = e.ts;
                if text { quote!(#ts.to_owned()) } else { quote!(#ts.clone()) }
            }
            Cat::Ref(d) => {
                let ts = e.ts;
                let stars = (1..d).map(|_| quote!(*));
                if text { quote!((#(#stars)* #ts).to_owned()) } else { quote!((#(#stars)* #ts).clone()) }
            }
            Cat::OptRef => {
                let ts = e.ts;
                quote!(#ts.cloned())
            }
            Cat::Iter(item) => {
                let ts = e.ts;
                match item {
                    Item1::Val => quote!(#ts.collect()),
                    Item1::Ref(_) => quote!(#ts.cloned().collect()),
                }
            }
            Cat::Count => e.ts,
        })
    }

    /// An argument for a parameter of type `t` of a generated fn.
    fn arg(&mut self, e: Ex, t: &T) -> R<TokenStream> {
        if self.copy(t) {
            return Ok(self.val(e));
        }
        if let T::Option(_) = t {
            return Ok(match e.cat {
                Cat::OptRef => e.ts,
                Cat::Ref(d) if d > 1 => {
                    let ts = self.refr(e);
                    quote!(#ts.as_ref())
                }
                _ => {
                    let ts = e.ts;
                    quote!(#ts.as_ref())
                }
            });
        }
        // A deeper reference coerces to `&T` at the call: no explicit deref.
        Ok(match e.cat {
            Cat::Ref(_) => e.ts,
            _ => self.refr(e),
        })
    }

    /// An iterator over a collection's elements.
    fn iter(&mut self, e: Ex, span: Span) -> R<(TokenStream, Item1)> {
        match e.cat {
            Cat::Iter(item) => Ok((e.ts, item)),
            Cat::Ref(d) if d > 1 => {
                let r = self.refr(e);
                Ok((quote!(#r.iter()), Item1::Ref(1)))
            }
            Cat::Ref(_) | Cat::Place { .. } | Cat::Val => {
                let ts = e.ts;
                Ok((quote!(#ts.iter()), Item1::Ref(1)))
            }
            _ => Err(self.unsupported(span, "iterating this value")),
        }
    }

    // ---- expressions ----

    pub fn expr(&mut self, e: &TExpr) -> R<Ex> {
        let ty = e.ty.clone();
        let ex = |ts: TokenStream, cat: Cat| Ok(Ex { ts, cat, ty: ty.clone() });
        match &e.kind {
            K::Unit => ex(quote!(()), Cat::Val),
            K::Bool(b) => ex(if *b { quote!(true) } else { quote!(false) }, Cat::Val),
            K::Int(n) => {
                let lit = Literal::i128_unsuffixed(*n);
                ex(quote!(#lit), Cat::Val)
            }
            K::Text(s) => {
                let lit = Literal::string(s);
                ex(quote!(#lit), Cat::Ref(1))
            }
            K::Interp(parts) => {
                let (fmt, args) = self.format(parts)?;
                ex(quote!(format!(#fmt #(, #args)*)), Cat::Val)
            }
            K::Local(name) => self.local(name, e),
            K::Const(_) => Err(self.unsupported(e.span, "a const")),
            K::State(s) => {
                let name = ident(&self.front.prog.state(*s).info.name);
                ex(quote!(world.#name), Cat::Place { movable: false })
            }
            K::Variant { ty: t, tag } => {
                if *t == self.front.prog.option_type {
                    return ex(quote!(None), Cat::Val);
                }
                let tn = self.ty_name(*t);
                let v = variant_name(self.front, *t, *tag);
                ex(quote!(#tn::#v), Cat::Val)
            }
            K::Some(x) => {
                let x = self.expr(x)?;
                let x = self.owned(x)?;
                ex(quote!(Some(#x)), Cat::Val)
            }
            K::Record { ty: t, fields } => {
                let tn = self.ty_name(*t);
                let decl = crate::front::record_fields(self.front.prog, *t).unwrap_or(&[]);
                let mut inits = Vec::new();
                for (f, v) in decl.iter().zip(fields) {
                    let fname = ident(&f.name);
                    let shorthand = matches!(&v.kind, K::Local(n) if n == &f.name);
                    let v = self.expr(v)?;
                    let v = self.owned(v)?;
                    if shorthand && fname == v.to_string() {
                        inits.push(quote!(#fname));
                    } else {
                        inits.push(quote!(#fname: #v));
                    }
                }
                ex(quote!(#tn { #(#inits),* }), Cat::Val)
            }
            K::CallFn { f, args } => {
                let decl = self.front.prog.func(*f);
                let name = decl.info.name.clone();
                let fid = self.item(Item::Fn(*f), &name);
                let tf = self.front.func(*f).ok_or_else(|| self.unsupported(e.span, "a fn the front did not type"))?;
                let ptys: Vec<T> = tf.params.iter().map(|p| p.ty.clone()).collect();
                let static_text = static_text_fn(tf);
                let mut out = Vec::new();
                for (a, t) in args.iter().zip(&ptys) {
                    let a = self.expr(a)?;
                    out.push(self.arg(a, t)?);
                }
                let cat = if static_text { Cat::Ref(1) } else { Cat::Val };
                ex(quote!(#fid(#(#out),*)), cat)
            }
            K::CallRule { rule, args } => self.call_rule(*rule, args, e.span),
            K::OutLine(x) => {
                // `out.line(f(t))` with `f` one interpolation: format into the sink, no string between.
                if let Some((parts, aliases)) = self.inline_format(x) {
                    let saved = std::mem::replace(&mut self.aliases, aliases);
                    let r = self.format(&parts);
                    self.aliases = saved;
                    let (fmt, args) = r?;
                    return ex(quote!(out.line(format_args!(#fmt #(, #args)*))), Cat::Val);
                }
                let arg = match &x.kind {
                    K::Interp(parts) => {
                        let (fmt, args) = self.format(parts)?;
                        if args.is_empty() && !fmt.to_string().contains('{') {
                            quote!(#fmt)
                        } else {
                            quote!(format_args!(#fmt #(, #args)*))
                        }
                    }
                    _ => {
                        let x = self.expr(x)?;
                        match x.cat {
                            Cat::Ref(d) if d > 1 => self.refr(x),
                            _ => x.ts,
                        }
                    }
                };
                ex(quote!(out.line(#arg)), Cat::Val)
            }
            K::OutText => ex(quote!(out.text()), Cat::Ref(1)),
            K::Field(x, i) => {
                let T::Named(rec) = &x.ty else {
                    return Err(self.unsupported(e.span, "a field of a value that is not a record"));
                };
                let fname = crate::front::record_fields(self.front.prog, *rec)
                    .and_then(|fs| fs.get(*i))
                    .map(|f| ident(&f.name))
                    .ok_or_else(|| self.unsupported(e.span, "a field out of range"))?;
                let x = self.expr(x)?;
                let base = match x.cat {
                    Cat::Val => {
                        let ts = x.ts;
                        quote!((#ts))
                    }
                    _ => x.ts,
                };
                ex(quote!(#base.#fname), Cat::Place { movable: false })
            }
            K::Core(op, ops) => self.core(*op, ops, e),
            K::Lambda { .. } => Err(self.unsupported(e.span, "a lambda outside a collection method")),
            K::Index { map, key } => {
                if let K::Local(k) = &key.kind
                    && self.entries_of.iter().any(|(p, m)| p == k && same_map(m, map))
                {
                    return ex(quote!(value), Cat::Ref(1));
                }
                if let (Some(entry), K::State(s), K::Local(k)) = (&self.entry, &map.kind, &key.kind)
                    && entry.state == *s
                    && &entry.key == k
                {
                    let b = entry.binding.clone();
                    return ex(quote!(#b), Cat::Ref(1));
                }
                let shown = matches!((&self.guarded, &map.kind, &key.kind), (Some((s, k)), K::State(s2), K::Local(k2)) if s == s2 && k == k2)
                    || matches!(&key.kind, K::Local(k) if self.keys_of.iter().any(|(p, m)| p == k && same_map(m, map)));
                if self.mode == Mode::Product && !shown {
                    return Err(Diag::error(
                        "E-UNSUPPORTED",
                        e.span,
                        "a map read whose key no guard shows present: no strategy keeps `Missing`",
                    )
                    .with_decl(self.decl.clone())
                    .with_note("add @guard(k in m else Error) before the read"));
                }
                let m = self.expr(map)?;
                let k = self.expr(key)?;
                let k = self.refr(k);
                let m = m.ts;
                ex(quote!(#m[#k]), Cat::Place { movable: false })
            }
            K::Not(x) => {
                let ts = self.negate(x)?;
                ex(ts, Cat::Val)
            }
            K::Neg(x) => {
                let x = self.expr(x)?;
                let x = self.val(x);
                ex(quote!(-#x), Cat::Val)
            }
            K::Bin(Bin::Implies, a, b) => {
                // `a implies b` is `!a || b`, with `!a` flipped where it is a comparison.
                let na = self.negate(a)?;
                let na = if strength(a) <= 2 && !matches!(a.kind, K::Compare(_, ref l) if l.len() == 1) {
                    quote!(#na)
                } else {
                    na
                };
                let b2 = self.expr(b)?;
                let b2 = self.val(b2);
                let b2 = paren_if_looser(b, Bin::Or, b2);
                ex(quote!(#na || #b2), Cat::Val)
            }
            K::Bin(Bin::And, ..) if fusable_contains(e) => {
                // `not t.contains("a") and not t.contains("b")`, each one char: one scan for any of them.
                let mut conj = Vec::new();
                flatten_and(e, &mut conj);
                let mut parts = Vec::new();
                let mut chars: Vec<(String, char)> = Vec::new();
                for c in &conj {
                    // `fusable_contains` showed every such conjunct is on the same local.
                    if let Some((recv, ch)) = one_char_absent(c) {
                        if chars.is_empty() {
                            parts.push(None);
                        }
                        chars.push((recv, ch));
                        continue;
                    }
                    let v = self.expr(c)?;
                    let v = self.val(v);
                    parts.push(Some(paren_if_looser(c, Bin::And, v)));
                }
                let recv = ident(&chars.first().map(|(r, _)| r.clone()).unwrap_or_default());
                let lits = chars.iter().map(|(_, c)| Literal::character(*c));
                let any = quote!(!#recv.contains([#(#lits),*]));
                let parts = parts.into_iter().map(|p| p.unwrap_or_else(|| any.clone()));
                ex(quote!(#(#parts)&&*), Cat::Val)
            }
            K::Bin(op, a, b) => {
                let a2 = self.expr(a)?;
                let b2 = self.expr(b)?;
                let (a2, b2) = (self.val(a2), self.val(b2));
                let a2 = paren_if_looser(a, *op, a2);
                let b2 = paren_if_looser(b, *op, b2);
                ex(
                    match op {
                        Bin::Add => quote!(#a2 + #b2),
                        Bin::Sub => quote!(#a2 - #b2),
                        Bin::Mul => quote!(#a2 * #b2),
                        Bin::And => quote!(#a2 && #b2),
                        Bin::Or => quote!(#a2 || #b2),
                        Bin::Implies => quote!(!#a2 || #b2),
                    },
                    Cat::Val,
                )
            }
            K::Compare(head, links) => {
                let ts = self.compare(head, links, false)?;
                ex(ts, Cat::Val)
            }
            K::Tuple(xs) => {
                let mut out = Vec::new();
                for x in xs {
                    let x = self.expr(x)?;
                    out.push(self.owned(x)?);
                }
                ex(quote!((#(#out),*)), Cat::Val)
            }
            K::List(xs) => {
                let mut out = Vec::new();
                for x in xs {
                    let x = self.expr(x)?;
                    out.push(self.owned(x)?);
                }
                ex(quote!(vec![#(#out),*]), Cat::Val)
            }
            K::SetOf(xs) => {
                let s = self.std("collections::BTreeSet");
                let mut out = Vec::new();
                for x in xs {
                    let x = self.expr(x)?;
                    out.push(self.owned(x)?);
                }
                if out.is_empty() {
                    ex(quote!(#s::new()), Cat::Val)
                } else {
                    ex(quote!(#s::from([#(#out),*])), Cat::Val)
                }
            }
            K::MapOf(pairs) => {
                let m = self.std("collections::BTreeMap");
                let mut out = Vec::new();
                for (k, v) in pairs {
                    let k = self.expr(k)?;
                    let k = self.owned(k)?;
                    let v = self.expr(v)?;
                    let v = self.owned(v)?;
                    out.push(quote!((#k, #v)));
                }
                if out.is_empty() {
                    ex(quote!(#m::new()), Cat::Val)
                } else {
                    ex(quote!(#m::from([#(#out),*])), Cat::Val)
                }
            }
            K::If { cond, then, els } => {
                let c = self.expr(cond)?;
                let c = self.val(c);
                let a = self.expr(then)?;
                let b = self.expr(els)?;
                let cat = a.cat;
                let (a, b) = (a.ts, b.ts);
                ex(quote!(if #c { #a } else { #b }), cat)
            }
            K::Match { scrutinee, arms } => self.match_expr(scrutinee, arms, e),
            K::Widen(x) => {
                let inner_ty = x.ty.clone();
                let x = self.expr(x)?;
                match (&inner_ty, &e.ty) {
                    (T::Byte, _) => {
                        let v = self.val(x);
                        ex(quote!(i64::from(#v)), Cat::Val)
                    }
                    (_, T::Int) => {
                        let ts = recv_ts(x);
                        ex(quote!(#ts.get()), Cat::Val)
                    }
                    (_, T::Text) => {
                        let ts = recv_ts(x);
                        ex(quote!(#ts.as_str()), Cat::Ref(1))
                    }
                    _ => Err(self.unsupported(e.span, "widening to this type")),
                }
            }
            K::Narrow { expr, proof } => {
                let T::Named(id) = e.ty else {
                    return Err(self.unsupported(e.span, "narrowing to a type that is not named"));
                };
                let tn = self.ty_name(id);
                let name = self.front.prog.ty(id).info.name.clone();
                let inner = self.expr(expr)?;
                match proof {
                    Proof::Static(_) => {
                        let v = self.owned(inner)?;
                        ex(quote!(#tn::from_proven(#v)), Cat::Val)
                    }
                    Proof::Runtime if self.mode == Mode::Test => {
                        let v =
                            if inner.ty == T::Text && inner.cat == Cat::Ref(1) { inner.ts } else { self.owned(inner)? };
                        let msg = Literal::string(&format!("a {name}"));
                        ex(quote!(#tn::new(#v).expect(#msg)), Cat::Val)
                    }
                    Proof::Runtime => Err(Diag::error(
                        "E-UNSUPPORTED",
                        e.span,
                        format!("the claim of {name} is not shown here, and a rule has no failure for it"),
                    )
                    .with_decl(self.decl.clone())
                    .with_note("give the value a type whose claim implies this one, or guard it")),
                }
            }
            K::StepPart(s, part) => {
                let s = self.expr(s)?;
                let ts = s.ts;
                let p = format_ident!(
                    "{}",
                    match part {
                        StepPart::Before => "before",
                        StepPart::After => "after",
                        StepPart::Event => "event",
                        StepPart::Reply => "reply",
                    }
                );
                ex(quote!(#ts.#p), Cat::Place { movable: false })
            }
            K::WorldState(w, s) => {
                let w = self.expr(w)?;
                let ts = w.ts;
                let name = ident(&self.front.prog.state(*s).info.name);
                ex(quote!(#ts.#name), Cat::Place { movable: false })
            }
            K::EventArg { event, name } => {
                let ev = self.expr(event)?;
                let ev = self.refr(ev);
                self.event_args.insert(name.clone());
                let f = format_ident!("event_{}", name);
                ex(quote!(#f(#ev)), Cat::Val)
            }
            K::IsEvent { event, rule } => {
                let ev = self.expr(event)?;
                let ts = ev.ts;
                let cmd = self.item(Item::Command, "Command");
                let r = self.front.rule(*rule).ok_or_else(|| self.unsupported(e.span, "a rule outside the app"))?;
                let v = format_ident!("{}", camel(&self.front.prog.rule(*rule).info.name));
                let pat = if r.params.is_empty() { quote!(#cmd::#v) } else { quote!(#cmd::#v { .. }) };
                ex(quote!(matches!(#ts, #pat)), Cat::Val)
            }
            K::IsFail(x) => {
                let x = self.expr(x)?;
                let ts = x.ts;
                ex(quote!(#ts.is_err()), Cat::Val)
            }
        }
    }

    /// `f(a, ...)` where `f`'s body is one interpolation and each argument a local:
    /// the interpolation's parts, and each parameter's local (strategy `sink.inline-format`).
    pub fn inline_format(&self, x: &TExpr) -> Option<(Vec<Part>, HashMap<String, String>)> {
        if !self.layout.on("sink.inline-format") {
            return None;
        }
        let K::CallFn { f, args } = &x.kind else { return None };
        let tf = self.front.func(*f)?;
        let [TStmt { kind: S::Return(Some(TExpr { kind: K::Interp(parts), .. })), .. }] = tf.body.as_slice() else {
            return None;
        };
        let mut aliases = HashMap::new();
        for (p, a) in tf.params.iter().zip(args) {
            let K::Local(n) = &a.kind else { return None };
            aliases.insert(p.name.clone(), n.clone());
        }
        Some((parts.clone(), aliases))
    }

    fn local(&mut self, name: &str, e: &TExpr) -> R<Ex> {
        let name = self.aliases.get(name).cloned().unwrap_or_else(|| name.to_string());
        let name = name.as_str();
        let depth = self.depth;
        let id = ident(name);
        let Some(l) = self.local_mut(name) else {
            return Err(Diag::unsupported(e.span, format!("the local `{name}` is not in scope here")));
        };
        l.uses = l.uses.saturating_sub(1);
        let cat = match l.cat {
            Cat::Place { .. } => Cat::Place { movable: l.uses == 0 && depth == 0 },
            c => c,
        };
        Ok(Ex { ts: quote!(#id), cat, ty: e.ty.clone() })
    }

    /// The format string and arguments of an interpolated text; a bare local is inlined.
    fn format(&mut self, parts: &[Part]) -> R<(Literal, Vec<TokenStream>)> {
        let mut fmt = String::new();
        let mut args = Vec::new();
        for p in parts {
            match p {
                Part::Lit(s) => fmt.push_str(&s.replace('{', "{{").replace('}', "}}")),
                Part::Expr(x) => {
                    if let K::Local(n) = &x.kind
                        && !n.starts_with("r#")
                    {
                        let _ = self.expr(x)?;
                        let n = self.aliases.get(n).unwrap_or(n);
                        fmt.push('{');
                        fmt.push_str(n);
                        fmt.push('}');
                    } else {
                        let v = self.expr(x)?;
                        fmt.push_str("{}");
                        args.push(match v.cat {
                            Cat::Ref(d) if d > 1 => self.refr(v),
                            Cat::Count => v.ts,
                            _ => v.ts,
                        });
                    }
                }
            }
        }
        Ok((Literal::string(&fmt), args))
    }

    fn call_rule(&mut self, rule: spec_lang_ir::RuleId, args: &[TExpr], span: Span) -> R<Ex> {
        let Some(r) = self.front.rule(rule) else {
            return Err(self.unsupported(span, "a rule the app does not expose"));
        };
        let index = self.front.rules.iter().position(|x| x.id == rule).unwrap_or(0);
        let read_only = self.layout.read_only(self.front, index);
        let name = self.front.prog.rule(rule).info.name.clone();
        let module = self.layout.module_of(Item::Rule(rule)).unwrap_or_default().to_string();
        self.uses.insert(format!("{}::{module}", self.root));
        let m = ident(&module);
        let f = ident(&name);
        let ptys: Vec<T> = r.params.iter().map(|p| p.ty.clone()).collect();
        let ty = r.result.clone();
        let fallible = !r.guards.is_empty();
        let mut out = Vec::new();
        for (a, t) in args.iter().zip(&ptys) {
            let a = self.expr(a)?;
            let _ = t;
            out.push(self.owned(a)?);
        }
        let world = match super::items::view_names(self, index) {
            Some((view, _)) => {
                let v = ident(&view);
                quote!(&#m::#v::of(&world))
            }
            None if read_only => quote!(&world),
            None => quote!(&mut world),
        };
        let ts = quote!(#m::#f(#world, &mut out #(, #out)*));
        let _ = fallible;
        Ok(Ex { ts, cat: Cat::Val, ty })
    }

    fn core(&mut self, op: Op, ops: &[TExpr], e: &TExpr) -> R<Ex> {
        let ty = e.ty.clone();
        let span = e.span;
        let ex = |ts: TokenStream, cat: Cat| Ok(Ex { ts, cat, ty: ty.clone() });
        match (op, ops) {
            (Op::Size, [x]) => {
                if let K::Local(n) = &x.kind
                    && self.local_mut(n).is_some_and(|l| l.cat == Cat::Count)
                {
                    let r = self.expr(x)?;
                    return ex(r.ts, Cat::Val);
                }
                let r = self.expr(x)?;
                if r.ty == T::Text {
                    let lang = self.runtime("lang");
                    let a = self.refr(r);
                    return ex(quote!(#lang::size(#a)), Cat::Val);
                }
                let lang = self.runtime("lang");
                match r.cat {
                    Cat::Iter(_) => {
                        let ts = r.ts;
                        ex(quote!(#lang::int(#ts.count())), Cat::Val)
                    }
                    _ => {
                        let ts = recv_ts(r);
                        ex(quote!(#lang::int(#ts.len())), Cat::Val)
                    }
                }
            }
            (Op::Trim, [x]) => {
                let lang = self.runtime("lang");
                let r = self.expr(x)?;
                let a = self.refr(r);
                ex(quote!(#lang::trim(#a)), Cat::Ref(1))
            }
            (Op::TextContains | Op::StartsWith, [x, needle]) => {
                let r = self.expr(x)?;
                let n = self.expr(needle)?;
                let n = self.refr(n);
                let ts = recv_ts(r);
                let m = if op == Op::TextContains { quote!(contains) } else { quote!(starts_with) };
                ex(quote!(#ts.#m(#n)), Cat::Val)
            }
            (Op::List, [x]) => {
                // A set iterates ascending already: its list is its iterator.
                let r = self.expr(x)?;
                let (it, item) = self.iter(r, span)?;
                ex(it, Cat::Iter(item))
            }
            (Op::Bytes, [x]) => {
                let r = self.expr(x)?;
                let ts = recv_ts(r);
                ex(quote!(#ts.bytes()), Cat::Iter(Item1::Val))
            }
            (Op::Keys | Op::Values, [x]) => {
                let r = self.expr(x)?;
                let ts = recv_ts(r);
                let m = if op == Op::Keys { quote!(keys) } else { quote!(values) };
                ex(quote!(#ts.#m()), Cat::Iter(Item1::Ref(1)))
            }
            (Op::States | Op::Steps, [x]) => {
                let r = self.expr(x)?;
                let ts = r.ts;
                let m = if op == Op::States { quote!(states) } else { quote!(steps) };
                ex(quote!(#ts.#m.iter()), Cat::Iter(Item1::Ref(1)))
            }
            (Op::Contains, [x, coll]) => {
                let c = self.expr(coll)?;
                let el = self.expr(x)?;
                let el = self.refr(el);
                let method = if matches!(c.ty, T::Map(..)) { quote!(contains_key) } else { quote!(contains) };
                let ts = recv_ts(c);
                ex(quote!(#ts.#method(#el)), Cat::Val)
            }
            (Op::Get, [m, k]) => {
                let m = self.expr(m)?;
                let k = self.expr(k)?;
                let k = self.refr(k);
                let ts = recv_ts(m);
                ex(quote!(#ts.get(#k)), Cat::OptRef)
            }
            (Op::All | Op::Any | Op::Filter | Op::Map, [recv, f]) => {
                // `m.keys.all(k => ... m[k] ...)`: `k` is a key of `m`, so `m[k]` cannot be `Missing`.
                let keyed = match (&recv.kind, &f.kind) {
                    (K::Core(Op::Keys, m), K::Lambda { param, .. }) if m.len() == 1 => {
                        Some((param.clone(), m[0].clone()))
                    }
                    _ => None,
                };
                // Over a map's keys with `m[k]` in the body: walk the entries, `m[k]` is the entry's value.
                if let (Some((param, map)), Op::All | Op::Any) = (&keyed, op)
                    && let K::Lambda { body, .. } = &f.kind
                    && reads_entry(body, param, map)
                {
                    let m = self.expr(map)?;
                    let m = recv_ts(m);
                    self.entries_of.push((param.clone(), map.clone()));
                    self.push();
                    self.depth += 1;
                    self.bind(param, Cat::Ref(1), None, usize::MAX);
                    let b = self.expr(body);
                    self.depth -= 1;
                    self.pop();
                    self.entries_of.pop();
                    let b = self.val(b?);
                    let p = ident(param);
                    let method = if op == Op::All { quote!(all) } else { quote!(any) };
                    return ex(quote!(#m.iter().#method(|(#p, value)| #b)), Cat::Val);
                }
                if let Some(k) = &keyed {
                    self.keys_of.push(k.clone());
                }
                let r = self.expr(recv)?;
                let (it, item) = self.iter(r, span)?;
                let param_cat = if op == Op::Filter { item.deeper() } else { item.cat() };
                let closure = self.closure(f, param_cat);
                if keyed.is_some() {
                    self.keys_of.pop();
                }
                let (p, body, body_cat) = closure?;
                let m = match op {
                    Op::All => quote!(all),
                    Op::Any => quote!(any),
                    Op::Filter => quote!(filter),
                    _ => quote!(map),
                };
                let cat = match op {
                    Op::All | Op::Any => Cat::Val,
                    Op::Filter => Cat::Iter(item),
                    _ => Cat::Iter(match body_cat {
                        Cat::Ref(d) => Item1::Ref(d),
                        _ => Item1::Val,
                    }),
                };
                ex(quote!(#it.#m(|#p| #body)), cat)
            }
            (Op::Sort, [recv, f]) => {
                // A set iterates in its order already; sorting it by the element itself changes nothing.
                if matches!(recv.ty, T::Set(_))
                    && let K::Lambda { param, body } = &f.kind
                    && matches!(&body.kind, K::Local(n) if n == param)
                {
                    let r = self.expr(recv)?;
                    let (it, item) = self.iter(r, span)?;
                    return ex(it, Cat::Iter(item));
                }
                let r = self.expr(recv)?;
                let (it, item) = self.iter(r, span)?;
                let (p, key, _) = self.closure(f, item.deeper())?;
                let v = format_ident!("sorted");
                ex(
                    quote!({ let mut #v: Vec<_> = #it.collect(); #v.sort_by_key(|#p| #key); #v.into_iter() }),
                    Cat::Iter(item),
                )
            }
            (Op::Join, [recv, sep]) => {
                let r = self.expr(recv)?;
                let (it, _) = self.iter(r, span)?;
                if matches!(&sep.kind, K::Text(s) if s.is_empty()) {
                    return ex(quote!(#it.collect::<String>()), Cat::Val);
                }
                let s = self.expr(sep)?;
                let s = s.ts;
                ex(quote!(#it.collect::<Vec<_>>().join(#s)), Cat::Val)
            }
            _ => Err(self.unsupported(span, format!("{op:?} with these operands"))),
        }
    }

    /// A one-parameter closure: its pattern, body and the body's category.
    fn closure(&mut self, f: &TExpr, cat: Cat) -> R<(TokenStream, TokenStream, Cat)> {
        let K::Lambda { param, body } = &f.kind else {
            return Err(self.unsupported(f.span, "a collection method whose argument is not a lambda"));
        };
        self.push();
        self.depth += 1;
        self.bind(param, cat, None, usize::MAX);
        let b = self.expr(body);
        self.depth -= 1;
        self.pop();
        let b = b?;
        let p = ident(param);
        let bc = b.cat;
        let ts = match bc {
            Cat::Ref(_) | Cat::Val | Cat::Place { .. } | Cat::OptRef if self.copy(&b.ty) => self.val(b),
            _ => b.ts,
        };
        Ok((quote!(#p), ts, bc))
    }

    /// `!p`, with a comparison flipped rather than negated.
    pub fn negate(&mut self, x: &TExpr) -> R<TokenStream> {
        if let K::Compare(head, links) = &x.kind
            && links.len() == 1
        {
            return self.compare(head, links, true);
        }
        if let K::Not(inner) = &x.kind {
            let v = self.expr(inner)?;
            return Ok(self.val(v));
        }
        let v = self.expr(x)?;
        let v = self.val(v);
        Ok(match &x.kind {
            K::Bin(..) => quote!(!(#v)),
            _ => quote!(!#v),
        })
    }

    /// A comparison chain; `negated` flips a single link.
    fn compare(&mut self, head: &TExpr, links: &[(CmpOp, TExpr)], negated: bool) -> R<TokenStream> {
        // `lo <= x <= hi` with literal bounds: `(lo..=hi).contains(&x)`.
        if let [(CmpOp::Le, mid), (CmpOp::Le, hi)] = links
            && let (K::Int(_), K::Int(_)) = (&head.kind, &hi.kind)
        {
            let lo = self.expr(head)?;
            let lo = self.val(lo);
            let hi = self.expr(hi)?;
            let hi = self.val(hi);
            let m = self.expr(mid)?;
            let m = self.val(m);
            return Ok(quote!((#lo..=#hi).contains(&#m)));
        }
        // `xs.size == 0`, `xs.size >= 1`: `is_empty`.
        if let [(op, rhs)] = links
            && let K::Core(Op::Size, recv) = &head.kind
            && let [recv] = recv.as_slice()
            && let K::Int(n) = rhs.kind
        {
            let empty = match (op, n) {
                (CmpOp::Eq, 0) => Some(true),
                (CmpOp::Ne, 0) | (CmpOp::Ge, 1) | (CmpOp::Gt, 0) => Some(false),
                _ => None,
            };
            if let Some(empty) = empty {
                let r = self.expr(recv)?;
                if !matches!(r.cat, Cat::Iter(_) | Cat::Count) {
                    let ts = recv_ts(r);
                    return Ok(if empty != negated { quote!(#ts.is_empty()) } else { quote!(!#ts.is_empty()) });
                }
            }
        }
        let mut parts = Vec::new();
        let mut left = head;
        for (op, right) in links {
            // Two sizes compare as lengths: no conversion to `Int` on either side.
            let (a, b) = match (self.raw_size(left)?, self.raw_size(right)?) {
                (Some(a), Some(b)) => (a, b),
                _ => {
                    let a = self.expr(left)?;
                    let b = self.expr(right)?;
                    self.operands(a, b)
                }
            };
            let op = if negated { flip(*op) } else { *op };
            parts.push(match op {
                CmpOp::Eq => quote!(#a == #b),
                CmpOp::Ne => quote!(#a != #b),
                CmpOp::Lt => quote!(#a < #b),
                CmpOp::Le => quote!(#a <= #b),
                CmpOp::Gt => quote!(#a > #b),
                CmpOp::Ge => quote!(#a >= #b),
                CmpOp::In => quote!(false),
            });
            left = right;
        }
        Ok(quote!(#(#parts)&&*))
    }

    /// `xs.size` of a collection as its raw length (`len()` or `count()`).
    fn raw_size(&mut self, e: &TExpr) -> R<Option<TokenStream>> {
        let K::Core(Op::Size, ops) = &e.kind else { return Ok(None) };
        let [x] = ops.as_slice() else { return Ok(None) };
        if x.ty == T::Text || matches!(&x.kind, K::Local(n) if self.local_mut(n).is_some_and(|l| l.cat == Cat::Count)) {
            return Ok(None);
        }
        let r = self.expr(x)?;
        Ok(Some(match r.cat {
            Cat::Iter(_) => {
                let ts = r.ts;
                quote!(#ts.count())
            }
            _ => {
                let ts = recv_ts(r);
                quote!(#ts.len())
            }
        }))
    }

    /// Two operands of a comparison, brought to the same level of reference.
    fn operands(&self, a: Ex, b: Ex) -> (TokenStream, TokenStream) {
        if self.copy(&a.ty) {
            return (self.val(a), self.val(b));
        }
        let is_ref = |e: &Ex| matches!(e.cat, Cat::Ref(_));
        match (is_ref(&a), is_ref(&b)) {
            (true, false) if b.cat != Cat::OptRef => (self.refr(a), {
                let ts = b.ts;
                quote!(&#ts)
            }),
            (false, true) if a.cat != Cat::OptRef => (
                {
                    let ts = a.ts;
                    quote!(&#ts)
                },
                self.refr(b),
            ),
            (true, true) => (self.refr(a), self.refr(b)),
            _ => (a.ts, b.ts),
        }
    }

    fn match_expr(&mut self, scrutinee: &TExpr, arms: &[(Pat, TExpr)], e: &TExpr) -> R<Ex> {
        let s = self.expr(scrutinee)?;
        let bind_cat = match s.cat {
            Cat::OptRef | Cat::Ref(_) | Cat::Place { .. } => Cat::Ref(1),
            _ => Cat::Val,
        };
        let s_ts = match s.cat {
            Cat::Place { .. } if !self.copy(&s.ty) => {
                let ts = s.ts;
                quote!(&#ts)
            }
            _ if self.copy(&s.ty) => self.val(s),
            _ => s.ts,
        };
        let mut out = Vec::new();
        let mut cat = Cat::Val;
        for (p, v) in arms {
            self.push();
            let pat = self.pattern(p, bind_cat)?;
            let v = self.expr(v);
            self.pop();
            let v = v?;
            cat = v.cat;
            let vt = if self.copy(&v.ty) { self.val(v) } else { v.ts };
            out.push(quote!(#pat => #vt));
        }
        if self.copy(&e.ty) {
            cat = Cat::Val;
        }
        Ok(Ex { ts: quote!(match #s_ts { #(#out),* }), cat, ty: e.ty.clone() })
    }

    fn pattern(&mut self, p: &Pat, bind_cat: Cat) -> R<TokenStream> {
        Ok(match p {
            Pat::Wild => quote!(_),
            Pat::Bind(n) => {
                self.bind(n, bind_cat, None, usize::MAX);
                let id = ident(n);
                quote!(#id)
            }
            Pat::Variant { ty, tag, args } => {
                if *ty == self.front.prog.option_type {
                    if *tag == spec_lang_ir::SOME_TAG {
                        let inner = match args.first() {
                            Some(a) => self.pattern(a, bind_cat)?,
                            None => quote!(_),
                        };
                        quote!(Some(#inner))
                    } else {
                        quote!(None)
                    }
                } else {
                    let tn = self.ty_name(*ty);
                    let v = variant_name(self.front, *ty, *tag);
                    quote!(#tn::#v)
                }
            }
        })
    }

    // ---- statements ----

    /// A body; the last `return` becomes the tail expression.
    pub fn body(&mut self, stmts: &[TStmt], tail_unit: bool) -> R<TokenStream> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < stmts.len() {
            let last = i + 1 == stmts.len();
            if let Some(r) = self.retain.clone()
                && r.at == i
            {
                out.push(self.retain_stmt(&stmts[i], &r)?);
                i += 2;
                continue;
            }
            match &stmts[i].kind {
                S::Return(Some(v)) if last => {
                    let v = self.expr(v)?;
                    let v = self.ret(v)?;
                    out.push(if self.fallible { quote!(Ok(#v)) } else { v });
                }
                S::Return(None) if last => {
                    if self.fallible {
                        out.push(quote!(Ok(())));
                    }
                }
                _ => {
                    out.push(self.stmt(&stmts[i])?);
                    if last && tail_unit && self.fallible {
                        out.push(quote!(Ok(())));
                    }
                }
            }
            i += 1;
        }
        if stmts.is_empty() && self.fallible {
            out.push(quote!(Ok(())));
        }
        Ok(quote!(#(#out)*))
    }

    /// A returned value: owned unless the fn returns a static text.
    fn ret(&mut self, v: Ex) -> R<TokenStream> {
        if v.ty == T::Text && v.cat == Cat::Ref(1) {
            return Ok(v.ts);
        }
        self.owned(v)
    }

    fn retain_stmt(&mut self, s: &TStmt, r: &Retain) -> R<TokenStream> {
        let S::Let { value, .. } = &s.kind else {
            return Err(self.unsupported(s.span, "the retain strategy off its `let`"));
        };
        let K::Core(Op::Filter, ops) = &value.kind else {
            return Err(self.unsupported(s.span, "the retain strategy off its filter"));
        };
        let [_, f] = ops.as_slice() else {
            return Err(self.unsupported(s.span, "the retain strategy off its filter"));
        };
        let K::Lambda { param, body } = &f.kind else {
            return Err(self.unsupported(s.span, "the retain strategy without a lambda"));
        };
        self.push();
        self.depth += 1;
        self.bind(param, Cat::Ref(1), None, usize::MAX);
        let keep = self.negate(body);
        self.depth -= 1;
        self.pop();
        let keep = keep?;
        let state = ident(&self.front.prog.state(r.state).info.name);
        let name = ident(&r.name);
        let p = ident(param);
        let lang = self.runtime("lang");
        self.bind(&r.name, Cat::Count, None, usize::MAX);
        Ok(quote! {
            let before = world.#state.len();
            world.#state.retain(|_, #p| #keep);
            let #name = #lang::int(before - world.#state.len());
        })
    }

    pub fn stmt(&mut self, s: &TStmt) -> R<TokenStream> {
        match &s.kind {
            S::Let { name, value } => self.let_stmt(name, value, s),
            S::Assign { state, path, value } => self.assign(*state, path, value, s.span),
            S::Remove { state, key } => {
                if let Some(e) = &self.entry
                    && e.state == *state
                {
                    return Ok(quote!());
                }
                let k = self.expr(key)?;
                let k = self.refr(k);
                let st = ident(&self.front.prog.state(*state).info.name);
                Ok(quote!(world.#st.remove(#k);))
            }
            S::Expr(e) if matches!(&e.kind, K::OutLine(x) if self.inline_format(x).is_some()) => {
                let K::OutLine(x) = &e.kind else { return Err(self.unsupported(s.span, "an inlined line")) };
                let K::CallFn { f, .. } = &x.kind else { return Err(self.unsupported(s.span, "an inlined line")) };
                let fname = self.front.prog.func(*f).info.name.clone();
                let origin = Origin::new(self.decl.clone(), self.front.at(s.span), "body: out.line").strategy(
                    "sink.inline-format",
                    format!("fn {fname} is pure and one interpolation; out.line takes any text: it formats into the buffer, no string between"),
                );
                let anchor = self.anchor(s.span, &format!("out.line, {fname} formatted in place"), origin);
                let v = self.expr(e)?;
                let ts = v.ts;
                Ok(quote!(#anchor #ts;))
            }
            S::Expr(e) => {
                let fallible = self.call_fallible(e);
                let v = self.expr(e)?;
                let ts = v.ts;
                if fallible && self.mode == Mode::Test {
                    // A call in a test that is refused fails the test.
                    let msg = Literal::string(&format!("{} is not refused", self.decl));
                    return Ok(quote!(#ts.expect(#msg);));
                }
                Ok(quote!(#ts;))
            }
            S::Return(None) => Ok(if self.fallible { quote!(return Ok(());) } else { quote!(return;) }),
            S::Return(Some(v)) => {
                let v = self.expr(v)?;
                let v = self.ret(v)?;
                Ok(if self.fallible { quote!(return Ok(#v);) } else { quote!(return #v;) })
            }
            S::Assert(e) => self.assert(e),
            S::For { var, iter, body } => {
                let (it, item) = match &iter.kind {
                    K::Local(n) => {
                        let elem = self.local_mut(n).and_then(|l| l.elem);
                        let v = self.expr(iter)?;
                        let movable = v.cat == Cat::Place { movable: true };
                        let ts = v.ts;
                        match (elem, movable) {
                            (Some(el), true) => (ts, el),
                            (Some(Item1::Val), false) => (quote!(&#ts), Item1::Ref(1)),
                            (Some(Item1::Ref(d)), false) => (quote!(&#ts), Item1::Ref(d + 1)),
                            (None, _) => (quote!(#ts.iter()), Item1::Ref(1)),
                        }
                    }
                    _ => {
                        let v = self.expr(iter)?;
                        self.iter(v, iter.span)?
                    }
                };
                self.push();
                self.depth += 1;
                self.bind(var, item.cat(), None, usize::MAX);
                let b = self.body_stmts(body);
                self.depth -= 1;
                self.pop();
                let b = b?;
                let v = ident(var);
                Ok(quote!(for #v in #it { #b }))
            }
            S::If { cond, then, els } => {
                let c = self.expr(cond)?;
                let c = self.val(c);
                self.push();
                let t = self.body_stmts(then);
                self.pop();
                let t = t?;
                if els.is_empty() {
                    return Ok(quote!(if #c { #t }));
                }
                self.push();
                let e = self.body_stmts(els);
                self.pop();
                let e = e?;
                Ok(quote!(if #c { #t } else { #e }))
            }
            S::ExpectValue { call, value } => {
                let fallible = self.call_fallible(call);
                let c = self.expr(call)?;
                let v = self.expr(value)?;
                let v = self.owned(v)?;
                let c = c.ts;
                Ok(if fallible { quote!(assert_eq!(#c, Ok(#v));) } else { quote!(assert_eq!(#c, #v);) })
            }
            S::ExpectFail { call, error } => {
                let refusal = self.item(Item::Refusal, "Refusal");
                let c = self.expr(call)?;
                let c = c.ts;
                let e = ident(error);
                Ok(quote!(assert_eq!(#c, Err(#refusal::#e));))
            }
        }
    }

    fn call_fallible(&self, call: &TExpr) -> bool {
        match &call.kind {
            K::CallRule { rule, .. } => self.front.rule(*rule).is_some_and(|r| !r.guards.is_empty()),
            _ => false,
        }
    }

    /// Statements of a nested block (no tail handling).
    fn body_stmts(&mut self, stmts: &[TStmt]) -> R<TokenStream> {
        let mut out = Vec::new();
        for s in stmts {
            out.push(self.stmt(s)?);
        }
        Ok(quote!(#(#out)*))
    }

    fn let_stmt(&mut self, name: &str, value: &TExpr, s: &TStmt) -> R<TokenStream> {
        let id = ident(name);
        let uses = self.uses_after.get(name).copied().unwrap_or(usize::MAX);
        // `let xs = ys.sort(key)`: collect once, sort in place.
        if let K::Core(Op::Sort, ops) = &value.kind
            && let [recv, f] = ops.as_slice()
            && !matches!(recv.ty, T::Set(_))
        {
            let r = self.expr(recv)?;
            let (it, item) = self.iter(r, s.span)?;
            let (p, key, _) = self.closure(f, item.deeper())?;
            self.bind(name, Cat::Place { movable: true }, Some(item), uses);
            // Unstable when no two elements share a key: the stable answer is the only answer.
            let unique = crate::front::facts::sort_key_unique(self.front, &self.front.facts, recv, f)
                .filter(|_| self.layout.on("sort.unstable"));
            let (sort, anchor) = match unique {
                Some(licence) => {
                    let at = self.front.at(s.span);
                    let origin = Origin::new(self.decl.clone(), at, "body: sort").strategy("sort.unstable", licence);
                    (quote!(sort_unstable_by_key), self.anchor(s.span, "body: sort (unstable: unique key)", origin))
                }
                None => (quote!(sort_by_key), quote!()),
            };
            return Ok(quote! {
                let mut #id: Vec<_> = #it.collect();
                #anchor
                #id.#sort(|#p| #key);
            });
        }
        let v = self.expr(value)?;
        let (ts, cat, elem) = match v.cat {
            // A list read from a world the rule then writes is copied, so the write can borrow it.
            Cat::Iter(Item1::Ref(_)) if self.writes_world => {
                let ts = v.ts;
                (quote!(#ts.cloned().collect::<Vec<_>>()), Cat::Place { movable: true }, Some(Item1::Val))
            }
            Cat::Iter(item) => {
                let ts = v.ts;
                (quote!(#ts.collect::<Vec<_>>()), Cat::Place { movable: true }, Some(item))
            }
            Cat::Val if self.copy(&v.ty) => (v.ts, Cat::Val, None),
            Cat::Val => (v.ts, Cat::Place { movable: true }, None),
            Cat::Place { .. } if self.copy(&v.ty) => (v.ts, Cat::Val, None),
            Cat::Place { .. } => {
                let ts = v.ts;
                (quote!(&#ts), Cat::Ref(1), None)
            }
            c => (v.ts, c, None),
        };
        self.bind(name, cat, elem, uses);
        Ok(quote!(let #id = #ts;))
    }

    fn assign(&mut self, state: StateId, path: &[PathStep], value: &TExpr, span: Span) -> R<TokenStream> {
        let st = ident(&self.front.prog.state(state).info.name);
        // `p = p op x` is `p op= x`.
        let (compound, value) = match &value.kind {
            K::Bin(op @ (Bin::Add | Bin::Sub | Bin::Mul), a, b) if is_place(a, state, path) => (Some(*op), &**b),
            _ => (None, value),
        };
        let v = self.expr(value)?;
        let v = self.owned(v)?;
        if let Some(op) = compound {
            let target = match path {
                [] => quote!(world.#st),
                _ => {
                    let full = self.assign(state, path, value, span)?;
                    let text = full.to_string();
                    let lhs = text.split(" = ").next().unwrap_or_default().to_string();
                    lhs.parse().map_err(|_| self.unsupported(span, "this place"))?
                }
            };
            return Ok(match op {
                Bin::Add => quote!(#target += #v;),
                Bin::Sub => quote!(#target -= #v;),
                _ => quote!(#target *= #v;),
            });
        }
        match path {
            [] => Ok(quote!(world.#st = #v;)),
            [PathStep::Index(k)] => {
                let k = self.expr(k)?;
                let k = self.owned(k)?;
                Ok(quote!(world.#st.insert(#k, #v);))
            }
            [PathStep::Index(k), PathStep::Field(i)] if self.entry.is_none() => {
                let shown =
                    matches!((&self.guarded, &k.kind), (Some((s, key)), K::Local(k2)) if *s == state && key == k2);
                if !shown {
                    return Err(self.unsupported(span, "writing into a map entry no guard shows present"));
                }
                let Some(T::Map(_, vt)) = self.front.states.iter().find(|s| s.id == state).map(|s| &s.ty) else {
                    return Err(self.unsupported(span, "this place"));
                };
                let T::Named(rec) = **vt else {
                    return Err(self.unsupported(span, "a field of a value that is not a record"));
                };
                let fname = crate::front::record_fields(self.front.prog, rec)
                    .and_then(|fs| fs.get(*i))
                    .map(|f| ident(&f.name))
                    .ok_or_else(|| self.unsupported(span, "a field out of range"))?;
                let k = self.expr(k)?;
                let k = self.refr(k);
                // The guard showed the key present: the `else` cannot run.
                Ok(quote!(if let Some(entry) = world.#st.get_mut(#k) { entry.#fname = #v; }))
            }
            [PathStep::Index(k), PathStep::Field(i)] => {
                let Some(entry) = self.entry.clone() else {
                    return Err(self.unsupported(span, "writing into a map entry no guard shows present"));
                };
                if entry.state != state || !matches!(&k.kind, K::Local(n) if *n == entry.key) {
                    return Err(self.unsupported(span, "writing into a map entry other than the guarded one"));
                }
                let Some(T::Map(_, vt)) = self.front.states.iter().find(|s| s.id == state).map(|s| &s.ty) else {
                    return Err(self.unsupported(span, "this place"));
                };
                let T::Named(rec) = **vt else {
                    return Err(self.unsupported(span, "a field of a value that is not a record"));
                };
                let fields = crate::front::record_fields(self.front.prog, rec).unwrap_or(&[]);
                let Some(f) = fields.get(*i) else {
                    return Err(self.unsupported(span, "a field out of range"));
                };
                let fname = ident(&f.name);
                let b = entry.binding;
                Ok(quote!(#b.#fname = #v;))
            }
            _ => Err(self.unsupported(span, "this place")),
        }
    }

    fn assert(&mut self, e: &TExpr) -> R<TokenStream> {
        if let K::Compare(head, links) = &e.kind
            && let [(CmpOp::Eq, right)] = links.as_slice()
        {
            let a = self.expr(head)?;
            let b = self.expr(right)?;
            let (a, b) = self.operands(a, b);
            return Ok(quote!(assert_eq!(#a, #b);));
        }
        let v = self.negate_twice(e)?;
        Ok(quote!(assert!(#v);))
    }

    fn negate_twice(&mut self, e: &TExpr) -> R<TokenStream> {
        let v = self.expr(e)?;
        Ok(self.val(v))
    }
}

impl Ctx<'_, '_> {
    /// Counts each local's uses in a body, so the last use can move.
    pub fn count_uses(&mut self, stmts: &[TStmt], extra: &[&TExpr]) {
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut visit = |e: &TExpr| count_locals(e, &mut counts);
        for s in stmts {
            crate::front::facts::stmt_exprs(s, &mut visit);
        }
        for e in extra {
            count_locals(e, &mut counts);
        }
        self.uses_after = counts;
    }
}

fn count_locals(e: &TExpr, counts: &mut HashMap<String, usize>) {
    if let K::Local(n) = &e.kind {
        *counts.entry(n.clone()).or_default() += 1;
    }
    crate::front::facts::children(e, &mut |c| count_locals(c, counts));
}

fn flip(op: CmpOp) -> CmpOp {
    match op {
        CmpOp::Eq => CmpOp::Ne,
        CmpOp::Ne => CmpOp::Eq,
        CmpOp::Lt => CmpOp::Ge,
        CmpOp::Le => CmpOp::Gt,
        CmpOp::Gt => CmpOp::Le,
        CmpOp::Ge => CmpOp::Lt,
        CmpOp::In => CmpOp::In,
    }
}

/// A receiver for a method call: a value in parentheses, anything else as is.
fn recv_ts(e: Ex) -> TokenStream {
    let ts = e.ts;
    match e.cat {
        Cat::Ref(d) if d > 1 => {
            let stars = (1..d).map(|_| quote!(*));
            quote!((#(#stars)* #ts))
        }
        _ => ts,
    }
}

/// Does `body` read `map[k]` (with `k` the parameter) and use no local named `value`?
fn reads_entry(body: &TExpr, k: &str, map: &TExpr) -> bool {
    fn walk(e: &TExpr, k: &str, map: &TExpr, hit: &mut bool, clash: &mut bool) {
        match &e.kind {
            K::Index { map: m, key } if same_map(m, map) && matches!(&key.kind, K::Local(n) if n == k) => *hit = true,
            K::Local(n) | K::Lambda { param: n, .. } if n == "value" => *clash = true,
            _ => {}
        }
        crate::front::facts::children(e, &mut |c| walk(c, k, map, hit, clash));
    }
    let (mut hit, mut clash) = (false, false);
    walk(body, k, map, &mut hit, &mut clash);
    hit && !clash
}

fn flatten_and<'a>(e: &'a TExpr, out: &mut Vec<&'a TExpr>) {
    match &e.kind {
        K::Bin(Bin::And, a, b) => {
            flatten_and(a, out);
            flatten_and(b, out);
        }
        _ => out.push(e),
    }
}

/// `not x.contains("c")` with `x` a local and `"c"` one char.
fn one_char_absent(e: &TExpr) -> Option<(String, char)> {
    let K::Not(x) = &e.kind else { return None };
    let K::Core(Op::TextContains, ops) = &x.kind else { return None };
    let [TExpr { kind: K::Local(r), .. }, TExpr { kind: K::Text(t), .. }] = ops.as_slice() else { return None };
    let mut cs = t.chars();
    let c = cs.next()?;
    cs.next().is_none().then(|| (r.clone(), c))
}

/// A conjunction with two or more `not x.contains("c")` on the same local.
fn fusable_contains(e: &TExpr) -> bool {
    let mut conj = Vec::new();
    flatten_and(e, &mut conj);
    let found: Vec<String> = conj.iter().filter_map(|c| one_char_absent(c)).map(|(r, _)| r).collect();
    found.len() >= 2 && found.iter().all(|r| r == &found[0])
}

/// The same map, wherever written: a state, or a state of the same world local.
fn same_map(a: &TExpr, b: &TExpr) -> bool {
    match (&a.kind, &b.kind) {
        (K::State(x), K::State(y)) => x == y,
        (K::WorldState(w1, x), K::WorldState(w2, y)) => {
            x == y && matches!((&w1.kind, &w2.kind), (K::Local(a), K::Local(b)) if a == b)
        }
        _ => false,
    }
}

/// Is `e` the place a statement assigns (`state` then `path`)?
fn is_place(e: &TExpr, state: StateId, path: &[PathStep]) -> bool {
    match (&e.kind, path.split_last()) {
        (K::State(s), None) => *s == state,
        (K::Field(x, i), Some((PathStep::Field(j), rest))) => i == j && is_place(x, state, rest),
        (K::Index { map, key }, Some((PathStep::Index(k), rest))) => {
            matches!((&key.kind, &k.kind), (K::Local(a), K::Local(b)) if a == b) && is_place(map, state, rest)
        }
        _ => false,
    }
}

/// Rust binding strength of an operand, higher binds tighter.
fn strength(e: &TExpr) -> u8 {
    match &e.kind {
        K::Bin(Bin::Or | Bin::Implies, ..) => 1,
        K::Bin(Bin::And, ..) => 2,
        K::Compare(_, links) if links.len() > 1 => 2,
        K::Compare(..) => 3,
        K::Bin(Bin::Add | Bin::Sub, ..) => 4,
        K::Bin(Bin::Mul, ..) => 5,
        _ => 9,
    }
}

/// Parentheses only where Rust would otherwise bind the operand differently.
fn paren_if_looser(e: &TExpr, parent: Bin, ts: TokenStream) -> TokenStream {
    let need = match parent {
        Bin::Or => 1,
        Bin::And => 2,
        Bin::Implies => 9,
        Bin::Add | Bin::Sub => 4,
        Bin::Mul => 5,
    };
    if strength(e) < need || (parent == Bin::Implies && strength(e) < 9) { quote!((#ts)) } else { ts }
}

/// A sum variant's name.
pub fn variant_name(front: &Front<'_>, ty: spec_lang_ir::TypeId, tag: u32) -> proc_macro2::Ident {
    match &front.prog.ty(ty).def {
        TypeDef::Sum(vs) => ident(vs.get(tag as usize).map_or("Unknown", |v| v.name.as_str())),
        _ => ident("Unknown"),
    }
}

/// A fn whose every result is a text literal returns `&'static str`.
pub fn static_text_fn(f: &crate::front::TFn) -> bool {
    f.result == T::Text && f.body.iter().all(|s| matches!(&s.kind, S::Return(Some(v)) if all_literal_text(v)))
}

fn all_literal_text(e: &TExpr) -> bool {
    match &e.kind {
        K::Text(_) => true,
        K::Match { arms, .. } => arms.iter().all(|(_, v)| all_literal_text(v)),
        K::If { then, els, .. } => all_literal_text(then) && all_literal_text(els),
        _ => false,
    }
}

/// The entry binding's name: the value type in snake case, unless a parameter has it.
pub fn entry_binding(front: &Front<'_>, state: StateId, params: &[crate::front::TParam]) -> proc_macro2::Ident {
    let name = match front.states.iter().find(|s| s.id == state).map(|s| &s.ty) {
        Some(T::Map(_, v)) => match **v {
            T::Named(id) => snake(&front.prog.ty(id).info.name),
            _ => "entry".into(),
        },
        _ => "entry".into(),
    };
    if params.iter().any(|p| p.name == name) { ident("entry") } else { ident(&name) }
}
