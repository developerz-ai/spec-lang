//! Expressions. `Fx` prints the body of one TypeScript function: its locals (each with a name unique in the
//! function, so no TypeScript scope ever shadows another), its lines, and every expression in it. An
//! expression prints as text with its precedence; one that needs statements (a block, a `match` that binds)
//! prints as statements where it stands at the end of a function, and as an immediately called arrow elsewhere.

use crate::names::{camel, int_literal, string_literal, template_part};
use crate::types::{Out, trivial};
use spec_lang_diag::Diag;
use spec_lang_lir::node::*;
use spec_lang_lir::program::TypeDef;
use spec_lang_lir::{LTy, Prim};
use std::collections::BTreeSet;

/// Precedence levels of the JavaScript grammar, as far as the printer needs them.
pub const P_ARROW: u8 = 2;
pub const P_COND: u8 = 3;
pub const P_OR: u8 = 4;
pub const P_AND: u8 = 5;
pub const P_EQ: u8 = 8;
pub const P_REL: u8 = 9;
pub const P_ADD: u8 = 11;
pub const P_MUL: u8 = 12;
pub const P_UNARY: u8 = 14;
pub const P_CALL: u8 = 17;
pub const P_PRIMARY: u8 = 18;

/// A printed expression and how tightly it binds.
#[derive(Clone, Debug)]
pub struct Ex {
    pub text: String,
    pub prec: u8,
}

impl Ex {
    pub fn new(text: impl Into<String>, prec: u8) -> Self {
        Ex { text: text.into(), prec }
    }
    pub fn primary(text: impl Into<String>) -> Self {
        Ex::new(text, P_PRIMARY)
    }
    /// The text, in parentheses when it binds looser than `min`.
    pub fn at(&self, min: u8) -> String {
        if self.prec < min { format!("({})", self.text) } else { self.text.clone() }
    }
}

/// Where a scope entry points: a local's TypeScript name, or a pattern's binding read again from its path.
#[derive(Clone, Debug)]
struct Local {
    lir: String,
    ts: String,
}

/// The printer of one TypeScript function body.
#[derive(Debug)]
pub struct Fx<'o, 'a, 'l> {
    pub out: &'o mut Out<'a, 'l>,
    scopes: Vec<Vec<Local>>,
    used: BTreeSet<String>,
    mutable: BTreeSet<String>,
    /// `w`, when the world is in scope.
    pub world: Option<String>,
    /// A `return` at the function's own level checks its value against this type.
    pub result: Option<String>,
    depth: usize,
    /// Local names some statement assigns: only these are `let`.
    pub assigned: BTreeSet<String>,
    /// Whether `w` was printed.
    pub world_used: bool,
    /// The `let` being printed is read by no later statement.
    pub unused_let: bool,
    lines: Vec<String>,
    indent: usize,
    pub errors: Vec<Diag>,
}

impl<'o, 'a, 'l> Fx<'o, 'a, 'l> {
    pub fn new(out: &'o mut Out<'a, 'l>, world: bool) -> Self {
        let mut used: BTreeSet<String> = BTreeSet::new();
        if let Some(m) = out.module {
            used.extend(out.plan.module_names[m.index()].iter().cloned());
        }
        used.extend(out.plan.module_alias.iter().cloned());
        Fx {
            out,
            scopes: vec![Vec::new()],
            used,
            mutable: BTreeSet::new(),
            world: world.then(|| "w".to_string()),
            result: None,
            depth: 0,
            assigned: BTreeSet::new(),
            world_used: false,
            unused_let: false,
            lines: Vec::new(),
            indent: 0,
            errors: Vec::new(),
        }
    }

    // ---- lines -------------------------------------------------------------------------------------

    pub fn line(&mut self, text: impl AsRef<str>) {
        self.note_rt(text.as_ref());
        for l in text.as_ref().split('\n') {
            self.lines.push(format!("{}{l}", "  ".repeat(self.indent)));
        }
    }

    pub fn open(&mut self, text: impl AsRef<str>) {
        self.line(text);
        self.indent += 1;
    }

    pub fn close(&mut self, text: impl AsRef<str>) {
        self.indent = self.indent.saturating_sub(1);
        self.line(text);
    }

    pub fn drop_last_line(&mut self) {
        self.lines.pop();
    }

    /// At the function's own level: not inside a lambda or a nested body.
    pub fn at_top(&self) -> bool {
        self.depth == 0
    }

    /// The lines printed so far, taken.
    pub fn take_lines(&mut self) -> Vec<String> {
        std::mem::take(&mut self.lines)
    }

    /// Prints with `f` into a fresh set of lines at indent 1 and gives them back as one text: a nested body.
    fn nested(&mut self, f: impl FnOnce(&mut Self)) -> String {
        let lines = std::mem::take(&mut self.lines);
        let indent = std::mem::replace(&mut self.indent, 1);
        self.depth += 1;
        f(self);
        self.depth -= 1;
        self.indent = indent;
        let body = std::mem::replace(&mut self.lines, lines);
        body.join("\n")
    }

    // ---- locals ------------------------------------------------------------------------------------

    pub fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// A new local named after `lir`, unique in this function.
    pub fn bind(&mut self, lir: &str) -> String {
        let base = camel(lir);
        let mut ts = base.clone();
        let mut n = 2;
        while self.used.contains(&ts) {
            ts = format!("{base}{n}");
            n += 1;
        }
        self.used.insert(ts.clone());
        self.bind_as(lir, &ts);
        ts
    }

    /// A local nothing reads: `_` and a name unique in this function.
    pub fn bind_unused(&mut self, lir: &str) -> String {
        let base = format!("_{}", camel(lir));
        let mut ts = base.clone();
        let mut n = 2;
        while self.used.contains(&ts) {
            ts = format!("{base}{n}");
            n += 1;
        }
        self.bind_named(lir, &ts);
        ts
    }

    /// A local `lir` whose TypeScript name was chosen elsewhere (a parameter).
    pub fn bind_named(&mut self, lir: &str, ts: &str) {
        self.used.insert(ts.to_string());
        self.bind_as(lir, ts);
    }

    /// `lir` reads as `text` from here on in this scope.
    fn bind_as(&mut self, lir: &str, text: &str) {
        if let Some(s) = self.scopes.last_mut() {
            s.push(Local { lir: lir.to_string(), ts: text.to_string() });
        }
    }

    pub fn mark_mutable(&mut self, ts: &str) {
        self.mutable.insert(ts.to_string());
    }

    fn lookup(&mut self, lir: &str) -> String {
        for s in self.scopes.iter().rev() {
            if let Some(l) = s.iter().rev().find(|l| l.lir == lir) {
                return l.ts.clone();
            }
        }
        self.unsupported(None, &format!("`{lir}` has no value here"));
        camel(lir)
    }

    pub fn unsupported(&mut self, span: Option<spec_lang_syntax::Span>, what: &str) {
        let mut d = Diag::new(spec_lang_diag::Severity::Error, "E-UNSUPPORTED", format!("TypeScript: {what}"));
        if let Some(s) = span {
            d = d.with_span(s);
        }
        self.errors.push(d);
    }

    /// Imports the runtime when printed text reads it.
    fn note_rt(&mut self, text: &str) {
        let reads = text.match_indices("rt.").any(|(i, _)| {
            !text[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '$')
        });
        if reads {
            self.out.rt();
        }
    }

    fn w(&mut self, span: spec_lang_syntax::Span) -> String {
        self.world_used = true;
        match &self.world {
            Some(w) => w.clone(),
            None => {
                self.unsupported(Some(span), "the world is read where no world is in scope (a constant, a claim or a default)");
                "w".into()
            }
        }
    }

    // ---- expressions -------------------------------------------------------------------------------

    pub fn expr(&mut self, e: &Expr) -> Ex {
        let ex = self.expr_inner(e);
        self.note_rt(&ex.text);
        ex
    }

    fn expr_inner(&mut self, e: &Expr) -> Ex {
        let rt = "rt";
        match &e.kind {
            ExprKind::Unit => Ex::primary("null"),
            ExprKind::Bool(b) => Ex::primary(b.to_string()),
            ExprKind::Int(n) => Ex::new(int_literal(*n), if *n < 0 { P_UNARY } else { P_PRIMARY }),
            ExprKind::Text(t) => Ex::primary(string_literal(t)),
            ExprKind::Interp(parts) => self.interp(parts),
            ExprKind::Local(n) => Ex::primary(self.lookup(n)),
            ExprKind::Const(c) => {
                let decl = self.out.plan.lir.constant(*c);
                let name = self.out.plan.const_name[c.index()].clone();
                let r = self.out.decl(decl.info.module, &name);
                Ex::primary(if self.out.plan.const_lazy[c.index()] { format!("{r}()") } else { r })
            }
            ExprKind::State(s) => {
                let w = self.w(e.span);
                Ex::primary(format!("{w}.{}", self.out.plan.state_prop[s.index()]))
            }
            ExprKind::Variant { ty, tag } => self.variant(*ty, *tag),
            ExprKind::Some(x) => {
                let x = self.expr(x);
                Ex::primary(format!("{rt}.some({})", x.text))
            }
            ExprKind::None => Ex::primary(format!("{rt}.none")),
            ExprKind::Construct { ty, tag, args } => self.construct(*ty, *tag, args),
            ExprKind::CallFn { func, args } => {
                let f = self.out.plan.lir.func(*func);
                let name = self.out.plan.fn_name[func.index()].clone();
                let callee = self.out.decl(f.info.module, &name);
                let world = self.out.plan.fn_world[func.index()];
                self.call(&callee, world, args, e.span)
            }
            ExprKind::CallRule { rule, args } => {
                let r = self.out.plan.lir.rule(*rule);
                let name = self.out.plan.rule_name[rule.index()].clone();
                let callee = self.out.decl(r.info.module, &name);
                self.call(&callee, true, args, e.span)
            }
            ExprKind::Effect { handler, op, args, .. } => {
                let w = self.w(e.span);
                let h = self.out.plan.handler_prop[handler.index()].clone();
                let o = self.out.plan.op_name[handler.index()].get(*op as usize).cloned().unwrap_or_default();
                let params = self.out.plan.lir.handler(*handler).ops.get(*op as usize).and_then(|o| o.params.as_ref());
                let mut texts = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let x = self.expr(a).text;
                    let check = params.and_then(|ps| ps.get(i)).map(|p| p.check.clone());
                    texts.push(match check {
                        Some(c) if !trivial(self.out.plan, &c) => {
                            let d = self.out.d_desc(&c);
                            format!("{rt}.check({d}, {x})")
                        }
                        _ => x,
                    });
                }
                Ex::new(format!("{w}.fx.{h}.{o}({})", texts.join(", ")), P_CALL)
            }
            ExprKind::EffectRead { handler, op, .. } => {
                let w = self.w(e.span);
                let h = self.out.plan.handler_prop[handler.index()].clone();
                let o = self.out.plan.op_name[handler.index()].get(*op as usize).cloned().unwrap_or_default();
                Ex::new(format!("{w}.fx.{h}.{o}()"), P_CALL)
            }
            ExprKind::Prim { op, args } => {
                let printed: Vec<Ex> = self.prim_args(*op, args);
                let tys: Vec<LTy> = args.iter().map(|a| a.ty.clone()).collect();
                self.prim(*op, printed, &tys, args)
            }
            ExprKind::Lambda(l) => self.lambda(l),
            ExprKind::CallLambda { callee, args } => {
                let f = self.expr(callee);
                let args: Vec<String> = args.iter().map(|a| self.expr(a).text).collect();
                Ex::new(format!("{}({})", f.at(P_CALL), args.join(", ")), P_CALL)
            }
            ExprKind::Field { recv, ty, index, .. } => {
                let r = self.expr(recv);
                let prop = self.field_prop(*ty, 0, *index);
                Ex::new(format!("{}.{prop}", r.at(P_CALL)), P_CALL)
            }
            ExprKind::VariantField { recv, ty, name, by_tag } => {
                let r = self.expr(recv);
                let prop = by_tag
                    .iter()
                    .enumerate()
                    .find_map(|(tag, i)| i.map(|i| self.field_prop(*ty, tag as u32, i)))
                    .unwrap_or_else(|| camel(name));
                if by_tag.iter().all(Option::is_some) {
                    Ex::new(format!("{}.{prop}", r.at(P_CALL)), P_CALL)
                } else {
                    let t = self.out.ts_type(&e.ty);
                    Ex::new(format!("{rt}.fieldOf<{t}>({}, {})", r.text, string_literal(&prop)), P_CALL)
                }
            }
            ExprKind::WorldState { recv, state } => {
                let r = self.expr(recv);
                Ex::new(format!("{}.{}", r.at(P_CALL), self.out.plan.state_prop[state.index()]), P_CALL)
            }
            ExprKind::EventArg { recv, name, by_rule } => {
                let r = self.expr(recv);
                let at: Vec<String> = by_rule
                    .iter()
                    .filter_map(|(rule, i)| {
                        i.map(|i| format!("{}: {i}", string_literal(&self.out.plan.lir.rule(*rule).info.qualified)))
                    })
                    .collect();
                let t = self.out.ts_type(&e.ty);
                Ex::new(
                    format!("{rt}.eventArg({}, {}, {{ {} }}) as {t}", r.text, string_literal(name), at.join(", ")),
                    P_ARROW + 1,
                )
            }
            ExprKind::With { recv, ty, changes } => {
                let r = self.expr(recv);
                let tr = self.out.type_ref(*ty);
                let fields: Vec<String> = changes
                    .iter()
                    .map(|(i, x)| {
                        let prop = self.field_prop(*ty, 0, *i);
                        let v = self.expr(x).text;
                        if v == prop { prop } else { format!("{prop}: {v}") }
                    })
                    .collect();
                Ex::new(format!("{tr}.with({}, {{ {} }})", r.text, fields.join(", ")), P_CALL)
            }
            ExprKind::Is { value, pat } => {
                if let Some(subject) = self.simple(value) {
                    self.push_scope();
                    let (conds, _) = self.pattern(&subject, &value.ty, pat);
                    self.pop_scope();
                    return conjunction(conds);
                }
                let v = self.expr(value);
                let t = self.out.ts_type(&value.ty);
                self.push_scope();
                let s = self.bind("subject");
                let (conds, _) = self.pattern(&s, &value.ty, pat);
                self.pop_scope();
                let body = conjunction(conds);
                Ex::new(format!("(({s}: {t}) => {})({})", body.at(P_ARROW), v.text), P_CALL)
            }
            ExprKind::If { cond, then, els } => {
                let c = self.expr(cond);
                let a = self.expr(then);
                let b = self.expr(els);
                Ex::new(format!("{} ? {} : {}", c.at(P_COND + 1), a.at(P_COND + 1), b.at(P_COND)), P_COND)
            }
            ExprKind::Logic { op, left, right } => {
                let l = self.expr(left);
                let r = self.expr(right);
                match op {
                    Logic::And => Ex::new(format!("{} && {}", and_operand(&l), and_operand(&r)), P_AND),
                    Logic::Or => Ex::new(format!("{} || {}", l.at(P_AND + 1), r.at(P_AND + 1)), P_OR),
                    Logic::Implies => Ex::new(format!("!{} || {}", l.at(P_UNARY), r.at(P_AND + 1)), P_OR),
                }
            }
            ExprKind::Compare { first, links } => self.compare(first, links),
            ExprKind::Else { value, fallback, unwrap } => {
                let fails = may_fail(value);
                let v = self.expr(value);
                let fb = self.expr(fallback);
                let option = matches!(value.ty, LTy::Option(_));
                match (*unwrap && option, fails) {
                    (true, false) => Ex::new(format!("{rt}.someOr({}, () => {})", v.text, fb.at(P_ARROW)), P_CALL),
                    (true, true) => {
                        Ex::new(format!("{rt}.someOrElse(() => {}, () => {})", v.at(P_ARROW), fb.at(P_ARROW)), P_CALL)
                    }
                    (false, false) => v,
                    (false, true) => {
                        Ex::new(format!("{rt}.orElse(() => {}, () => {})", v.at(P_ARROW), fb.at(P_ARROW)), P_CALL)
                    }
                }
            }
            ExprKind::Fail { name, args } => {
                let data: Vec<String> = args.iter().map(|a| self.expr(a).text).collect();
                if data.is_empty() {
                    Ex::new(format!("{rt}.fail({})", string_literal(name)), P_CALL)
                } else {
                    Ex::new(format!("{rt}.fail({}, [{}])", string_literal(name), data.join(", ")), P_CALL)
                }
            }
            ExprKind::Match { scrutinee, arms } => self.match_expr(scrutinee, arms, &e.ty),
            ExprKind::Block(b) => self.block_expr(b),
            ExprKind::Hole(h) => Ex::new(format!("{rt}.hole({})", string_literal(h)), P_CALL),
            ExprKind::Tuple(xs) => {
                let items: Vec<String> = xs.iter().map(|x| self.expr(x).text).collect();
                Ex::new(format!("[{}] as const", items.join(", ")), P_ARROW + 1)
            }
            ExprKind::List(xs) => {
                let items: Vec<String> = xs.iter().map(|x| self.expr(x).text).collect();
                Ex::primary(format!("[{}]", items.join(", ")))
            }
            ExprKind::SetOf(xs) => {
                let LTy::Set(of) = &e.ty else { return Ex::primary("undefined") };
                let d = self.out.l_desc(of);
                if xs.is_empty() {
                    return Ex::new(format!("{rt}.OrdSet.empty({d})"), P_CALL);
                }
                let items: Vec<String> = xs.iter().map(|x| self.expr(x).text).collect();
                Ex::new(format!("{rt}.OrdSet.of({d}, [{}])", items.join(", ")), P_CALL)
            }
            ExprKind::MapOf(pairs) => {
                let LTy::Map(k, v) = &e.ty else { return Ex::primary("undefined") };
                let (kt, vt) = (self.out.ts_type(k), self.out.ts_type(v));
                let d = self.out.l_desc(k);
                if pairs.is_empty() {
                    return Ex::new(format!("{rt}.OrdMap.empty<{kt}, {vt}>({d})"), P_CALL);
                }
                let items: Vec<String> = pairs
                    .iter()
                    .map(|(a, b)| {
                        let a = self.expr(a).text;
                        let b = self.expr(b).text;
                        format!("[{a}, {b}]")
                    })
                    .collect();
                Ex::new(format!("{rt}.OrdMap.of<{kt}, {vt}>({d}, [{}])", items.join(", ")), P_CALL)
            }
            ExprKind::Range { lo, hi, inclusive } => {
                let lo = self.expr(lo).text;
                let hi = self.expr(hi).text;
                Ex::new(format!("{rt}.range({lo}, {hi}, {inclusive})"), P_CALL)
            }
        }
    }

    /// The property of field `index` of variant `tag` (0 for a record) of `ty`.
    pub fn field_prop(&self, ty: spec_lang_ir::TypeId, tag: u32, index: u32) -> String {
        // The built-in `Option` and `Result` have the runtime's properties.
        if ty == self.out.plan.lir.option_type {
            return "value".into();
        }
        if ty == self.out.plan.lir.result_type {
            return if tag == 0 { "value".into() } else { "error".into() };
        }
        self.out
            .plan
            .field_prop
            .get(ty.index())
            .and_then(|v| v.get(tag as usize))
            .and_then(|f| f.get(index as usize))
            .cloned()
            .unwrap_or_else(|| format!("f{index}"))
    }

    fn interp(&mut self, parts: &[Part]) -> Ex {
        if parts.iter().all(|p| matches!(p, Part::Lit(_))) {
            let s: String = parts.iter().map(|p| if let Part::Lit(l) = p { l.as_str() } else { "" }).collect();
            return Ex::primary(string_literal(&s));
        }
        let mut out = String::from("`");
        for p in parts {
            match p {
                Part::Lit(l) => out.push_str(&template_part(l)),
                Part::Show(x) => {
                    let v = self.expr(x);
                    let shown = match &x.ty {
                        LTy::Text | LTy::Int | LTy::Bool => v.text,
                        t => {
                            let d = self.out.l_desc(t);
                            format!("{d}.show({})", v.text)
                        }
                    };
                    out.push_str(&format!("${{{shown}}}"));
                }
            }
        }
        out.push('`');
        Ex::primary(out)
    }

    fn variant(&mut self, ty: spec_lang_ir::TypeId, tag: u32) -> Ex {
        if ty == self.out.plan.lir.option_type {
            let rt = "rt";
            return Ex::primary(format!("{rt}.none"));
        }
        let name = variant_name(self.out.plan.lir, ty, tag);
        let t = self.out.type_ref(ty);
        Ex::new(format!("{t}.make({})", string_literal(&name)), P_CALL)
    }

    fn construct(&mut self, ty: spec_lang_ir::TypeId, tag: Option<u32>, args: &[Option<Expr>]) -> Ex {
        let mut items: Vec<String> =
            args.iter().map(|a| a.as_ref().map_or_else(|| "undefined".to_string(), |x| self.expr(x).text)).collect();
        while items.last().is_some_and(|x| x == "undefined") {
            items.pop();
        }
        let t = self.out.type_ref(ty);
        match tag {
            None => Ex::new(format!("{t}.make([{}])", items.join(", ")), P_CALL),
            Some(tag) => {
                let name = string_literal(&variant_name(self.out.plan.lir, ty, tag));
                if items.is_empty() {
                    Ex::new(format!("{t}.make({name})"), P_CALL)
                } else {
                    Ex::new(format!("{t}.make({name}, [{}])", items.join(", ")), P_CALL)
                }
            }
        }
    }

    fn call(&mut self, callee: &str, world: bool, args: &[Option<Expr>], span: spec_lang_syntax::Span) -> Ex {
        let mut items: Vec<String> = Vec::new();
        if world {
            items.push(self.w(span));
        }
        let mut given: Vec<String> =
            args.iter().map(|a| a.as_ref().map_or_else(|| "undefined".to_string(), |x| self.expr(x).text)).collect();
        while given.last().is_some_and(|x| x == "undefined") {
            given.pop();
        }
        items.extend(given);
        Ex::new(format!("{callee}({})", items.join(", ")), P_CALL)
    }

    fn lambda(&mut self, l: &Lambda) -> Ex {
        let copies: Vec<String> = l
            .captures
            .iter()
            .filter_map(|(n, _)| {
                let ts = self.lookup(n);
                self.mutable.contains(&ts).then_some(ts)
            })
            .collect();
        self.push_scope();
        let params: Vec<String> = l
            .params
            .iter()
            .map(|(n, t)| {
                let ts = self.bind(n);
                let t = self.out.ts_type(t);
                format!("{ts}: {t}")
            })
            .collect();
        let saved_result = self.result.take();
        let text = match &l.body {
            LambdaBody::Expr(x) => {
                self.depth += 1;
                let b = self.expr(x);
                self.depth -= 1;
                format!("({}) => {}", params.join(", "), b.at(P_ARROW))
            }
            LambdaBody::Block(b) => {
                let body = self.nested(|fx| fx.tail_block(b));
                format!("({}) => {{\n{body}\n}}", params.join(", "))
            }
        };
        self.result = saved_result;
        self.pop_scope();
        if copies.is_empty() {
            return Ex::new(text, P_ARROW);
        }
        // A lambda keeps the values its locals had when it was made: the mutable ones are copied now.
        let ps: Vec<String> = copies.iter().map(|c| c.to_string()).collect();
        Ex::new(format!("(({}) => {text})({})", ps.join(", "), ps.join(", ")), P_CALL)
    }

    /// A subject read twice without harm: a local, a state, or fields of one.
    fn simple(&mut self, e: &Expr) -> Option<String> {
        match &e.kind {
            ExprKind::Local(_) | ExprKind::State(_) => Some(self.expr(e).text),
            ExprKind::Field { recv, .. } | ExprKind::WorldState { recv, .. } if self.simple(recv).is_some() => {
                Some(self.expr(e).text)
            }
            _ => None,
        }
    }

    fn compare(&mut self, first: &Expr, links: &[(Prim, Expr)]) -> Ex {
        let simple_middle = links.iter().rev().skip(1).all(|(_, x)| is_simple(x));
        if links.len() == 1 || simple_middle {
            let mut left = first;
            let mut parts: Vec<Ex> = Vec::new();
            for (op, right) in links {
                let args = if op.is_containment() { [right, left] } else { [left, right] };
                let printed = vec![self.expr(args[0]), self.expr(args[1])];
                let tys = vec![args[0].ty.clone(), args[1].ty.clone()];
                parts.push(self.prim(*op, printed, &tys, &[]));
                left = right;
            }
            if parts.len() == 1 {
                return parts.remove(0);
            }
            let text: Vec<String> = parts.iter().map(and_operand).collect();
            return Ex::new(text.join(" && "), P_AND);
        }
        // A middle operand that is not a plain read: each value once, each right side only while the chain holds.
        let body = self.nested(|fx| {
            let mut left = fx.bind("left");
            let v = fx.expr(first).text;
            fx.line(format!("const {left} = {v};"));
            for (op, right) in links {
                let r = fx.bind("right");
                let v = fx.expr(right).text;
                fx.line(format!("const {r} = {v};"));
                let (a, b) = if op.is_containment() { (&r, &left) } else { (&left, &r) };
                let tys = if op.is_containment() {
                    vec![right.ty.clone(), first.ty.clone()]
                } else {
                    vec![first.ty.clone(), right.ty.clone()]
                };
                let held = fx.prim(*op, vec![Ex::primary(a.clone()), Ex::primary(b.clone())], &tys, &[]);
                fx.line(format!("if (!{}) return false;", held.at(P_UNARY)));
                left = r;
            }
            fx.line("return true;");
        });
        Ex::new(format!("(() => {{\n{body}\n}})()"), P_CALL)
    }

    fn block_expr(&mut self, b: &Block) -> Ex {
        match b.stmts.as_slice() {
            [] => Ex::primary("null"),
            [Stmt { kind: StmtKind::Return(Some(x)), .. }] => self.expr(x),
            _ => {
                let saved = self.result.take();
                let body = self.nested(|fx| fx.tail_block(b));
                self.result = saved;
                Ex::new(format!("(() => {{\n{body}\n}})()"), P_CALL)
            }
        }
    }

    /// A `match` as an expression: a chain of conditionals over a subject read twice without harm.
    fn match_expr(&mut self, scrutinee: &Expr, arms: &[Arm], ty: &LTy) -> Ex {
        let subject = self.simple(scrutinee);
        let Some(subject) = subject else {
            let saved = self.result.take();
            let body = self.nested(|fx| {
                let s = fx.bind("subject");
                let v = fx.expr(scrutinee).text;
                fx.line(format!("const {s} = {v};"));
                fx.match_arms(&s, &scrutinee.ty, arms, Sink::Return);
            });
            self.result = saved;
            let _ = ty;
            return Ex::new(format!("(() => {{\n{body}\n}})()"), P_CALL);
        };
        let rt = "rt";
        let mut out: Vec<(Option<Ex>, Ex)> = Vec::new();
        for arm in arms {
            self.push_scope();
            let (conds, _) = self.pattern(&subject, &scrutinee.ty, &arm.pat);
            let value = self.expr(&arm.value);
            self.pop_scope();
            let unconditional = conds.is_empty();
            out.push(((!unconditional).then(|| conjunction(conds)), value));
            if unconditional {
                break;
            }
        }
        let mut tail = match out.last() {
            Some((None, v)) => {
                let v = v.clone();
                out.pop();
                v
            }
            _ => Ex::new(format!("{rt}.noMatch()"), P_CALL),
        };
        for (cond, value) in out.into_iter().rev() {
            let cond = cond.map_or_else(|| "true".to_string(), |c| c.at(P_COND + 1));
            tail = Ex::new(format!("{cond} ? {} : {}", value.at(P_COND + 1), tail.at(P_COND)), P_COND);
        }
        tail
    }

    /// The arguments of a primitive, printed in order.
    fn prim_args(&mut self, _op: Prim, args: &[Expr]) -> Vec<Ex> {
        args.iter().map(|a| self.expr(a)).collect()
    }

    /// A primitive applied to printed arguments. `exprs`, when given, are the argument expressions.
    pub fn prim(&mut self, op: Prim, a: Vec<Ex>, tys: &[LTy], exprs: &[Expr]) -> Ex {
        let rt = "rt";
        let arg = |i: usize| a.get(i).cloned().unwrap_or_else(|| Ex::primary("undefined"));
        let recv = |i: usize| arg(i).at(P_CALL);
        let call = |f: &str, n: usize| {
            let xs: Vec<String> = (0..n).map(|i| arg(i).text).collect();
            Ex::new(format!("{rt}.{f}({})", xs.join(", ")), P_CALL)
        };
        let method = |name: &str, n: usize| {
            let xs: Vec<String> = (1..n).map(|i| arg(i).text).collect();
            Ex::new(format!("{}.{name}({})", recv(0), xs.join(", ")), P_CALL)
        };
        let prop = |name: &str| Ex::new(format!("{}.{name}", recv(0)), P_CALL);
        let bin = |op: &str, prec: u8| Ex::new(format!("{} {op} {}", arg(0).at(prec), arg(1).at(prec + 1)), prec);
        let ty = |i: usize| tys.get(i).cloned().unwrap_or(LTy::Unit);
        let elem = |t: LTy| match t {
            LTy::List(x) | LTy::Set(x) => *x,
            _ => LTy::Unit,
        };
        // The key type of a lambda argument: what it gives.
        let key = |t: LTy| match t {
            LTy::Func(_, r) => *r,
            _ => LTy::Unit,
        };
        let primitive = |t: &LTy| matches!(t, LTy::Int | LTy::Text | LTy::Bool | LTy::Unit);
        match op {
            Prim::Eq | Prim::Ne => {
                let negate = op == Prim::Ne;
                if primitive(&ty(0)) {
                    return bin(if negate { "!==" } else { "===" }, P_EQ);
                }
                // `x == Open`: a variant without data is its kind.
                if let Some(kind) = exprs.get(1).and_then(|e| self.dataless(e)) {
                    return Ex::new(format!("{}.kind {} {kind}", recv(0), if negate { "!==" } else { "===" }), P_EQ);
                }
                if let Some(kind) = exprs.first().and_then(|e| self.dataless(e)) {
                    return Ex::new(format!("{}.kind {} {kind}", arg(1).at(P_CALL), if negate { "!==" } else { "===" }), P_EQ);
                }
                let eq = Ex::new(format!("{rt}.equal({}, {})", arg(0).text, arg(1).text), P_CALL);
                if negate { Ex::new(format!("!{}", eq.text), P_UNARY) } else { eq }
            }
            Prim::Not => Ex::new(format!("!{}", arg(0).at(P_UNARY)), P_UNARY),
            Prim::IntAdd => bin("+", P_ADD),
            Prim::IntSub => bin("-", P_ADD),
            Prim::IntMul => bin("*", P_MUL),
            Prim::IntNeg => {
                let x = arg(0).at(P_UNARY);
                Ex::new(if x.starts_with('-') { format!("-({x})") } else { format!("-{x}") }, P_UNARY)
            }
            Prim::IntMod => call("mod", 2),
            Prim::IntPow => call("pow", 2),
            Prim::IntDiv => call("div", 2),
            Prim::IntBand => call("band", 2),
            Prim::IntBor => call("bor", 2),
            Prim::IntBxor => call("bxor", 2),
            Prim::IntShl => call("shl", 2),
            Prim::IntShr => call("shr", 2),
            Prim::IntLt => bin("<", P_REL),
            Prim::IntLe => bin("<=", P_REL),
            Prim::IntGt => bin(">", P_REL),
            Prim::IntGe => bin(">=", P_REL),
            Prim::IntAbs => call("abs", 1),
            Prim::IntMin => call("minInt", 2),
            Prim::IntMax => call("maxInt", 2),
            Prim::IntClamp => call("clamp", 3),
            Prim::TextLt => call("textLt", 2),
            Prim::TextLe => call("textLe", 2),
            Prim::TextGt => call("textGt", 2),
            Prim::TextGe => call("textGe", 2),
            Prim::TextSize => call("textSize", 1),
            Prim::TextBytes => call("bytes", 1),
            Prim::TextTrim => call("trim", 1),
            Prim::TextLower => method("toLowerCase", 1),
            Prim::TextStartsWith => method("startsWith", 2),
            Prim::TextContains => method("includes", 2),
            Prim::TextSplit => call("split", 2),
            Prim::TextReplace => call("replace", 3),
            Prim::TextTake => call("takeText", 2),
            Prim::TextDrop => call("dropText", 2),
            Prim::ListSize => Ex::new(format!("BigInt({}.length)", recv(0)), P_CALL),
            Prim::ListFirst => call("first", 1),
            Prim::ListLast => call("last", 1),
            Prim::ListGet => call("get", 2),
            Prim::ListIndex => call("at", 2),
            Prim::ListFilter => method("filter", 2),
            Prim::ListMap => method("map", 2),
            Prim::ListFlatMap => method("flatMap", 2),
            Prim::ListFold => Ex::new(format!("{}.reduce({}, {})", recv(0), arg(2).text, arg(1).text), P_CALL),
            Prim::ListAll => method("every", 2),
            Prim::ListAny => method("some", 2),
            Prim::ListFind => call("find", 2),
            Prim::ListPosition => call("position", 2),
            Prim::ListSum => call("sum", 1),
            Prim::ListSumBy => call("sumBy", 2),
            Prim::ListMin | Prim::ListMax | Prim::ListDistinct => {
                let d = self.out.l_desc(&elem(ty(0)));
                let f = match op {
                    Prim::ListMin => "min",
                    Prim::ListMax => "max",
                    _ => "distinct",
                };
                Ex::new(format!("{rt}.{f}({}, {d})", arg(0).text), P_CALL)
            }
            Prim::ListMinBy | Prim::ListMaxBy | Prim::ListGroupBy => {
                let d = self.out.l_desc(&key(ty(1)));
                let f = match op {
                    Prim::ListMinBy => "minBy",
                    Prim::ListMaxBy => "maxBy",
                    _ => "groupBy",
                };
                Ex::new(format!("{rt}.{f}({}, {}, {d})", arg(0).text, arg(1).text), P_CALL)
            }
            Prim::ListContains => {
                if primitive(&elem(ty(0))) {
                    method("includes", 2)
                } else {
                    call("contains", 2)
                }
            }
            Prim::ListSort => {
                let d = self.out.l_desc(&elem(ty(0)));
                let desc = if arg(1).text == "false" { String::new() } else { format!(", {}", arg(1).text) };
                Ex::new(format!("{rt}.sort({}, {d}{desc})", arg(0).text), P_CALL)
            }
            Prim::ListSortBy => {
                let d = self.out.l_desc(&key(ty(1)));
                let desc = if arg(2).text == "false" { String::new() } else { format!(", {}", arg(2).text) };
                Ex::new(format!("{rt}.sortBy({}, {}, {d}{desc})", arg(0).text, arg(1).text), P_CALL)
            }
            Prim::ListReversed => method("toReversed", 1),
            Prim::ListTake => call("take", 2),
            Prim::ListDrop => call("drop", 2),
            Prim::ListZip => call("zip", 2),
            Prim::ListJoin => method("join", 2),
            Prim::ListConcat => Ex::primary(format!("[...{}, ...{}]", arg(0).at(P_ARROW + 1), arg(1).at(P_ARROW + 1))),
            Prim::ListPush => Ex::primary(format!("[...{}, {}]", arg(0).at(P_ARROW + 1), arg(1).text)),
            Prim::ListInsert => call("insert", 3),
            Prim::ListRemove => call("removeAt", 2),
            Prim::SetList => method("list", 1),
            Prim::SetOfList => {
                let d = self.out.l_desc(&elem(ty(0)));
                Ex::new(format!("{rt}.OrdSet.of({d}, {})", arg(0).text), P_CALL)
            }
            Prim::SetSize | Prim::MapSize => prop("size"),
            Prim::SetContains | Prim::MapHas => method("has", 2),
            Prim::SetUnion => method("union", 2),
            Prim::SetDifference => method("difference", 2),
            Prim::SetInsert => method("insert", 2),
            Prim::SetRemove | Prim::MapRemove => method("remove", 2),
            Prim::MapKeys => method("keys", 1),
            Prim::MapValues => method("values", 1),
            Prim::MapGet => method("get", 2),
            Prim::MapIndex => method("at", 2),
            Prim::RangeList => method("list", 1),
            Prim::RangeHas => method("has", 2),
            Prim::TraceSteps => prop("steps"),
            Prim::TraceStates => prop("states"),
            Prim::StepEvent => prop("event"),
            Prim::StepReply => prop("reply"),
            Prim::StepBefore => prop("before"),
            Prim::StepAfter => prop("after"),
        }
    }

    /// A variant without data, as its kind's string: what `x == Open` compares.
    fn dataless(&self, e: &Expr) -> Option<String> {
        match &e.kind {
            ExprKind::Variant { ty, tag } => Some(string_literal(&variant_name(self.out.plan.lir, *ty, *tag))),
            ExprKind::None => Some(string_literal("none")),
            _ => None,
        }
    }

    // ---- patterns ----------------------------------------------------------------------------------

    /// A pattern on `subject`: the conditions in order, and the bindings with their text. Each binding is
    /// also put in scope as its path, so a later step and the arm may read it.
    pub fn pattern(&mut self, subject: &str, ty: &LTy, p: &Pat) -> (Vec<Ex>, Vec<(String, String, LTy)>) {
        let mut conds = Vec::new();
        let mut binds = Vec::new();
        for step in &p.steps {
            match step {
                PatStep::Tag { path, ty: t, tag } => {
                    let at = self.path(subject, ty, path);
                    let name = variant_name(self.out.plan.lir, *t, *tag);
                    conds.push(Ex::new(format!("{}.kind === {}", at.at(P_CALL), string_literal(&name)), P_EQ));
                }
                PatStep::Event { path, rule } => {
                    let at = self.path(subject, ty, path);
                    let id = string_literal(&self.out.plan.lir.rule(*rule).info.qualified);
                    conds.push(Ex::new(format!("{}.rule.id === {id}", at.at(P_CALL)), P_EQ));
                }
                PatStep::Equals { path, value } => {
                    let at = self.path(subject, ty, path);
                    let v = self.expr(value);
                    let tys = vec![value.ty.clone(), value.ty.clone()];
                    conds.push(self.prim(Prim::Eq, vec![at, v], &tys, &[]));
                }
                PatStep::Never => conds.push(Ex::primary("false")),
                PatStep::Bind { name, path, ty: bt } => {
                    let at = self.path(subject, ty, path);
                    self.bind_as(name, &at.at(P_CALL));
                    binds.push((name.clone(), at.text, bt.clone()));
                }
            }
        }
        (conds, binds)
    }

    fn path(&mut self, subject: &str, ty: &LTy, path: &[PathStep]) -> Ex {
        let mut text = Ex::primary(subject);
        let _ = ty;
        for s in path {
            text = match s {
                PathStep::VariantField { ty, tag, index } => {
                    Ex::new(format!("{}.{}", text.at(P_CALL), self.field_prop(*ty, *tag, *index)), P_CALL)
                }
                PathStep::TupleItem(i) => Ex::new(format!("{}[{i}]", text.at(P_CALL)), P_CALL),
                PathStep::EventArg { index, .. } => Ex::new(format!("{}.args[{index}]", text.at(P_CALL)), P_CALL),
            };
        }
        text
    }

    /// The arms of a `match` as statements, each giving its value to `sink`: returned by each arm, or one
    /// `if` / `else if` chain whose arms run for their effects.
    pub fn match_arms(&mut self, subject: &str, ty: &LTy, arms: &[Arm], sink: Sink) {
        let rt = "rt";
        let chain = sink == Sink::Discard;
        let mut opened = false;
        let mut exhausted = false;
        for arm in arms {
            self.push_scope();
            let (conds, binds) = self.pattern(subject, ty, &arm.pat);
            let unconditional = conds.is_empty();
            match (unconditional, chain && opened) {
                (false, true) => {
                    self.close(format!("}} else if ({}) {{", conjunction(conds).text));
                    self.indent_in();
                }
                (false, false) => {
                    self.open(format!("if ({}) {{", conjunction(conds).text));
                    opened = true;
                }
                (true, true) => {
                    self.close("} else {");
                    self.indent_in();
                }
                (true, false) => {}
            }
            // Bindings the arm reads, as locals inside it.
            for (name, path, bt) in binds {
                if !mentions(&arm.value, &name) {
                    continue;
                }
                let ts = self.bind(&name);
                let cast = if path.contains(".args[") { format!(" as {}", self.out.ts_type(&bt)) } else { String::new() };
                self.line(format!("const {ts} = {path}{cast};"));
            }
            self.tail(&arm.value, sink);
            if !unconditional && !chain {
                self.close("}");
                opened = false;
            }
            self.pop_scope();
            if unconditional {
                exhausted = true;
                break;
            }
        }
        match (exhausted, chain && opened) {
            (true, true) => self.close("}"),
            (true, false) => {}
            (false, true) => {
                self.close("} else {");
                self.indent_in();
                self.line(format!("{rt}.noMatch();"));
                self.close("}");
            }
            (false, false) => {
                let ret = if sink == Sink::Return { "return " } else { "" };
                self.line(format!("{ret}{rt}.noMatch();"));
            }
        }
    }

    fn indent_in(&mut self) {
        self.indent += 1;
    }
}

/// Whether `e` reads the local `name`.
pub fn mentions(e: &Expr, name: &str) -> bool {
    let mut found = false;
    spec_lang_lir::walk::exprs(e, &mut |x| {
        found |= matches!(&x.kind, ExprKind::Local(n) if n == name);
        if let ExprKind::Block(b) = &x.kind {
            found |= block_mentions(b, name);
        }
    });
    found
}

/// Whether a statement of `b` reads or writes the local `name`.
pub fn block_mentions(b: &Block, name: &str) -> bool {
    stmts_mention(&b.stmts, name)
}

/// Whether one of the statements reads or writes the local `name`.
pub fn stmts_mention(stmts: &[Stmt], name: &str) -> bool {
    let block = Block { stmts: stmts.to_vec(), ty: LTy::Unit, span: spec_lang_syntax::Span::default() };
    let mut found = false;
    spec_lang_lir::walk::block_exprs(&block, &mut |x| found |= matches!(&x.kind, ExprKind::Local(n) if n == name));
    found
        || stmts.iter().any(|s| match &s.kind {
            StmtKind::Assign { place, .. } | StmtKind::Mutate { place, .. } => {
                matches!(&place.root, PlaceRoot::Local(n) if n == name)
            }
            StmtKind::For { body, .. } | StmtKind::While { body, .. } => block_mentions(body, name),
            StmtKind::If { then, els, .. } => {
                block_mentions(then, name) || els.as_ref().is_some_and(|e| block_mentions(e, name))
            }
            _ => false,
        })
}

/// How a value at the end of a body is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sink {
    /// Returned from the TypeScript function.
    Return,
    /// Evaluated for its effects.
    Discard,
}

fn and_operand(e: &Ex) -> String {
    e.at(P_AND)
}

fn conjunction(conds: Vec<Ex>) -> Ex {
    match conds.len() {
        0 => Ex::primary("true"),
        1 => conds.into_iter().next().unwrap_or_else(|| Ex::primary("true")),
        _ => Ex::new(conds.iter().map(and_operand).collect::<Vec<_>>().join(" && "), P_AND),
    }
}

pub fn variant_name(lir: &spec_lang_lir::Lir, ty: spec_lang_ir::TypeId, tag: u32) -> String {
    match &lir.ty(ty).def {
        TypeDef::Sum { variants } => variants.get(tag as usize).map(|v| v.name.clone()).unwrap_or_default(),
        _ => String::new(),
    }
}

/// A read that gives the same value however often it is evaluated, and never fails.
fn is_simple(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Local(_) | ExprKind::State(_) | ExprKind::Int(_) | ExprKind::Text(_) | ExprKind::Bool(_) => true,
        ExprKind::Const(_) => true,
        ExprKind::Field { recv, .. } => is_simple(recv),
        _ => false,
    }
}

/// Whether evaluating `e` can end in a language failure (which `else` handles): conservatively yes for any call.
pub fn may_fail(e: &Expr) -> bool {
    let mut fails = false;
    spec_lang_lir::walk::exprs(e, &mut |x| {
        fails |= match &x.kind {
            ExprKind::CallFn { .. }
            | ExprKind::CallRule { .. }
            | ExprKind::Effect { .. }
            | ExprKind::EffectRead { .. }
            | ExprKind::CallLambda { .. }
            | ExprKind::Fail { .. }
            | ExprKind::Const(_)
            | ExprKind::Block(_) => true,
            ExprKind::Prim { op, .. } => matches!(
                op,
                Prim::IntMod
                    | Prim::ListFirst
                    | Prim::ListLast
                    | Prim::ListIndex
                    | Prim::ListMin
                    | Prim::ListMax
                    | Prim::ListMinBy
                    | Prim::ListMaxBy
                    | Prim::ListZip
                    | Prim::ListInsert
                    | Prim::ListRemove
                    | Prim::MapIndex
            ),
            _ => false,
        };
    });
    fails
}

/// A claim's body, `self` its one local: an expression of a value of the type.
pub fn claim_body(out: &mut Out<'_, '_>, body: &Expr) -> String {
    let mut fx = Fx::new(out, false);
    fx.bind_as("self", "self");
    let text = fx.expr(body).at(P_ARROW);
    // A claim that needed the world is refused where the declaration is printed.
    text
}
