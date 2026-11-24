//! Declared types.

use crate::expr::Expr;
use crate::ids::{ModuleId, RuleId, TypeId};

/// A type as written in a parameter, field, state, const or `type` body,
/// with every name resolved.
#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    /// `()`.
    Unit,
    Bool,
    /// Exact and unbounded in the language; the engine's limit is in its docs.
    Int,
    /// Unicode text; size counts code points.
    Text,
    /// A declared type: record, sum, alias or rule set. See [`crate::TypeDef`].
    Named(TypeId),
    Option(Box<Ty>),
    List(Box<Ty>),
    /// Iterates in ascending order.
    Set(Box<Ty>),
    /// Iterates in ascending key order.
    Map(Box<Ty>, Box<Ty>),
    Tuple(Vec<Ty>),
    /// `lo..=hi` (inclusive) or `lo..<hi`: `Int` with both bounds.
    Range {
        lo: i128,
        hi: i128,
        inclusive: bool,
    },
    /// `T where claim`: values of `T` for which the claim holds.
    Where(Box<Ty>, Box<Claim>),
    /// `Trace[r | ...]`: test and law parameters only.
    Trace(Vec<RuleId>),
}

/// A claim on `self`: a `Bool` expression in which the local `self` is the value.
#[derive(Clone, Debug, PartialEq)]
pub struct Claim {
    pub expr: Expr,
    /// The module whose names the claim was resolved in.
    pub module: ModuleId,
}

impl Ty {
    /// The type without its claims: `Title` (`Text where is_title(self)`) is not
    /// unfolded here, because `Named` is opaque; `Text where p` gives `Text`.
    pub fn strip_claims(&self) -> &Ty {
        match self {
            Ty::Where(base, _) => base.strip_claims(),
            t => t,
        }
    }
}
