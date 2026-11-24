//! Where each declaration lands in the generated crate, and how it is named
//! and imported. Output follows the target, not the source tree
//! (docs/idea/32-build-layout.md): one file per type, per rule, per small
//! group; a module with one file is a single `<module>.rs`.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Ident, Span as PSpan};
use spec_lang_ir::{DeclInfo, FnId, ModuleId, RuleId, TypeDef, TypeId};

use crate::front::Front;
use crate::front::typed::{K, T, TExpr};

/// A Rust identifier, raw when it is a keyword.
pub fn ident(name: &str) -> Ident {
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if", "impl",
        "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static", "struct", "trait",
        "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
        "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
    ];
    // `self` is a claim's value; the generated constructor names it `value`.
    if name == "self" {
        return Ident::new("value", PSpan::call_site());
    }
    if KEYWORDS.contains(&name) {
        Ident::new_raw(name, PSpan::call_site())
    } else {
        Ident::new(name, PSpan::call_site())
    }
}

/// `TaskNo` -> `task_no`.
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

/// `add` -> `Add`, `next_no` -> `NextNo`.
pub fn camel(name: &str) -> String {
    name.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().chain(c).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

/// A Rust module of the generated crate, and the files in it.
#[derive(Debug)]
pub struct RustModule {
    pub module: ModuleId,
    /// `board` for `todo.board`.
    pub name: String,
    /// One file per entry; `None` key is the module's own file (`fns`).
    pub files: BTreeMap<String, Vec<Item>>,
}

/// One declaration placed in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Item {
    Type(TypeId),
    Fn(FnId),
    Rule(RuleId),
    /// The module's states: `World`.
    World,
    /// The guard errors: `Refusal`.
    Refusal,
    /// The events of the app's rules and their dispatch: `Command`, `apply`.
    Command,
}

#[derive(Debug)]
pub struct Layout {
    pub modules: Vec<RustModule>,
    /// Item -> (rust module name, file stem).
    pub place: BTreeMap<Item, (String, String)>,
    /// The module that owns the states and rules: where `World`, `Refusal`, `Command` live.
    pub board: String,
    /// Strategies switched off ([`crate::Options`]).
    pub off: BTreeSet<String>,
}

impl Layout {
    /// Is this strategy on.
    pub fn on(&self, strategy: &str) -> bool {
        !self.off.contains(strategy)
    }

    /// The rule takes the world shared and never writes it: required of every
    /// translation by the contract, licensed by the write set and `@uses`.
    pub fn read_only(&self, front: &Front<'_>, index: usize) -> bool {
        front.facts.rules.get(index).is_some_and(|f| f.read_only())
    }

    /// Is the module emitted as a directory of files.
    pub fn is_dir(&self, module: &str) -> bool {
        self.modules.iter().any(|m| m.name == module && m.files.len() > 1)
    }

    pub fn module_of(&self, item: Item) -> Option<&str> {
        self.place.get(&item).map(|(m, _)| m.as_str())
    }
}

/// Modules that hold product declarations (types, fns, states, rules), named
/// by their path with the common prefix dropped.
pub fn plan(front: &Front<'_>, off: &BTreeSet<String>) -> Layout {
    let prog = front.prog;
    let mut product: BTreeSet<ModuleId> = BTreeSet::new();
    for t in &prog.types {
        if !matches!(t.def, TypeDef::Rules(_)) && t.info.module.0 != 0 {
            product.insert(t.info.module);
        }
    }
    for f in &prog.fns {
        product.insert(f.info.module);
    }
    for r in &front.rules {
        product.insert(prog.rule(r.id).info.module);
    }
    let names: Vec<Vec<&str>> = product.iter().map(|m| prog.module(*m).name.split('.').collect()).collect();
    let common = common_prefix(&names);
    let short = |m: ModuleId| -> String {
        let parts: Vec<&str> = prog.module(m).name.split('.').collect();
        parts[common.min(parts.len().saturating_sub(1))..].join("_")
    };
    let mut place: BTreeMap<Item, (String, String)> = BTreeMap::new();
    let mut board = String::new();
    // Types: one file each.
    for (i, t) in prog.types.iter().enumerate() {
        if product.contains(&t.info.module) && !matches!(t.def, TypeDef::Rules(_)) {
            place.insert(Item::Type(TypeId(i as u32)), (short(t.info.module), snake(&t.info.name)));
        }
    }
    // Rules: one file each; the module's world, refusals and commands beside them.
    for r in &front.rules {
        let info = &prog.rule(r.id).info;
        let m = short(info.module);
        place.insert(Item::Rule(r.id), (m.clone(), info.name.clone()));
        board = m;
    }
    if !board.is_empty() {
        place.insert(Item::World, (board.clone(), "world".into()));
        place.insert(Item::Refusal, (board.clone(), "refusal".into()));
        place.insert(Item::Command, (board.clone(), "command".into()));
    }
    // Fns: with the one type whose claim uses them, or the one rule that calls them, else `fns`.
    for (i, f) in prog.fns.iter().enumerate() {
        let id = FnId(i as u32);
        let m = short(f.info.module);
        let users = fn_users(front, id);
        let home = match users.as_slice() {
            [it @ (Item::Type(_) | Item::Rule(_))] if place.get(it).is_some_and(|(pm, _)| *pm == m) => {
                place[it].1.clone()
            }
            _ => "fns".into(),
        };
        place.insert(Item::Fn(id), (m, home));
    }
    let mut modules: BTreeMap<String, RustModule> = BTreeMap::new();
    for (item, (m, file)) in &place {
        let module = match item {
            Item::Type(t) => prog.ty(*t).info.module,
            Item::Fn(f) => prog.func(*f).info.module,
            Item::Rule(r) => prog.rule(*r).info.module,
            _ => front.rules.first().map(|r| prog.rule(r.id).info.module).unwrap_or(ModuleId(0)),
        };
        modules
            .entry(m.clone())
            .or_insert_with(|| RustModule { module, name: m.clone(), files: BTreeMap::new() })
            .files
            .entry(file.clone())
            .or_default()
            .push(*item);
    }
    // A module of one file is that file: `render.rs`, never `render/fns.rs`.
    let mut modules: Vec<RustModule> = modules.into_values().collect();
    modules.sort_by_key(|m| prog.module(m.module).decls.first().map(|_| m.module));
    Layout { modules, place, board, off: off.clone() }
}

fn common_prefix(names: &[Vec<&str>]) -> usize {
    let Some(first) = names.first() else { return 0 };
    let mut n = 0;
    while n < first.len().saturating_sub(1) && names.iter().all(|x| x.len() > n + 1 && x[n] == first[n]) {
        n += 1;
    }
    n
}

/// The declarations that call or claim with a fn.
fn fn_users(front: &Front<'_>, id: FnId) -> Vec<Item> {
    let mut out = Vec::new();
    let calls = |e: &TExpr| mentions_fn(e, id);
    for (i, t) in front.prog.types.iter().enumerate() {
        if let TypeDef::Alias(spec_lang_ir::Ty::Where(_, claim)) = &t.def
            && ir_calls_fn(&claim.expr, id)
        {
            out.push(Item::Type(TypeId(i as u32)));
        }
    }
    for f in &front.fns {
        if f.id != id && f.body.iter().any(|s| stmt_mentions(s, &calls)) {
            out.push(Item::Fn(f.id));
        }
    }
    for r in &front.rules {
        if r.body.iter().any(|s| stmt_mentions(s, &calls)) || r.guards.iter().any(|g| calls(&g.cond)) {
            out.push(Item::Rule(r.id));
        }
    }
    out
}

fn stmt_mentions(s: &crate::front::typed::TStmt, f: &impl Fn(&TExpr) -> bool) -> bool {
    let mut hit = false;
    crate::front::facts::stmt_exprs(s, &mut |e| hit |= f(e));
    hit
}

pub fn mentions_fn(e: &TExpr, id: FnId) -> bool {
    if let K::CallFn { f, .. } = &e.kind
        && *f == id
    {
        return true;
    }
    let mut hit = false;
    crate::front::facts::children(e, &mut |c| hit |= mentions_fn(c, id));
    hit
}

fn ir_calls_fn(e: &spec_lang_ir::Expr, id: FnId) -> bool {
    use spec_lang_ir::{Callee, ExprKind};
    match &e.kind {
        ExprKind::Call { callee: Callee::Fn(f), .. } if *f == id => true,
        ExprKind::Call { args, .. } => args.iter().flatten().any(|a| ir_calls_fn(a, id)),
        ExprKind::Binary(_, a, b) => ir_calls_fn(a, id) || ir_calls_fn(b, id),
        ExprKind::Unary(_, a) => ir_calls_fn(a, id),
        _ => false,
    }
}

/// The declaration's info, for headers and provenance.
pub fn info<'p>(front: &Front<'p>, item: Item) -> Option<&'p DeclInfo> {
    let prog = front.prog;
    Some(match item {
        Item::Type(t) => &prog.ty(t).info,
        Item::Fn(f) => &prog.func(f).info,
        Item::Rule(r) => &prog.rule(r).info,
        _ => return None,
    })
}

/// Is a value of this type `Copy` in Rust.
pub fn is_copy(front: &Front<'_>, t: &T) -> bool {
    match t {
        T::Unit | T::Bool | T::Int | T::Byte => true,
        T::Named(id) => match &front.prog.ty(*id).def {
            TypeDef::Sum(vs) => vs.iter().all(|v| v.fields.is_empty()),
            TypeDef::Alias(_) => crate::front::typer::alias_base(front.prog, *id).is_some_and(|(b, _)| b == T::Int),
            _ => false,
        },
        T::Tuple(ts) => ts.iter().all(|x| is_copy(front, x)),
        T::Option(x) => is_copy(front, x),
        _ => false,
    }
}
