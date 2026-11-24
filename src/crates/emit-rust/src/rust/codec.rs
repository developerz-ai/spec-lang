//! `store/codec.rs`: the world's JSON form, generated from the states' types
//! (contract section 3). Top level: `version`, then every state by name.
//! A record is an object of its fields in declaration order; a sum without
//! data is its variant's name; a `Set` is an array in order, no element
//! twice; a `Map` keyed by a field of its values is the array of its values
//! by key. Each state invariant (a law over every state) is checked on read.

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use spec_lang_diag::Diag;
use spec_lang_ir::{StateId, TypeDef};

use super::layout::{Item, ident, snake};
use super::lower::{Cat, Ctx, R};
use crate::front::typed::{K, T};
use crate::front::typer::{alias_base, type_name};

/// Readers and writers needed, by type, in first-use order.
#[derive(Default)]
struct Need {
    done: Vec<T>,
    fns: Vec<TokenStream>,
}

fn fn_name(ctx: &Ctx<'_, '_>, t: &T, verb: &str) -> proc_macro2::Ident {
    format_ident!("{verb}_{}", shape(ctx, t))
}

fn shape(ctx: &Ctx<'_, '_>, t: &T) -> String {
    match t {
        T::Named(id) => snake(&ctx.front.prog.ty(*id).info.name),
        T::Set(x) => format!("{}_set", shape(ctx, x)),
        T::List(x) => format!("{}_list", shape(ctx, x)),
        T::Map(_, v) => format!("{}_map", shape(ctx, v)),
        T::Int => "int".into(),
        T::Text => "text".into(),
        _ => "value".into(),
    }
}

/// The states in the store's key order: by name.
pub fn states(ctx: &Ctx<'_, '_>) -> Vec<(StateId, T)> {
    let mut s: Vec<(StateId, T)> = ctx.front.states.iter().map(|s| (s.id, s.ty.clone())).collect();
    s.sort_by_key(|(id, _)| ctx.front.prog.state(*id).info.name.clone());
    s
}

pub fn codec(ctx: &mut Ctx<'_, '_>) -> R<TokenStream> {
    let world = ctx.item(Item::World, "World");
    let mut need = Need::default();
    let states = states(ctx);
    let mut slots = Vec::new();
    let mut arms = Vec::new();
    let mut fields = Vec::new();
    let mut writes = Vec::new();
    for (i, (id, ty)) in states.iter().enumerate() {
        let name = ctx.front.prog.state(*id).info.name.clone();
        let n = ident(&name);
        let key = Literal::string(&name);
        let read = reader(ctx, ty, Some(*id), &mut need)?;
        slots.push(quote!(let mut #n = None;));
        arms.push(quote!(#key => set(&mut #n, #read(cursor)?),));
        fields.push(n.clone());
        let lead = Literal::string(&format!(",\"{name}\":"));
        let _ = i;
        let w = writer(ctx, ty, quote!(world.#n), Some(*id), &mut need)?;
        writes.push(quote!(w.raw(#lead); #w));
    }
    let checks = invariant_checks(ctx)?;
    let mut projected = Vec::new();
    for index in 0..ctx.front.rules.len() {
        if let Some(ts) = projected_decoder(ctx, index, &mut need)? {
            projected.push(ts);
        }
    }
    let fns = need.fns;
    Ok(quote! {
        /// The JSON codec of the world, for `Store`.
        #[derive(Debug)]
        pub struct Json;

        impl Decode for Json {
            type World = #world;

            fn decode(bytes: &[u8]) -> Result<#world, Unreadable> {
                decode(bytes)
            }
        }

        impl Codec for Json {
            fn encode(world: &#world) -> Result<Vec<u8>, StoreError> {
                encode(world)
            }
        }

        /// Checks the whole store against the schema and every state invariant.
        pub fn decode(bytes: &[u8]) -> Result<#world, Unreadable> {
            let mut cursor = Cursor::new(bytes)?;
            let mut version = None;
            #(#slots)*
            cursor.object(|cursor, key| match key {
                "version" => match cursor.integer()? {
                    1 => set(&mut version, ()),
                    _ => Err(Unreadable::Version),
                },
                #(#arms)*
                _ => Err(Unreadable::UnknownKey),
            })?;
            cursor.end()?;
            let (Some(()), #(Some(#fields)),*) = (version, #(#fields),*) else {
                return Err(Unreadable::MissingKey);
            };
            let world = #world { #(#fields),* };
            #(#checks)*
            Ok(world)
        }

        /// The one canonical form; `Full` when an `Int` is beyond 2^53.
        pub fn encode(world: &#world) -> Result<Vec<u8>, StoreError> {
            let mut w = Writer::default();
            w.raw("{\"version\":1");
            #(#writes)*
            w.raw("}");
            Ok(w.finish())
        }

        #(#projected)*

        /// Fills a key's slot; a slot already full is the key given twice.
        fn set<T>(slot: &mut Option<T>, value: T) -> Result<(), Unreadable> {
            match slot.replace(value) {
                None => Ok(()),
                Some(_) => Err(Unreadable::DuplicateKey),
            }
        }

        #(#fns)*
    })
}

/// Every state invariant, as statements that refuse a `world` breaking it.
fn invariant_checks(ctx: &mut Ctx<'_, '_>) -> R<Vec<TokenStream>> {
    let mut checks = Vec::new();
    for law in ctx.front.facts.invariants.clone() {
        let tl = ctx.front.laws.iter().find(|l| l.id == law).map(|l| l.body.clone());
        let Some(body) = tl else { continue };
        let K::Core(_, ops) = &body.kind else { continue };
        let [_, lambda] = ops.as_slice() else { continue };
        let K::Lambda { param, body } = &lambda.kind else { continue };
        ctx.reset_scope();
        ctx.decl = ctx.front.decl_path(&ctx.front.prog.law(law).info);
        ctx.bind(param, Cat::Ref(1), None, usize::MAX);
        let p = ident(param);
        let broken = ctx.negate(body)?;
        let name = Literal::string(&ctx.front.prog.law(law).info.name);
        checks.push(quote! {
            let #p = &world;
            if #broken {
                return Err(Unreadable::Law(#name));
            }
        });
    }
    Ok(checks)
}

fn refuse(ctx: &Ctx<'_, '_>, t: &T, why: &str) -> Diag {
    Diag::new(
        spec_lang_diag::Severity::Error,
        "E-UNSUPPORTED",
        format!("the store cannot carry {}: {why}", type_name(ctx.front.prog, t)),
    )
    .with_decl(ctx.front.decl_path(&ctx.front.app().info))
}

/// The name of a reader fn for `t`, generated on first use.
fn reader(ctx: &mut Ctx<'_, '_>, t: &T, state: Option<StateId>, need: &mut Need) -> R<proc_macro2::Ident> {
    let name = fn_name(ctx, t, "read");
    let tag = (t.clone(), "read");
    if need.done.contains(&tag.0) && need.fns.iter().any(|f| f.to_string().contains(&format!("fn {name} "))) {
        return Ok(name);
    }
    let rty = ctx.ty(t);
    let body = match t {
        T::Int => quote!(cursor.integer()),
        T::Text => quote!(cursor.string().map(std::borrow::Cow::into_owned)),
        T::Named(id) => {
            let tn = ctx.ty_name(*id);
            let tname = Literal::string(&ctx.front.prog.ty(*id).info.name);
            match &ctx.front.prog.ty(*id).def {
                TypeDef::Sum(_) => quote!(#tn::from_name(&cursor.string()?).ok_or(Unreadable::Variant(#tname))),
                TypeDef::Alias(_) => match alias_base(ctx.front.prog, *id).map(|(b, _)| b) {
                    Some(T::Int) => quote!(#tn::new(cursor.integer()?).ok_or(Unreadable::Claim(#tname))),
                    Some(T::Text) => quote!(#tn::new(cursor.string()?).ok_or(Unreadable::Claim(#tname))),
                    _ => return Err(refuse(ctx, t, "an alias of this base")),
                },
                TypeDef::Record(fs) => {
                    let mut slots = Vec::new();
                    let mut arms = Vec::new();
                    let mut names = Vec::new();
                    let t2 = crate::front::typer::Typer::new(ctx.front.prog, String::new());
                    for f in fs {
                        let ft = t2.ty(&f.ty, f.span)?;
                        let n = ident(&f.name);
                        let key = Literal::string(&f.name);
                        let r = reader(ctx, &ft, None, need)?;
                        slots.push(quote!(let mut #n = None;));
                        arms.push(quote!(#key => set(&mut #n, #r(cursor)?),));
                        names.push(n);
                    }
                    quote! {{
                        #(#slots)*
                        cursor.object(|cursor, key| match key {
                            #(#arms)*
                            _ => Err(Unreadable::UnknownKey),
                        })?;
                        let (#(Some(#names)),*) = (#(#names),*) else {
                            return Err(Unreadable::MissingKey);
                        };
                        Ok(#tn { #(#names),* })
                    }}
                }
                _ => return Err(refuse(ctx, t, "a type of this kind")),
            }
        }
        T::Set(x) => {
            let r = reader(ctx, x, None, need)?;
            let name = Literal::string(&type_name(ctx.front.prog, x));
            let s = ctx.std("collections::BTreeSet");
            quote! {{
                let mut set = #s::new();
                cursor.array(|cursor| {
                    if set.insert(#r(cursor)?) { Ok(()) } else { Err(Unreadable::Twice(#name)) }
                })?;
                Ok(set)
            }}
        }
        T::Map(k, v) => {
            let field = state.and_then(|s| ctx.front.facts.keyed.get(&s).copied());
            let (Some(field), T::Named(rec)) = (field, &**v) else {
                return Err(refuse(ctx, t, "a map whose values do not carry their key"));
            };
            let fname = crate::front::record_fields(ctx.front.prog, *rec)
                .and_then(|fs| fs.get(field))
                .map(|f| ident(&f.name))
                .ok_or_else(|| refuse(ctx, t, "the key field"))?;
            let r = reader(ctx, v, None, need)?;
            let name = Literal::string(&type_name(ctx.front.prog, k));
            let m = ctx.std("collections::BTreeMap");
            quote! {{
                let mut map = #m::new();
                cursor.array(|cursor| {
                    let value = #r(cursor)?;
                    match map.insert(value.#fname, value) {
                        None => Ok(()),
                        Some(_) => Err(Unreadable::Twice(#name)),
                    }
                })?;
                Ok(map)
            }}
        }
        _ => return Err(refuse(ctx, t, "no JSON form is defined for it")),
    };
    need.done.push(tag.0);
    need.fns.push(quote! {
        fn #name(cursor: &mut Cursor<'_>) -> Result<#rty, Unreadable> {
            #body
        }
    });
    Ok(name)
}

/// Statements that write `value` (a place of type `t`) into `w`.
fn writer(ctx: &mut Ctx<'_, '_>, t: &T, value: TokenStream, state: Option<StateId>, need: &mut Need) -> R<TokenStream> {
    Ok(match t {
        T::Int => quote!(w.int(#value)?;),
        T::Text => quote!(w.string(&#value);),
        T::Named(id) => match &ctx.front.prog.ty(*id).def {
            TypeDef::Sum(_) => quote!(w.string(#value.name());),
            TypeDef::Alias(_) => match alias_base(ctx.front.prog, *id).map(|(b, _)| b) {
                Some(T::Int) => quote!(w.int(#value.get())?;),
                Some(T::Text) => quote!(w.string(#value.as_str());),
                _ => return Err(refuse(ctx, t, "an alias of this base")),
            },
            TypeDef::Record(fs) => {
                let name = fn_name(ctx, t, "write");
                if !need.fns.iter().any(|f| f.to_string().contains(&format!("fn {name} "))) {
                    let tn = ctx.ty_name(*id);
                    let t2 = crate::front::typer::Typer::new(ctx.front.prog, String::new());
                    let mut parts = Vec::new();
                    for (i, f) in fs.iter().enumerate() {
                        let ft = t2.ty(&f.ty, f.span)?;
                        let n = ident(&f.name);
                        let lead = Literal::string(&format!("{}\"{}\":", if i == 0 { "{" } else { "," }, f.name));
                        let w = writer(ctx, &ft, quote!(value.#n), None, need)?;
                        parts.push(quote!(w.raw(#lead); #w));
                    }
                    need.fns.push(quote! {
                        fn #name(w: &mut Writer, value: &#tn) -> Result<(), StoreError> {
                            #(#parts)*
                            w.raw("}");
                            Ok(())
                        }
                    });
                }
                quote!(#name(w, &#value)?;)
            }
            _ => return Err(refuse(ctx, t, "a type of this kind")),
        },
        T::Set(x) | T::List(x) => {
            let item = writer(ctx, x, quote!(*item), None, need)?;
            let item = if **x == T::Int { item } else { strip_deref_for_methods(item) };
            quote!(w.array(&#value, |w, item| { #item Ok(()) })?;)
        }
        T::Map(_, v) => {
            if state.and_then(|s| ctx.front.facts.keyed.get(&s)).is_none() {
                return Err(refuse(ctx, t, "a map whose values do not carry their key"));
            }
            let item = writer(ctx, v, quote!(*item), None, need)?;
            let item = strip_deref_for_methods(item);
            quote!(w.array(#value.values(), |w, item| { #item Ok(()) })?;)
        }
        _ => return Err(refuse(ctx, t, "no JSON form is defined for it")),
    })
}

/// `(*item).name()` reads better as `item.name()`: auto-deref does the rest.
fn strip_deref_for_methods(ts: TokenStream) -> TokenStream {
    let s = ts.to_string().replace("& * item", "item").replace("* item", "item");
    s.parse().unwrap_or(ts)
}

/// The fused read of a projected rule: the whole store is checked exactly as
/// `decode` checks it, but of the projected map's values only the fields the
/// rule reads are kept, in a key-ordered `Entries`; no other value is built.
fn projected_decoder(ctx: &mut Ctx<'_, '_>, index: usize, need: &mut Need) -> R<Option<TokenStream>> {
    let Some(p) = ctx.front.facts.rules[index].projection.clone() else { return Ok(None) };
    let Some((view, value)) = super::items::view_names(ctx, index) else { return Ok(None) };
    let rule = ctx.front.rules[index].id;
    let rname = ctx.front.prog.rule(rule).info.name.clone();
    let view_t = ctx.item(Item::Rule(rule), &view);
    let value_t = ctx.item(Item::Rule(rule), &value);
    let reader_name = format_ident!("{}Read", super::layout::camel(&rname));
    let decode_name = format_ident!("decode_{}", rname);
    let entry_name = format_ident!("read_{}_entry", rname);
    let states = states(ctx);
    let mut slots = Vec::new();
    let mut arms = Vec::new();
    let mut fields = Vec::new();
    let mut entry_fn = quote!();
    for (id, ty) in &states {
        let name = ctx.front.prog.state(*id).info.name.clone();
        let n = ident(&name);
        let key = Literal::string(&name);
        slots.push(quote!(let mut #n = None;));
        fields.push(n.clone());
        if *id != p.state {
            let read = reader(ctx, ty, Some(*id), need)?;
            arms.push(quote!(#key => set(&mut #n, #read(cursor)?),));
            continue;
        }
        let (T::Map(k, v), Some(keyed)) = (ty, ctx.front.facts.keyed.get(id).copied()) else {
            return Ok(None);
        };
        let T::Named(rec) = **v else { return Ok(None) };
        let kt = ctx.ty(k);
        let all = crate::front::record_fields(ctx.front.prog, rec).unwrap_or(&[]).to_vec_names();
        let t2 = crate::front::typer::Typer::new(ctx.front.prog, String::new());
        let mut fslots = Vec::new();
        let mut farms = Vec::new();
        let mut fnames = Vec::new();
        let mut kept = Vec::new();
        let mut pats = Vec::new();
        for (i, (fname, fty)) in all.iter().enumerate() {
            let fty = t2.ty(fty, ctx.front.prog.ty(rec).info.span)?;
            let f = ident(fname);
            let fkey = Literal::string(fname);
            let read = if i == keyed || p.fields.contains(&i) {
                reader(ctx, &fty, None, need)?
            } else {
                checker(ctx, &fty, need)?
            };
            fslots.push(quote!(let mut #f = None;));
            farms.push(quote!(#fkey => set(&mut #f, #read(cursor)?),));
            fnames.push(f.clone());
            if p.fields.contains(&i) || i == keyed {
                pats.push(quote!(Some(#f)));
            } else {
                pats.push(quote!(Some(())));
            }
            if p.fields.contains(&i) {
                kept.push(f);
            }
        }
        let keyf = ident(&all[keyed].0);
        let kname = Literal::string(&type_name(ctx.front.prog, k));
        entry_fn = quote! {
            /// One value of the map: every field checked, only the read ones kept.
            fn #entry_name(cursor: &mut Cursor<'_>) -> Result<(#kt, #value_t), Unreadable> {
                #(#fslots)*
                cursor.object(|cursor, key| match key {
                    #(#farms)*
                    _ => Err(Unreadable::UnknownKey),
                })?;
                let (#(#pats),*) = (#(#fnames),*) else {
                    return Err(Unreadable::MissingKey);
                };
                Ok((#keyf, #value_t { #(#kept),* }))
            }
        };
        let entries = ctx.runtime("lang::Entries");
        arms.push(quote!(#key => set(&mut #n, {
            let mut entries = Vec::new();
            cursor.array(|cursor| {
                entries.push(#entry_name(cursor)?);
                Ok(())
            })?;
            #entries::new(entries).ok_or(Unreadable::Twice(#kname))?
        }),));
    }
    let checks = invariant_checks(ctx)?;
    let doc = Literal::string(&format!(" The fused read for rule `{rname}`: {}.", p.licence));
    let _ = &mut entry_fn;
    Ok(Some(quote! {
        #[doc = #doc]
        #[derive(Debug)]
        pub struct #reader_name;

        impl Decode for #reader_name {
            type World = #view_t;

            fn decode(bytes: &[u8]) -> Result<#view_t, Unreadable> {
                #decode_name(bytes)
            }
        }

        /// Checks the whole store as `decode` does; builds only what the rule reads.
        pub fn #decode_name(bytes: &[u8]) -> Result<#view_t, Unreadable> {
            let mut cursor = Cursor::new(bytes)?;
            let mut version = None;
            #(#slots)*
            cursor.object(|cursor, key| match key {
                "version" => match cursor.integer()? {
                    1 => set(&mut version, ()),
                    _ => Err(Unreadable::Version),
                },
                #(#arms)*
                _ => Err(Unreadable::UnknownKey),
            })?;
            cursor.end()?;
            let (Some(()), #(Some(#fields)),*) = (version, #(#fields),*) else {
                return Err(Unreadable::MissingKey);
            };
            let world = #view_t { #(#fields),* };
            #(#checks)*
            Ok(world)
        }

        #entry_fn
    }))
}

/// A field's names and types, owned.
trait FieldNames {
    fn to_vec_names(&self) -> Vec<(String, spec_lang_ir::Ty)>;
}

impl FieldNames for [spec_lang_ir::Field] {
    fn to_vec_names(&self) -> Vec<(String, spec_lang_ir::Ty)> {
        self.iter().map(|f| (f.name.clone(), f.ty.clone())).collect()
    }
}

/// A function that checks a value of `t` where it stands and keeps nothing.
fn checker(ctx: &mut Ctx<'_, '_>, t: &T, need: &mut Need) -> R<proc_macro2::Ident> {
    let name = fn_name(ctx, t, "check");
    if need.fns.iter().any(|f| f.to_string().contains(&format!("fn {name} "))) {
        return Ok(name);
    }
    let body = match t {
        T::Named(id)
            if matches!(ctx.front.prog.ty(*id).def, TypeDef::Alias(_))
                && alias_base(ctx.front.prog, *id).is_some_and(|(b, _)| b == T::Text) =>
        {
            let tn = ctx.ty_name(*id);
            let tname = Literal::string(&ctx.front.prog.ty(*id).info.name);
            quote!(if #tn::admits(&cursor.string()?) { Ok(()) } else { Err(Unreadable::Claim(#tname)) })
        }
        T::Text => quote!(cursor.string().map(drop)),
        T::Set(x)
            if matches!(**x, T::Text)
                || matches!(**x, T::Named(id) if alias_base(ctx.front.prog, id).is_some_and(|(b, _)| b == T::Text)) =>
        {
            let check = match &**x {
                T::Named(id) => {
                    let tn = ctx.ty_name(*id);
                    let tname = Literal::string(&ctx.front.prog.ty(*id).info.name);
                    quote!(if !#tn::admits(&text) { return Err(Unreadable::Claim(#tname)); })
                }
                _ => quote!(),
            };
            let name = Literal::string(&type_name(ctx.front.prog, x));
            quote! {{
                // Texts compare as their values do, so duplicates are found on the borrowed
                // text. A canonical store lists them ascending: checked in passing, nothing
                // kept; any other order is collected and sorted once.
                let mut last: Option<std::borrow::Cow<'_, str>> = None;
                let mut unordered: Vec<std::borrow::Cow<'_, str>> = Vec::new();
                cursor.array(|cursor| {
                    let text = cursor.string()?;
                    #check
                    if !unordered.is_empty() {
                        unordered.push(text);
                    } else if last.as_ref().is_none_or(|before| *before < text) {
                        last = Some(text);
                    } else {
                        unordered.extend(last.take());
                        unordered.push(text);
                    }
                    Ok(())
                })?;
                unordered.sort_unstable();
                if unordered.windows(2).any(|pair| pair[0] == pair[1]) { Err(Unreadable::Twice(#name)) } else { Ok(()) }
            }}
        }
        _ => {
            // Scalars cost nothing to build: read them in full and drop them.
            let read = reader(ctx, t, None, need)?;
            quote!(#read(cursor).map(drop))
        }
    };
    need.fns.push(quote! {
        fn #name(cursor: &mut Cursor<'_>) -> Result<(), Unreadable> {
            #body
        }
    });
    Ok(name)
}
