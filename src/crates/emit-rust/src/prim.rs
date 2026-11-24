//! The 93 primitives, each once: an operator or method where Rust's meaning is the LIR's, else the runtime.

use crate::code::{Code, Fx};
use spec_lang_lir::node::{Expr, ExprKind};
use spec_lang_lir::{LTy, Prim};

/// A prepared argument: its code and type; `closure` when it is a lambda written in place.
struct Arg {
    code: Code,
    ty: LTy,
    closure: bool,
}

impl Fx<'_, '_> {
    pub fn prim(&mut self, op: Prim, args: &[Expr], _ty: &LTy) -> Code {
        let mut items = Vec::with_capacity(args.len());
        let mut tys = Vec::with_capacity(args.len());
        let mut closures = Vec::with_capacity(args.len());
        for a in args {
            let (c, closure) = match (&a.ty, &a.kind) {
                (LTy::Func(..), ExprKind::Lambda(l)) => (self.lambda_inline(l), true),
                _ => (self.expr(a), false),
            };
            items.push(c);
            tys.push(a.ty.clone());
            closures.push(closure);
        }
        let (pre, items) = self.args(items, false);
        let args =
            items.into_iter().zip(tys).zip(closures).map(|((code, ty), closure)| Arg { code, ty, closure }).collect();
        let mut c = self.apply(op, args);
        let mut all = pre;
        all.append(&mut c.pre);
        c.pre = all;
        c
    }

    /// A primitive on values whose statements are already placed (no function arguments).
    pub fn prim_on(&mut self, op: Prim, args: Vec<(Code, LTy)>, _ty: &LTy) -> Code {
        self.apply(op, args.into_iter().map(|(code, ty)| Arg { code, ty, closure: false }).collect())
    }

    fn apply(&mut self, op: Prim, args: Vec<Arg>) -> Code {
        let m = args.iter().any(|a| a.code.m);
        let fail_in = args.iter().any(|a| a.code.fail);
        let o = |i: usize| -> String {
            let a = &args[i];
            if a.code.place && !a.code.copy { format!("{}.clone()", a.code.operand()) } else { a.code.text.clone() }
        };
        // A list literal read as a slice needs no `Vec`.
        let r = |i: usize| match args[i].code.text.strip_prefix("vec![") {
            Some(items) if !args[i].code.place && one_literal(items) => format!("&[{items}"),
            _ => args[i].code.reference(),
        };
        let v = |i: usize| args[i].code.operand();
        // A text literal compared as `&str`: `t == "milk"`, no `String` made for it.
        let cmp = |i: usize| str_literal(&args[i].code.text).map_or_else(|| v(i), str::to_string);
        let f = |i: usize| -> String {
            let a = &args[i];
            match &a.ty {
                LTy::Func(ps, _) if !a.closure => {
                    let names: Vec<String> = (0..ps.len()).map(|k| format!("a{k}")).collect();
                    format!(
                        "|{}| {}.call(m, {})",
                        names.join(", "),
                        a.code.operand(),
                        crate::expr::tuple_text(names.clone())
                    )
                }
                _ => a.code.text.clone(),
            }
        };
        let fun_m = args.iter().any(|a| matches!(a.ty, LTy::Func(..)) && !a.closure);
        let call = |t: String| Code::val(t).failing();
        let pure = Code::val;
        let bin = |t: String| Code::op(t);
        let c = match op {
            Prim::Eq | Prim::Ne => {
                let (a, b) = (cmp(0), cmp(1));
                let empty = if b == "\"\"" {
                    Some(v(0))
                } else if a == "\"\"" {
                    Some(v(1))
                } else {
                    None
                };
                match (empty, op == Prim::Eq) {
                    (Some(x), true) => pure(format!("{x}.is_empty()")),
                    (Some(x), false) => bin(format!("!{x}.is_empty()")),
                    (None, true) => bin(format!("{a} == {b}")),
                    (None, false) => bin(format!("{a} != {b}")),
                }
            }
            Prim::Not => bin(format!("!{}", v(0))),
            Prim::IntAdd => call(format!("rt::int::add({}, {})?", o(0), o(1))),
            Prim::IntSub => call(format!("rt::int::sub({}, {})?", o(0), o(1))),
            Prim::IntMul => call(format!("rt::int::mul({}, {})?", o(0), o(1))),
            Prim::IntNeg => call(format!("rt::int::neg({})?", o(0))),
            Prim::IntMod => call(format!("rt::int::modulo({}, {})?", o(0), o(1))),
            Prim::IntPow => call(format!("rt::int::pow({}, {})?", o(0), o(1))),
            Prim::IntDiv => call(format!("rt::int::div({}, {})?", o(0), o(1))),
            Prim::IntBand => call(format!("rt::int::band({}, {})?", o(0), o(1))),
            Prim::IntBor => call(format!("rt::int::bor({}, {})?", o(0), o(1))),
            Prim::IntBxor => call(format!("rt::int::bxor({}, {})?", o(0), o(1))),
            Prim::IntShl => call(format!("rt::int::shl({}, {})?", o(0), o(1))),
            Prim::IntShr => call(format!("rt::int::shr({}, {})?", o(0), o(1))),
            Prim::IntLt | Prim::TextLt => bin(format!("{} < {}", v(0), v(1))),
            Prim::IntLe | Prim::TextLe => bin(format!("{} <= {}", v(0), v(1))),
            Prim::IntGt | Prim::TextGt => bin(format!("{} > {}", v(0), v(1))),
            Prim::IntGe | Prim::TextGe => bin(format!("{} >= {}", v(0), v(1))),
            Prim::IntAbs => call(format!("rt::int::abs({})?", o(0))),
            Prim::IntMin => pure(format!("{}.min({})", v(0), o(1))),
            Prim::IntMax => pure(format!("{}.max({})", v(0), o(1))),
            Prim::IntClamp => call(format!("rt::int::clamp({}, {}, {})?", o(0), o(1), o(2))),
            Prim::TextSize => pure(format!("rt::text::size({})", r(0))),
            Prim::TextBytes => pure(format!("rt::text::bytes({})", r(0))),
            Prim::TextTrim => pure(format!("rt::text::trim({})", r(0))),
            Prim::TextLower => pure(format!("rt::text::lower({})", r(0))),
            Prim::TextStartsWith => pure(format!("{}.starts_with({}.as_str())", v(0), v(1))),
            Prim::TextContains => pure(format!("{}.contains({}.as_str())", v(0), v(1))),
            Prim::TextSplit => call(format!("rt::text::split({}, {})?", r(0), r(1))),
            Prim::TextReplace => call(format!("rt::text::replace({}, {}, {})?", r(0), r(1), r(2))),
            Prim::TextTake => pure(format!("rt::text::take({}, {})", r(0), o(1))),
            Prim::TextDrop => pure(format!("rt::text::drop({}, {})", r(0), o(1))),
            Prim::ListSize | Prim::SetSize | Prim::MapSize => pure(format!("rt::int::count({}.len())", v(0))),
            Prim::ListFirst => call(format!("rt::list::first({})?", r(0))),
            Prim::ListLast => call(format!("rt::list::last({})?", r(0))),
            Prim::ListGet => pure(format!("rt::list::get({}, {})", r(0), o(1))),
            Prim::ListIndex => call(format!("rt::list::at({}, {})?", r(0), o(1))),
            Prim::ListFilter => call(format!("rt::list::filter({}, {})?", r(0), f(1))),
            Prim::ListMap => call(format!("rt::list::map({}, {})?", r(0), f(1))),
            Prim::ListFlatMap => call(format!("rt::list::flat_map({}, {})?", r(0), f(1))),
            Prim::ListFold => call(format!("rt::list::fold({}, {}, {})?", r(0), o(1), f(2))),
            Prim::ListAll => call(format!("rt::list::all({}, {})?", r(0), f(1))),
            Prim::ListAny => call(format!("rt::list::any({}, {})?", r(0), f(1))),
            Prim::ListFind => call(format!("rt::list::find({}, {})?", r(0), f(1))),
            Prim::ListPosition => call(format!("rt::list::position({}, {})?", r(0), f(1))),
            Prim::ListSum => call(format!("rt::list::sum({})?", r(0))),
            Prim::ListSumBy => call(format!("rt::list::sum_by({}, {})?", r(0), f(1))),
            Prim::ListMin => call(format!("rt::list::min({})?", r(0))),
            Prim::ListMax => call(format!("rt::list::max({})?", r(0))),
            Prim::ListMinBy => call(format!("rt::list::min_by({}, {})?", r(0), f(1))),
            Prim::ListMaxBy => call(format!("rt::list::max_by({}, {})?", r(0), f(1))),
            Prim::ListContains | Prim::SetContains => pure(format!("{}.contains(&{})", v(0), v(1))),
            Prim::ListDistinct => pure(format!("rt::list::distinct({})", r(0))),
            Prim::ListSort => pure(format!("rt::list::sort({}, {})", r(0), o(1))),
            Prim::ListSortBy => call(format!("rt::list::sort_by({}, {}, {})?", r(0), f(1), o(2))),
            Prim::ListReversed => pure(format!("rt::list::reversed({})", r(0))),
            Prim::ListTake => call(format!("rt::list::take({}, {})?", r(0), o(1))),
            Prim::ListDrop => call(format!("rt::list::drop({}, {})?", r(0), o(1))),
            Prim::ListZip => call(format!("rt::list::zip({}, {})?", r(0), r(1))),
            Prim::ListGroupBy => call(format!("rt::list::group_by({}, {})?", r(0), f(1))),
            Prim::ListJoin => pure(format!("{}.join({}.as_str())", v(0), v(1))),
            Prim::ListConcat => pure(format!("rt::list::concat({}, {})", r(0), r(1))),
            Prim::ListPush => pure(format!("rt::list::push({}, {})", r(0), o(1))),
            Prim::ListInsert => call(format!("rt::list::insert({}, {}, {})?", r(0), o(1), o(2))),
            Prim::ListRemove => call(format!("rt::list::remove({}, {})?", r(0), o(1))),
            Prim::SetList => pure(format!("{}.iter().cloned().collect::<Vec<_>>()", v(0))),
            Prim::SetOfList => pure(format!("{}.iter().cloned().collect::<BTreeSet<_>>()", v(0))),
            Prim::SetUnion => pure(format!("rt::list::union({}, {})", r(0), r(1))),
            Prim::SetDifference => pure(format!("rt::list::difference({}, {})", r(0), r(1))),
            Prim::SetInsert => pure(format!("rt::list::set_insert({}, {})", r(0), o(1))),
            Prim::SetRemove => pure(format!("rt::list::set_remove({}, {})", r(0), r(1))),
            Prim::MapKeys => pure(format!("{}.keys().cloned().collect::<Vec<_>>()", v(0))),
            Prim::MapValues => pure(format!("{}.values().cloned().collect::<Vec<_>>()", v(0))),
            Prim::MapGet => pure(format!("rt::list::map_get({}, {})", r(0), r(1))),
            Prim::MapIndex => call(format!("rt::list::map_at({}, {})?", r(0), r(1))),
            Prim::MapHas => pure(format!("{}.contains_key(&{})", v(0), v(1))),
            Prim::MapRemove => pure(format!("rt::list::map_remove({}, {})", r(0), r(1))),
            Prim::RangeList => call(format!("{}.list()?", v(0))),
            Prim::RangeHas => pure(format!("{}.has({})", v(0), o(1))),
            Prim::TraceSteps
            | Prim::TraceStates
            | Prim::StepEvent
            | Prim::StepReply
            | Prim::StepBefore
            | Prim::StepAfter => {
                let field = match op {
                    Prim::TraceSteps => "steps",
                    Prim::TraceStates => "states",
                    Prim::StepEvent => "event",
                    Prim::StepReply => "reply",
                    Prim::StepBefore => "before",
                    _ => "after",
                };
                let a = &args[0].code;
                Code {
                    text: format!("{}.{field}", a.operand()),
                    atomic: true,
                    place: a.place,
                    simple: a.simple,
                    ..Code::default()
                }
            }
        };
        let mut c = c.m(m || fun_m);
        c.fail |= fail_in;
        c
    }
}

/// `"x"` when `text` is `String::from("x")`.
fn str_literal(text: &str) -> Option<&str> {
    let inner = text.strip_prefix("String::from(")?.strip_suffix(')')?;
    let body = inner.strip_prefix('"')?.strip_suffix('"')?;
    let mut escaped = false;
    for c in body.chars() {
        match (c, escaped) {
            ('\\', false) => escaped = true,
            ('"', false) => return None,
            _ => escaped = false,
        }
    }
    Some(inner)
}

/// Whether `items]` closes the bracket of its `vec![` only at its end.
fn one_literal(items: &str) -> bool {
    let mut depth = 1i32;
    let (mut quoted, mut escaped) = (false, false);
    for (i, c) in items.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '[' if !quoted => depth += 1,
            ']' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    return i + 1 == items.len();
                }
            }
            _ => {}
        }
    }
    false
}
