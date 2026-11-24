//! The runtime every program printed by spec-lang's Rust printer shares: exact `Int`, text by code point, the
//! value order, `show`, failures and halts, the generator, and the check runner. Copied verbatim into each project;
//! it names no program. Meaning, primitive by primitive: `src/crates/lir/README.md` of spec-lang.

pub mod app;
pub mod data;
pub mod fun;
pub mod int;
pub mod list;
pub mod rng;
pub mod runner;
pub mod show;
pub mod text;

pub use fun::Fun;
pub use show::Show;

/// The language's `Int`: exact. `i128` with every operation checked; a result outside it stops with `E-RUNTIME`
/// instead of wrapping (the engine's range; the language's is unbounded).
pub type Int = i128;

/// A refusal: an error name and its data, each value as `show` prints it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Failure {
    pub name: String,
    pub data: Vec<String>,
}

/// Why evaluation gave no value.
#[derive(Clone, Debug)]
pub enum Halt {
    /// A language failure: `else` handles it, a rule reports it as its refusal.
    Fail(Failure),
    /// Evaluation stopped; never handled.
    Stop { code: &'static str, message: String },
    /// A test step or `assert` did not hold (`F-TEST`).
    Check(String),
    /// A `?hole` (`F-BLOCKED`) or a foreign body (`B-FOREIGN`).
    Blocked { code: &'static str, message: String },
}

impl Halt {
    /// The diagnostic code a check reports for this halt.
    pub fn code(&self) -> &'static str {
        match self {
            Halt::Fail(_) | Halt::Check(_) => "F-TEST",
            Halt::Stop { code, .. } | Halt::Blocked { code, .. } => code,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Halt::Fail(f) => format!("failed with {}", f.show()),
            Halt::Check(m) | Halt::Stop { message: m, .. } | Halt::Blocked { message: m, .. } => m.clone(),
        }
    }
}

pub type R<T> = Result<T, Halt>;

/// `fail Name(data)`.
pub fn fail(name: &str, data: Vec<String>) -> Halt {
    Halt::Fail(Failure { name: name.to_string(), data })
}

/// An absent key or index, an empty `first` or `min`.
pub fn missing() -> Halt {
    fail("Missing", Vec::new())
}

pub fn stop(code: &'static str, message: impl Into<String>) -> Halt {
    Halt::Stop { code, message: message.into() }
}

/// A value outside the 128-bit range.
pub fn overflow() -> Halt {
    stop("E-RUNTIME", "an Int beyond the 128-bit range (the language's Int is unbounded; this runtime's is not)")
}

/// `?name`: the check is blocked.
pub fn hole(name: &str) -> Halt {
    Halt::Blocked { code: "F-BLOCKED", message: format!("reached ?{name}") }
}

/// A `foreign` body: only a backend's app runs it; a check that reaches it is blocked.
pub fn foreign(decl: &str, written: &str) -> Halt {
    Halt::Blocked { code: "B-FOREIGN", message: format!("reached {decl}, which is {written}: only a backend runs it") }
}

/// Runs `f`: a `try` block, so `?` inside stops at it.
pub fn catch<T>(f: impl FnOnce() -> R<T>) -> R<T> {
    f()
}

/// `Some(v)` for a value, `None` for a language failure; any other halt goes on.
pub fn handled<T>(r: R<T>) -> R<Option<T>> {
    match r {
        Ok(v) => Ok(Some(v)),
        Err(Halt::Fail(_)) => Ok(None),
        Err(h) => Err(h),
    }
}

/// A value or a refusal; any other halt goes on.
pub fn settle<T>(r: R<T>) -> R<Result<T, Failure>> {
    match r {
        Ok(v) => Ok(Ok(v)),
        Err(Halt::Fail(f)) => Ok(Err(f)),
        Err(h) => Err(h),
    }
}

/// A rule's guard: false, or a failure while evaluating it, refuses with `error`.
pub fn guard(held: R<bool>, error: &str) -> R<()> {
    match held {
        Ok(true) => Ok(()),
        Ok(false) | Err(Halt::Fail(_)) => Err(fail(error, Vec::new())),
        Err(h) => Err(h),
    }
}

/// A value checked against its declared type (`conform`): false stops with `E-CLAIM`.
pub fn claim(holds: bool, what: &str) -> R<()> {
    if holds { Ok(()) } else { Err(stop("E-CLAIM", format!("{what} breaks its declared type"))) }
}

/// A claim of a `where` type: true holds; false or a failure does not.
pub fn holds(r: R<bool>) -> R<bool> {
    Ok(handled(r)?.unwrap_or(false))
}

/// `assert`.
pub fn assert(held: bool) -> R<()> {
    if held { Ok(()) } else { Err(Halt::Check("an assert is false".into())) }
}

/// A test step `subject -> value;`.
pub fn expect_value<T: PartialEq + Show>(got: Result<T, Failure>, want: T) -> R<()> {
    match got {
        Ok(v) if v == want => Ok(()),
        Ok(v) => Err(Halt::Check(format!("the step gave {}, expected {}", v.show(), want.show()))),
        Err(f) => Err(Halt::Check(format!("the step gave {}, expected {}", f.show(), want.show()))),
    }
}

/// A test step `subject -> fail Name(data);`: the name, and the data when some is given.
pub fn expect_fail<T: Show>(got: Result<T, Failure>, name: &str, data: Vec<String>) -> R<()> {
    match got {
        Err(f) if f.name == name && (data.is_empty() || f.data == data) => Ok(()),
        Err(f) => Err(Halt::Check(format!("the step gave {}, expected fail {name}", f.show()))),
        Ok(v) => Err(Halt::Check(format!("the step gave {}, expected fail {name}", v.show()))),
    }
}

/// `Option[T]`: `Some(value) | none`, in that declared order, so `Some(x) < none` in the value order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Opt<T> {
    Some(T),
    #[default]
    None,
}

impl<T> From<Option<T>> for Opt<T> {
    fn from(o: Option<T>) -> Self {
        match o {
            Some(v) => Opt::Some(v),
            None => Opt::None,
        }
    }
}

/// `lo..=hi` or `lo..<hi` as a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Range {
    pub lo: Int,
    pub hi: Int,
    pub inclusive: bool,
}

impl Range {
    /// `range.has`: `x in lo..=hi`.
    pub fn has(self, x: Int) -> bool {
        x >= self.lo && if self.inclusive { x <= self.hi } else { x < self.hi }
    }

    /// `range.list`: the numbers ascending; more than ten million stops.
    pub fn list(self) -> R<Vec<Int>> {
        let end = if self.inclusive { self.hi } else { self.hi.saturating_sub(1) };
        if end.saturating_sub(self.lo) > 10_000_000 {
            return Err(stop("E-RUNTIME", "a range of more than ten million numbers"));
        }
        Ok((self.lo..=end).collect())
    }
}

/// The type of an expression that never gives a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Never {}

/// A map built from entries left to right: a later equal key replaces an earlier one.
pub fn map_of<K: Ord, V>(entries: Vec<(K, V)>) -> std::collections::BTreeMap<K, V> {
    let mut m = std::collections::BTreeMap::new();
    for (k, v) in entries {
        m.insert(k, v);
    }
    m
}

/// The value at a key of a place being written into: absent fails `Missing`.
pub fn slot<'a, K: Ord, V>(m: &'a mut std::collections::BTreeMap<K, V>, k: &K) -> R<&'a mut V> {
    m.get_mut(k).ok_or_else(missing)
}

/// The item at an index of a place being written into: outside fails `Missing`.
pub fn item<T>(xs: &mut [T], i: Int) -> R<&mut T> {
    usize::try_from(i).ok().and_then(|i| xs.get_mut(i)).ok_or_else(missing)
}
