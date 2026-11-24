//! The core library as the rust backend carries it: `Text.size`, `Text.trim`
//! (docs/contract.md section 1) and `Int` from a length.

/// Unicode `White_Space`: exactly the 25 code points the contract lists,
/// written out so the set cannot move with the toolchain's Unicode tables.
pub const fn is_white_space(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

/// `Text.trim`: both ends, `White_Space` only.
pub fn trim(text: &str) -> &str {
    text.trim_matches(is_white_space)
}

/// `Text.size`: code points, not bytes and not UTF-16 units.
pub fn size(text: &str) -> i64 {
    if text.is_ascii() {
        int(text.len())
    } else {
        int(text.chars().count())
    }
}

/// A length as an `Int`. No collection in memory is longer than `i64::MAX`.
pub fn int(length: usize) -> i64 {
    i64::try_from(length).unwrap_or(i64::MAX)
}

/// A map read by a rule that only reads it: entries in key order, each key
/// once. What a projected store read builds instead of a `BTreeMap`, with the
/// same reading methods, so the rule's body is the same text over either.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entries<K, V> {
    entries: Vec<(K, V)>,
}

impl<K: Ord, V> Entries<K, V> {
    /// `None` when a key appears twice. Entries already in key order (the
    /// canonical store) cost one comparison each; others are sorted once.
    pub fn new(mut entries: Vec<(K, V)>) -> Option<Self> {
        if !entries.windows(2).all(|pair| pair[0].0 < pair[1].0) {
            entries.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            if entries.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return None;
            }
        }
        Some(Self { entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.binary_search_by(|(k, _)| k.cmp(key)).is_ok()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let at = self.entries.binary_search_by(|(k, _)| k.cmp(key)).ok()?;
        self.entries.get(at).map(|(_, v)| v)
    }
}

impl<K: Ord + Clone, V> Entries<K, V> {
    /// The entries of an ordered map, keys already unique.
    pub fn of<'a, W: 'a>(map: impl IntoIterator<Item = (&'a K, &'a W)>, mut value: impl FnMut(&W) -> V) -> Self
    where
        K: 'a,
    {
        Self { entries: map.into_iter().map(|(k, w)| (k.clone(), value(w))).collect() }
    }
}

/// `m[k]` for a key the caller knows is there (it came from `m.keys`); panics
/// otherwise, as `BTreeMap` indexing does.
impl<K: Ord, V> std::ops::Index<&K> for Entries<K, V> {
    type Output = V;

    fn index(&self, key: &K) -> &V {
        let at = self.entries.binary_search_by(|(k, _)| k.cmp(key)).unwrap_or(self.entries.len());
        &self.entries[at].1
    }
}
