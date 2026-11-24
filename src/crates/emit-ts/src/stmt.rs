//! Statements, blocks and places: a block at the end of a function prints as that function's statements, its
//! `return`s the function's; a write to `state[k].f` prints as a new value of the state built around the change.

use crate::expr::{Ex, Fx, P_ARROW, P_CALL, Sink};
use crate::types::trivial;
use spec_lang_lir::LTy;
use spec_lang_lir::node::*;

/// One step into a place, its key printed.
enum Key {
    Field(String),
    Map(String),
    List(String),
}

impl Fx<'_, '_, '_> {
    /// A block that is the body of the TypeScript function being printed: its value is returned.
    pub fn tail_block(&mut self, b: &Block) {
        self.push_scope();
        self.stmts(&b.stmts);
        self.pop_scope();
        if !ends(&b.stmts) && matches!(b.ty, LTy::Unit) {
            self.line("return null;");
        }
    }

    /// `e`, given to `sink`, as statements.
    pub fn tail(&mut self, e: &Expr, sink: Sink) {
        match (&e.kind, sink) {
            (ExprKind::Block(b), Sink::Return) => self.tail_block(b),
            (ExprKind::Block(b), Sink::Discard) if !returns(&b.stmts) => {
                self.push_scope();
                self.stmts(&b.stmts);
                self.pop_scope();
            }
            (ExprKind::If { cond, then, els }, _) if sink == Sink::Discard || complex(then) || complex(els) => {
                let c = self.expr(cond).text;
                self.open(format!("if ({c}) {{"));
                self.tail(then, sink);
                if sink == Sink::Return {
                    self.close("}");
                    self.tail(els, sink);
                } else if is_nothing(els) {
                    self.close("}");
                } else {
                    self.indent_else();
                    self.tail(els, sink);
                    self.close("}");
                }
            }
            (ExprKind::Match { scrutinee, arms }, _) => {
                let subject = match &scrutinee.kind {
                    ExprKind::Local(_) => self.expr(scrutinee).text,
                    _ => {
                        let s = self.bind("subject");
                        let v = self.expr(scrutinee).text;
                        self.line(format!("const {s} = {v};"));
                        s
                    }
                };
                self.match_arms(&subject, &scrutinee.ty, arms, sink);
            }
            (_, Sink::Return) => {
                let v = self.expr(e);
                self.ret(Some(v), &e.ty);
            }
            (_, Sink::Discard) => {
                if !is_nothing(e) {
                    let v = self.expr(e);
                    self.line(format!("{};", statement(&v)));
                }
            }
        }
    }

    fn indent_else(&mut self) {
        self.close("} else {");
        self.open("");
        // `open` printed an empty line; take it back.
        self.drop_last_line();
    }

    fn ret(&mut self, v: Option<Ex>, ty: &LTy) {
        let rt = "rt";
        let text = match v {
            None => "null".to_string(),
            Some(v) if matches!(ty, LTy::Never) => {
                self.line(format!("{};", statement(&v)));
                return;
            }
            Some(v) => match self.result.clone() {
                Some(d) if self.at_top() => format!("{rt}.check({d}, {})", v.text),
                _ => v.text,
            },
        };
        self.line(format!("return {text};"));
    }

    pub fn stmt(&mut self, s: &Stmt) {
        let rt = "rt";
        match &s.kind {
            StmtKind::Let { name, mutable, ty, check, value } => {
                let v = self.expr(value);
                let v = match check {
                    Some(c) if !trivial(self.out.plan, c) => {
                        let d = self.out.d_desc(c);
                        format!("{rt}.check({d}, {})", v.text)
                    }
                    _ => v.text,
                };
                let mutable = *mutable && self.assigned.contains(name);
                let ts = if self.unused_let { self.bind_unused(name) } else { self.bind(name) };
                let annotate = mutable || needs_annotation(value);
                let t = if annotate { format!(": {}", self.out.ts_type(ty)) } else { String::new() };
                if mutable {
                    self.mark_mutable(&ts);
                    self.line(format!("let {ts}{t} = {v};"));
                } else {
                    self.line(format!("const {ts}{t} = {v};"));
                }
            }
            StmtKind::Assign { place, value } => {
                let v = self.expr(value).text;
                self.write(place, v, s.span);
            }
            StmtKind::Mutate { place, op, args } => {
                let printed: Vec<Ex> = args.iter().map(|a| self.expr(a)).collect();
                let keys = self.keys(place);
                let read = self.read(place, &keys, s.span);
                let mut all = vec![read];
                all.extend(printed);
                let mut tys = vec![place.ty.clone()];
                tys.extend(args.iter().map(|a| a.ty.clone()));
                let new = self.prim(*op, all, &tys, &[]).text;
                self.write_keys(place, &keys, new, s.span);
            }
            StmtKind::Expr(e) => self.tail(e, Sink::Discard),
            StmtKind::Return(v) => match v {
                Some(e) => {
                    let x = self.expr(e);
                    self.ret(Some(x), &e.ty);
                }
                None => self.ret(None, &LTy::Unit),
            },
            StmtKind::Assert(e) => {
                let c = self.expr(e).text;
                let check = self.checks();
                self.line(format!("{check}.assert({c});"));
            }
            StmtKind::For { var, iter, body, .. } => {
                let xs = self.expr(iter).text;
                self.push_scope();
                let v = if crate::expr::block_mentions(body, var) { self.bind(var) } else { self.bind_unused(var) };
                self.open(format!("for (const {v} of {xs}) {{"));
                self.inner(body);
                self.close("}");
                self.pop_scope();
            }
            StmtKind::While { cond, body } => {
                let c = self.expr(cond).text;
                self.open(format!("while ({c}) {{"));
                self.inner(body);
                self.close("}");
            }
            StmtKind::If { cond, then, els } => {
                let c = self.expr(cond).text;
                self.open(format!("if ({c}) {{"));
                self.inner(then);
                let mut els = els.as_ref();
                loop {
                    match els {
                        None => {
                            self.close("}");
                            break;
                        }
                        Some(b) => match b.stmts.as_slice() {
                            [Stmt { kind: StmtKind::If { cond, then, els: next }, .. }] => {
                                let c = self.expr(cond).text;
                                self.close(format!("}} else if ({c}) {{"));
                                self.open("");
                                self.drop_last_line();
                                self.inner(then);
                                els = next.as_ref();
                            }
                            _ => {
                                self.indent_else();
                                self.inner(b);
                                self.close("}");
                                break;
                            }
                        },
                    }
                }
            }
            StmtKind::Expect { subject, expect } => {
                let check = self.checks();
                let got = self.expr(subject).at(P_ARROW);
                match expect {
                    Expected::Value(want) => {
                        let w = self.expr(want).at(P_ARROW);
                        self.line(format!("{check}.expect(() => {got}, () => {w});"));
                    }
                    Expected::Fail { name, args } => {
                        let name = crate::names::string_literal(name);
                        if args.is_empty() {
                            self.line(format!("{check}.expectFail(() => {got}, {name});"));
                        } else {
                            let data: Vec<String> = args.iter().map(|a| self.expr(a).text).collect();
                            self.line(format!("{check}.expectFail(() => {got}, {name}, () => [{}]);", data.join(", ")));
                        }
                    }
                }
            }
        }
    }

    /// The statements of a block inside a statement (a loop, an `if`), in their own scope.
    fn inner(&mut self, b: &Block) {
        self.push_scope();
        self.stmts(&b.stmts);
        self.pop_scope();
    }

    /// Statements in order; a `let` no later statement reads is named so.
    pub fn stmts(&mut self, stmts: &[Stmt]) {
        for (i, s) in stmts.iter().enumerate() {
            self.unused_let = matches!(&s.kind, StmtKind::Let { name, .. } if !crate::expr::stmts_mention(&stmts[i + 1..], name));
            self.stmt(s);
            self.unused_let = false;
        }
    }

    fn checks(&mut self) -> String {
        self.out.file.namespace("test/support/checks.ts", "check");
        "check".into()
    }

    fn keys(&mut self, p: &Place) -> Vec<Key> {
        p.path
            .iter()
            .map(|s| match s {
                PlaceStep::Field { ty, index } => Key::Field(self.field_prop(*ty, 0, *index)),
                PlaceStep::MapKey(k) => Key::Map(self.expr(k).text),
                PlaceStep::ListIndex(k) => Key::List(self.expr(k).text),
            })
            .collect()
    }

    fn root(&mut self, p: &Place, span: spec_lang_syntax::Span) -> String {
        match &p.root {
            PlaceRoot::State(s) => {
                self.world_used = true;
                let w = self.world.clone().unwrap_or_else(|| {
                    self.unsupported(Some(span), "a state is written where no world is in scope");
                    "w".into()
                });
                format!("{w}.{}", self.out.plan.state_prop[s.index()])
            }
            PlaceRoot::Local(n) => self.expr(&Expr::new(ExprKind::Local(n.clone()), p.ty.clone(), span)).text,
        }
    }

    /// The value at a place.
    fn read(&mut self, p: &Place, keys: &[Key], span: spec_lang_syntax::Span) -> Ex {
        let rt = "rt";
        let mut cur = Ex::primary(self.root(p, span));
        for k in keys {
            cur = match k {
                Key::Field(f) => Ex::new(format!("{}.{f}", cur.at(P_CALL)), P_CALL),
                Key::Map(k) => Ex::new(format!("{}.at({k})", cur.at(P_CALL)), P_CALL),
                Key::List(i) => Ex::new(format!("{rt}.at({}, {i})", cur.text), P_CALL),
            };
        }
        cur
    }

    fn write(&mut self, p: &Place, value: String, span: spec_lang_syntax::Span) {
        // The value first, then the keys: when a key is more than a plain read, both are held in locals.
        let plain = p.path.iter().all(|s| match s {
            PlaceStep::Field { .. } => true,
            PlaceStep::MapKey(k) | PlaceStep::ListIndex(k) => {
                matches!(k.kind, ExprKind::Local(_) | ExprKind::Int(_) | ExprKind::Text(_) | ExprKind::Bool(_))
            }
        });
        let value = if plain {
            value
        } else {
            let v = self.bind("value");
            self.line(format!("const {v} = {value};"));
            v
        };
        let keys = if plain {
            self.keys(p)
        } else {
            let printed = self.keys(p);
            printed
                .into_iter()
                .map(|k| {
                    let mut hold = |t: String| {
                        let n = self.bind("key");
                        self.line(format!("const {n} = {t};"));
                        n
                    };
                    match k {
                        Key::Field(f) => Key::Field(f),
                        Key::Map(t) => Key::Map(hold(t)),
                        Key::List(t) => Key::List(hold(t)),
                    }
                })
                .collect()
        };
        self.write_keys(p, &keys, value, span);
    }

    fn write_keys(&mut self, p: &Place, keys: &[Key], value: String, span: spec_lang_syntax::Span) {
        let rt = "rt";
        let value = match &p.check {
            Some(c) if !trivial(self.out.plan, c) => {
                let d = self.out.d_desc(c);
                format!("{rt}.check({d}, {value})")
            }
            _ => value,
        };
        let root = self.root(p, span);
        let new = self.rebuild(&Ex::primary(root.clone()), keys, value);
        self.line(format!("{root} = {new};"));
    }

    /// `cur` with the value at `keys` replaced: a copy of each level on the way.
    fn rebuild(&mut self, cur: &Ex, keys: &[Key], value: String) -> String {
        let rt = "rt";
        let Some((k, rest)) = keys.split_first() else { return value };
        match k {
            Key::Field(f) => {
                let inner = Ex::new(format!("{}.{f}", cur.at(P_CALL)), P_CALL);
                let v = self.rebuild(&inner, rest, value);
                format!("{{ ...{}, {f}: {v} }}", cur.text)
            }
            Key::Map(key) => {
                if rest.is_empty() {
                    return format!("{}.set({key}, {value})", cur.at(P_CALL));
                }
                let inner = Ex::new(format!("{}.at({key})", cur.at(P_CALL)), P_CALL);
                let v = self.rebuild(&inner, rest, value);
                format!("{}.set({key}, {v})", cur.at(P_CALL))
            }
            Key::List(i) => {
                let inner = Ex::new(format!("{rt}.at({}, {i})", cur.text), P_CALL);
                let v = self.rebuild(&inner, rest, value);
                format!("{rt}.setAt({}, {i}, {v})", cur.text)
            }
        }
    }
}

/// Whether the statements always end in a `return` or a failure.
pub fn ends(stmts: &[Stmt]) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Return(_)) => true,
        Some(StmtKind::Expr(e)) => matches!(e.ty, LTy::Never),
        Some(StmtKind::If { then, els: Some(els), .. }) => ends(&then.stmts) && ends(&els.stmts),
        _ => false,
    }
}

/// Whether a `return` is among the statements (outside any lambda): such a block keeps its own function.
fn returns(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match &s.kind {
        StmtKind::Return(_) => true,
        StmtKind::For { body, .. } | StmtKind::While { body, .. } => returns(&body.stmts),
        StmtKind::If { then, els, .. } => returns(&then.stmts) || els.as_ref().is_some_and(|b| returns(&b.stmts)),
        _ => false,
    })
}

/// An expression better printed as statements than as one expression.
fn complex(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Block(b) => !matches!(b.stmts.as_slice(), [] | [Stmt { kind: StmtKind::Return(Some(_)), .. }]),
        ExprKind::Match { .. } => true,
        ExprKind::If { then, els, .. } => complex(then) || complex(els),
        _ => false,
    }
}

/// `()` or an empty block: nothing to evaluate.
fn is_nothing(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Unit => true,
        ExprKind::Block(b) => b.stmts.is_empty(),
        _ => false,
    }
}

/// A value whose TypeScript type a `const` would not infer as the program's.
fn needs_annotation(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::List(xs) => xs.is_empty(),
        ExprKind::None | ExprKind::Lambda(_) => true,
        _ => false,
    }
}

/// An expression as a statement: one that starts with `{` would parse as a block.
fn statement(v: &Ex) -> String {
    if v.text.starts_with('{') { format!("({})", v.text) } else { v.text.clone() }
}
