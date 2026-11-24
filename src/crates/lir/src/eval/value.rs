//! Runtime values of the LIR evaluator. Equality is structural; the order is
//! the value order of `docs/idea/28-standard-library.md`: `false < true`,
//! numbers by value, text by code point, records field by field, sums by
//! declared variant then fields, lists and tuples item by item. Values of
//! different kinds order by kind (only traces mix kinds; the order between
//! them is the engine's).

use crate::node::Lambda;
use spec_lang_ir::{RuleId, TypeId};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Value<'l> {
    Unit,
    Bool(bool),
    Int(i128),
    Text(Rc<str>),
    List(Rc<Vec<Value<'l>>>),
    Set(Rc<BTreeSet<Value<'l>>>),
    Map(Rc<BTreeMap<Value<'l>, Value<'l>>>),
    Tuple(Rc<Vec<Value<'l>>>),
    Record(TypeId, Rc<Vec<Value<'l>>>),
    /// A sum value; `Option` and a trace reply (`Result`) too.
    Variant(TypeId, u32, Rc<Vec<Value<'l>>>),
    Range(i128, i128, bool),
    Func(Rc<Closure<'l>>),
    Failure(Rc<Failure<'l>>),
    Event(Rc<Event<'l>>),
    /// Every state, by `StateId`.
    World(Rc<Vec<Value<'l>>>),
    Step(Rc<Step<'l>>),
    Trace(Rc<Trace<'l>>),
}

/// A lambda and the values of the locals it captured.
#[derive(Debug)]
pub struct Closure<'l> {
    pub lambda: &'l Lambda,
    pub captured: Vec<(Rc<str>, Value<'l>)>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Failure<'l> {
    pub name: String,
    pub data: Vec<Value<'l>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Event<'l> {
    pub rule: RuleId,
    pub args: Vec<Value<'l>>,
}

#[derive(Clone, Debug)]
pub struct Step<'l> {
    pub event: Value<'l>,
    pub reply: Value<'l>,
    pub before: Value<'l>,
    pub after: Value<'l>,
}

#[derive(Clone, Debug)]
pub struct Trace<'l> {
    pub steps: Vec<Value<'l>>,
    pub states: Vec<Value<'l>>,
}

impl<'l> Value<'l> {
    pub fn text(s: &str) -> Self {
        Value::Text(Rc::from(s))
    }

    pub fn list(v: Vec<Value<'l>>) -> Self {
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
}

impl Ord for Value<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        use Value::*;
        match (self, other) {
            (Unit, Unit) => Ordering::Equal,
            (Bool(a), Bool(b)) => a.cmp(b),
            (Int(a), Int(b)) => a.cmp(b),
            (Text(a), Text(b)) => a.cmp(b),
            (List(a), List(b)) | (Tuple(a), Tuple(b)) | (World(a), World(b)) => a.cmp(b),
            (Set(a), Set(b)) => a.cmp(b),
            (Map(a), Map(b)) => a.cmp(b),
            (Record(t, a), Record(u, b)) => t.cmp(u).then_with(|| a.cmp(b)),
            (Variant(t, x, a), Variant(u, y, b)) => t.cmp(u).then(x.cmp(y)).then_with(|| a.cmp(b)),
            (Range(a, b, c), Range(d, e, f)) => (a, b, c).cmp(&(d, e, f)),
            (Func(a), Func(b)) => (Rc::as_ptr(a) as usize).cmp(&(Rc::as_ptr(b) as usize)),
            (Failure(a), Failure(b)) => a.cmp(b),
            (Event(a), Event(b)) => a.cmp(b),
            (Step(a), Step(b)) => {
                (&a.event, &a.reply, &a.before, &a.after).cmp(&(&b.event, &b.reply, &b.before, &b.after))
            }
            (Trace(a), Trace(b)) => (&a.steps, &a.states).cmp(&(&b.steps, &b.states)),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

impl PartialOrd for Value<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Value<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Value<'_> {}
