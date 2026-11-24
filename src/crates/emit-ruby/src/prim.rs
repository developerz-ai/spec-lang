//! The LIR's primitives. Where a Ruby method means exactly the primitive it is called; the rest are in the
//! runtime (`lib/spec/prims.rb`), each once.

use crate::code::{self, ADD, ATOM, CMP, Code, EQ, MUL, UNARY};
use crate::gen_::{Fx, R};
use spec_lang_diag::Diag;
use spec_lang_lir::Prim;
use spec_lang_lir::node::{Expr, ExprKind};

fn binary(op: &str, prec: u8, a: &Code, b: &Code) -> Code {
    Code::new(format!("{} {op} {}", a.at(prec), b.at(prec + 1)), prec)
}

fn compare(op: &str, a: &Code, b: &Code) -> Code {
    Code::new(format!("{} {op} {}", a.at(CMP + 1), b.at(CMP + 1)), CMP)
}

fn method(recv: &Code, name: &str) -> Code {
    Code::atom(format!("{}.{name}", recv.at(ATOM)))
}

fn send(recv: &Code, name: &str, args: &[&Code]) -> Code {
    code::call(&format!("{}.{name}", recv.at(ATOM)), &args.iter().map(|a| a.text.clone()).collect::<Vec<_>>())
}

fn helper(name: &str, args: &[Code]) -> Code {
    code::call(name, &args.iter().map(|a| a.text.clone()).collect::<Vec<_>>())
}

/// Whether the last argument of the primitive is a function.
fn takes_function(op: Prim) -> bool {
    use Prim::*;
    matches!(
        op,
        ListFilter
            | ListMap
            | ListFlatMap
            | ListFold
            | ListAll
            | ListAny
            | ListFind
            | ListPosition
            | ListSumBy
            | ListMinBy
            | ListMaxBy
            | ListGroupBy
            | ListSortBy
    )
}

impl Fx<'_> {
    pub(crate) fn prim(&mut self, op: Prim, args: &[Expr]) -> R<Code> {
        if !takes_function(op) {
            let codes = args.iter().map(|a| self.expr(a)).collect::<R<Vec<_>>>()?;
            return self.prim_of(op, codes);
        }
        use Prim::*;
        let recv = match args.first() {
            Some(a) => self.expr(a)?,
            None => return Err(wrong(op)),
        };
        let at = recv.at(ATOM);
        // `sort_by(xs, key, desc)`: the function is the middle argument.
        let (f, head) = match (op, args) {
            (ListFilter, [_, f]) => (f, format!("{at}.select")),
            (ListMap, [_, f]) => (f, format!("{at}.map")),
            (ListFlatMap, [_, f]) => (f, format!("{at}.flat_map")),
            (ListAll, [_, f]) => (f, format!("{at}.all?")),
            (ListAny, [_, f]) => (f, format!("{at}.any?")),
            (ListSumBy, [_, f]) => (f, format!("{at}.sum")),
            (ListFold, [_, start, f]) => (f, format!("{at}.reduce({})", self.expr(start)?.text)),
            (ListFind, [_, f]) => (f, format!("Spec::Lists.find({})", recv.text)),
            (ListPosition, [_, f]) => (f, format!("Spec::Lists.position({})", recv.text)),
            (ListMinBy, [_, f]) => (f, format!("Spec::Lists.min_by({})", recv.text)),
            (ListMaxBy, [_, f]) => (f, format!("Spec::Lists.max_by({})", recv.text)),
            (ListGroupBy, [_, f]) => (f, format!("Spec::Lists.group_by({})", recv.text)),
            (ListSortBy, [_, f, desc]) => (f, format!("Spec::Lists.sort_by({}{})", recv.text, self.desc(desc)?)),
            _ => return Err(wrong(op)),
        };
        self.with_block(&head, f)
    }

    /// `, desc: true`, or nothing for the ascending default.
    fn desc(&mut self, desc: &Expr) -> R<String> {
        Ok(match desc.kind {
            ExprKind::Bool(false) => String::new(),
            ExprKind::Bool(true) => ", desc: true".into(),
            _ => format!(", desc: {}", self.expr(desc)?.text),
        })
    }

    /// A primitive over arguments already printed, in the primitive's order.
    pub(crate) fn prim_of(&mut self, op: Prim, a: Vec<Code>) -> R<Code> {
        use Prim::*;
        Ok(match (op, a.as_slice()) {
            (Eq, [x, y]) => Code::new(format!("{} == {}", x.at(EQ + 1), y.at(EQ + 1)), EQ),
            (Ne, [x, y]) => Code::new(format!("{} != {}", x.at(EQ + 1), y.at(EQ + 1)), EQ),
            (Not, [x]) => Code::new(format!("!{}", x.at(ATOM)), UNARY),
            (IntAdd, [x, y]) => binary("+", ADD, x, y),
            (IntSub, [x, y]) => binary("-", ADD, x, y),
            (IntMul, [x, y]) => binary("*", MUL, x, y),
            (IntNeg, [x]) => Code::new(format!("-{}", x.at(ATOM)), UNARY),
            (IntMod, [x, y]) if nonzero_literal(y) => binary("%", MUL, x, y),
            (IntMod, _) => helper("Spec::Ints.mod", &a),
            (IntPow, _) => helper("Spec::Ints.pow", &a),
            (IntDiv, _) => helper("Spec::Ints.div", &a),
            (IntBand, _) => helper("Spec::Ints.band", &a),
            (IntBor, _) => helper("Spec::Ints.bor", &a),
            (IntBxor, _) => helper("Spec::Ints.bxor", &a),
            (IntShl, _) => helper("Spec::Ints.shl", &a),
            (IntShr, _) => helper("Spec::Ints.shr", &a),
            (IntLt | TextLt, [x, y]) => compare("<", x, y),
            (IntLe | TextLe, [x, y]) => compare("<=", x, y),
            (IntGt | TextGt, [x, y]) => compare(">", x, y),
            (IntGe | TextGe, [x, y]) => compare(">=", x, y),
            (IntAbs, [x]) => method(x, "abs"),
            (IntMin, [x, y]) => Code::atom(format!("[{}, {}].min", x.text, y.text)),
            (IntMax, [x, y]) => Code::atom(format!("[{}, {}].max", x.text, y.text)),
            (IntClamp, [x, lo, hi]) => send(x, "clamp", &[lo, hi]),
            (TextSize | ListSize | SetSize | MapSize, [x]) => method(x, "size"),
            (TextBytes, [x]) => method(x, "bytes"),
            (TextTrim, _) => helper("Spec::Texts.trim", &a),
            (TextLower, _) => helper("Spec::Texts.lower", &a),
            (TextStartsWith, [x, y]) => send(x, "start_with?", &[y]),
            (TextContains | ListContains | SetContains, [x, y]) => send(x, "include?", &[y]),
            (TextSplit, _) => helper("Spec::Texts.split", &a),
            (TextReplace, _) => helper("Spec::Texts.replace", &a),
            (TextTake, _) => helper("Spec::Texts.take", &a),
            (TextDrop, _) => helper("Spec::Texts.drop", &a),
            (ListFirst, _) => helper("Spec::Lists.first", &a),
            (ListLast, _) => helper("Spec::Lists.last", &a),
            (ListGet, _) => helper("Spec::Lists.get", &a),
            (ListIndex, _) => helper("Spec::Lists.at", &a),
            (ListSum, [x]) => method(x, "sum"),
            (ListMin, _) => helper("Spec::Lists.min", &a),
            (ListMax, _) => helper("Spec::Lists.max", &a),
            (ListDistinct, [x]) => method(x, "uniq"),
            (ListSort, [x, desc]) if desc.text == "false" => helper("Spec::Lists.sort", std::slice::from_ref(x)),
            (ListSort, [x, desc]) => Code::atom(format!("Spec::Lists.sort({}, desc: {})", x.text, desc.text)),
            (ListReversed, [x]) => method(x, "reverse"),
            (ListTake, [x, n]) => send(x, "take", &[n]),
            (ListDrop, [x, n]) => send(x, "drop", &[n]),
            (ListZip, _) => helper("Spec::Lists.zip", &a),
            (ListJoin, [x, sep]) if sep.text == "\"\"" => method(x, "join"),
            (ListJoin, [x, sep]) => send(x, "join", &[sep]),
            (ListConcat, [x, y]) => binary("+", ADD, x, y),
            (ListPush, [x, y]) => Code::new(format!("{} + [{}]", x.at(ADD), y.text), ADD),
            (ListInsert, _) => helper("Spec::Lists.insert", &a),
            (ListRemove, _) => helper("Spec::Lists.remove", &a),
            (SetList, [x]) => method(x, "to_a"),
            (SetOfList, _) => helper("Spec::Sets.of", &a),
            (SetUnion, _) => helper("Spec::Sets.union", &a),
            (SetDifference, [x, y]) => binary("-", ADD, x, y),
            (SetInsert, _) => helper("Spec::Sets.add", &a),
            (SetRemove, [x, y]) => Code::new(format!("{} - [{}]", x.at(ADD), y.text), ADD),
            (MapKeys, [x]) => method(x, "keys"),
            (MapValues, [x]) => method(x, "values"),
            (MapGet, _) => helper("Spec::Maps.get", &a),
            (MapIndex, _) => helper("Spec::Maps.at", &a),
            (MapHas, [x, y]) => send(x, "key?", &[y]),
            (MapRemove, [x, y]) => send(x, "except", &[y]),
            (RangeList, _) => helper("Spec::Ranges.list", &a),
            (RangeHas, [x, y]) => send(x, "cover?", &[y]),
            (TraceSteps, [x]) => method(x, "steps"),
            (TraceStates, [x]) => method(x, "states"),
            (StepEvent, [x]) => method(x, "event"),
            (StepReply, [x]) => method(x, "reply"),
            (StepBefore, [x]) => method(x, "before"),
            (StepAfter, [x]) => method(x, "after"),
            _ => return Err(wrong(op)),
        })
    }
}

fn nonzero_literal(c: &Code) -> bool {
    c.text.parse::<i128>().is_ok_and(|n| n != 0)
}

fn wrong(op: Prim) -> Diag {
    Diag::new(
        spec_lang_diag::Severity::Error,
        "E-UNSUPPORTED",
        format!("unsupported: the primitive `{}` with these arguments (the Ruby printer)", op.name()),
    )
}
