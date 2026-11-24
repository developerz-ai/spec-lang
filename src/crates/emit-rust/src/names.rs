//! Rust names for the LIR's declarations: modules as module paths, keywords escaped, every name unique where
//! Rust needs it to be (world fields, event variants, reply payloads).

use spec_lang_ir::{ModuleId, RuleId, StateId};
use spec_lang_lir::{LTy, Lir};
use std::collections::BTreeMap;

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "dyn", "else", "enum", "extern", "false", "fn", "for", "gen",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static", "struct",
    "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Names the printer reserves in every body: the machine, temporaries and the runtime.
const RESERVED: &[&str] = &["m", "rt", "rng", "pools", "result", "crate", "self", "super", "Self", "core", "std"];

/// A Rust identifier for a spec-lang name: keywords raw, `$x` (a lowering temporary) as `x__`.
pub fn ident(name: &str) -> String {
    let mut s: String = name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if name.starts_with('$') {
        s = format!("{}__", &s[1..]);
    }
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s = format!("n_{s}");
    }
    if RESERVED.contains(&s.as_str()) || (s.starts_with("tmp") && s[3..].chars().all(|c| c.is_ascii_digit())) {
        return format!("{s}_");
    }
    if KEYWORDS.contains(&s.as_str()) { format!("r#{s}") } else { s }
}

/// A local variable's name.
pub fn local(name: &str) -> String {
    ident(name)
}

/// `CamelCase` from `snake_case` or a dotted path.
pub fn camel(name: &str) -> String {
    name.split(['_', '.'])
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                Some(f) => f.to_uppercase().chain(c).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

/// `SCREAMING_CASE` for a constant.
pub fn screaming(name: &str) -> String {
    ident(&name.to_uppercase())
}

#[derive(Debug)]
pub struct Names {
    /// Module path segments, by `ModuleId`.
    modules: Vec<Vec<String>>,
    /// World field per state.
    states: Vec<String>,
    /// Event variant per rule.
    events: Vec<String>,
    /// Reply payload variants, by result type.
    outs: BTreeMap<LTy, String>,
}

impl Names {
    pub fn new(lir: &Lir) -> Self {
        let modules = lir
            .modules
            .iter()
            .map(|m| {
                if m.id.index() == 0 {
                    return vec!["core_".to_string()];
                }
                m.name
                    .split('.')
                    .map(ident)
                    .map(|s| {
                        if s == "rt"
                            || s == "machine"
                            || s == "checks"
                            || s == "conform"
                            || s == "draw"
                            || s == "lib"
                            || s == "main"
                            || s == "bin"
                        {
                            format!("{s}_")
                        } else {
                            s
                        }
                    })
                    .collect()
            })
            .collect();
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for s in &lir.states {
            *seen.entry(s.info.name.clone()).or_default() += 1;
        }
        let states =
            lir.states
                .iter()
                .map(|s| {
                    if seen[&s.info.name] > 1 {
                        ident(&s.info.qualified.replace('.', "_"))
                    } else {
                        ident(&s.info.name)
                    }
                })
                .collect();
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for r in &lir.rules {
            *seen.entry(camel(&r.info.name)).or_default() += 1;
        }
        let events = lir
            .rules
            .iter()
            .map(|r| {
                let c = camel(&r.info.name);
                if seen[&c] > 1 { camel(&r.info.qualified) } else { c }
            })
            .collect();
        let mut outs = BTreeMap::new();
        let mut used: BTreeMap<String, usize> = BTreeMap::new();
        for r in &lir.rules {
            if outs.contains_key(&r.result) {
                continue;
            }
            let base = ty_word(lir, &r.result);
            let n = used.entry(base.clone()).or_default();
            *n += 1;
            let name = if *n == 1 { base } else { format!("{base}{n}") };
            outs.insert(r.result.clone(), name);
        }
        Names { modules, states, events, outs }
    }

    pub fn module(&self, m: ModuleId) -> &[String] {
        &self.modules[m.index()]
    }

    /// The path of `item` of module `of`, written from module `from` (`None`: the crate root's files).
    pub fn path(&self, from: Option<ModuleId>, of: ModuleId, item: &str) -> String {
        if from == Some(of) {
            return item.to_string();
        }
        format!("crate::{}::{item}", self.modules[of.index()].join("::"))
    }

    pub fn state(&self, s: StateId) -> &str {
        &self.states[s.index()]
    }

    pub fn event(&self, r: RuleId) -> &str {
        &self.events[r.index()]
    }

    pub fn out(&self, t: &LTy) -> Option<&str> {
        self.outs.get(t).map(String::as_str)
    }

    pub fn outs(&self) -> impl Iterator<Item = (&LTy, &String)> {
        self.outs.iter()
    }
}

/// A word for a type, for a variant name: `Int`, `ListTask`.
fn ty_word(lir: &Lir, t: &LTy) -> String {
    match t {
        LTy::Unit => "Unit".into(),
        LTy::Bool => "Bool".into(),
        LTy::Int => "Int".into(),
        LTy::Text => "Text".into(),
        LTy::Named(id) => camel(&lir.ty(*id).info.name),
        LTy::Option(x) => format!("Opt{}", ty_word(lir, x)),
        LTy::List(x) => format!("List{}", ty_word(lir, x)),
        LTy::Set(x) => format!("Set{}", ty_word(lir, x)),
        LTy::Map(k, v) => format!("Map{}{}", ty_word(lir, k), ty_word(lir, v)),
        LTy::Tuple(xs) => format!("Tuple{}", xs.iter().map(|x| ty_word(lir, x)).collect::<String>()),
        other => camel(&format!("{other:?}").chars().filter(|c| c.is_alphanumeric()).collect::<String>()),
    }
}
