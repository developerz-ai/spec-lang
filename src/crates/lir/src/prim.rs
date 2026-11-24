//! Every library operation and operator the LIR names, each with one fixed
//! meaning. A printer implements each once in its runtime; `README.md`
//! lists them all (a unit test keeps the list and this table equal).
//!
//! Arguments are evaluated left to right in the order of the signature, the
//! receiver first. "fails `E`" is a language failure (handled by `else`,
//! reported by a rule as its refusal); "stops `CODE`" ends the check with
//! that diagnostic and is never handled.

macro_rules! prims {
    ($($v:ident = $name:literal, $sig:literal, $meaning:literal;)*) => {
        /// A named primitive.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Prim { $($v,)* }

        impl Prim {
            /// Every primitive, in table order.
            pub const ALL: &'static [Prim] = &[$(Prim::$v,)*];

            /// The stable name printers and the JSON use: `list.filter`.
            pub fn name(self) -> &'static str {
                match self { $(Prim::$v => $name,)* }
            }

            /// The signature, receiver first.
            pub fn signature(self) -> &'static str {
                match self { $(Prim::$v => $sig,)* }
            }

            /// What it does, exactly.
            pub fn meaning(self) -> &'static str {
                match self { $(Prim::$v => $meaning,)* }
            }
        }
    };
}

prims! {
    Eq = "eq", "(T, T) -> Bool", "structural equality on every type but functions";
    Ne = "ne", "(T, T) -> Bool", "not `eq`";
    Not = "not", "(Bool) -> Bool", "negation";

    IntAdd = "int.add", "(Int, Int) -> Int", "exact sum; stops E-RUNTIME outside the printer's integer range (the engine's is i128)";
    IntSub = "int.sub", "(Int, Int) -> Int", "exact difference; stops E-RUNTIME on overflow";
    IntMul = "int.mul", "(Int, Int) -> Int", "exact product; stops E-RUNTIME on overflow";
    IntNeg = "int.neg", "(Int) -> Int", "negation; stops E-RUNTIME on overflow";
    IntMod = "int.mod", "(Int, Int) -> Int", "floored modulo: the result has the sign of the divisor (`-1 mod 4 == 3`, `1 mod -4 == -3`); divisor 0 fails DivByZero";
    IntPow = "int.pow", "(Int, Int) -> Int", "exact power; a negative exponent stops E-TYPE-MISMATCH; overflow stops E-RUNTIME";
    IntDiv = "int.div", "(Int, Int) -> Int", "always stops E-OPEN: `/` on Int is an open choice (write `a.div(b, floor)`)";
    IntBand = "int.band", "(Int, Int) -> Int", "bitwise and; a negative operand stops E-TYPE-MISMATCH";
    IntBor = "int.bor", "(Int, Int) -> Int", "bitwise or; a negative operand stops E-TYPE-MISMATCH";
    IntBxor = "int.bxor", "(Int, Int) -> Int", "bitwise xor; a negative operand stops E-TYPE-MISMATCH";
    IntShl = "int.shl", "(Int, Int) -> Int", "a * 2^b; a negative operand stops E-TYPE-MISMATCH; overflow stops E-RUNTIME";
    IntShr = "int.shr", "(Int, Int) -> Int", "floor(a / 2^b), 0 when b >= 128; a negative operand stops E-TYPE-MISMATCH";
    IntLt = "int.lt", "(Int, Int) -> Bool", "a < b";
    IntLe = "int.le", "(Int, Int) -> Bool", "a <= b";
    IntGt = "int.gt", "(Int, Int) -> Bool", "a > b";
    IntGe = "int.ge", "(Int, Int) -> Bool", "a >= b";
    IntAbs = "int.abs", "(Int) -> Int", "absolute value; stops E-TYPE-MISMATCH when it does not fit";
    IntMin = "int.min", "(Int, Int) -> Int", "the smaller";
    IntMax = "int.max", "(Int, Int) -> Int", "the larger";
    IntClamp = "int.clamp", "(Int, lo: Int, hi: Int) -> Int", "the value pulled into lo..=hi; lo > hi stops E-TYPE-MISMATCH";

    TextLt = "text.lt", "(Text, Text) -> Bool", "code point order; a prefix comes first";
    TextLe = "text.le", "(Text, Text) -> Bool", "code point order";
    TextGt = "text.gt", "(Text, Text) -> Bool", "code point order";
    TextGe = "text.ge", "(Text, Text) -> Bool", "code point order";
    TextSize = "text.size", "(Text) -> Int", "number of code points";
    TextBytes = "text.bytes", "(Text) -> List[Int]", "the UTF-8 bytes, each 0..=255";
    TextTrim = "text.trim", "(Text) -> Text", "removes Unicode White_Space (25 code points; not U+200B, U+FEFF, U+180E) from both ends";
    TextLower = "text.lower", "(Text) -> Text", "full Unicode lowercase mapping, no locale";
    TextStartsWith = "text.starts_with", "(Text, Text) -> Bool", "prefix test";
    TextContains = "text.contains", "(Text, Text) -> Bool", "substring test (also `x in t`)";
    TextSplit = "text.split", "(Text, sep: Text) -> List[Text]", "pieces between separators, empty ones kept; an empty separator stops E-TYPE-ARITY";
    TextReplace = "text.replace", "(Text, old: Text, new: Text) -> Text", "every non-overlapping match, left to right; an empty `old` stops E-TYPE-ARITY";
    TextTake = "text.take", "(Text, Int) -> Text", "the first n code points (a negative n is 0)";
    TextDrop = "text.drop", "(Text, Int) -> Text", "all but the first n code points (a negative n is 0)";

    ListSize = "list.size", "(List[T]) -> Int", "number of items";
    ListFirst = "list.first", "(List[T]) -> T", "the first item; empty fails Missing";
    ListLast = "list.last", "(List[T]) -> T", "the last item; empty fails Missing";
    ListGet = "list.get", "(List[T], Int) -> Option[T]", "the item at an index from 0, or none";
    ListIndex = "list.index", "(List[T], Int) -> T", "`xs[i]`: the item at i; outside fails Missing";
    ListFilter = "list.filter", "(List[T], T -> Bool) -> List[T]", "the items the predicate keeps, in order";
    ListMap = "list.map", "(List[T], T -> U) -> List[U]", "f of every item, in order; the first failure wins";
    ListFlatMap = "list.flat_map", "(List[T], T -> List[U]) -> List[U]", "the lists f gives, joined in order";
    ListFold = "list.fold", "(List[T], A, (A, T) -> A) -> A", "left to right from the start value";
    ListAll = "list.all", "(List[T], T -> Bool) -> Bool", "every item holds; stops at the first false; empty is true";
    ListAny = "list.any", "(List[T], T -> Bool) -> Bool", "some item holds; stops at the first true; empty is false";
    ListFind = "list.find", "(List[T], T -> Bool) -> Option[T]", "the earliest item that holds";
    ListPosition = "list.position", "(List[T], T -> Bool) -> Option[Int]", "the index of the earliest item that holds";
    ListSum = "list.sum", "(List[Int]) -> Int", "the sum, 0 when empty; overflow stops E-RUNTIME";
    ListSumBy = "list.sum_by", "(List[T], T -> Int) -> Int", "the sum of f of every item";
    ListMin = "list.min", "(List[T]) -> T", "the least item in the value order, the earliest of ties; empty fails Missing";
    ListMax = "list.max", "(List[T]) -> T", "the greatest item, the earliest of ties; empty fails Missing";
    ListMinBy = "list.min_by", "(List[T], T -> K) -> T", "the item with the least key, the earliest of ties; empty fails Missing";
    ListMaxBy = "list.max_by", "(List[T], T -> K) -> T", "the item with the greatest key, the earliest of ties; empty fails Missing";
    ListContains = "list.contains", "(List[T], T) -> Bool", "some item equals x";
    ListDistinct = "list.distinct", "(List[T]) -> List[T]", "the earliest of each group of equal items, in order";
    ListSort = "list.sort", "(List[T], desc: Bool) -> List[T]", "stable sort by the value order; desc reverses the order, not the ties";
    ListSortBy = "list.sort_by", "(List[T], T -> K, desc: Bool) -> List[T]", "stable sort by the key's value order; desc reverses the order, not the ties";
    ListReversed = "list.reversed", "(List[T]) -> List[T]", "the items in reverse order";
    ListTake = "list.take", "(List[T], Int) -> List[T]", "the first n items (all when fewer); a negative n stops E-TYPE-ARITY";
    ListDrop = "list.drop", "(List[T], Int) -> List[T]", "all but the first n items; a negative n stops E-TYPE-ARITY";
    ListZip = "list.zip", "(List[T], List[U]) -> List[(T, U)]", "pairs by position; unequal sizes fail Uneven";
    ListGroupBy = "list.group_by", "(List[T], T -> K) -> Map[K, List[T]]", "items by key, order kept inside a group";
    ListJoin = "list.join", "(List[Text], Text) -> Text", "the texts with the separator between";
    ListConcat = "list.concat", "(List[T], List[T]) -> List[T]", "`a + b`: a then b";
    ListPush = "list.push", "(List[T], T) -> List[T]", "the list with x at the end";
    ListInsert = "list.insert", "(List[T], Int, T) -> List[T]", "x put before index at (0..=size); outside fails Missing";
    ListRemove = "list.remove", "(List[T], Int) -> List[T]", "the item at index removed; outside fails Missing";

    SetList = "set.list", "(Set[T]) -> List[T]", "the elements in ascending value order";
    SetOfList = "set.of_list", "(List[T]) -> Set[T]", "the set of the items";
    SetSize = "set.size", "(Set[T]) -> Int", "number of elements";
    SetContains = "set.contains", "(Set[T], T) -> Bool", "membership (also `x in s`)";
    SetUnion = "set.union", "(Set[T], Set[T]) -> Set[T]", "`a + b`";
    SetDifference = "set.difference", "(Set[T], Set[T]) -> Set[T]", "`a - b`";
    SetInsert = "set.insert", "(Set[T], T) -> Set[T]", "the set with x; already there: unchanged";
    SetRemove = "set.remove", "(Set[T], T) -> Set[T]", "the set without x; absent: unchanged";

    MapSize = "map.size", "(Map[K, V]) -> Int", "number of entries";
    MapKeys = "map.keys", "(Map[K, V]) -> List[K]", "keys ascending";
    MapValues = "map.values", "(Map[K, V]) -> List[V]", "values in ascending key order";
    MapGet = "map.get", "(Map[K, V], K) -> Option[V]", "the value at k, or none";
    MapIndex = "map.index", "(Map[K, V], K) -> V", "`m[k]`: the value at k; absent fails Missing";
    MapHas = "map.has", "(Map[K, V], K) -> Bool", "`k in m`: k is a key";
    MapRemove = "map.remove", "(Map[K, V], K) -> Map[K, V]", "the map without k; absent: unchanged";

    RangeList = "range.list", "(Range) -> List[Int]", "the numbers lo..=hi (or lo..<hi) ascending; more than ten million stops E-RUNTIME";
    RangeHas = "range.has", "(Range, Int) -> Bool", "`x in lo..=hi`";

    TraceSteps = "trace.steps", "(Trace[R]) -> List[Step[R]]", "the steps in order";
    TraceStates = "trace.states", "(Trace[R]) -> List[World]", "the start world, then the world after each step";
    StepEvent = "step.event", "(Step[R]) -> Event[R]", "the call: its rule and arguments";
    StepReply = "step.reply", "(Step[R]) -> Reply[R]", "Ok(value) or Err(failure); a refused step has before == after";
    StepBefore = "step.before", "(Step[R]) -> World", "the world before the call";
    StepAfter = "step.after", "(Step[R]) -> World", "the world after the call";
}

impl Prim {
    /// The primitives `x in c` names: in a comparison chain the collection
    /// (the right side) is their first argument.
    pub fn is_containment(self) -> bool {
        matches!(self, Prim::ListContains | Prim::SetContains | Prim::MapHas | Prim::RangeHas | Prim::TextContains)
    }
}

#[cfg(test)]
mod tests {
    use super::Prim;

    #[test]
    fn names_are_unique_and_documented() {
        let mut names: Vec<&str> = Prim::ALL.iter().map(|p| p.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Prim::ALL.len());
        let readme = include_str!("../README.md");
        let missing: Vec<&str> =
            Prim::ALL.iter().map(|p| p.name()).filter(|n| !readme.contains(&format!("`{n}`"))).collect();
        assert!(missing.is_empty(), "primitives missing from README.md: {missing:?}");
    }
}
