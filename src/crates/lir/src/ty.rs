//! The two kinds of type in the LIR.
//!
//! * [`LTy`]: the type of every expression, with claims erased. An alias is
//!   its base (`Title` is `Text`), a range type is `Int`, `T where p` is `T`.
//!   Records and sums stay nominal ([`LTy::Named`]).
//! * [`DTy`]: a declared type, kept where a value is checked against it
//!   (parameters, results, fields, typed `let`s, state writes, `@given`
//!   values, generated values). It keeps ranges and claims: checking a value
//!   against a `DTy` is the LIR's `conform` (see `README.md`).

use spec_lang_ir::{RuleId, TypeId};

/// The type of an expression.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LTy {
    Unit,
    Bool,
    /// Exact. A printer picks a width and stops on overflow (`README.md`, Int).
    Int,
    /// Unicode text; sizes count code points.
    Text,
    /// A record, a sum, or the built-in `Option`/`Result` declared by name.
    Named(TypeId),
    Option(Box<LTy>),
    List(Box<LTy>),
    /// Ordered by the value order; iterates ascending.
    Set(Box<LTy>),
    /// Ordered by key; iterates in ascending key order.
    Map(Box<LTy>, Box<LTy>),
    Tuple(Vec<LTy>),
    /// The value of `lo..=hi` / `lo..<hi` in an expression: a range of `Int`.
    Range,
    /// A lambda: parameter types and result.
    Func(Vec<LTy>, Box<LTy>),
    /// A call of one of these rules, as a trace records it (`s.event`).
    Event(Vec<RuleId>),
    /// A trace step's reply: `Ok(value)` or `Err(failure)`.
    Reply(Vec<RuleId>),
    /// A refusal: an error name and its data.
    Failure,
    /// One step of a trace over these rules.
    Step(Vec<RuleId>),
    /// Every state's value at one moment.
    World,
    /// A whole trace over these rules.
    Trace(Vec<RuleId>),
    /// The type of an expression that never gives a value (`fail E`, `?hole`).
    Never,
    /// Lowering only: an unknown being solved. Never in a finished LIR.
    Var(u32),
}

impl LTy {
    pub fn list(t: LTy) -> LTy {
        LTy::List(Box::new(t))
    }
    pub fn set(t: LTy) -> LTy {
        LTy::Set(Box::new(t))
    }
    pub fn option(t: LTy) -> LTy {
        LTy::Option(Box::new(t))
    }
    pub fn map(k: LTy, v: LTy) -> LTy {
        LTy::Map(Box::new(k), Box::new(v))
    }
    pub fn func(params: Vec<LTy>, ret: LTy) -> LTy {
        LTy::Func(params, Box::new(ret))
    }

    /// Whether a value of this type can be compared with `==` or kept in a
    /// `Set` or a `Map` key: everything except a function.
    pub fn comparable(&self) -> bool {
        match self {
            LTy::Func(..) => false,
            LTy::Option(t) | LTy::List(t) | LTy::Set(t) => t.comparable(),
            LTy::Map(k, v) => k.comparable() && v.comparable(),
            LTy::Tuple(ts) => ts.iter().all(LTy::comparable),
            _ => true,
        }
    }

    /// Whether a lowering unknown is left anywhere inside.
    pub fn has_var(&self) -> bool {
        match self {
            LTy::Var(_) => true,
            LTy::Option(t) | LTy::List(t) | LTy::Set(t) => t.has_var(),
            LTy::Map(k, v) => k.has_var() || v.has_var(),
            LTy::Tuple(ts) => ts.iter().any(LTy::has_var),
            LTy::Func(ps, r) => ps.iter().any(LTy::has_var) || r.has_var(),
            _ => false,
        }
    }
}

/// An index into [`crate::Lir::claims`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClaimId(pub u32);

impl ClaimId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A declared type: what `conform` checks and what a generator draws from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DTy {
    Unit,
    Bool,
    Int,
    Text,
    /// A declared type by name: alias, record, sum or rule set.
    Named(TypeId),
    Option(Box<DTy>),
    List(Box<DTy>),
    Set(Box<DTy>),
    Map(Box<DTy>, Box<DTy>),
    Tuple(Vec<DTy>),
    /// An `Int` in `lo..=hi` (inclusive) or `lo..<hi`.
    Range {
        lo: i128,
        hi: i128,
        inclusive: bool,
    },
    /// The base, then the claim on `self`.
    Where(Box<DTy>, ClaimId),
    /// `Trace[r | ...]`: a law parameter, never generated as a value.
    Trace(Vec<RuleId>),
}
