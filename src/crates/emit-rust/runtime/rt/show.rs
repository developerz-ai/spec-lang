//! `show`: a value as spec-lang source, which `"{x}"` prints for anything but `Text` (Text prints raw).

use super::{Failure, Fun, Int, Never, Opt, Range, text};
use std::collections::{BTreeMap, BTreeSet};

pub trait Show {
    fn show(&self) -> String;
}

/// `open item, item close`.
pub fn seq(open: &str, items: impl Iterator<Item = String>, close: &str) -> String {
    format!("{open}{}{close}", items.collect::<Vec<_>>().join(", "))
}

/// A record or a variant: `Name(first, name: value, ...)`; a variant without fields is its name.
pub fn record(name: &str, fields: &[(&str, String)]) -> String {
    if fields.is_empty() {
        return name.to_string();
    }
    let items = fields.iter().enumerate().map(|(i, (n, v))| if i == 0 { v.clone() } else { format!("{n}: {v}") });
    seq(&format!("{name}("), items, ")")
}

impl Show for () {
    fn show(&self) -> String {
        "()".into()
    }
}

impl Show for bool {
    fn show(&self) -> String {
        self.to_string()
    }
}

impl Show for Int {
    fn show(&self) -> String {
        self.to_string()
    }
}

impl Show for String {
    fn show(&self) -> String {
        text::quote(self)
    }
}

impl Show for Never {
    fn show(&self) -> String {
        match *self {}
    }
}

impl<T: Show + ?Sized> Show for Box<T> {
    fn show(&self) -> String {
        (**self).show()
    }
}

impl<T: Show> Show for Vec<T> {
    fn show(&self) -> String {
        seq("[", self.iter().map(Show::show), "]")
    }
}

impl<T: Show> Show for BTreeSet<T> {
    fn show(&self) -> String {
        seq("Set(", self.iter().map(Show::show), ")")
    }
}

impl<K: Show, V: Show> Show for BTreeMap<K, V> {
    fn show(&self) -> String {
        seq("Map(", self.iter().map(|(k, v)| format!("{} -> {}", k.show(), v.show())), ")")
    }
}

impl<T: Show> Show for Opt<T> {
    fn show(&self) -> String {
        match self {
            Opt::Some(v) => format!("Some({})", v.show()),
            Opt::None => "none".into(),
        }
    }
}

impl Show for Range {
    fn show(&self) -> String {
        format!("{}{}{}", self.lo, if self.inclusive { "..=" } else { "..<" }, self.hi)
    }
}

impl<A, B> Show for Fun<A, B> {
    fn show(&self) -> String {
        "<function>".into()
    }
}

impl Show for Failure {
    fn show(&self) -> String {
        if self.data.is_empty() {
            format!("fail {}", self.name)
        } else {
            seq(&format!("fail {}(", self.name), self.data.iter().cloned(), ")")
        }
    }
}

macro_rules! tuple {
    ($($t:ident $i:tt),+) => {
        impl<$($t: Show),+> Show for ($($t,)+) {
            fn show(&self) -> String {
                seq("(", [$(self.$i.show()),+].into_iter(), ")")
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
