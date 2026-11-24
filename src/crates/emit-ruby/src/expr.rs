//! Expressions. Evaluation order is the LIR's: Ruby evaluates a receiver, then its arguments left to right,
//! which is the order every node here is written in.

use crate::code::{self, AND, ATOM, Code, LOW, OR, UNARY};
use crate::gen_::{Fx, R};
use crate::names;
use spec_lang_ir::{FnId, RuleId, TypeId};
use spec_lang_lir::node::*;
use spec_lang_lir::{LTy, Param, TypeDef};

impl Fx<'_> {
    pub(crate) fn expr(&mut self, e: &Expr) -> R<Code> {
        Ok(match &e.kind {
            ExprKind::Unit => Code::atom("nil"),
            ExprKind::Bool(b) => Code::atom(b.to_string()),
            ExprKind::Int(n) if *n < 0 => Code::new(n.to_string(), UNARY),
            ExprKind::Int(n) => Code::atom(int(*n)),
            ExprKind::Text(t) => Code::atom(code::string(t)),
            ExprKind::Interp(parts) => self.interp(parts)?,
            ExprKind::Local(n) => Code::atom(self.lookup(n)),
            ExprKind::Const(c) => {
                let info = &self.g.lir.constant(*c).info;
                Code::atom(self.callee(info.module, &info.name, false))
            }
            ExprKind::State(s) => {
                let info = &self.g.lir.state(*s).info;
                Code::atom(self.callee(info.module, &info.name, false))
            }
            ExprKind::Variant { ty, tag } => Code::atom(self.variant_ref(*ty, *tag)),
            ExprKind::Some(x) => code::call("Spec::Some.new", &[self.expr(x)?.text]),
            ExprKind::None => Code::atom("Spec::NONE"),
            ExprKind::Construct { ty, tag, args } => self.construct(e, *ty, *tag, args)?,
            ExprKind::CallFn { func, args } => self.call_fn(e, *func, args)?,
            ExprKind::CallRule { rule, args } => self.call_rule(e, *rule, args)?,
            ExprKind::Effect { handler, name, args, .. } => {
                let mut all = self.effect_head(*handler, name);
                for a in args {
                    all.push(self.expr(a)?.text);
                }
                code::call("Spec.effect", &all)
            }
            ExprKind::EffectRead { handler, name, .. } => code::call("Spec.effect", &self.effect_head(*handler, name)),
            ExprKind::Prim { op, args } => self.prim(*op, args)?,
            ExprKind::Lambda(l) => self.lambda(l)?,
            ExprKind::CallLambda { callee, args } => {
                let f = self.expr(callee)?.at(ATOM);
                let args = self.exprs(args)?;
                code::call(&format!("{f}.call"), &args)
            }
            ExprKind::Field { recv, ty, index, .. } => {
                let member = self.member(*ty, None, *index);
                Code::atom(format!("{}.{member}", self.expr(recv)?.at(ATOM)))
            }
            ExprKind::VariantField { recv, name, .. } => {
                Code::atom(format!("{}.{}", self.expr(recv)?.at(ATOM), names::member(name)))
            }
            ExprKind::WorldState { recv, state } => {
                let member = self.g.names.world.get(state.index()).cloned().unwrap_or_default();
                Code::atom(format!("{}.{member}", self.expr(recv)?.at(ATOM)))
            }
            ExprKind::EventArg { recv, name, .. } => {
                Code::atom(format!("{}.arg({})", self.expr(recv)?.at(ATOM), code::string(name)))
            }
            ExprKind::With { recv, ty, changes } => {
                let recv = self.expr(recv)?.at(ATOM);
                let mut items = Vec::new();
                for (i, x) in changes {
                    let member = self.member(*ty, None, *i);
                    items.push(keyword(&member, &self.expr(x)?.text));
                }
                code::call(&format!("{recv}.with"), &items)
            }
            ExprKind::Is { value, pat } => self.is(value, pat)?,
            ExprKind::If { .. } | ExprKind::Match { .. } | ExprKind::Block(_) | ExprKind::Else { .. } => {
                let lines = self.value_lines(e)?;
                wrap(lines)
            }
            ExprKind::Logic { op: Logic::Implies, left, right } => {
                let l = self.negated(left)?;
                let r = self.expr(right)?;
                Code::new(code::chain(&[l.at(OR), r.at(OR + 1)], "||"), OR)
            }
            ExprKind::Logic { op, .. } => {
                // `a and b and c` is one chain: its operands side by side, or one per line.
                let mut operands = Vec::new();
                self.operands(e, *op, &mut operands)?;
                let (word, prec) = if *op == Logic::And { ("&&", AND) } else { ("||", OR) };
                Code::new(code::chain(&operands, word), prec)
            }
            ExprKind::Compare { first, links } => self.compare(first, links)?,
            ExprKind::Fail { name, args } => {
                let mut all = vec![code::string(name)];
                all.extend(self.exprs(args)?);
                code::call("Spec.fail!", &all)
            }
            ExprKind::Hole(h) => code::call("Spec.hole!", &[code::string(h)]),
            ExprKind::Tuple(xs) => Code::atom(code::list("Spec::Tuple", "[", &self.exprs(xs)?, "]")),
            ExprKind::List(xs) => Code::atom(code::list("", "[", &self.exprs(xs)?, "]")),
            ExprKind::SetOf(xs) if xs.is_empty() => Code::atom("Spec::Sets::EMPTY"),
            ExprKind::SetOf(xs) => {
                let items = code::list("", "[", &self.exprs(xs)?, "]");
                code::call("Spec::Sets.of", &[items])
            }
            ExprKind::MapOf(pairs) if pairs.is_empty() => Code::atom("Spec::Maps::EMPTY"),
            ExprKind::MapOf(pairs) => {
                let mut items = Vec::new();
                for (k, v) in pairs {
                    items.push(format!("[{}, {}]", self.expr(k)?.text, self.expr(v)?.text));
                }
                code::call("Spec::Maps.of", &[code::list("", "[", &items, "]")])
            }
            ExprKind::Range { lo, hi, inclusive } => {
                let lo = self.expr(lo)?.at(ATOM);
                let hi = self.expr(hi)?.at(ATOM);
                Code::atom(format!("({lo}{}{hi})", if *inclusive { ".." } else { "..." }))
            }
        })
    }

    fn operands(&mut self, e: &Expr, op: Logic, out: &mut Vec<String>) -> R<()> {
        match &e.kind {
            ExprKind::Logic { op: o, left, right } if *o == op => {
                self.operands(left, op, out)?;
                let prec = if op == Logic::And { AND } else { OR };
                out.push(self.expr(right)?.at(prec + 1));
            }
            _ => out.push(self.expr(e)?.at(if op == Logic::And { AND } else { OR })),
        }
        Ok(())
    }

    /// `not e`, with `==` and `!=` turned round where `e` is one of them.
    fn negated(&mut self, e: &Expr) -> R<Code> {
        use spec_lang_lir::Prim;
        match &e.kind {
            ExprKind::Prim { op: Prim::Eq, args } => self.prim(Prim::Ne, args),
            ExprKind::Prim { op: Prim::Ne, args } => self.prim(Prim::Eq, args),
            ExprKind::Prim { op: Prim::Not, args } if args.len() == 1 => self.expr(&args[0]),
            _ => Ok(Code::new(format!("!{}", self.expr(e)?.at(ATOM)), UNARY)),
        }
    }

    pub(crate) fn exprs(&mut self, xs: &[Expr]) -> R<Vec<String>> {
        xs.iter().map(|x| self.expr(x).map(|c| c.text)).collect()
    }

    fn effect_head(&self, handler: spec_lang_ir::HandlerId, name: &str) -> Vec<String> {
        let h = self.g.lir.handler(handler);
        let op = name.rsplit('.').next().unwrap_or(name);
        vec![code::string(&format!("{}.{}", h.library, h.name)), format!(":{}", names::method(op))]
    }

    /// The Ruby member of field `index` of a record (`tag` none) or of a variant.
    pub(crate) fn member(&self, ty: TypeId, tag: Option<u32>, index: u32) -> String {
        let lir = self.g.lir;
        if ty == lir.option_type {
            return "value".into();
        }
        if ty == lir.result_type {
            return if tag == Some(spec_lang_ir::ERR_TAG) { "error".into() } else { "value".into() };
        }
        let fields = match (&lir.ty(ty).def, tag) {
            (TypeDef::Record { fields }, _) => Some(fields),
            (TypeDef::Sum { variants }, Some(t)) => variants.get(t as usize).map(|v| &v.fields),
            _ => None,
        };
        fields.and_then(|fs| fs.get(index as usize)).map_or_else(|| "x".into(), |f| names::member(&f.name))
    }

    fn interp(&mut self, parts: &[Part]) -> R<Code> {
        if let [Part::Show(x)] = parts
            && x.ty == LTy::Text
        {
            return self.expr(x);
        }
        let mut body = String::new();
        for p in parts {
            match p {
                Part::Lit(l) => body.push_str(&code::escape(l)),
                Part::Show(x) => {
                    let c = self.expr(x)?;
                    let shown = match x.ty {
                        LTy::Text | LTy::Int | LTy::Bool => c.text,
                        _ => format!("Spec.show({})", c.text),
                    };
                    body.push_str(&format!("#{{{shown}}}"));
                }
            }
        }
        Ok(Code::atom(format!("\"{body}\"")))
    }

    fn construct(&mut self, e: &Expr, ty: TypeId, tag: Option<u32>, args: &[Option<Expr>]) -> R<Code> {
        let lir = self.g.lir;
        let (target, fields) = match (&lir.ty(ty).def, tag) {
            (TypeDef::Record { fields }, None) => (self.type_ref(ty), fields),
            (TypeDef::Sum { variants }, Some(t)) => match variants.get(t as usize) {
                Some(v) => (self.variant_ref(ty, t), &v.fields),
                None => return Err(self.g.unsupported(e.span, "a variant the type does not have")),
            },
            _ => return Err(self.g.unsupported(e.span, "a constructor of a type that is no record and no sum")),
        };
        if fields.is_empty() && tag.is_some() {
            return Ok(Code::atom(target));
        }
        let mut items = Vec::new();
        for (f, a) in fields.iter().zip(args) {
            let value = match (a, &f.default) {
                (Some(x), _) => self.expr(x)?.text,
                // A record's defaults are its `initialize`'s; a variant's are written out here.
                (None, Some(_)) if tag.is_none() => continue,
                (None, Some(d)) => self.closed(d)?,
                (None, None) => return Err(self.g.unsupported(e.span, format!("no value for field `{}`", f.name))),
            };
            items.push(keyword(&names::member(&f.name), &value));
        }
        Ok(if items.is_empty() {
            Code::atom(format!("{target}.new"))
        } else {
            code::call(&format!("{target}.new"), &items)
        })
    }

    /// An expression "evaluated in no scope" (a default), printed where it is used.
    pub(crate) fn closed(&mut self, e: &Expr) -> R<String> {
        let bare = std::mem::replace(&mut self.bare, false);
        let out = self.expr(e);
        self.bare = bare;
        Ok(out?.text)
    }

    /// Arguments of a `fn`, a rule or a test: by position up to the first parameter with a default, by keyword
    /// from there (an argument left out takes Ruby's default, with the earlier parameters in scope).
    pub(crate) fn args(&mut self, e: &Expr, params: &[Param], args: &[Option<Expr>]) -> R<Vec<String>> {
        let keywords = keywords_from(params);
        let mut out = Vec::new();
        for (i, (p, a)) in params.iter().zip(args).enumerate() {
            match (a, &p.default) {
                (Some(x), _) if i < keywords => out.push(self.expr(x)?.text),
                (Some(x), _) => {
                    let value = self.expr(x)?.text;
                    out.push(keyword(&names::local(&p.name), &value));
                }
                (None, Some(_)) => {}
                (None, None) => return Err(self.g.unsupported(e.span, format!("no argument `{}`", p.name))),
            }
        }
        Ok(out)
    }

    fn call_fn(&mut self, e: &Expr, f: FnId, args: &[Option<Expr>]) -> R<Code> {
        let d = self.g.lir.func(f);
        let args = self.args(e, &d.params, args)?;
        Ok(self.call_named(d.info.module, &d.info.name, &args))
    }

    /// A call of a module's method; without arguments it is written without parentheses.
    pub(crate) fn call_named(&self, module: spec_lang_ir::ModuleId, name: &str, args: &[String]) -> Code {
        if args.is_empty() {
            return Code::atom(self.callee(module, name, false));
        }
        code::call(&self.callee(module, name, true), args)
    }

    fn call_rule(&mut self, e: &Expr, r: RuleId, args: &[Option<Expr>]) -> R<Code> {
        let d = self.g.lir.rule(r);
        let args = self.args(e, &d.params, args)?;
        Ok(self.call_named(d.info.module, &d.info.name, &args))
    }

    /// `a < b <= c`: each right side once, and only while every earlier link held.
    fn compare(&mut self, first: &Expr, links: &[(spec_lang_lir::Prim, Expr)]) -> R<Code> {
        let mut left = self.expr(first)?;
        let mut parts = Vec::new();
        for (i, (op, x)) in links.iter().enumerate() {
            let mut right = self.expr(x)?;
            // The right side is read again by the next link: hold it unless reading it twice is the same.
            let held = i + 1 < links.len() && !repeatable(x);
            let name = if held { self.fresh("link") } else { String::new() };
            let next = if held { Code::atom(name.clone()) } else { right.clone() };
            if held {
                right = Code::atom(format!("({name} = {})", right.text));
            }
            let link = if op.is_containment() {
                self.prim_of(*op, vec![right, left])?
            } else {
                self.prim_of(*op, vec![left, right])?
            };
            parts.push(link);
            left = next;
        }
        if let [one] = parts.as_slice() {
            return Ok(one.clone());
        }
        Ok(Code::new(parts.iter().map(|p| p.at(AND + 1)).collect::<Vec<_>>().join(" && "), AND))
    }

    /// A lambda as a value.
    fn lambda(&mut self, l: &Lambda) -> R<Code> {
        let copied: Vec<&str> = l.captures.iter().map(|(n, _)| n.as_str()).filter(|n| self.changes(n)).collect();
        let outer: Vec<String> = copied.iter().map(|n| self.lookup(n)).collect();
        self.push();
        let saved = (std::mem::take(&mut self.result_check), std::mem::replace(&mut self.whiles, 0));
        // A captured local that can change later is copied now: it becomes a parameter of a lambda called at once.
        let inner: Vec<String> = copied.iter().map(|n| self.bind_param(n)).collect();
        let params: Vec<String> = l.params.iter().map(|(n, _)| self.bind_param(n)).collect();
        let body = match &l.body {
            LambdaBody::Expr(x) => self.value_lines(x),
            LambdaBody::Block(b) => self.body_lines(b, None, true),
        };
        (self.result_check, self.whiles) = saved;
        self.pop();
        let body = body?;
        let made = lambda_text(&params, &body);
        if copied.is_empty() {
            return Ok(Code::atom(made));
        }
        Ok(code::call(&format!("{}.call", lambda_text(&inner, &[made])), &outer))
    }

    /// A lambda as the block of a call: `xs.map { |x| ... }`. `head` is the call without its block.
    pub(crate) fn with_block(&mut self, head: &str, f: &Expr) -> R<Code> {
        let ExprKind::Lambda(l) = &f.kind else {
            return Ok(Code::atom(append_arg(head, &format!("&{}", self.expr(f)?.at(ATOM)))));
        };
        let inline = match &l.body {
            LambdaBody::Expr(_) => true,
            LambdaBody::Block(b) => crate::stmt::inlinable(b),
        };
        if !inline || l.captures.iter().any(|(n, _)| self.changes(n)) {
            return Ok(Code::atom(append_arg(head, &format!("&{}", self.lambda(l)?.at(ATOM)))));
        }
        self.push();
        let saved = (std::mem::take(&mut self.result_check), std::mem::replace(&mut self.whiles, 0));
        let params: Vec<String> = l.params.iter().map(|(n, _)| self.bind_param(n)).collect();
        let body = match &l.body {
            LambdaBody::Expr(x) => self.value_lines(x),
            LambdaBody::Block(b) => self.body_lines(b, None, true),
        };
        (self.result_check, self.whiles) = saved;
        self.pop();
        Ok(code::block(head, &params, &body?))
    }
}

fn lambda_text(params: &[String], body: &[String]) -> String {
    let head = if params.is_empty() { "-> ".to_string() } else { format!("->({}) ", params.join(", ")) };
    if let [one] = body
        && !one.contains('\n')
        && head.len() + one.len() + 4 <= code::WIDTH
    {
        return format!("{head}{{ {one} }}");
    }
    let bars = if params.is_empty() { String::new() } else { format!(" |{}|", params.join(", ")) };
    format!("lambda do{bars}\n{}\nend", code::indent_lines(body))
}

/// `f(a)` with one more argument; `f` alone becomes `f(arg)`.
fn append_arg(head: &str, arg: &str) -> String {
    match head.strip_suffix(')') {
        Some(open) if open.ends_with('(') => format!("{open}{arg})"),
        Some(rest) => format!("{rest}, {arg})"),
        None => format!("{head}({arg})"),
    }
}

/// The first parameter Ruby takes by keyword: the first with a default.
pub(crate) fn keywords_from(params: &[Param]) -> usize {
    params.iter().position(|p| p.default.is_some()).unwrap_or(params.len())
}

/// `name: value`, or `name:` when the value is the local of that name.
pub(crate) fn keyword(name: &str, value: &str) -> String {
    if name == value { format!("{name}:") } else { format!("{name}: {value}") }
}

/// Statements as one expression: the last line's value. A single line is taken as loose, so it gets
/// parentheses wherever it becomes an operand.
pub(crate) fn wrap(lines: Vec<String>) -> Code {
    match lines.as_slice() {
        [one] => Code::new(one.clone(), LOW),
        _ => Code::atom(format!("begin\n{}\nend", code::indent_lines(&lines))),
    }
}

/// Whether evaluating `e` twice gives the same value with no effect: a literal or a local.
pub(crate) fn repeatable(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Int(_) | ExprKind::Text(_) | ExprKind::Bool(_) | ExprKind::Local(_) | ExprKind::Unit)
}

/// `1_000_000` for a long literal.
fn int(n: i128) -> String {
    let digits = n.to_string();
    if digits.len() < 5 {
        return digits;
    }
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('_');
        }
        out.push(c);
    }
    out
}
