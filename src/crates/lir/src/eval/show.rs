//! `show`: a value as spec-lang source, which `"{x}"` prints for anything
//! but `Text` (Text prints raw). Every printer's runtime prints the same:
//! `true`, `-3`, `"a\n"` (quoted, escapes `\" \\ \n \t \{` and `\u{X}` for
//! other control or white space but ` `), `[1, 2]`, `Set(1, 2)`,
//! `Map(1 -> "a")`, `(1, "a")`, a record `Task(1, title: "milk", ...)` (the
//! first field positional, the rest named), a variant `Held(by, until: 3)`
//! or `Open`, `Some(1)`, `none`, a range `1..=5`, a failure `fail Name(data)`.

use super::value::Value;
use crate::program::{Lir, TypeDef};

pub fn show(lir: &Lir, v: &Value<'_>) -> String {
    let mut s = String::new();
    write(lir, v, &mut s);
    s
}

/// As `"{x}"` prints it.
pub fn interpolate(lir: &Lir, v: &Value<'_>) -> String {
    match v {
        Value::Text(t) => t.to_string(),
        _ => show(lir, v),
    }
}

fn list(items: impl Iterator<Item = String>, open: &str, close: &str, out: &mut String) {
    out.push_str(open);
    out.push_str(&items.collect::<Vec<_>>().join(", "));
    out.push_str(close);
}

fn fields(lir: &Lir, names: &[String], xs: &[Value<'_>]) -> Vec<String> {
    xs.iter()
        .enumerate()
        .map(|(i, x)| match (i, names.get(i)) {
            (0, _) | (_, None) => show(lir, x),
            (_, Some(n)) => format!("{n}: {}", show(lir, x)),
        })
        .collect()
}

fn write(lir: &Lir, v: &Value<'_>, out: &mut String) {
    match v {
        Value::Unit => out.push_str("()"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Text(t) => quote(t, out),
        Value::List(xs) => list(xs.iter().map(|x| show(lir, x)), "[", "]", out),
        Value::Set(xs) => list(xs.iter().map(|x| show(lir, x)), "Set(", ")", out),
        Value::Map(m) => list(m.iter().map(|(k, x)| format!("{} -> {}", show(lir, k), show(lir, x))), "Map(", ")", out),
        Value::Tuple(xs) => list(xs.iter().map(|x| show(lir, x)), "(", ")", out),
        Value::Record(t, xs) => {
            let d = lir.ty(*t);
            let names: Vec<String> = match &d.def {
                TypeDef::Record { fields } => fields.iter().map(|f| f.name.clone()).collect(),
                _ => Vec::new(),
            };
            list(fields(lir, &names, xs).into_iter(), &format!("{}(", d.info.name), ")", out);
        }
        Value::Variant(t, tag, xs) => {
            let (name, names): (String, Vec<String>) = match &lir.ty(*t).def {
                TypeDef::Sum { variants } => match variants.get(*tag as usize) {
                    Some(v) => (v.name.clone(), v.fields.iter().map(|f| f.name.clone()).collect()),
                    None => ("?".into(), Vec::new()),
                },
                _ => ("?".into(), Vec::new()),
            };
            out.push_str(&name);
            if !xs.is_empty() {
                list(fields(lir, &names, xs).into_iter(), "(", ")", out);
            }
        }
        Value::Range(lo, hi, incl) => out.push_str(&format!("{lo}{}{hi}", if *incl { "..=" } else { "..<" })),
        Value::Func(_) => out.push_str("<function>"),
        Value::Failure(f) => {
            out.push_str("fail ");
            out.push_str(&f.name);
            if !f.data.is_empty() {
                list(f.data.iter().map(|x| show(lir, x)), "(", ")", out);
            }
        }
        Value::Event(e) => {
            let r = lir.rule(e.rule);
            let module = lir.modules[r.info.module.index()].name.rsplit('.').next().unwrap_or("").to_string();
            let args: Vec<String> = e
                .args
                .iter()
                .enumerate()
                .map(|(i, a)| match (i, r.params.get(i)) {
                    (0, _) | (_, None) => show(lir, a),
                    (_, Some(p)) => format!("{}: {}", p.name, show(lir, a)),
                })
                .collect();
            out.push_str(&format!("{module}.{}({})", r.info.name, args.join(", ")));
        }
        Value::World(w) => {
            let items =
                w.iter().enumerate().map(|(i, x)| format!("{}: {}", lir.states[i].info.qualified, show(lir, x)));
            list(items, "World(", ")", out);
        }
        Value::Step(s) => out.push_str(&format!("{} -> {}", show(lir, &s.event), show(lir, &s.reply))),
        Value::Trace(t) => out.push_str(&format!("<trace of {} steps>", t.steps.len())),
    }
}

/// Text with escapes, in quotes.
pub fn quote(t: &str, out: &mut String) {
    out.push('"');
    for c in t.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '{' => out.push_str("\\{"),
            c if c.is_control() || c.is_whitespace() && c != ' ' => out.push_str(&format!("\\u{{{:X}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
