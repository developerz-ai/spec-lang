//! Values and types printed as spec-lang source, so a counterexample can be
//! pasted back into a test.

use crate::value::Value;
use spec_lang_ir::{Program, Ty, TypeDef};

/// A value as spec-lang source: `Task(1, title: "milk", ...)`, `Set("a")`.
pub fn show(p: &Program, v: &Value) -> String {
    let mut s = String::new();
    write(p, v, &mut s);
    s
}

/// A value as `"{x}"` prints it: text raw, anything else as [`show`].
pub fn interpolate(p: &Program, v: &Value) -> String {
    match v {
        Value::Text(t) => t.to_string(),
        _ => show(p, v),
    }
}

fn list(p: &Program, items: impl Iterator<Item = String>, open: &str, close: &str, out: &mut String) {
    let _ = p;
    out.push_str(open);
    out.push_str(&items.collect::<Vec<_>>().join(", "));
    out.push_str(close);
}

fn write(p: &Program, v: &Value, out: &mut String) {
    match v {
        Value::Unit => out.push_str("()"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Text(t) => quote(t, out),
        Value::List(xs) => list(p, xs.iter().map(|x| show(p, x)), "[", "]", out),
        Value::Set(xs) => list(p, xs.iter().map(|x| show(p, x)), "Set(", ")", out),
        Value::Map(m) => list(p, m.iter().map(|(k, x)| format!("{} -> {}", show(p, k), show(p, x))), "Map(", ")", out),
        Value::Tuple(xs) => list(p, xs.iter().map(|x| show(p, x)), "(", ")", out),
        Value::Record(t, fields) => {
            let names: Vec<String> = match &p.ty(*t).def {
                TypeDef::Record(fs) => fs.iter().map(|f| f.name.clone()).collect(),
                _ => Vec::new(),
            };
            let items = fields.iter().enumerate().map(|(i, x)| match (i, names.get(i)) {
                (0, _) | (_, None) => show(p, x),
                (_, Some(n)) => format!("{n}: {}", show(p, x)),
            });
            list(p, items, &format!("{}(", p.ty(*t).info.name), ")", out);
        }
        Value::Variant(t, tag, fields) => {
            let (name, names): (String, Vec<String>) = match &p.ty(*t).def {
                TypeDef::Sum(vs) => match vs.get(*tag as usize) {
                    Some(v) => (v.name.clone(), v.fields.iter().map(|f| f.name.clone()).collect()),
                    None => ("?".into(), Vec::new()),
                },
                _ => ("?".into(), Vec::new()),
            };
            out.push_str(&name);
            if !fields.is_empty() {
                let items = fields.iter().enumerate().map(|(i, x)| match (i, names.get(i)) {
                    (0, _) | (_, None) => show(p, x),
                    (_, Some(n)) => format!("{n}: {}", show(p, x)),
                });
                list(p, items, "(", ")", out);
            }
        }
        Value::Range(lo, hi, incl) => out.push_str(&format!("{lo}{}{hi}", if *incl { "..=" } else { "..<" })),
        Value::Func(_) => out.push_str("<function>"),
        Value::Failure(f) => {
            out.push_str("fail ");
            out.push_str(&f.name);
            if !f.data.is_empty() {
                list(p, f.data.iter().map(|x| show(p, x)), "(", ")", out);
            }
        }
        Value::Event(e) => out.push_str(&call(p, e)),
        Value::World(w) => {
            let items =
                w.states.iter().enumerate().map(|(i, x)| format!("{}: {}", p.qualified(&p.states[i].info), show(p, x)));
            list(p, items, "World(", ")", out);
        }
        Value::Step(s) => out.push_str(&format!("{} -> {}", show(p, &s.event), show(p, &s.reply))),
        Value::Trace(t) => out.push_str(&format!("<trace of {} steps>", t.steps.len())),
    }
}

/// `board.done(2)`: a rule call as a test would write it, named arguments
/// after the first.
pub fn call(p: &Program, e: &crate::value::Event) -> String {
    let r = p.rule(e.rule);
    let module = p.module(r.info.module).name.rsplit('.').next().unwrap_or("").to_string();
    let args: Vec<String> = e
        .args
        .iter()
        .enumerate()
        .map(|(i, a)| {
            if i == 0 {
                show(p, a)
            } else {
                format!("{}: {}", r.params.get(i).map_or("?", |x| x.name.as_str()), show(p, a))
            }
        })
        .collect();
    format!("{module}.{}({})", r.info.name, args.join(", "))
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

/// A type as written in source: `Set[Tag]`, `Text where ...`.
pub fn show_ty(p: &Program, t: &Ty) -> String {
    match t {
        Ty::Unit => "()".into(),
        Ty::Bool => "Bool".into(),
        Ty::Int => "Int".into(),
        Ty::Text => "Text".into(),
        Ty::Named(id) => p.ty(*id).info.name.clone(),
        Ty::Option(x) => format!("Option[{}]", show_ty(p, x)),
        Ty::List(x) => format!("List[{}]", show_ty(p, x)),
        Ty::Set(x) => format!("Set[{}]", show_ty(p, x)),
        Ty::Map(k, v) => format!("Map[{}, {}]", show_ty(p, k), show_ty(p, v)),
        Ty::Tuple(xs) => format!("({})", xs.iter().map(|x| show_ty(p, x)).collect::<Vec<_>>().join(", ")),
        Ty::Range { lo, hi, inclusive } => format!("{lo}{}{hi}", if *inclusive { "..=" } else { "..<" }),
        Ty::Where(b, c) => format!("{} where {}", show_ty(p, b), p.snippet(c.expr.span)),
        Ty::Trace(rs) => {
            format!("Trace[{}]", rs.iter().map(|r| p.rule(*r).info.name.clone()).collect::<Vec<_>>().join(" | "))
        }
    }
}
