//! Module scopes: what a bare name means at module level.
//!
//! Lookup order for a bare name (`docs/idea/30-type-rules.md` section 2):
//! locals and parameters (held by the expression lowerer), then this module,
//! then names bound by `use`, then the core library. A name found both in
//! this module and through `use` is ambiguous.

use crate::ids::{DeclRef, ModuleId, TypeId};
use std::collections::BTreeMap;

/// What a module-level name is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Decl(DeclRef),
    Variant(TypeId, u32),
    Module(ModuleId),
}

#[derive(Debug, Default)]
pub(crate) struct Scopes {
    /// Per module: its own declarations and the variants of its sums.
    pub(crate) own: Vec<BTreeMap<String, Binding>>,
    /// Per module: names bound by its `use` items.
    pub(crate) imported: Vec<BTreeMap<String, Binding>>,
}

/// Why a lookup failed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LookupError {
    Unknown,
    Ambiguous,
}

impl Scopes {
    pub(crate) fn ensure(&mut self, n: usize) {
        self.own.resize_with(n, BTreeMap::new);
        self.imported.resize_with(n, BTreeMap::new);
    }

    pub(crate) fn lookup(&self, m: ModuleId, name: &str) -> Result<Binding, LookupError> {
        let own = self.own.get(m.index()).and_then(|s| s.get(name));
        let imported = self.imported.get(m.index()).and_then(|s| s.get(name));
        match (own, imported) {
            (Some(a), Some(b)) if a != b => Err(LookupError::Ambiguous),
            (Some(a), _) | (None, Some(a)) => Ok(*a),
            (None, None) => Err(LookupError::Unknown),
        }
    }

    /// A member of another module: only what that module itself declares.
    pub(crate) fn member(&self, m: ModuleId, name: &str) -> Option<Binding> {
        self.own.get(m.index()).and_then(|s| s.get(name)).copied()
    }
}
