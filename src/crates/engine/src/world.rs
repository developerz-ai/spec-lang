//! The world: the value of every state. Cloning shares structure.

use crate::value::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    /// Indexed by `StateId`.
    pub states: Vec<Value>,
}
