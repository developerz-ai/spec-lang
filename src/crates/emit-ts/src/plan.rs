//! The program-wide decisions made once, before any file is printed: each module's file and import alias,
//! each declaration's TypeScript name, each state's property, each field's property, and which functions
//! read or change the world (and so take `w`).

use crate::names::{camel, pascal};
use spec_lang_ir::{FnId, ModuleId};
use spec_lang_lir::node::{Block, Expr, ExprKind, Place, PlaceRoot};
use spec_lang_lir::program::{Body, TypeDef};
use spec_lang_lir::{Lir, walk};
use std::collections::BTreeSet;

/// Paths and names the generated project itself uses, so a module may not take them.
const TAKEN_PATHS: &[&str] = &["runtime", "lib", "machine", "program", "main", "app"];

#[derive(Debug)]
pub struct Plan<'l> {
    pub lir: &'l Lir,
    /// Per module: its file (`src/todo/board.ts`), or empty for `core`.
    pub module_file: Vec<String>,
    /// Per module: its `.slg` file relative to the project (`todo/board.slg`).
    pub module_slg: Vec<String>,
    /// Per module: the alias other files import it under.
    pub module_alias: Vec<String>,
    /// Per module: every top-level name it declares.
    pub module_names: Vec<BTreeSet<String>>,
    pub type_name: Vec<String>,
    pub fn_name: Vec<String>,
    pub rule_name: Vec<String>,
    pub const_name: Vec<String>,
    pub law_name: Vec<String>,
    pub test_name: Vec<String>,
    /// A constant that is a literal is a plain `const`; any other is computed once, on first use.
    pub const_lazy: Vec<bool>,
    pub state_prop: Vec<String>,
    /// Per type, per variant (one for a record), per field: its property.
    pub field_prop: Vec<Vec<Vec<String>>>,
    pub handler_prop: Vec<String>,
    /// Per handler, per operation: its method.
    pub op_name: Vec<Vec<String>>,
    /// A `fn` takes the world when its body reads a state, calls a rule, performs an effect, or calls a `fn` that does.
    pub fn_world: Vec<bool>,
}

impl<'l> Plan<'l> {
    pub fn new(lir: &'l Lir) -> Self {
        let module_file: Vec<String> = lir
            .modules
            .iter()
            .enumerate()
            .map(|(i, m)| {
                if i == 0 {
                    return String::new();
                }
                let mut parts: Vec<String> = m.name.split('.').map(crate::names::kebab).collect();
                if let Some(first) = parts.first_mut()
                    && TAKEN_PATHS.contains(&first.as_str())
                {
                    first.push_str("-module");
                }
                format!("src/{}.ts", parts.join("/"))
            })
            .collect();
        let module_slg = lir.modules.iter().map(|m| format!("{}.slg", m.name.replace('.', "/"))).collect();

        let type_name = lir.types.iter().map(|t| pascal(&t.info.name)).collect();
        let fn_name = lir.fns.iter().map(|f| camel(&f.info.name)).collect();
        let rule_name = lir.rules.iter().map(|r| camel(&r.info.name)).collect();
        let const_name = lir.consts.iter().map(|c| camel(&c.info.name)).collect();
        let law_name = lir.laws.iter().map(|l| camel(&l.info.name)).collect();
        let test_name = lir.tests.iter().map(|t| camel(&t.info.name)).collect();
        let const_lazy = lir.consts.iter().map(|c| !is_literal(&c.value)).collect();

        let mut plan = Plan {
            lir,
            module_file,
            module_slg,
            module_alias: Vec::new(),
            module_names: vec![BTreeSet::new(); lir.modules.len()],
            type_name,
            fn_name,
            rule_name,
            const_name,
            law_name,
            test_name,
            const_lazy,
            state_prop: Vec::new(),
            field_prop: Vec::new(),
            handler_prop: lir.handlers.iter().map(|h| camel(&h.name)).collect(),
            op_name: lir.handlers.iter().map(|h| h.ops.iter().map(|o| method(&o.name)).collect()).collect(),
            fn_world: vec![false; lir.fns.len()],
        };
        plan.names();
        plan.aliases();
        plan.states();
        plan.fields();
        plan.world();
        plan
    }

    fn names(&mut self) {
        let lir = self.lir;
        let mut add = |m: ModuleId, n: &str| {
            self.module_names[m.index()].insert(n.to_string());
        };
        for (t, n) in lir.types.iter().zip(&self.type_name) {
            add(t.info.module, n);
        }
        for (f, n) in lir.fns.iter().zip(&self.fn_name) {
            add(f.info.module, n);
        }
        for (r, n) in lir.rules.iter().zip(&self.rule_name) {
            add(r.info.module, n);
        }
        for (c, n) in lir.consts.iter().zip(&self.const_name) {
            add(c.info.module, n);
        }
        for (l, n) in lir.laws.iter().zip(&self.law_name) {
            add(l.info.module, n);
        }
    }

    /// The last part of the module's path, longer where two modules would share one.
    fn aliases(&mut self) {
        let lir = self.lir;
        // Names the generated files give their own values.
        let mut taken: BTreeSet<String> =
            ["a", "r", "v", "program", "states", "worldType", "pretend", "suite", "checks", "tests", "app"]
                .map(String::from)
                .into();
        for m in &lir.modules {
            let parts: Vec<&str> = m.name.split('.').collect();
            let mut alias = String::new();
            for k in 1..=parts.len() {
                alias = camel(&parts[parts.len() - k..].join("_"));
                if !taken.contains(&alias) && !self.module_names.iter().any(|ns| ns.contains(&alias)) {
                    break;
                }
            }
            while taken.contains(&alias) || self.module_names.iter().any(|ns| ns.contains(&alias)) {
                alias.push_str("Module");
            }
            taken.insert(alias.clone());
            self.module_alias.push(alias);
        }
    }

    /// A state's property on the world: its name, or with its module's where two states share one.
    fn states(&mut self) {
        let lir = self.lir;
        let leaf: Vec<String> = lir.states.iter().map(|s| camel(&s.info.name)).collect();
        let world_members = ["fx", "save", "restore", "value", "constructor"];
        self.state_prop = lir
            .states
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let shared = leaf.iter().filter(|n| **n == leaf[i]).count() > 1;
                let mut p = if shared { camel(&s.info.qualified.replace('.', "_")) } else { leaf[i].clone() };
                if world_members.contains(&p.as_str()) {
                    p.push('_');
                }
                p
            })
            .collect();
    }

    fn fields(&mut self) {
        let props = |fields: &[spec_lang_lir::program::Field], sum: bool| -> Vec<String> {
            let mut out: Vec<String> = Vec::new();
            for f in fields {
                let mut p = camel(&f.name);
                if (sum && p == "kind") || out.contains(&p) {
                    p = format!("{p}_");
                }
                out.push(p);
            }
            out
        };
        self.field_prop = self
            .lir
            .types
            .iter()
            .map(|t| match &t.def {
                TypeDef::Record { fields } => vec![props(fields, false)],
                TypeDef::Sum { variants } => variants.iter().map(|v| props(&v.fields, true)).collect(),
                _ => Vec::new(),
            })
            .collect();
    }

    /// Which `fn`s need `w`: a fixed point over calls.
    fn world(&mut self) {
        let lir = self.lir;
        let direct: Vec<(bool, Vec<FnId>)> = lir
            .fns
            .iter()
            .map(|f| {
                let mut uses = false;
                let mut calls = Vec::new();
                let mut see = |e: &Expr| scan(e, &mut uses, &mut calls);
                for p in &f.params {
                    if let Some(d) = &p.default {
                        walk::exprs(d, &mut see);
                    }
                }
                if let Body::Expr(e) = &f.body {
                    walk::exprs(e, &mut see);
                }
                (uses, calls)
            })
            .collect();
        let mut world: Vec<bool> = direct.iter().map(|(u, _)| *u).collect();
        loop {
            let mut changed = false;
            for (i, (_, calls)) in direct.iter().enumerate() {
                if !world[i] && calls.iter().any(|c| world[c.index()]) {
                    world[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        self.fn_world = world;
    }

    /// Whether evaluating `e` reads or changes the world.
    pub fn needs_world(&self, e: &Expr) -> bool {
        let mut uses = false;
        let mut calls = Vec::new();
        walk::exprs(e, &mut |x| scan(x, &mut uses, &mut calls));
        uses || calls.iter().any(|c| self.fn_world[c.index()])
    }

    /// The `.slg` position of a span: `todo/board.slg:21`.
    pub fn slg(&self, module: ModuleId, span: spec_lang_syntax::Span) -> String {
        let line = self.lir.sources.get(span.file).map_or(0, |f| f.line_col(span.lo).0);
        format!("{}:{line}", self.module_slg[module.index()])
    }

    /// Every module whose file is printed, in id order.
    pub fn printed_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.lir.modules.iter().skip(1).map(|m| m.id)
    }
}

/// Every local name a statement under `b` assigns: a `var` never assigned prints as `const`.
pub fn assigned_in_block(b: &Block, out: &mut BTreeSet<String>) {
    for s in &b.stmts {
        use spec_lang_lir::node::StmtKind as K;
        match &s.kind {
            K::Assign { place, .. } | K::Mutate { place, .. } => {
                if let PlaceRoot::Local(n) = &place.root {
                    out.insert(n.clone());
                }
            }
            K::For { body, .. } | K::While { body, .. } => assigned_in_block(body, out),
            K::If { then, els, .. } => {
                assigned_in_block(then, out);
                if let Some(e) = els {
                    assigned_in_block(e, out);
                }
            }
            _ => {}
        }
    }
    walk::block_exprs(b, &mut |e| assigned_in_expr_shallow(e, out));
}

/// As [`assigned_in_block`], for the blocks inside an expression.
pub fn assigned_in(e: &Expr, out: &mut BTreeSet<String>) {
    walk::exprs(e, &mut |x| assigned_in_expr_shallow(x, out));
}

fn assigned_in_expr_shallow(e: &Expr, out: &mut BTreeSet<String>) {
    match &e.kind {
        ExprKind::Block(b) => assigned_in_block(b, out),
        ExprKind::Lambda(l) => {
            if let spec_lang_lir::node::LambdaBody::Block(b) = &l.body {
                assigned_in_block(b, out);
            }
        }
        _ => {}
    }
}

/// An operation's method; `save` and `restore` are every handler's own.
fn method(name: &str) -> String {
    let m = camel(name);
    if ["save", "restore", "constructor"].contains(&m.as_str()) { format!("{m}_") } else { m }
}

fn place_is_state(p: &Place) -> bool {
    matches!(p.root, PlaceRoot::State(_))
}

fn scan(e: &Expr, uses: &mut bool, calls: &mut Vec<FnId>) {
    match &e.kind {
        ExprKind::State(_) | ExprKind::CallRule { .. } | ExprKind::Effect { .. } | ExprKind::EffectRead { .. } => {
            *uses = true;
        }
        ExprKind::Block(b) => {
            for s in &b.stmts {
                if let spec_lang_lir::node::StmtKind::Assign { place, .. }
                | spec_lang_lir::node::StmtKind::Mutate { place, .. } = &s.kind
                {
                    *uses |= place_is_state(place);
                }
            }
        }
        ExprKind::CallFn { func, .. } => calls.push(*func),
        _ => {}
    }
}

/// A literal a constant can hold without computing anything.
fn is_literal(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Bool(_) | ExprKind::Int(_) | ExprKind::Text(_) | ExprKind::Unit => true,
        ExprKind::Prim { op: spec_lang_lir::Prim::IntNeg, args } => {
            args.len() == 1 && matches!(args[0].kind, ExprKind::Int(_))
        }
        _ => false,
    }
}
