//! Declared types as code: one `conform` function per checked type (`conform.rs`), and one generator per type a
//! property, a law or a trace draws from (`gen.rs`), the engine's algorithm draw for draw.

use crate::cx::{Cx, RECORD};
use crate::expr::int_lit;
use crate::names::{camel, ident};
use spec_lang_lir::{DTy, LTy, TypeDef};
use std::collections::BTreeSet;

fn word(cx: &Cx<'_>, d: &DTy) -> String {
    match d {
        DTy::Unit => "unit".into(),
        DTy::Bool => "bool".into(),
        DTy::Int => "int".into(),
        DTy::Text => "text".into(),
        DTy::Named(t) => snake(&cx.lir.ty(*t).info.name),
        DTy::Option(x) => format!("option_{}", word(cx, x)),
        DTy::List(x) => format!("list_{}", word(cx, x)),
        DTy::Set(x) => format!("set_{}", word(cx, x)),
        DTy::Map(k, v) => format!("map_{}_{}", word(cx, k), word(cx, v)),
        DTy::Tuple(xs) => format!("tuple_{}", xs.iter().map(|x| word(cx, x)).collect::<Vec<_>>().join("_")),
        DTy::Range { lo, hi, inclusive } => {
            let n = |v: i128| if v < 0 { format!("minus{}", v.unsigned_abs()) } else { v.to_string() };
            format!("range_{}_{}{}", n(*lo), n(*hi), if *inclusive { "" } else { "_open" })
        }
        DTy::Where(b, c) => format!("{}_where{}", word(cx, b), c.index()),
        DTy::Trace(_) => "trace".into(),
    }
}

pub fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn unique(base: String, taken: &BTreeSet<String>) -> String {
    let base = ident(&base);
    if !taken.contains(&base) {
        return base;
    }
    (2..).map(|n| format!("{base}_{n}")).find(|n| !taken.contains(n)).unwrap_or(base)
}

/// The name of the function in `conform.rs` checking a value against `d` (never asked for a trivial `d`).
pub fn conform_fn(cx: &mut Cx<'_>, d: &DTy) -> String {
    let w = word(cx, d);
    cx.conforms.name(d, |taken| unique(w, taken))
}

/// The name of the generator in `gen.rs` for `d`.
pub fn gen_fn(cx: &mut Cx<'_>, d: &DTy) -> String {
    let w = word(cx, d);
    cx.gens.name(d, |taken| unique(w, taken))
}

fn m_arg(cx: &Cx<'_>, d: &DTy) -> &'static str {
    if cx.dty_m(d) { "m, " } else { "" }
}

/// A check of the value at `v` (an expression of type `&T`) against `d`: `if !c(&v)? { return Ok(false); }`.
fn sub_check(cx: &mut Cx<'_>, d: &DTy, v: &str) -> Option<String> {
    if cx.trivial(d) {
        return None;
    }
    let f = conform_fn(cx, d);
    Some(format!("if !{f}({}{v})? {{ return Ok(false); }}", m_arg(cx, d)))
}

fn conform_body(cx: &mut Cx<'_>, d: &DTy) -> Vec<String> {
    let lir = cx.lir;
    let mut out = Vec::new();
    match d {
        DTy::Range { lo, hi, inclusive } => {
            let op = if *inclusive { "..=" } else { ".." };
            out.push(format!("Ok(({}{op}{}).contains(v))", int_lit(*lo).text, int_lit(*hi).text));
            return out;
        }
        DTy::Option(x) => {
            if let Some(c) = sub_check(cx, x, "x") {
                let c = c.trim_start_matches("if ");
                out.push(format!("if let Opt::Some(x) = v && {c}"));
            }
        }
        DTy::List(x) | DTy::Set(x) => {
            if let Some(c) = sub_check(cx, x, "x") {
                out.push(format!("for x in v {{ {c} }}"));
            }
        }
        DTy::Map(k, x) => {
            if let Some(c) = sub_check(cx, k, "x") {
                out.push(format!("for x in v.keys() {{ {c} }}"));
            }
            if let Some(c) = sub_check(cx, x, "x") {
                out.push(format!("for x in v.values() {{ {c} }}"));
            }
        }
        DTy::Tuple(xs) => {
            for (i, x) in xs.iter().enumerate() {
                if let Some(c) = sub_check(cx, x, &format!("&v.{i}")) {
                    out.push(c);
                }
            }
        }
        DTy::Where(b, c) => {
            if let Some(line) = sub_check(cx, b, "v") {
                out.push(line);
            }
            let (module, name) = claim_name(cx, *c);
            let m = if cx.claim_m[c.index()] { "m, " } else { "" };
            let path = cx.names.path(None, module, &name);
            let owned = if cx.copy(&cx.lty(d)) { "*v" } else { "v.to_owned()" };
            out.push(format!("rt::holds({path}({m}{owned}))"));
            return out;
        }
        DTy::Named(t) => match &lir.ty(*t).def {
            TypeDef::Alias { of } => {
                let f = conform_fn(cx, of);
                out.push(format!("{f}({}v)", m_arg(cx, of)));
                return out;
            }
            TypeDef::Record { fields } => {
                for f in fields {
                    if let Some(c) = sub_check(cx, &f.check, &format!("&v.{}", ident(&f.name))) {
                        out.push(c);
                    }
                }
            }
            TypeDef::Sum { variants } => {
                let path = cx.type_path(*t, None);
                let mut arms = Vec::new();
                for (vi, var) in variants.iter().enumerate() {
                    let mut checks = Vec::new();
                    let mut names = Vec::new();
                    for (fi, f) in var.fields.iter().enumerate() {
                        let x = format!("x{fi}");
                        if let Some(c) = sub_check(cx, &f.check, &x) {
                            checks.push(c);
                            names.push(format!("{}: {x}", ident(&f.name)));
                        }
                    }
                    if !checks.is_empty() {
                        let _ = vi;
                        arms.push(format!("{path}::{} {{ {}, .. }} => {{ {} }}", ident(&camel(&var.name)), names.join(", "), checks.join(" ")));
                    }
                }
                arms.push("_ => {}".into());
                out.push(format!("match v {{ {} }}", arms.join(" ")));
            }
            TypeDef::RuleSet { rules } => {
                let pats: Vec<String> =
                    rules.iter().map(|r| format!("crate::machine::Event::{} {{ .. }}", cx.names.event(*r))).collect();
                out.push(format!("Ok(matches!(v, {}))", pats.join(" | ")));
                return out;
            }
        },
        _ => {}
    }
    out.push("Ok(true)".into());
    out
}

/// The module and name of a claim's function: `<alias>_holds`, else `claim<n>`.
pub fn claim_name(cx: &Cx<'_>, c: spec_lang_lir::ClaimId) -> (spec_lang_ir::ModuleId, String) {
    let lir = cx.lir;
    let module = lir.claim(c).module;
    for decl in &lir.types {
        if let TypeDef::Alias { of: DTy::Where(_, x) } = &decl.def
            && *x == c
            && decl.info.module == module
        {
            return (module, ident(&format!("{}_holds", snake(&decl.info.name))));
        }
    }
    (module, format!("claim{}", c.index()))
}

/// `conform.rs`: every registered check, including the ones registering more as they are printed.
pub fn conform_file(cx: &mut Cx<'_>) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < cx.conforms.items.len() {
        let (d, name) = cx.conforms.items[i].clone();
        let lty = cx.lty(&d);
        // A slice where Rust would borrow one: `&str`, `&[T]`.
        let t = match &lty {
            LTy::Text => "str".to_string(),
            LTy::List(x) => format!("[{}]", cx.ty(x, None)),
            other => cx.ty(other, None),
        };
        let m = if cx.dty_m(&d) { "m: &mut Machine, " } else { "" };
        let body = conform_body(cx, &d);
        out.push(format!(
            "/// Whether a value is of `{}`.\npub fn {name}({m}v: &{t}) -> R<bool> {{ {} }}",
            describe(cx, &d),
            body.join(" ")
        ));
        i += 1;
    }
    out.join("\n\n")
}

/// A declared type as spec-lang writes it, for doc comments.
pub fn describe(cx: &Cx<'_>, d: &DTy) -> String {
    match d {
        DTy::Unit => "()".into(),
        DTy::Bool => "Bool".into(),
        DTy::Int => "Int".into(),
        DTy::Text => "Text".into(),
        DTy::Named(t) => cx.lir.ty(*t).info.qualified.clone(),
        DTy::Option(x) => format!("Option[{}]", describe(cx, x)),
        DTy::List(x) => format!("List[{}]", describe(cx, x)),
        DTy::Set(x) => format!("Set[{}]", describe(cx, x)),
        DTy::Map(k, v) => format!("Map[{}, {}]", describe(cx, k), describe(cx, v)),
        DTy::Tuple(xs) => format!("({})", xs.iter().map(|x| describe(cx, x)).collect::<Vec<_>>().join(", ")),
        DTy::Range { lo, hi, inclusive } => format!("{lo}{}{hi}", if *inclusive { "..=" } else { "..<" }),
        DTy::Where(b, c) => format!("{} where {}", describe(cx, b), cx.lir.claim(*c).text),
        DTy::Trace(_) => "Trace".into(),
    }
}

fn base(cx: &mut Cx<'_>, d: &DTy) -> String {
    let lir = cx.lir;
    let call = |cx: &mut Cx<'_>, x: &DTy| format!("{}(m, rng, pools)?", gen_fn(cx, x));
    match d {
        DTy::Unit => "()".into(),
        DTy::Bool => "rng.chance(1, 2)".into(),
        DTy::Int => "rt::rng::int(rng)".into(),
        DTy::Text => "rt::rng::text(rng)".into(),
        DTy::Range { lo, hi, inclusive } => {
            let hi = if *inclusive { *hi } else { hi - 1 };
            format!("rt::rng::in_range(rng, {}, {})", int_lit(*lo).text, int_lit(hi).text)
        }
        DTy::Option(x) => format!("if rng.chance(1, 3) {{ Opt::None }} else {{ Opt::Some({}) }}", call(cx, x)),
        DTy::List(x) => format!(
            "{{ let n = rng.below(4); let mut xs = Vec::new(); for _ in 0..n {{ xs.push({}); }} xs }}",
            call(cx, x)
        ),
        DTy::Set(x) => format!(
            "{{ let n = rng.below(4); let mut xs = BTreeSet::new(); for _ in 0..n {{ xs.insert({}); }} xs }}",
            call(cx, x)
        ),
        DTy::Map(k, v) => {
            let (k, v) = (call(cx, k), call(cx, v));
            format!(
                "{{ let n = rng.below(4); let mut map = BTreeMap::new(); for _ in 0..n {{ let k = {k}; let v = {v}; map.insert(k, v); }} map }}"
            )
        }
        DTy::Tuple(xs) => {
            let items: Vec<String> = xs.iter().map(|x| call(cx, x)).collect();
            crate::expr::tuple_text(items)
        }
        DTy::Where(b, _) => call(cx, b),
        DTy::Named(t) => {
            let decl = lir.ty(*t);
            match &decl.def {
                TypeDef::Alias { of } => call(cx, of),
                TypeDef::Record { fields } => {
                    let path = cx.type_path(*t, None);
                    let inits: Vec<String> = fields
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            let v = call(cx, &f.check);
                            let v = if cx.is_boxed(*t, RECORD, i as u32) { format!("Box::new({v})") } else { v };
                            format!("{}: {v}", ident(&f.name))
                        })
                        .collect();
                    format!("{path} {{ {} }}", inits.join(", "))
                }
                TypeDef::Sum { variants } if !decl.builtin => {
                    let path = cx.type_path(*t, None);
                    let n = variants.len();
                    let mut arms = Vec::new();
                    for (vi, var) in variants.iter().enumerate() {
                        let head = format!("{path}::{}", ident(&camel(&var.name)));
                        let value = if var.fields.is_empty() {
                            head
                        } else {
                            let inits: Vec<String> = var
                                .fields
                                .iter()
                                .enumerate()
                                .map(|(i, f)| {
                                    let v = call(cx, &f.check);
                                    let v = if cx.is_boxed(*t, vi as u32, i as u32) { format!("Box::new({v})") } else { v };
                                    format!("{}: {v}", ident(&f.name))
                                })
                                .collect();
                            format!("{head} {{ {} }}", inits.join(", "))
                        };
                        let pat = if vi + 1 == n { "_".to_string() } else { vi.to_string() };
                        arms.push(format!("{pat} => {value},"));
                    }
                    format!("match rng.below({n}) {{ {} }}", arms.join(" "))
                }
                _ => format!("return Err(rt::stop(\"E-GEN\", \"no value of `{}` is generated\"))", decl.info.qualified),
            }
        }
        DTy::Trace(_) => "return Err(rt::stop(\"E-GEN\", \"a Trace is not generated\"))".into(),
    }
}

/// `gen.rs`: every registered generator, and the pools of the world.
pub fn gen_file(cx: &mut Cx<'_>, pools: &[DTy]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < cx.gens.items.len() {
        let (d, name) = cx.gens.items[i].clone();
        let t = cx.ty(&cx.lty(&d), None);
        let mut b = base(cx, &d);
        if let Some(p) = pools.iter().position(|x| *x == d) {
            b = format!("match rt::rng::from_pool(rng, &pools.p{p}) {{ Some(v) => v, None => {b} }}");
        }
        let body = if cx.trivial(&d) {
            format!("Ok({b})")
        } else {
            let f = conform_fn(cx, &d);
            let m = if cx.dty_m(&d) { "m, " } else { "" };
            format!(
                "for _ in 0..rt::rng::TRIES {{ let v = {b}; if crate::conform::{f}({m}&v)? {{ return Ok(v); }} }} Err(rt::stop(\"E-GEN\", {:?}))",
                format!("no value of `{}` found in 400 tries", describe(cx, &d))
            )
        };
        out.push(format!(
            "/// A value of `{}`.\n#[allow(unused_variables)]\npub fn {name}(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<{t}> {{ {body} }}",
            describe(cx, &d)
        ));
        i += 1;
    }
    out.join("\n\n")
}

/// The DTys pools hold values of: the keys of each state declared `Map[K, V]`, the elements of each `Set[K]`.
pub fn pool_types(cx: &Cx<'_>) -> Vec<DTy> {
    let mut out: Vec<DTy> = Vec::new();
    for s in &cx.lir.states {
        if let DTy::Map(k, _) | DTy::Set(k) = &s.check
            && !out.contains(k)
        {
            out.push((**k).clone());
        }
    }
    out
}

