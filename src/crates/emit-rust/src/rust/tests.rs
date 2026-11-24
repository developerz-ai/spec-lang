//! Tests generated from the spec's own tests (contract section 4):
//! every `@cases` row as a table test, every `test` by name (a test with
//! parameters is a seeded property), every `law` checked after every step of
//! seeded random traces of the rules it names. No mocks: rules run on a world.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use spec_lang_diag::Diag;
use spec_lang_ir::{ModuleId, TestId, TypeDef};

use super::layout::{Item, Layout, camel, ident, snake};
use super::lower::{Cat, Ctx, Mode};
use super::{Out, package, simple_file};
use crate::front::typed::*;
use crate::front::typer::{alias_base, type_name};
use crate::front::{Front, TGiven};
use crate::provenance::Origin;

/// Traces per law and steps per trace: at least the contract's 200 of 40.
const TRACES: u64 = 240;
const STEPS: usize = 40;
/// Runs of a property test.
const RUNS: u64 = 300;
/// Values of a law's other parameters drawn per trace.
const PER_TRACE: u64 = 8;

pub fn emit(front: &Front<'_>, layout: &Layout, out: &mut Out) -> Result<(), Diag> {
    let pkg = package(front);
    let mut gens = Gens::default();
    cases(front, layout, &pkg, out)?;
    let mut by_module: BTreeMap<ModuleId, Vec<TestId>> = BTreeMap::new();
    for t in &front.tests {
        by_module.entry(front.prog.test(t.id).info.module).or_default().push(t.id);
    }
    let given_tests: BTreeSet<TestId> = front
        .tests
        .iter()
        .flat_map(|t| t.given.iter())
        .filter_map(|g| match g {
            TGiven::Test { test, .. } => Some(*test),
            TGiven::State { .. } => None,
        })
        .collect();
    for (m, tests) in &by_module {
        scenario_file(front, layout, &pkg, *m, tests, &given_tests, &mut gens, out)?;
    }
    let mut law_modules: BTreeMap<ModuleId, Vec<usize>> = BTreeMap::new();
    for (i, l) in front.laws.iter().enumerate() {
        law_modules.entry(front.prog.law(l.id).info.module).or_default().push(i);
    }
    for (m, laws) in &law_modules {
        law_file(front, layout, &pkg, *m, laws, &given_tests, &mut gens, out)?;
    }
    support_file(front, layout, &pkg, &given_tests, &gens, out)
}

fn module_file_name(front: &Front<'_>, m: ModuleId) -> String {
    front.prog.module(m).name.rsplit('.').next().unwrap_or("tests").to_string()
}

/// Types the generated tests need generators for.
#[derive(Default)]
struct Gens {
    types: Vec<T>,
}

impl Gens {
    fn need(&mut self, t: &T) {
        match t {
            T::Option(x) | T::Set(x) | T::List(x) => self.need(x),
            _ => {}
        }
        if !self.types.contains(t) {
            self.types.push(t.clone());
        }
    }
}

fn gen_name(front: &Front<'_>, t: &T) -> proc_macro2::Ident {
    format_ident!("{}", gen_stem(front, t))
}

fn gen_stem(front: &Front<'_>, t: &T) -> String {
    match t {
        T::Bool => "bool".into(),
        T::Int => "int".into(),
        T::Text => "text".into(),
        T::Named(id) => snake(&front.prog.ty(*id).info.name),
        T::Option(x) => format!("{}_option", gen_stem(front, x)),
        T::Set(x) => format!("{}_set", gen_stem(front, x)),
        T::List(x) => format!("{}_list", gen_stem(front, x)),
        _ => "value".into(),
    }
}

/// `@cases` of every fn, one test file per module that has some.
fn cases(front: &Front<'_>, layout: &Layout, pkg: &str, out: &mut Out) -> Result<(), Diag> {
    let mut by_module: BTreeMap<String, Vec<&crate::front::TFn>> = BTreeMap::new();
    for f in &front.fns {
        if !f.cases.is_empty()
            && let Some(m) = layout.module_of(Item::Fn(f.id))
        {
            by_module.entry(m.to_string()).or_default().push(f);
        }
    }
    for (m, fns) in by_module {
        let file = format!("{m}_cases");
        let mut ctx = Ctx::new(front, layout, ("tests".into(), file.clone()), pkg, Mode::Test);
        let mut tests = Vec::new();
        let mut names = Vec::new();
        let mut first_at = String::new();
        for f in fns {
            let info = &front.prog.func(f.id).info;
            names.push(info.name.clone());
            ctx.decl = front.decl_path(info);
            if first_at.is_empty() {
                first_at = front.at(info.span);
            }
            let fname = ident(&info.name);
            let fid = ctx.item(Item::Fn(f.id), &info.name);
            let mut rows = Vec::new();
            for (i, c) in f.cases.iter().enumerate() {
                ctx.reset_scope();
                let role = format!("case[{i}]");
                let origin = Origin::new(front.decl_path(info), front.at(c.span), &role);
                let anchor = ctx.anchor(c.span, &role, origin);
                let call =
                    TExpr { kind: K::CallFn { f: f.id, args: c.args.clone() }, ty: f.result.clone(), span: c.span };
                let check = match &c.expect.kind {
                    K::Bool(true) => {
                        let v = ctx.expr(&call)?;
                        let v = v.ts;
                        quote!(assert!(#v);)
                    }
                    K::Bool(false) => {
                        let v = ctx.expr(&call)?;
                        let v = v.ts;
                        quote!(assert!(!#v);)
                    }
                    _ => {
                        let v = ctx.expr(&call)?.ts;
                        let e = ctx.expr(&c.expect)?;
                        let e = if f.result == T::Text { e.ts } else { ctx.owned(e)? };
                        quote!(assert_eq!(#v, #e);)
                    }
                };
                rows.push(quote!(#anchor #check));
            }
            let _ = fid;
            let tname = format_ident!("{}_cases", fname);
            let doc = Literal::string(&format!(" Every `@cases` row of `fn {}`.", info.name));
            tests.push(quote! {
                #[doc = #doc]
                #[test]
                fn #tname() {
                    #(#rows)*
                }
            });
        }
        let anchors = ctx.anchors.clone();
        let origin = Origin::new(
            format!(
                "{}.cases",
                front.prog.module(layout.modules.iter().find(|x| x.name == m).map_or(ModuleId(0), |x| x.module)).name
            ),
            first_at,
            "@cases table tests",
        );
        let buf = simple_file(
            &format!("tests/{file}.rs"),
            &format!("the @cases of fn {}", names.join(", ")),
            origin,
            quote!(#(#tests)*),
            &anchors,
            &ctx.uses,
            Some("#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]\n"),
        )?;
        out.add(buf);
    }
    Ok(())
}

/// The world a test starts from, and its setup statements.
fn setup(ctx: &mut Ctx<'_, '_>, given: &[TGiven], needs_mut: bool) -> Result<TokenStream, Diag> {
    let world_t = ctx.item(Item::World, "World");
    let mut fields = Vec::new();
    let mut base = None;
    for g in given {
        match g {
            TGiven::Test { test, .. } => {
                let name = ident(&ctx.front.prog.test(*test).info.name);
                ctx.uses.insert("support".into());
                base = Some(quote!(support::#name()));
            }
            TGiven::State { state, value, .. } => {
                let s = ident(&ctx.front.prog.state(*state).info.name);
                let v = ctx.expr(value)?;
                let v = ctx.owned(v)?;
                fields.push((s, v));
            }
        }
    }
    let m = if needs_mut { quote!(mut) } else { quote!() };
    let every_state = fields.len() == ctx.front.states.len();
    Ok(match (base, fields.is_empty()) {
        (None, true) => quote!(let #m world = #world_t::default();),
        (None, false) => {
            let (names, values): (Vec<_>, Vec<_>) = fields.into_iter().unzip();
            let rest = if every_state { quote!() } else { quote!(..#world_t::default()) };
            quote!(let #m world = #world_t { #(#names: #values,)* #rest };)
        }
        (Some(b), true) => quote!(let #m world = #b;),
        (Some(b), false) => {
            let assigns = fields.into_iter().map(|(n, v)| quote!(world.#n = #v;));
            quote!(let mut world = #b; #(#assigns)*)
        }
    })
}

/// Does a body write the world, call a rule, or read `out`?
fn body_needs(front: &Front<'_>, layout: &Layout, body: &[TStmt]) -> (bool, bool, bool) {
    let (mut writes, mut calls, mut reads_out) = (false, false, false);
    for s in body {
        crate::front::facts::stmt_exprs(s, &mut |e| scan(front, layout, e, &mut writes, &mut calls, &mut reads_out));
    }
    (writes, calls, reads_out)
}

fn scan(front: &Front<'_>, layout: &Layout, e: &TExpr, writes: &mut bool, calls: &mut bool, reads_out: &mut bool) {
    match &e.kind {
        K::CallRule { rule, .. } => {
            *calls = true;
            let i = front.rules.iter().position(|r| r.id == *rule);
            if i.is_some_and(|i| !layout.read_only(front, i)) {
                *writes = true;
            }
        }
        K::OutText => *reads_out = true,
        _ => {}
    }
    crate::front::facts::children(e, &mut |c| scan(front, layout, c, writes, calls, reads_out));
}

#[allow(clippy::too_many_arguments)]
fn scenario_file(
    front: &Front<'_>,
    layout: &Layout,
    pkg: &str,
    module: ModuleId,
    tests: &[TestId],
    given_tests: &BTreeSet<TestId>,
    gens: &mut Gens,
    out: &mut Out,
) -> Result<(), Diag> {
    let file = module_file_name(front, module);
    let mut ctx = Ctx::new(front, layout, ("tests".into(), file.clone()), pkg, Mode::Test);
    let mut items = Vec::new();
    let mut names = Vec::new();
    for id in tests {
        let t = front.test(*id).ok_or_else(|| Diag::unsupported(front.prog.test(*id).info.span, "an untyped test"))?;
        let info = &front.prog.test(*id).info;
        names.push(info.name.clone());
        ctx.decl = front.decl_path(info);
        ctx.reset_scope();
        ctx.fallible = false;
        let (writes, calls, reads_out) = body_needs(front, layout, &t.body);
        if given_tests.contains(id) && t.body.is_empty() && t.params.is_empty() {
            // A named start world: its test checks that it is one (the store's rules would admit it).
            let name = ident(&info.name);
            let doc =
                Literal::string(&format!(" `test {}`: its world is a world the rules and laws allow.", info.name));
            ctx.uses.insert(format!("{pkg}::store::codec"));
            items.push(quote! {
                #[doc = #doc]
                #[test]
                fn #name() {
                    let world = support::#name();
                    let bytes = codec::encode(&world).expect("the world fits the store");
                    assert_eq!(codec::decode(&bytes).expect("the store reads it back"), world);
                }
            });
            ctx.uses.insert("support".into());
            continue;
        }
        ctx.count_uses(&t.body, &[]);
        let world = setup(&mut ctx, &t.given, writes)?;
        let out_decl = if calls {
            let o = ctx.runtime("out::Out");
            quote!(let mut out = #o::default();)
        } else if reads_out {
            let o = ctx.runtime("out::Out");
            quote!(let out = #o::default();)
        } else {
            quote!()
        };
        let world_decl = if calls || writes || body_reads_world(&t.body) { world } else { quote!() };
        let name = ident(&info.name);
        let doc = Literal::string(&format!(" `test {}` ({}).", info.name, front.at(info.span)));
        if t.params.is_empty() {
            let body = ctx.body(&t.body, false)?;
            items.push(quote! {
                #[doc = #doc]
                #[test]
                fn #name() {
                    #world_decl
                    #out_decl
                    #body
                }
            });
        } else {
            // A property: parameters generated from their types, seeded.
            let mut params = Vec::new();
            let mut draws = Vec::new();
            let mut args = Vec::new();
            for p in &t.params {
                gens.need(&p.ty);
                let g = gen_name(front, &p.ty);
                let pn = ident(&p.name);
                let pt = ctx.ty(&p.ty);
                ctx.bind(
                    &p.name,
                    if ctx.copy(&p.ty) { Cat::Val } else { Cat::Place { movable: true } },
                    None,
                    usize::MAX,
                );
                params.push(quote!(#pn: #pt));
                draws.push(quote!(let #pn = support::#g(&mut rng);));
                args.push(quote!(#pn));
            }
            ctx.uses.insert("support".into());
            let body = ctx.body(&t.body, false)?;
            let runs = Literal::u64_unsuffixed(RUNS);
            let pnames: Vec<_> = t.params.iter().map(|p| ident(&p.name)).collect();
            let shown = Literal::string(
                &t.params.iter().map(|p| format!("{} = {{:?}}", p.name)).collect::<Vec<_>>().join(", "),
            );
            let _ = shown;
            items.push(quote! {
                #[doc = #doc]
                #[test]
                fn #name() {
                    fn case(#(#params),*) {
                        #world_decl
                        #out_decl
                        #body
                    }
                    let mut rng = support::Rng::new(0);
                    for _ in 0..#runs {
                        #(#draws)*
                        case(#(#pnames),*);
                    }
                }
            });
        }
    }
    let info = &front.prog.module(module);
    let origin = Origin::new(info.name.clone(), format!("{}:1:1", info.path), "scenario tests");
    let mut uses = ctx.uses.clone();
    uses.remove("support");
    let mut prelude =
        String::from("#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]\n\nmod support;\n");
    if !ctx.uses.contains("support") {
        prelude = "#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]\n".into();
    }
    let buf = simple_file(
        &format!("tests/{file}.rs"),
        &format!("test {}", names.join(", ")),
        origin,
        quote!(#(#items)*),
        &ctx.anchors,
        &uses,
        Some(&prelude),
    )?;
    out.add(buf);
    Ok(())
}

fn body_reads_world(body: &[TStmt]) -> bool {
    let mut hit = false;
    for s in body {
        crate::front::facts::stmt_exprs(s, &mut |e| hit |= has_state(e));
    }
    hit
}

fn has_state(e: &TExpr) -> bool {
    if matches!(e.kind, K::State(_)) {
        return true;
    }
    let mut hit = false;
    crate::front::facts::children(e, &mut |c| hit |= has_state(c));
    hit
}

#[allow(clippy::too_many_arguments)]
fn law_file(
    front: &Front<'_>,
    layout: &Layout,
    pkg: &str,
    module: ModuleId,
    laws: &[usize],
    given_tests: &BTreeSet<TestId>,
    gens: &mut Gens,
    out: &mut Out,
) -> Result<(), Diag> {
    let file = module_file_name(front, module);
    let mut ctx = Ctx::new(front, layout, ("tests".into(), file.clone()), pkg, Mode::Test);
    let world_t = ctx.item(Item::World, "World");
    let cmd = ctx.item(Item::Command, "Command");
    let refusal = ctx.item(Item::Refusal, "Refusal");
    let out_t = ctx.runtime("out::Out");
    let apply = ctx.item(Item::Command, "apply");
    // Every rule's event, with generated arguments.
    let mut arms = Vec::new();
    for (i, r) in front.rules.iter().enumerate() {
        let v = format_ident!("{}", camel(&front.prog.rule(r.id).info.name));
        let n = Literal::u8_unsuffixed(i as u8);
        if r.params.is_empty() {
            arms.push(quote!(#n => #cmd::#v,));
            continue;
        }
        let mut fields = Vec::new();
        for p in &r.params {
            gens.need(&p.ty);
            let g = gen_name(front, &p.ty);
            let pn = ident(&p.name);
            fields.push(quote!(#pn: support::#g(rng)));
        }
        arms.push(quote!(#n => #cmd::#v { #(#fields),* },));
    }
    let starts: Vec<_> = given_tests.iter().map(|t| ident(&front.prog.test(*t).info.name)).collect();
    let nstarts = Literal::u64_unsuffixed(starts.len() as u64 + 1);
    let start_arms: Vec<_> = starts
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let n = Literal::u64_unsuffixed(i as u64 + 1);
            quote!(#n => support::#s(),)
        })
        .collect();
    let traces = Literal::u64_unsuffixed(TRACES);
    let steps = Literal::usize_unsuffixed(STEPS);
    let mut tests = Vec::new();
    let mut names = Vec::new();
    for &li in laws {
        let law = &front.laws[li];
        let info = &front.prog.law(law.id).info;
        names.push(info.name.clone());
        ctx.decl = front.decl_path(info);
        ctx.reset_scope();
        let rules: Vec<_> = law
            .rules
            .iter()
            .filter_map(|r| front.rules.iter().position(|x| x.id == *r))
            .map(|i| Literal::u8_unsuffixed(i as u8))
            .collect();
        let tn = ident(&law.trace);
        ctx.bind(&law.trace, Cat::Place { movable: false }, None, usize::MAX);
        let mut draws = Vec::new();
        for p in &law.others {
            gens.need(&p.ty);
            let g = gen_name(front, &p.ty);
            let pn = ident(&p.name);
            ctx.bind(&p.name, if ctx.copy(&p.ty) { Cat::Val } else { Cat::Place { movable: false } }, None, usize::MAX);
            draws.push(quote!(let #pn = support::#g(&mut rng);));
        }
        let origin = Origin::new(front.decl_path(info), front.at(info.span), "law body");
        let anchor = ctx.anchor(info.span, "law", origin);
        let body = ctx.expr(&law.body)?;
        let body = ctx.val(body);
        let name = ident(&info.name);
        let doc = Literal::string(&format!(
            " `law {}` ({}), after every step of {TRACES} seeded traces of {STEPS} calls.",
            info.name,
            front.at(info.span)
        ));
        let check = if law.others.is_empty() {
            let msg = Literal::string(&format!("law {} broken by the trace of seed {{seed}}", info.name));
            quote!(assert!(#body, #msg);)
        } else {
            let per = Literal::u64_unsuffixed(PER_TRACE);
            let shown =
                law.others.iter().map(|p| format!("{} = {{{}:?}}", p.name, p.name)).collect::<Vec<_>>().join(", ");
            let msg = Literal::string(&format!("law {} broken by the trace of seed {{seed}} with {shown}", info.name));
            quote! {
                let mut rng = support::Rng::new(seed ^ 0x5EED);
                for _ in 0..#per {
                    #(#draws)*
                    assert!(#body, #msg);
                }
            }
        };
        tests.push(quote! {
            #[doc = #doc]
            #[test]
            fn #name() {
                for seed in 0..#traces {
                    let #tn = trace(seed, &[#(#rules),*]);
                    #anchor
                    #check
                }
            }
        });
    }
    // Event parameters the laws read: one accessor each.
    let mut accessors = Vec::new();
    for name in ctx.event_args.clone() {
        let f = format_ident!("event_{}", name);
        let pn = ident(&name);
        let mut pats = Vec::new();
        let mut ty = None;
        for r in &front.rules {
            if let Some(p) = r.params.iter().find(|p| p.name == name) {
                let v = format_ident!("{}", camel(&front.prog.rule(r.id).info.name));
                pats.push(quote!(#cmd::#v { #pn, .. }));
                ty = Some(p.ty.clone());
            }
        }
        let Some(ty) = ty else { continue };
        let rt = ctx.ty(&ty);
        let take = if ctx.copy(&ty) { quote!(*#pn) } else { quote!(#pn.clone()) };
        let msg = Literal::string(&format!("the law's trace holds only events with `{name}`"));
        accessors.push(quote! {
            /// The `{name}` of an event; a law reads it only on events that have it.
            fn #f(event: &#cmd) -> #rt {
                match event {
                    #(#pats)|* => #take,
                    _ => unreachable!(#msg),
                }
            }
        });
    }
    let module_info = front.prog.module(module);
    let origin = Origin::new(module_info.name.clone(), format!("{}:1:1", module_info.path), "law tests");
    let ts = quote! {
        /// One element of `t.steps`.
        #[derive(Debug)]
        struct Step {
            event: #cmd,
            reply: Result<(), #refusal>,
            before: #world_t,
            after: #world_t,
        }

        /// A trace: every world, the first included, and every step between them.
        #[derive(Debug)]
        struct Trace {
            states: Vec<#world_t>,
            steps: Vec<Step>,
        }

        /// A seeded trace of the given rules (indexes into the app's rule list),
        /// from the initial world or from a named test world.
        fn trace(seed: u64, rules: &[u8]) -> Trace {
            let mut rng = support::Rng::new(seed);
            let mut world = match seed % #nstarts {
                #(#start_arms)*
                _ => #world_t::default(),
            };
            let mut t = Trace { states: vec![world.clone()], steps: Vec::with_capacity(#steps) };
            for _ in 0..#steps {
                let index = *rng.pick(rules);
                let event = event(&mut rng, index);
                let before = world.clone();
                let reply = #apply(&mut world, &mut #out_t::default(), event.clone());
                t.states.push(world.clone());
                t.steps.push(Step { event, reply, before, after: world.clone() });
            }
            t
        }

        /// One call of the rule at `index`, its arguments generated from their types.
        fn event(rng: &mut support::Rng, index: u8) -> #cmd {
            match index {
                #(#arms)*
                _ => unreachable!("an index into the app's rules"),
            }
        }

        #(#accessors)*

        #(#tests)*
    };
    let buf = simple_file(
        &format!("tests/{file}.rs"),
        &format!("law {}", names.join(", ")),
        origin,
        ts,
        &ctx.anchors,
        &ctx.uses,
        Some("#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]\n\nmod support;\n"),
    )?;
    out.add(buf);
    Ok(())
}

/// `tests/support/mod.rs`: named test worlds and one generator per type.
fn support_file(
    front: &Front<'_>,
    layout: &Layout,
    pkg: &str,
    given_tests: &BTreeSet<TestId>,
    gens: &Gens,
    out: &mut Out,
) -> Result<(), Diag> {
    let mut ctx = Ctx::new(front, layout, ("tests".into(), "support".into()), pkg, Mode::Test);
    let mut items = Vec::new();
    let world_t = ctx.item(Item::World, "World");
    for id in given_tests {
        let t = front.test(*id).ok_or_else(|| Diag::unsupported(front.prog.test(*id).info.span, "an untyped test"))?;
        let info = &front.prog.test(*id).info;
        ctx.decl = front.decl_path(info);
        ctx.reset_scope();
        if !t.params.is_empty() {
            return Err(
                Diag::unsupported(info.span, "a property test used as a start world").with_decl(ctx.decl.clone())
            );
        }
        ctx.count_uses(&t.body, &[]);
        let (writes, calls, _) = body_needs(front, layout, &t.body);
        let world = setup(&mut ctx, &t.given, writes)?;
        let out_decl = if calls {
            let o = ctx.runtime("out::Out");
            quote!(let mut out = #o::default();)
        } else {
            quote!()
        };
        let body = ctx.body(&t.body, false)?;
        let name = ident(&info.name);
        let doc = Literal::string(&format!(" The end world of `test {}` ({}).", info.name, front.at(info.span)));
        // With nothing to run, the world is the value: `let world = ...; world` says nothing more.
        let fn_body = if t.body.is_empty() { strip_let_world(world) } else { quote!(#world #out_decl #body world) };
        items.push(quote! {
            #[doc = #doc]
            pub fn #name() -> #world_t {
                #fn_body
            }
        });
    }
    for t in &gens.types {
        items.push(generator(&mut ctx, t)?);
    }
    let origin = Origin::new(
        front.decl_path(&front.app().info),
        front.at(front.app().info.span),
        "test support: named worlds and generators",
    )
    .strategy("gen.by-type", "a parameter's values come from its type: variants, claims over a fixed text pool, small numbers near the claim's bound");
    let buf = simple_file(
        "tests/support/mod.rs",
        "the generators behind property and law parameters, and the tests used as @given",
        origin,
        quote!(#(#items)*),
        &ctx.anchors,
        &ctx.uses,
        Some(
            "#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]\n\nmod rng;\n\npub use rng::{Rng, TEXTS};\n",
        ),
    )?;
    out.add(buf);
    Ok(())
}

fn generator(ctx: &mut Ctx<'_, '_>, t: &T) -> Result<TokenStream, Diag> {
    let front = ctx.front;
    let name = gen_name(front, t);
    let rt = ctx.ty(t);
    let span = front.app().info.span;
    let body = match t {
        T::Bool => quote!(rng.below(2) == 0),
        T::Int => quote!(rng.below(16) as i64),
        T::Text => quote!((*rng.pick(TEXTS)).to_owned()),
        T::Option(x) => {
            let g = gen_name(front, x);
            quote!(if rng.below(2) == 0 { None } else { Some(#g(rng)) })
        }
        T::Set(x) => {
            let g = gen_name(front, x);
            quote!((0..rng.below(4)).map(|_| #g(rng)).collect())
        }
        T::List(x) => {
            let g = gen_name(front, x);
            quote!((0..rng.below(4)).map(|_| #g(rng)).collect())
        }
        T::Named(id) => match &front.prog.ty(*id).def {
            TypeDef::Sum(_) => quote!(*rng.pick(&#rt::ALL)),
            TypeDef::Alias(_) => match alias_base(front.prog, *id) {
                Some((T::Int, claim)) => {
                    let lo = claim.and_then(|c| crate::front::typer::int_claim_lower_bound(&c.expr)).unwrap_or(0);
                    let lo = Literal::i64_unsuffixed(lo as i64);
                    let msg = Literal::string(&format!("a {}", type_name(front.prog, t)));
                    quote! {
                        // Mostly near the claim's bound, where rules find things; now and then far away.
                        let value = match rng.below(20) {
                            0 => i64::MAX,
                            1 => (rng.next() >> 1) as i64,
                            _ => #lo + rng.below(16) as i64,
                        };
                        #rt::new(value.max(#lo)).expect(#msg)
                    }
                }
                Some((T::Text, _)) => {
                    let msg = Literal::string(&format!("E-GEN: no text in the pool is a {}", type_name(front.prog, t)));
                    quote! {
                        let admitted: Vec<&str> = TEXTS.iter().copied().filter(|text| #rt::new(*text).is_some()).collect();
                        assert!(!admitted.is_empty(), #msg);
                        #rt::new(*rng.pick(&admitted)).expect("admitted above")
                    }
                }
                _ => return Err(Diag::unsupported(span, format!("generating a {}", type_name(front.prog, t)))),
            },
            _ => return Err(Diag::unsupported(span, format!("generating a {}", type_name(front.prog, t)))),
        },
        _ => return Err(Diag::unsupported(span, format!("generating a {}", type_name(front.prog, t)))),
    };
    let doc = Literal::string(&format!(" A generated {}.", type_name(front.prog, t)));
    Ok(quote! {
        #[doc = #doc]
        pub fn #name(rng: &mut Rng) -> #rt {
            #body
        }
    })
}

/// `let world = X;` as `X`.
fn strip_let_world(ts: TokenStream) -> TokenStream {
    use proc_macro2::TokenTree;
    let mut toks: Vec<TokenTree> = ts.into_iter().collect();
    let semis = toks.iter().filter(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ';')).count();
    if semis != 1 {
        let ts: TokenStream = toks.into_iter().collect();
        return quote!(#ts world);
    }
    let eq = toks.iter().position(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == '='));
    if let (Some(eq), Some(TokenTree::Punct(p))) = (eq, toks.last())
        && p.as_char() == ';'
    {
        toks.pop();
        return toks.into_iter().skip(eq + 1).collect();
    }
    toks.into_iter().collect()
}
