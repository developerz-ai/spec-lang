//! A value as plain data, and a declared type as plain data: what a library's runtime reads and writes without
//! knowing the program's Rust types (arguments from a command line, a world in a store, a foreign call's
//! arguments). The printer gives every program type an `AsData` and every type an app's libraries meet a `Ty`.

use super::{Fun, Int, Never, Opt, Range};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Data {
    Unit,
    Bool(bool),
    Int(Int),
    Text(String),
    List(Vec<Data>),
    /// Ascending, no item twice.
    Set(Vec<Data>),
    /// Ascending keys, no key twice.
    Map(Vec<(Data, Data)>),
    Tuple(Vec<Data>),
    /// Fields in declared order.
    Record(Vec<(&'static str, Data)>),
    /// A variant by its declared name, its fields in declared order.
    Variant(&'static str, Vec<(&'static str, Data)>),
    Some(Box<Data>),
    None,
    Range(Range),
    /// A value with no data form (a function).
    Opaque,
}

impl Data {
    /// A record's or variant's field by name.
    pub fn field(&self, name: &str) -> Option<&Data> {
        match self {
            Data::Record(fs) | Data::Variant(_, fs) => fs.iter().find(|(n, _)| *n == name).map(|(_, d)| d),
            _ => None,
        }
    }
}

/// A value to and from `Data`.
pub trait AsData: Sized {
    fn to_data(&self) -> Data;
    fn from_data(d: &Data) -> Option<Self>;
}

impl AsData for () {
    fn to_data(&self) -> Data {
        Data::Unit
    }
    fn from_data(d: &Data) -> Option<Self> {
        matches!(d, Data::Unit).then_some(())
    }
}

impl AsData for bool {
    fn to_data(&self) -> Data {
        Data::Bool(*self)
    }
    fn from_data(d: &Data) -> Option<Self> {
        if let Data::Bool(b) = d { Some(*b) } else { None }
    }
}

impl AsData for Int {
    fn to_data(&self) -> Data {
        Data::Int(*self)
    }
    fn from_data(d: &Data) -> Option<Self> {
        if let Data::Int(n) = d { Some(*n) } else { None }
    }
}

impl AsData for String {
    fn to_data(&self) -> Data {
        Data::Text(self.clone())
    }
    fn from_data(d: &Data) -> Option<Self> {
        if let Data::Text(t) = d { Some(t.clone()) } else { None }
    }
}

impl AsData for Range {
    fn to_data(&self) -> Data {
        Data::Range(*self)
    }
    fn from_data(d: &Data) -> Option<Self> {
        if let Data::Range(r) = d { Some(*r) } else { None }
    }
}

impl AsData for Never {
    fn to_data(&self) -> Data {
        match *self {}
    }
    fn from_data(_: &Data) -> Option<Self> {
        None
    }
}

impl<A, B> AsData for Fun<A, B> {
    fn to_data(&self) -> Data {
        Data::Opaque
    }
    fn from_data(_: &Data) -> Option<Self> {
        None
    }
}

impl<T: AsData> AsData for Box<T> {
    fn to_data(&self) -> Data {
        (**self).to_data()
    }
    fn from_data(d: &Data) -> Option<Self> {
        T::from_data(d).map(Box::new)
    }
}

impl<T: AsData> AsData for Opt<T> {
    fn to_data(&self) -> Data {
        match self {
            Opt::Some(v) => Data::Some(Box::new(v.to_data())),
            Opt::None => Data::None,
        }
    }
    fn from_data(d: &Data) -> Option<Self> {
        match d {
            Data::Some(v) => T::from_data(v).map(Opt::Some),
            Data::None => Some(Opt::None),
            _ => None,
        }
    }
}

impl<T: AsData> AsData for Vec<T> {
    fn to_data(&self) -> Data {
        Data::List(self.iter().map(AsData::to_data).collect())
    }
    fn from_data(d: &Data) -> Option<Self> {
        if let Data::List(xs) = d { xs.iter().map(T::from_data).collect() } else { None }
    }
}

impl<T: AsData + Ord> AsData for BTreeSet<T> {
    fn to_data(&self) -> Data {
        Data::Set(self.iter().map(AsData::to_data).collect())
    }
    fn from_data(d: &Data) -> Option<Self> {
        let Data::Set(xs) = d else { return None };
        let set: BTreeSet<T> = xs.iter().map(T::from_data).collect::<Option<_>>()?;
        (set.len() == xs.len()).then_some(set)
    }
}

impl<K: AsData + Ord, V: AsData> AsData for BTreeMap<K, V> {
    fn to_data(&self) -> Data {
        Data::Map(self.iter().map(|(k, v)| (k.to_data(), v.to_data())).collect())
    }
    fn from_data(d: &Data) -> Option<Self> {
        let Data::Map(xs) = d else { return None };
        let map: BTreeMap<K, V> =
            xs.iter().map(|(k, v)| Some((K::from_data(k)?, V::from_data(v)?))).collect::<Option<_>>()?;
        (map.len() == xs.len()).then_some(map)
    }
}

macro_rules! tuple {
    ($($t:ident $i:tt),+) => {
        impl<$($t: AsData),+> AsData for ($($t,)+) {
            fn to_data(&self) -> Data {
                Data::Tuple(vec![$(self.$i.to_data()),+])
            }
            fn from_data(d: &Data) -> Option<Self> {
                let Data::Tuple(xs) = d else { return None };
                Some(($($t::from_data(xs.get($i)?)?,)+))
            }
        }
    };
}

tuple!(A 0);
tuple!(A 0, B 1);
tuple!(A 0, B 1, C 2);
tuple!(A 0, B 1, C 2, D 3);
tuple!(A 0, B 1, C 2, D 3, E 4);
tuple!(A 0, B 1, C 2, D 3, E 4, F 5);

/// A declared type as data: its name as the program writes it, its shape, and whether a value is of it
/// (ranges and `where` claims included).
#[derive(Debug)]
pub struct Ty {
    pub name: &'static str,
    pub shape: Shape,
    pub admits: fn(&Data) -> bool,
}

#[derive(Debug)]
pub enum Shape {
    Unit,
    Bool,
    Int,
    Text,
    Option(&'static Ty),
    List(&'static Ty),
    Set(&'static Ty),
    Map(&'static Ty, &'static Ty),
    Tuple(&'static [&'static Ty]),
    Range,
    /// Fields: name and type.
    Record(&'static [(&'static str, &'static Ty)]),
    /// Variants: name and fields.
    Sum(&'static [(&'static str, &'static [(&'static str, &'static Ty)])]),
    /// No data form: functions, events, traces.
    Opaque,
}

/// Whether a value of `T` decodes from `d` and passes `check`.
pub fn admits<T: AsData>(d: &Data, check: impl FnOnce(&T) -> bool) -> bool {
    T::from_data(d).is_some_and(|v| check(&v))
}
