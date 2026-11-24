//! Smaller values to try after a failure: the first that still fails wins,
//! and shrinking repeats from it.

use spec_lang_engine::Value;
use std::rc::Rc;

/// Candidates smaller than `v`, smallest first. The caller keeps only those
/// that still meet the type.
pub(crate) fn candidates(v: &Value) -> Vec<Value> {
    match v {
        Value::Int(n) => {
            let mut c = vec![0, 1, n / 2, n - n.signum()];
            if *n < 0 {
                c.push(-n);
            }
            c.into_iter().filter(|x| x.abs() < n.abs() || (x.abs() == n.abs() && *x > *n)).map(Value::Int).collect()
        }
        Value::Text(t) => {
            let chars: Vec<char> = t.chars().collect();
            let mut c = Vec::new();
            if !chars.is_empty() {
                c.push(String::new());
                c.push(chars[..chars.len() / 2].iter().collect());
                c.push(chars[..chars.len() - 1].iter().collect());
                c.push(chars[1..].iter().collect());
            }
            c.into_iter().map(|s| Value::text(&s)).collect()
        }
        Value::List(xs) => (0..xs.len())
            .map(|i| {
                let mut ys = (**xs).clone();
                ys.remove(i);
                Value::list(ys)
            })
            .collect(),
        Value::Set(xs) => xs
            .iter()
            .map(|x| {
                let mut ys = (**xs).clone();
                ys.remove(x);
                Value::Set(Rc::new(ys))
            })
            .collect(),
        Value::Map(m) => {
            let mut c: Vec<Value> = m
                .keys()
                .map(|k| {
                    let mut n = (**m).clone();
                    n.remove(k);
                    Value::Map(Rc::new(n))
                })
                .collect();
            for (k, v) in m.iter() {
                for s in candidates(v) {
                    let mut n = (**m).clone();
                    n.insert(k.clone(), s);
                    c.push(Value::Map(Rc::new(n)));
                }
            }
            c
        }
        Value::Variant(t, tag, fields) => {
            let mut c = Vec::new();
            for (i, f) in fields.iter().enumerate() {
                for s in candidates(f) {
                    let mut fs = (**fields).clone();
                    fs[i] = s;
                    c.push(Value::Variant(*t, *tag, Rc::new(fs)));
                }
            }
            c
        }
        Value::Record(t, fields) => {
            let mut c = Vec::new();
            for (i, f) in fields.iter().enumerate() {
                for s in candidates(f) {
                    let mut fs = (**fields).clone();
                    fs[i] = s;
                    c.push(Value::Record(*t, Rc::new(fs)));
                }
            }
            c
        }
        _ => Vec::new(),
    }
}
