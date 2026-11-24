//! Runtime values. Equality is structural on every value; the order is the
//! one `docs/idea/28-standard-library.md` fixes: `false < true`, numbers by
//! value, text by code point, records field by field, sums by declared
//! variant then fields, lists and tuples item by item.

use crate::world::World;
use spec_lang_ir::{LambdaId, RuleId, TypeId};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Value {
    Unit,
    Bool(bool),
    Int(i128),
    Text(Rc<str>),
    List(Rc<Vec<Value>>),
    Set(Rc<BTreeSet<Value>>),
    Map(Rc<BTreeMap<Value, Value>>),
    Tuple(Rc<Vec<Value>>),
    Record(TypeId, Rc<Vec<Value>>),
    Variant(TypeId, u32, Rc<Vec<Value>>),
    /// `lo..=hi` or `lo..<hi` over `Int`.
    Range(i128, i128, bool),
    Func(Rc<Closure>),
    /// A refused reply in a trace.
    Failure(Rc<Failure>),
    /// A rule call in a trace: `t.steps[i].event`.
    Event(Rc<Event>),
    World(Rc<World>),
    Step(Rc<Step>),
    Trace(Rc<Trace>),
}

/// A lambda with the locals it saw.
#[derive(Debug)]
pub struct Closure {
    pub lambda: LambdaId,
    pub captured: Vec<(Rc<str>, Value)>,
}

/// A failure: its error name and data.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Failure {
    pub name: String,
    pub data: Vec<Value>,
}

/// One call of a rule, with its arguments in parameter order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Event {
    pub rule: RuleId,
    pub args: Vec<Value>,
}

/// One step of a trace.
#[derive(Clone, Debug)]
pub struct Step {
    pub event: Value,
    /// The reply value, or [`Value::Failure`].
    pub reply: Value,
    pub before: Value,
    pub after: Value,
}

/// A whole trace: its steps and the world before and after each.
#[derive(Clone, Debug)]
pub struct Trace {
    pub steps: Vec<Value>,
    pub states: Vec<Value>,
}

impl Value {
    pub fn text(s: &str) -> Value {
        Value::Text(Rc::from(s))
    }

    pub fn list(v: Vec<Value>) -> Value {
        Value::List(Rc::new(v))
    }

    fn rank(&self) -> u8 {
        match self {
            Value::Unit => 0,
            Value::Bool(_) => 1,
            Value::Int(_) => 2,
            Value::Text(_) => 3,
            Value::List(_) => 4,
            Value::Set(_) => 5,
            Value::Map(_) => 6,
            Value::Tuple(_) => 7,
            Value::Record(..) => 8,
            Value::Variant(..) => 9,
            Value::Range(..) => 10,
            Value::Func(_) => 11,
            Value::Failure(_) => 12,
            Value::Event(_) => 13,
            Value::World(_) => 14,
            Value::Step(_) => 15,
            Value::Trace(_) => 16,
        }
    }

    /// Whether `==` is defined: everything but functions.
    pub fn comparable(&self) -> bool {
        match self {
            Value::Func(_) => false,
            Value::List(xs) | Value::Tuple(xs) | Value::Record(_, xs) | Value::Variant(_, _, xs) => {
                xs.iter().all(Value::comparable)
            }
            Value::Map(m) => m.values().all(Value::comparable),
            _ => true,
        }
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        use Value::*;
        match (self, other) {
            (Unit, Unit) => Ordering::Equal,
            (Bool(a), Bool(b)) => a.cmp(b),
            (Int(a), Int(b)) => a.cmp(b),
            (Text(a), Text(b)) => a.cmp(b),
            (List(a), List(b)) | (Tuple(a), Tuple(b)) => a.cmp(b),
            (Set(a), Set(b)) => a.cmp(b),
            (Map(a), Map(b)) => a.cmp(b),
            (Record(t, a), Record(u, b)) => t.cmp(u).then_with(|| a.cmp(b)),
            (Variant(t, x, a), Variant(u, y, b)) => t.cmp(u).then(x.cmp(y)).then_with(|| a.cmp(b)),
            (Range(a, b, c), Range(d, e, f)) => (a, b, c).cmp(&(d, e, f)),
            (Func(a), Func(b)) => (Rc::as_ptr(a) as usize).cmp(&(Rc::as_ptr(b) as usize)),
            (Failure(a), Failure(b)) => a.cmp(b),
            (Event(a), Event(b)) => a.cmp(b),
            (World(a), World(b)) => a.states.cmp(&b.states),
            (Step(a), Step(b)) => {
                (&a.event, &a.reply, &a.before, &a.after).cmp(&(&b.event, &b.reply, &b.before, &b.after))
            }
            (Trace(a), Trace(b)) => (&a.steps, &a.states).cmp(&(&b.steps, &b.states)),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Value {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_is_the_library_order() {
        assert!(Value::Bool(false) < Value::Bool(true));
        assert!(Value::text("B") < Value::text("a"), "code point order");
        let t = TypeId(1);
        let a = Value::Variant(t, 0, Rc::new(vec![Value::Int(9)]));
        let b = Value::Variant(t, 1, Rc::new(vec![]));
        assert!(a < b, "declared variant order first");
        assert_eq!(Value::list(vec![Value::Int(1)]), Value::list(vec![Value::Int(1)]));
    }
}
