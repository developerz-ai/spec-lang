//! `cli/table.rs`: the parameter table and help text derived from the rules'
//! signatures (pocs/todo-app/docs/cli-vocabulary.md). Static: the table and
//! the help cost nothing at run time.

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use spec_lang_ir::TypeDef;

use super::layout::{Item, camel, ident, snake};
use super::lower::{Ctx, R};
use crate::front::cli::{self as fcli, FlagKind};
use crate::front::typed::{K, T};
use crate::front::typer::alias_base;
use crate::provenance::Origin;

fn variant(ctx: &Ctx<'_, '_>, t: &T) -> proc_macro2::Ident {
    match t {
        T::Named(id) => ident(&ctx.front.prog.ty(*id).info.name),
        T::Int => ident("Int"),
        T::Text => ident("Text"),
        _ => ident("Value"),
    }
}

fn accessor(ctx: &Ctx<'_, '_>, t: &T) -> proc_macro2::Ident {
    format_ident!("{}", snake(&variant(ctx, t).to_string()))
}

/// Returns the items and, per command, the line where its table row starts (for provenance).
pub fn table(ctx: &mut Ctx<'_, '_>) -> R<(TokenStream, Vec<(usize, Origin)>)> {
    let types = fcli::value_types(&ctx.front.cli);
    let mut variants = Vec::new();
    let mut values = Vec::new();
    let mut names = Vec::new();
    let mut parses = Vec::new();
    let mut accessors = Vec::new();
    for t in &types {
        let v = variant(ctx, t);
        let rt = ctx.ty(t);
        let a = accessor(ctx, t);
        let tname = Literal::string(&fcli::value_name(ctx.front, t));
        variants.push(quote!(#v));
        values.push(quote!(#v(#rt)));
        names.push(quote!(Self::#v => #tname));
        let parse = match t {
            T::Int => quote!(int(&text)),
            T::Text => quote!(Some(text)),
            T::Named(id) => match &ctx.front.prog.ty(*id).def {
                TypeDef::Sum(vs) => {
                    let tn = ctx.ty_name(*id);
                    let lower: Vec<_> = vs.iter().map(|v| Literal::string(&v.name.to_lowercase())).collect();
                    let vn: Vec<_> = vs.iter().map(|v| ident(&v.name)).collect();
                    quote!(match text.as_str() { #(#lower => Some(#tn::#vn),)* _ => None })
                }
                TypeDef::Alias(_) => {
                    let tn = ctx.ty_name(*id);
                    match alias_base(ctx.front.prog, *id).map(|(b, _)| b) {
                        Some(T::Int) => quote!(int(&text).and_then(#tn::new)),
                        Some(T::Text) => quote!(#tn::new(text)),
                        _ => {
                            return Err(ctx.unsupported(
                                ctx.front.app().info.span,
                                "a parameter of this type on the command line",
                            ));
                        }
                    }
                }
                _ => return Err(ctx.unsupported(ctx.front.app().info.span, "a record parameter on the command line")),
            },
            _ => return Err(ctx.unsupported(ctx.front.app().info.span, "a parameter of this type on the command line")),
        };
        parses.push(quote!(Self::#v => #parse.map(Value::#v)));
        accessors.push(quote! {
            pub fn #a(self) -> Option<#rt> {
                match self { Self::#v(value) => Some(value), _ => None }
            }
        });
    }
    let cmd_t = ctx.item(Item::Command, "Command");
    let mut rows = Vec::new();
    let mut origins = Vec::new();
    let cli = ctx.front.cli.clone();
    for (i, c) in cli.iter().enumerate() {
        let decl = ctx.front.prog.rule(c.rule);
        let rule = ctx.front.rule(c.rule).ok_or_else(|| ctx.unsupported(c.span, "a cli rule outside the app"))?;
        let params = rule.params.clone();
        let at = ctx.front.at(decl.info.span);
        let derivation = c
            .flags
            .iter()
            .map(|f| format!("--{}: {}", f.name, f.derivation))
            .chain(c.positionals.iter().map(|p| format!("<{}>: a parameter without a default", p.name)))
            .collect::<Vec<_>>()
            .join("; ");
        origins.push((
            i,
            Origin::new(ctx.front.decl_path(&decl.info), at, "cli param table row")
                .strategy("cli.derive", if derivation.is_empty() { "no parameters".into() } else { derivation }),
        ));
        let name = Literal::string(&c.name);
        let pos: Vec<_> = c
            .positionals
            .iter()
            .map(|p| {
                let n = Literal::string(&p.name);
                let v = variant(ctx, &p.ty);
                quote!(Positional { name: #n, value: Type::#v })
            })
            .collect();
        let flags: Vec<_> = c
            .flags
            .iter()
            .map(|f| {
                let n = Literal::string(&f.name);
                let kind = match &f.kind {
                    FlagKind::Switch => quote!(FlagKind::Switch),
                    FlagKind::Single(t) => {
                        let v = variant(ctx, t);
                        quote!(FlagKind::Single(Type::#v))
                    }
                    FlagKind::Set(t) => {
                        let v = variant(ctx, t);
                        quote!(FlagKind::Set(Type::#v))
                    }
                };
                quote!(Flag { name: #n, kind: #kind })
            })
            .collect();
        let v = format_ident!("{}", camel(&decl.info.name));
        let mut binds = Vec::new();
        for (pi, p) in params.iter().enumerate() {
            let pn = ident(&p.name);
            let n = Literal::string(&p.name);
            let bind = if c.positionals.iter().any(|x| x.param == pi) {
                let a = accessor(ctx, &p.ty);
                quote!(required(given.take(#n).and_then(Value::#a), #n)?)
            } else {
                let Some(f) = c.flags.iter().find(|f| f.param == pi) else { continue };
                match &f.kind {
                    FlagKind::Switch => quote!(given.switch(#n)),
                    FlagKind::Set(t) => {
                        if !matches!(&p.default, Some(d) if matches!(&d.kind, K::SetOf(xs) if xs.is_empty())) {
                            return Err(ctx.unsupported(p.span, "a Set parameter whose default is not empty"));
                        }
                        let a = accessor(ctx, t);
                        quote!(given.take_all(#n).into_iter().filter_map(Value::#a).collect())
                    }
                    FlagKind::Single(t) => {
                        let a = accessor(ctx, t);
                        if matches!(p.ty, T::Option(_)) {
                            quote!(given.take(#n).and_then(Value::#a))
                        } else {
                            let d =
                                p.default.clone().ok_or_else(|| ctx.unsupported(p.span, "a flag without a default"))?;
                            ctx.reset_scope();
                            let d = ctx.expr(&d)?;
                            let d = ctx.owned(d)?;
                            quote!(given.take(#n).and_then(Value::#a).unwrap_or(#d))
                        }
                    }
                }
            };
            binds.push(quote!(#pn: #bind));
        }
        let uses_given = !binds.is_empty();
        let body = if params.is_empty() { quote!(Ok(#cmd_t::#v)) } else { quote!(Ok(#cmd_t::#v { #(#binds),* })) };
        let given = if uses_given { quote!(mut given) } else { quote!(_) };
        rows.push(quote! {
            CommandSpec {
                name: #name,
                positionals: &[#(#pos),*],
                flags: &[#(#flags),*],
                bind: |#given| #body,
            }
        });
    }
    let n = Literal::usize_unsuffixed(rows.len());
    let help = Literal::string(&fcli::help(ctx.front, ctx.front.app().bin.as_deref().unwrap_or("app")));
    let help_line = ctx.front.cli.len();
    let _ = help_line;
    Ok((
        quote! {
            /// The parameter types the app's rules take on the command line.
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub enum Type { #(#variants),* }

            /// One argument, parsed, its claim checked at the edge.
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub enum Value { #(#values),* }

            impl Value {
                #(#accessors)*
            }

            impl ValueType for Type {
                type Value = Value;

                fn name(self) -> &'static str {
                    match self { #(#names),* }
                }

                fn parse(self, text: String) -> Option<Value> {
                    match self { #(#parses),* }
                }
            }

            /// Every `cli` rule in the app's order, which is the order of `--help`.
            pub static COMMANDS: [CommandSpec<Type, #cmd_t>; #n] = [#(#rows),*];

            /// The help text, folded at build time from the table.
            pub const HELP: &str = #help;
        },
        origins,
    ))
}
