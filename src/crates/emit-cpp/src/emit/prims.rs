//! The 93 primitives, each as the C++ the runtime gives it.

use super::Code;
use spec_lang_lir::Prim;

/// A primitive applied to printed arguments.
pub fn apply(op: Prim, a: &[Code]) -> Code {
    // Comparisons do not chain in C++: both sides bind tighter.
    let bin = |sym: &str, p: u8| {
        let (left, right) = if p == 8 || p == 9 { (10, 10) } else { (p, p + 1) };
        Code::new(format!("{} {sym} {}", a[0].at(left), a[1].at(right)), p)
    };
    let call = |name: &str| Code::atom(format!("rt::{name}({})", a.iter().map(|x| x.at(2)).collect::<Vec<_>>().join(", ")));
    let member = |name: &str| Code::atom(format!("{}.{name}", a[0].at(16)));
    match op {
        Prim::Eq => bin("==", 8),
        Prim::Ne => bin("!=", 8),
        Prim::Not => Code::new(format!("!{}", a[0].at(15)), 15),
        Prim::IntAdd => bin("+", 12),
        Prim::IntSub => bin("-", 12),
        Prim::IntMul => bin("*", 13),
        Prim::IntNeg => Code::new(format!("-{}", a[0].at(16)), 15),
        Prim::IntMod => call("mod"),
        Prim::IntPow => call("pow"),
        Prim::IntDiv => call("div"),
        Prim::IntBand => call("band"),
        Prim::IntBor => call("bor"),
        Prim::IntBxor => call("bxor"),
        Prim::IntShl => call("shl"),
        Prim::IntShr => call("shr"),
        Prim::IntLt | Prim::TextLt => bin("<", 9),
        Prim::IntLe | Prim::TextLe => bin("<=", 9),
        Prim::IntGt | Prim::TextGt => bin(">", 9),
        Prim::IntGe | Prim::TextGe => bin(">=", 9),
        Prim::IntAbs => call("abs"),
        Prim::IntMin | Prim::ListMin => call("min"),
        Prim::IntMax | Prim::ListMax => call("max"),
        Prim::IntClamp => call("clamp"),
        Prim::TextSize | Prim::ListSize | Prim::SetSize | Prim::MapSize => call("size"),
        Prim::TextBytes => call("bytes"),
        Prim::TextTrim => call("trim"),
        Prim::TextLower => call("lower"),
        Prim::TextStartsWith => call("starts_with"),
        Prim::TextContains | Prim::ListContains | Prim::SetContains | Prim::MapHas | Prim::RangeHas => call("contains"),
        Prim::TextSplit => call("split"),
        Prim::TextReplace => call("replace"),
        Prim::TextTake | Prim::ListTake => call("take"),
        Prim::TextDrop | Prim::ListDrop => call("drop"),
        Prim::ListFirst => call("first"),
        Prim::ListLast => call("last"),
        Prim::ListGet | Prim::MapGet => call("get"),
        Prim::ListIndex | Prim::MapIndex => call("at"),
        Prim::ListFilter => call("filter"),
        Prim::ListMap => call("map"),
        Prim::ListFlatMap => call("flat_map"),
        Prim::ListFold => call("fold"),
        Prim::ListAll => call("all"),
        Prim::ListAny => call("any"),
        Prim::ListFind => call("find"),
        Prim::ListPosition => call("position"),
        Prim::ListSum => call("sum"),
        Prim::ListSumBy => call("sum_by"),
        Prim::ListMinBy => call("min_by"),
        Prim::ListMaxBy => call("max_by"),
        Prim::ListDistinct => call("distinct"),
        Prim::ListSort | Prim::ListSortBy => {
            // `desc: false` is the default of the runtime's sort.
            let n = if a.last().is_some_and(|d| d.s == "false") { a.len() - 1 } else { a.len() };
            let name = if op == Prim::ListSort { "sort" } else { "sort_by" };
            Code::atom(format!("rt::{name}({})", a[..n].iter().map(|x| x.at(2)).collect::<Vec<_>>().join(", ")))
        }
        Prim::ListReversed => call("reversed"),
        Prim::ListZip => call("zip"),
        Prim::ListGroupBy => call("group_by"),
        Prim::ListJoin => call("join"),
        Prim::ListConcat => call("concat"),
        Prim::ListPush => call("pushed"),
        Prim::ListInsert => call("inserted_at"),
        Prim::ListRemove => call("removed_at"),
        Prim::SetList | Prim::RangeList => call("list"),
        Prim::SetOfList => call("set_of"),
        Prim::SetUnion => call("set_union"),
        Prim::SetDifference => call("difference"),
        Prim::SetInsert => call("with"),
        Prim::SetRemove | Prim::MapRemove => call("without"),
        Prim::MapKeys => call("keys"),
        Prim::MapValues => call("values"),
        Prim::TraceSteps => member("steps"),
        Prim::TraceStates => member("states"),
        Prim::StepEvent => member("event"),
        Prim::StepReply => member("reply"),
        Prim::StepBefore => member("before"),
        Prim::StepAfter => member("after"),
    }
}

/// A `Mutate` done in place: the statement, given the place and the other arguments.
pub fn in_place(op: Prim, place: &str, a: &[Code]) -> Option<String> {
    let args = a.iter().map(|x| x.at(2)).collect::<Vec<_>>().join(", ");
    Some(match op {
        Prim::ListPush => format!("rt::push({place}, {args});"),
        Prim::ListInsert => format!("rt::insert_at({place}, {args});"),
        Prim::ListRemove => format!("rt::remove_at({place}, {args});"),
        Prim::SetInsert => format!("{place}.insert({args});"),
        Prim::SetRemove | Prim::MapRemove => format!("{place}.erase({args});"),
        _ => return None,
    })
}
