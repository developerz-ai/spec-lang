//! Facts derived from the typed program that license a backend's strategies
//! (docs/idea/10-compiler-and-ir.md, Planner; the Optimizations tables of the
//! hand projections). Each fact says what it is and where it was found, so a
//! backend can name it as the licence in `provenance.json`.

use std::collections::{BTreeMap, BTreeSet};

use spec_lang_diag::Diag;
use spec_lang_ir::{LawId, StateId};

use super::Front;
use super::typed::*;

#[derive(Debug, Default)]
pub struct Facts {
    /// Parallel to [`Front::rules`].
    pub rules: Vec<RuleFacts>,
    /// A `Map[K, R]` state whose every value sits under its own field `R.f`:
    /// every write is `m[k] = R(..., f: k, ...)` and no rule assigns `m[_].f`.
    /// The field's index.
    pub keyed: BTreeMap<StateId, usize>,
    /// Laws of the form `t.states.all(w => P(w))`: invariants of every world,
    /// so a world read from outside must meet them.
    pub invariants: Vec<LawId>,
    /// A refined `Text` whose claim bounds its byte length: every byte ASCII and
    /// `size <= n`. The bound, and the claim's fn, for provenance.
    pub inline_text: BTreeMap<spec_lang_ir::TypeId, (usize, String)>,
}

#[derive(Debug, Default)]
pub struct RuleFacts {
    /// States the body writes, from its assignments and removals.
    pub writes: BTreeSet<StateId>,
    /// The first guard is `k in m`, and the rest of the rule reaches `m` only as `m[k]`.
    pub entry: Option<Entry>,
    /// `let g = m.values.filter(p); for x in g { m.remove(x.f); }` with `m` keyed by `f`
    /// and `g` used after only for its size.
    pub retain: Option<Retain>,
    /// A read-only rule that reads the values of a keyed map only through
    /// some of their fields: a store read may build only those.
    pub projection: Option<Projection>,
}

/// The fields of a keyed map's values that a rule (and every state invariant) reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    pub state: StateId,
    /// Field indexes, ascending; a strict subset of the record's fields, each a
    /// scalar (`Bool`, `Int`, a refined `Int`, a sum without data).
    pub fields: Vec<usize>,
    /// The read set, as a sentence, for provenance.
    pub licence: String,
}

impl RuleFacts {
    pub fn read_only(&self) -> bool {
        self.writes.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub state: StateId,
    /// The key: a parameter of the rule.
    pub key: String,
    /// The body assigns a field of `m[k]`.
    pub mutable: bool,
    /// The body's only use of `m` is `m.remove(k)`, at its top level.
    pub removes: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Retain {
    /// Index of the `let` in the body; the `for` follows it.
    pub at: usize,
    pub state: StateId,
    /// The local bound by the `let`.
    pub name: String,
}

pub fn derive(front: &Front<'_>) -> Result<Facts, Diag> {
    let mut facts = Facts::default();
    for state in &front.states {
        if let Some(f) = keyed_field(front, state.id) {
            facts.keyed.insert(state.id, f);
        }
    }
    for rule in &front.rules {
        let mut rf = RuleFacts::default();
        walk_stmts(&rule.body, &mut |s| match &s.kind {
            S::Assign { state, .. } | S::Remove { state, .. } => {
                rf.writes.insert(*state);
            }
            _ => {}
        });
        check_uses(front, rule, &rf.writes)?;
        rf.entry = entry(rule);
        rf.retain = retain(rule, &facts.keyed);
        facts.rules.push(rf);
    }
    for law in &front.laws {
        if law.others.is_empty()
            && let K::Core(Op::All, ops) = &law.body.kind
            && let [states, _] = ops.as_slice()
            && let K::Core(Op::States, t) = &states.kind
            && matches!(t.as_slice(), [TExpr { kind: K::Local(n), .. }] if n == &law.trace)
        {
            facts.invariants.push(law.id);
        }
    }
    for (i, _) in front.prog.types.iter().enumerate() {
        let id = spec_lang_ir::TypeId(i as u32);
        if let Some(bound) = inline_bound(front, id) {
            facts.inline_text.insert(id, bound);
        }
    }
    for (i, rule) in front.rules.iter().enumerate() {
        if facts.rules[i].read_only() {
            facts.rules[i].projection = projection(front, rule, &facts);
        }
    }
    Ok(facts)
}

/// A body that writes a state its `@uses` does not list is an error; `@uses` is the interface.
fn check_uses(front: &Front<'_>, rule: &super::TRule, writes: &BTreeSet<StateId>) -> Result<(), Diag> {
    let decl = front.prog.rule(rule.id);
    for s in writes {
        let name = &front.prog.state(*s).info.name;
        let declared = decl.uses.iter().any(|u| u == &format!("{name}.write"));
        if !declared {
            return Err(Diag::error("E-EFFECT-UNDECLARED", decl.info.span, format!("rule writes {name}"))
                .with_decl(front.decl_path(&decl.info))
                .with_expected(format!("@uses({name}.write, ...)"), format!("@uses({})", decl.uses.join(", "))));
        }
    }
    Ok(())
}

fn keyed_field(front: &Front<'_>, state: StateId) -> Option<usize> {
    let st = front.states.iter().find(|s| s.id == state)?;
    let T::Map(_, v) = &st.ty else { return None };
    let T::Named(rec) = **v else { return None };
    super::record_fields(front.prog, rec)?;
    let mut field: Option<usize> = None;
    let mut ok = true;
    for rule in &front.rules {
        walk_stmts(&rule.body, &mut |s| {
            let S::Assign { state: s2, path, value } = &s.kind else { return };
            if *s2 != state {
                return;
            }
            match path.as_slice() {
                [PathStep::Index(k)] => {
                    let K::Record { fields, .. } = &value.kind else {
                        ok = false;
                        return;
                    };
                    let found = fields.iter().position(|f| same_value(f, k));
                    match (found, field) {
                        (Some(i), None) => field = Some(i),
                        (Some(i), Some(j)) if i == j => {}
                        _ => ok = false,
                    }
                }
                [PathStep::Index(_), PathStep::Field(j), ..] => {
                    if field == Some(*j) {
                        ok = false;
                    }
                }
                _ => ok = false,
            }
        });
    }
    // A later field write to the key field is caught on a second pass.
    if let Some(f) = field {
        for rule in &front.rules {
            walk_stmts(&rule.body, &mut |s| {
                if let S::Assign { state: s2, path, .. } = &s.kind
                    && *s2 == state
                    && matches!(path.as_slice(), [PathStep::Index(_), PathStep::Field(j), ..] if *j == f)
                {
                    ok = false;
                }
            });
        }
    }
    if ok { field } else { None }
}

/// Two expressions that are the same local, through any refinement change.
fn same_value(a: &TExpr, b: &TExpr) -> bool {
    match (&a.kind, &b.kind) {
        (K::Local(x), K::Local(y)) => x == y,
        (K::Widen(x) | K::Narrow { expr: x, .. }, _) => same_value(x, b),
        (_, K::Widen(y) | K::Narrow { expr: y, .. }) => same_value(a, y),
        _ => false,
    }
}

fn entry(rule: &super::TRule) -> Option<Entry> {
    let first = rule.guards.first()?;
    let K::Core(Op::Contains, ops) = &first.cond.kind else { return None };
    let [k, m] = ops.as_slice() else { return None };
    let (K::Local(key), K::State(state)) = (&k.kind, &m.kind) else { return None };
    if !matches!(m.ty, T::Map(..)) || !rule.params.iter().any(|p| &p.name == key) {
        return None;
    }
    let state = *state;
    let mut uses = Uses { state, key, ok: true, mutable: false, removes: 0, other: 0 };
    for g in &rule.guards[1..] {
        uses.expr(&g.cond);
    }
    for s in &rule.body {
        uses.stmt(s, true);
    }
    if !uses.ok || uses.removes > 1 || (uses.removes == 1 && (uses.other > 0 || rule.guards.len() > 1)) {
        return None;
    }
    Some(Entry { state, key: key.clone(), mutable: uses.mutable, removes: uses.removes == 1 })
}

/// How a rule reaches one state map, for [`entry`].
struct Uses<'a> {
    state: StateId,
    key: &'a str,
    ok: bool,
    mutable: bool,
    removes: usize,
    /// Reads and writes of `m[k]`.
    other: usize,
}

impl Uses<'_> {
    fn is_key(&self, e: &TExpr) -> bool {
        matches!(&e.kind, K::Local(n) if n == self.key)
    }

    fn expr(&mut self, e: &TExpr) {
        match &e.kind {
            K::State(s) if *s == self.state => self.ok = false,
            K::Index { map, key } if matches!(map.kind, K::State(s) if s == self.state) => {
                if self.is_key(key) {
                    self.other += 1;
                } else {
                    self.ok = false;
                }
            }
            _ => children(e, &mut |c| self.expr(c)),
        }
    }

    fn stmt(&mut self, s: &TStmt, top: bool) {
        match &s.kind {
            S::Remove { state, key } if *state == self.state => {
                if top && self.is_key(key) {
                    self.removes += 1;
                } else {
                    self.ok = false;
                }
            }
            S::Assign { state, path, value } if *state == self.state => {
                match path.as_slice() {
                    [PathStep::Index(k), PathStep::Field(_), ..] if self.is_key(k) => {
                        self.mutable = true;
                        self.other += 1;
                    }
                    _ => self.ok = false,
                }
                self.expr(value);
            }
            _ => {
                let mut exprs = Vec::new();
                let mut blocks = Vec::new();
                stmt_children(s, &mut |e| exprs.push(e.clone()), &mut |b| blocks.push(b.to_vec()));
                for e in &exprs {
                    self.expr(e);
                }
                for b in &blocks {
                    for x in b {
                        self.stmt(x, false);
                    }
                }
            }
        }
    }
}

fn retain(rule: &super::TRule, keyed: &BTreeMap<StateId, usize>) -> Option<Retain> {
    for (at, pair) in rule.body.windows(2).enumerate() {
        let (S::Let { name, value }, S::For { var, iter, body }) = (&pair[0].kind, &pair[1].kind) else { continue };
        let K::Core(Op::Filter, ops) = &value.kind else { continue };
        let [values, _] = ops.as_slice() else { continue };
        let K::Core(Op::Values, m) = &values.kind else { continue };
        let [TExpr { kind: K::State(state), .. }] = m.as_slice() else { continue };
        let Some(field) = keyed.get(state) else { continue };
        if !matches!(&iter.kind, K::Local(n) if n == name) {
            continue;
        }
        let [TStmt { kind: S::Remove { state: s2, key }, .. }] = body.as_slice() else { continue };
        let K::Field(x, f) = &key.kind else { continue };
        if s2 != state || f != field || !matches!(&x.kind, K::Local(n) if n == var) {
            continue;
        }
        // After the loop, `g` may only be counted.
        let mut only_size = true;
        for s in &rule.body[at + 2..] {
            stmt_exprs(s, &mut |e| check_only_size(e, name, &mut only_size));
        }
        if only_size {
            return Some(Retain { at, state: *state, name: name.clone() });
        }
    }
    None
}

fn check_only_size(e: &TExpr, name: &str, ok: &mut bool) {
    match &e.kind {
        K::Core(Op::Size, ops) if matches!(ops.as_slice(), [TExpr { kind: K::Local(n), .. }] if n == name) => {}
        K::Local(n) if n == name => *ok = false,
        _ => children(e, &mut |c| check_only_size(c, name, ok)),
    }
}

/// Every statement, nested ones included.
pub fn walk_stmts(body: &[TStmt], f: &mut impl FnMut(&TStmt)) {
    for s in body {
        f(s);
        match &s.kind {
            S::For { body, .. } => walk_stmts(body, f),
            S::If { then, els, .. } => {
                walk_stmts(then, f);
                walk_stmts(els, f);
            }
            _ => {}
        }
    }
}

/// Every expression directly in a statement, and the statements nested in it.
pub fn stmt_children(s: &TStmt, e: &mut impl FnMut(&TExpr), b: &mut impl FnMut(&[TStmt])) {
    match &s.kind {
        S::Let { value, .. } | S::Expr(value) | S::Assert(value) | S::Return(Some(value)) => e(value),
        S::Return(None) => {}
        S::Assign { path, value, .. } => {
            for p in path {
                if let PathStep::Index(k) = p {
                    e(k);
                }
            }
            e(value);
        }
        S::Remove { key, .. } => e(key),
        S::For { iter, body, .. } => {
            e(iter);
            b(body);
        }
        S::If { cond, then, els } => {
            e(cond);
            b(then);
            b(els);
        }
        S::ExpectValue { call, value } => {
            e(call);
            e(value);
        }
        S::ExpectFail { call, .. } => e(call),
    }
}

/// Every expression in a statement, nested statements included.
pub fn stmt_exprs(s: &TStmt, f: &mut impl FnMut(&TExpr)) {
    let mut nested: Vec<TStmt> = Vec::new();
    stmt_children(s, f, &mut |b| nested.extend(b.iter().cloned()));
    for n in &nested {
        stmt_exprs(n, f);
    }
}

/// The direct sub-expressions of an expression.
pub fn children(e: &TExpr, f: &mut impl FnMut(&TExpr)) {
    match &e.kind {
        K::Unit | K::Bool(_) | K::Int(_) | K::Text(_) | K::Local(_) | K::Const(_) | K::State(_) | K::OutText => {}
        K::Variant { .. } => {}
        K::Interp(parts) => {
            for p in parts {
                if let Part::Expr(x) = p {
                    f(x);
                }
            }
        }
        K::Some(x) | K::OutLine(x) | K::Field(x, _) | K::Not(x) | K::Neg(x) | K::Widen(x) | K::IsFail(x) => f(x),
        K::Narrow { expr, .. } => f(expr),
        K::StepPart(x, _) | K::WorldState(x, _) => f(x),
        K::EventArg { event, .. } | K::IsEvent { event, .. } => f(event),
        K::Lambda { body, .. } => f(body),
        K::Record { fields: xs, .. }
        | K::CallFn { args: xs, .. }
        | K::CallRule { args: xs, .. }
        | K::Core(_, xs)
        | K::Tuple(xs)
        | K::List(xs)
        | K::SetOf(xs) => xs.iter().for_each(f),
        K::MapOf(pairs) => {
            for (a, b) in pairs {
                f(a);
                f(b);
            }
        }
        K::Index { map, key } => {
            f(map);
            f(key);
        }
        K::Bin(_, a, b) => {
            f(a);
            f(b);
        }
        K::Compare(a, links) => {
            f(a);
            for (_, b) in links {
                f(b);
            }
        }
        K::If { cond, then, els } => {
            f(cond);
            f(then);
            f(els);
        }
        K::Match { scrutinee, arms } => {
            f(scrutinee);
            for (_, v) in arms {
                f(v);
            }
        }
    }
}

/// Which fields of which keyed map a read-only rule reads, if it reads that map
/// only by size, by key, and through lambdas that touch scalar fields of the values.
fn projection(front: &Front<'_>, rule: &super::TRule, facts: &Facts) -> Option<Projection> {
    for &state in facts.keyed.keys() {
        let mut reads = Reads { state, fields: BTreeSet::new(), ok: true, touched: false };
        for s in &rule.body {
            stmt_exprs(s, &mut |e| reads.top(e));
        }
        for g in &rule.guards {
            reads.top(&g.cond);
        }
        if !reads.ok || !reads.touched {
            continue;
        }
        let rule_fields = reads.fields.clone();
        // Every state invariant checked on read must be decidable from the same fields.
        for law in &facts.invariants {
            let l = front.laws.iter().find(|l| l.id == *law)?;
            reads.top(&l.body);
        }
        if !reads.ok {
            continue;
        }
        let st = front.states.iter().find(|s| s.id == state)?;
        let T::Map(_, v) = &st.ty else { continue };
        let T::Named(rec) = **v else { continue };
        let all = super::record_fields(front.prog, rec)?;
        if reads.fields.len() >= all.len() {
            continue;
        }
        let t = super::typer::Typer::new(front.prog, String::new());
        let scalar = reads
            .fields
            .iter()
            .all(|i| all.get(*i).and_then(|f| t.ty(&f.ty, f.span).ok()).is_some_and(|ty| is_scalar(front, &ty)));
        if !scalar {
            continue;
        }
        let name = &front.prog.state(state).info.name;
        let shown = |fs: &BTreeSet<usize>| {
            fs.iter().filter_map(|i| all.get(*i)).map(|f| format!("{name}[_].{}", f.name)).collect::<Vec<_>>()
        };
        let mut read = vec![format!("{name}.size")];
        read.extend(shown(&rule_fields));
        let licence = format!(
            "read set of the rule: {}; @uses({name}.read) and no write; the store's invariants read no other field of the values",
            read.join(", ")
        );
        return Some(Projection { state, fields: reads.fields.into_iter().collect(), licence });
    }
    None
}

fn is_scalar(front: &Front<'_>, t: &T) -> bool {
    match t {
        T::Bool | T::Int => true,
        T::Named(id) => match &front.prog.ty(*id).def {
            spec_lang_ir::TypeDef::Sum(vs) => vs.iter().all(|v| v.fields.is_empty()),
            spec_lang_ir::TypeDef::Alias(_) => {
                super::typer::alias_base(front.prog, *id).is_some_and(|(b, _)| b == T::Int)
            }
            _ => false,
        },
        _ => false,
    }
}

/// How a rule reaches one keyed map, for [`projection`].
struct Reads {
    state: StateId,
    fields: BTreeSet<usize>,
    ok: bool,
    touched: bool,
}

impl Reads {
    fn is_map(&self, e: &TExpr) -> bool {
        matches!(&e.kind, K::State(s) if *s == self.state) || matches!(&e.kind, K::WorldState(_, s) if *s == self.state)
    }

    fn top(&mut self, e: &TExpr) {
        match &e.kind {
            _ if self.is_map(e) => {
                self.touched = true;
                self.ok = false;
            }
            K::Core(Op::Size | Op::Keys, ops) if ops.len() == 1 && self.is_map(&ops[0]) => self.touched = true,
            K::Core(Op::Size, ops) if ops.len() == 1 && self.values_chain(&ops[0]) => {}
            K::Core(Op::All | Op::Any, ops) if ops.len() == 2 && self.values_chain(&ops[0]) => self.lambda(&ops[1]),
            // `m.keys.all(k => ...)`: `k` may be used only as a key, `m[k]` only for its fields.
            K::Core(Op::All | Op::Any, ops) if ops.len() == 2 && self.keys_chain(&ops[0]) => {
                if let K::Lambda { param, body } = &ops[1].kind {
                    self.key_uses(body, param);
                }
            }
            _ => children(e, &mut |c| self.top(c)),
        }
    }

    /// `m.values`, then any `filter`s: true when it is one, with the lambdas recorded.
    fn values_chain(&mut self, e: &TExpr) -> bool {
        match &e.kind {
            K::Core(Op::Values, ops) if ops.len() == 1 && self.is_map(&ops[0]) => {
                self.touched = true;
                true
            }
            K::Core(Op::Filter, ops) if ops.len() == 2 => {
                let inner = ops[0].clone();
                if self.values_chain(&inner) {
                    self.lambda(&ops[1]);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn keys_chain(&mut self, e: &TExpr) -> bool {
        matches!(&e.kind, K::Core(Op::Keys, ops) if ops.len() == 1 && self.is_map(&ops[0])) && {
            self.touched = true;
            true
        }
    }

    /// Inside `m.keys.all(k => ...)`: `m[k].f` reads a field; other uses of `m` are not allowed.
    fn key_uses(&mut self, e: &TExpr, k: &str) {
        match &e.kind {
            K::Field(x, i) if matches!(&x.kind, K::Index { map, key } if self.is_map(map) && matches!(&key.kind, K::Local(n) if n == k)) =>
            {
                self.fields.insert(*i);
            }
            _ if self.is_map(e) => self.ok = false,
            _ => children(e, &mut |c| self.key_uses(c, k)),
        }
    }

    /// A lambda over values: its parameter may appear only as `p.field`.
    fn lambda(&mut self, l: &TExpr) {
        let K::Lambda { param, body } = &l.kind else {
            self.ok = false;
            return;
        };
        self.fields_of(body, param);
    }

    fn fields_of(&mut self, e: &TExpr, p: &str) {
        match &e.kind {
            // `m[k].f` with `k` a key of `m`: a read of field `f`.
            K::Field(x, i) if matches!(&x.kind, K::Index { map, key } if self.is_map(map) && matches!(&key.kind, K::Local(n) if n == p)) =>
            {
                self.fields.insert(*i);
            }
            K::Field(x, i) if matches!(&x.kind, K::Local(n) if n == p) => {
                self.fields.insert(*i);
            }
            K::Local(n) if n == p => self.ok = false,
            _ => {
                if self.is_map(e) {
                    self.ok = false;
                }
                children(e, &mut |c| self.fields_of(c, p));
            }
        }
    }
}

/// When a `sort` key ends in the field that keys the sorted map's values, no
/// two values share a key, so an unstable sort has the one answer a stable one has.
/// Returns the licence.
pub fn sort_key_unique(front: &Front<'_>, facts: &Facts, recv: &TExpr, key: &TExpr) -> Option<String> {
    let mut source = recv;
    let state = loop {
        match &source.kind {
            K::Core(Op::Filter, ops) => source = ops.first()?,
            K::Core(Op::Values, ops) => match &ops.first()?.kind {
                K::State(s) => break *s,
                _ => return None,
            },
            _ => return None,
        }
    };
    let field = *facts.keyed.get(&state)?;
    let K::Lambda { param, body } = &key.kind else { return None };
    let ends_in_key = |e: &TExpr, p: &str| -> bool {
        let last = match &e.kind {
            K::Tuple(xs) => xs.last(),
            _ => Some(e),
        };
        let mut last = last;
        while let Some(TExpr { kind: K::Widen(x), .. }) = last {
            last = Some(x);
        }
        matches!(last.map(|x| &x.kind), Some(K::Field(x, i)) if *i == field && matches!(&x.kind, K::Local(n) if n == p))
    };
    let via = match &body.kind {
        K::CallFn { f, args } if matches!(args.as_slice(), [TExpr { kind: K::Local(n), .. }] if n == param) => {
            let tf = front.func(*f)?;
            let [TStmt { kind: S::Return(Some(v)), .. }] = tf.body.as_slice() else { return None };
            let p = &tf.params.first()?.name;
            ends_in_key(v, p).then(|| format!("fn {}", front.prog.func(*f).info.name))
        }
        _ => ends_in_key(body, param).then(|| "the key".to_string()),
    }?;
    let sname = &front.prog.state(state).info.name;
    let fname = super::record_fields(
        front.prog,
        match front.states.iter().find(|s| s.id == state)?.ty {
            T::Map(_, ref v) => match **v {
                T::Named(r) => r,
                _ => return None,
            },
            _ => return None,
        },
    )?
    .get(field)?
    .name
    .clone();
    Some(format!("{via} ends in `.{fname}`, which keys {sname}: no two values share a sort key"))
}

/// For `type X = Text where f(self)` with `f`'s body a conjunction holding
/// `t.size <= n` (or `lo <= t.size <= n`) and `t.bytes.all(...)` admitting only
/// bytes below 128: every value is at most `n` bytes.
fn inline_bound(front: &Front<'_>, id: spec_lang_ir::TypeId) -> Option<(usize, String)> {
    let (base, claim) = super::typer::alias_base(front.prog, id)?;
    if base != T::Text {
        return None;
    }
    let claim = claim?;
    let spec_lang_ir::ExprKind::Call { callee: spec_lang_ir::Callee::Fn(f), .. } = &claim.expr.kind else {
        return None;
    };
    let tf = front.func(*f)?;
    let p = &tf.params.first()?.name;
    let [TStmt { kind: S::Return(Some(body)), .. }] = tf.body.as_slice() else { return None };
    let mut conj = Vec::new();
    conjuncts(body, &mut conj);
    let size_bound = conj.iter().find_map(|c| size_upper(c, p))?;
    let ascii = conj.iter().any(|c| {
        matches!(&c.kind, K::Core(Op::All, ops) if matches!(ops.as_slice(),
            [TExpr { kind: K::Core(Op::Bytes, b), .. }, TExpr { kind: K::Lambda { param, body }, .. }]
            if matches!(b.as_slice(), [TExpr { kind: K::Local(n), .. }] if n == p) && ascii_only(body, param)))
    });
    let fname = &front.prog.func(*f).info.name;
    ascii.then(|| {
        (size_bound, format!("fn {fname} admits only ASCII bytes and size <= {size_bound}: at most {size_bound} bytes"))
    })
}

fn conjuncts<'a>(e: &'a TExpr, out: &mut Vec<&'a TExpr>) {
    match &e.kind {
        K::Bin(Bin::And, a, b) => {
            conjuncts(a, out);
            conjuncts(b, out);
        }
        _ => out.push(e),
    }
}

/// `t.size <= n` as the last link of a comparison.
fn size_upper(e: &TExpr, p: &str) -> Option<usize> {
    let K::Compare(head, links) = &e.kind else { return None };
    let mut left = &**head;
    for (op, right) in links {
        let is_size = |x: &TExpr| matches!(&x.kind, K::Core(Op::Size, o) if matches!(o.as_slice(), [TExpr { kind: K::Local(n), .. }] if n == p));
        if *op == spec_lang_ir::CmpOp::Le
            && is_size(left)
            && let K::Int(n) = right.kind
        {
            return usize::try_from(n).ok();
        }
        left = right;
    }
    None
}

/// A predicate on a byte that holds only below 128.
fn ascii_only(e: &TExpr, p: &str) -> bool {
    match &e.kind {
        K::Bin(Bin::Or, a, b) => ascii_only(a, p) && ascii_only(b, p),
        K::Bin(Bin::And, a, b) => ascii_only(a, p) || ascii_only(b, p),
        K::Compare(head, links) => {
            let ops: Vec<&TExpr> = std::iter::once(&**head).chain(links.iter().map(|(_, x)| x)).collect();
            let at = ops.iter().position(|x| matches!(&x.kind, K::Local(n) if n == p));
            match (at, links.as_slice()) {
                // `x == c` or `lo <= x <= hi`.
                (Some(_), [(spec_lang_ir::CmpOp::Eq, c)]) => matches!(c.kind, K::Int(n) if n < 128),
                (Some(1), [(spec_lang_ir::CmpOp::Le, _), (spec_lang_ir::CmpOp::Le, hi)]) => {
                    matches!(hi.kind, K::Int(n) if n < 128)
                }
                _ => false,
            }
        }
        _ => false,
    }
}
