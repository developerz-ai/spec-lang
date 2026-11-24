//! Typed indexes into the tables of a [`crate::Program`].

macro_rules! id {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);

        impl $name {
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    )*};
}

id! {
    /// A module: one `.slg` file, or the built-in core module.
    ModuleId,
    /// A `type` declaration, or a built-in type such as `Option`.
    TypeId,
    /// A `const`.
    ConstId,
    /// A `state`: one place of the world.
    StateId,
    /// A `fn`.
    FnId,
    /// A `rule`.
    RuleId,
    /// A `test`.
    TestId,
    /// A `law`.
    LawId,
    /// An `app`.
    AppId,
    /// A lambda expression; the table lets values refer to code by id.
    LambdaId,
    /// A handler of a loaded library ([`crate::library::Handler`]).
    HandlerId,
}

/// Any top-level declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeclRef {
    Type(TypeId),
    Const(ConstId),
    State(StateId),
    Fn(FnId),
    Rule(RuleId),
    Test(TestId),
    Law(LawId),
    App(AppId),
}

/// Convert a table length to an id index.
pub(crate) fn next_id(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}
