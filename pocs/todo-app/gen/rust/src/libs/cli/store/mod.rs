//! `@store(file(path))`: the world between runs. Load before the rule, one lock around it, an atomic replace
//! after it; a world read is checked like any other (its states' types, their claims, then every law).

pub mod atomic;
pub mod json;
pub mod lock;

use crate::machine::World;
use crate::rt::app::App;
use crate::rt::data::Data;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum StoreError {
    /// The lock was not had within the patience allowed.
    Contended,
    /// The store cannot be read, or what it holds is not a world.
    Unreadable,
    /// The store or its lock cannot be created or written.
    Unwritable,
    /// The world holds an `Int` beyond what the store carries.
    Full,
}

#[derive(Debug)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn at(path: PathBuf) -> Self {
        Store { path }
    }

    /// The path as configured, for messages.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The lock beside the store, created if missing and never deleted.
    pub fn lock(&self, exclusive: bool) -> Result<lock::Lock, StoreError> {
        let mode = if exclusive { lock::Mode::Exclusive } else { lock::Mode::Shared };
        lock::acquire(&atomic::sibling(&self.path, ".lock"), mode, lock::Patience::CONTRACT).map_err(|e| match e {
            lock::LockError::Contended => StoreError::Contended,
            lock::LockError::Unavailable(_) => StoreError::Unwritable,
        })
    }

    /// The store's bytes; `None` when it does not exist, which is the initial world.
    pub fn bytes(&self) -> Result<Option<Vec<u8>>, StoreError> {
        match fs::read(&self.path) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(StoreError::Unreadable),
        }
    }

    pub fn replace(&self, bytes: &[u8]) -> Result<(), StoreError> {
        atomic::replace(&self.path, bytes).map_err(|_| StoreError::Unwritable)
    }
}

/// A world from the store's bytes: exactly `version` (the integer 1) and every state, each of its type.
pub fn load(app: &App, bytes: &[u8]) -> Result<World, StoreError> {
    let doc = json::Document::parse(bytes).ok_or(StoreError::Unreadable)?;
    let mut keys = doc.keys();
    keys.sort_unstable();
    let mut want: Vec<&str> = app.states.iter().map(|s| s.name).chain(["version"]).collect();
    want.sort_unstable();
    if keys != want || doc.get("version", &VERSION) != Some(Data::Int(1)) {
        return Err(StoreError::Unreadable);
    }
    let mut states = Vec::new();
    for s in app.states {
        let d = doc.get(s.name, s.ty).ok_or(StoreError::Unreadable)?;
        if !(s.ty.admits)(&d) {
            return Err(StoreError::Unreadable);
        }
        states.push((s.name, d));
    }
    let world = (app.data_world)(&Data::Record(states)).ok_or(StoreError::Unreadable)?;
    if !(app.lawful)(&world) {
        return Err(StoreError::Unreadable);
    }
    Ok(world)
}

static VERSION: crate::rt::data::Ty =
    crate::rt::data::Ty { name: "version", shape: crate::rt::data::Shape::Int, range: None, admits: |_| true };

/// The canonical bytes of a world: `version`, then the states by name; one line.
pub fn dump(app: &App, world: &World) -> Result<Vec<u8>, StoreError> {
    let data = (app.world_data)(world);
    let mut states: Vec<_> = app.states.iter().collect();
    states.sort_by_key(|s| s.name);
    let mut out = String::from("{\"version\":1");
    for s in states {
        out.push(',');
        json::text(&mut out, s.name);
        out.push(':');
        let d = data.field(s.name).ok_or(StoreError::Unwritable)?;
        json::encode(&mut out, d, s.ty).map_err(|json::Full| StoreError::Full)?;
    }
    out.push_str("}\n");
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::json::{Document, encode};
    use super::{Store, StoreError, atomic, lock};
    use crate::rt::data::{Data, Shape, Ty};
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;

    static INT: Ty = Ty { name: "Int", shape: Shape::Int, range: None, admits: |_| true };
    static TEXT: Ty = Ty { name: "Text", shape: Shape::Text, range: None, admits: |_| true };
    static ROW: Ty =
        Ty { name: "Row", shape: Shape::Record(&[("no", &INT), ("t", &TEXT)]), range: None, admits: |_| true };
    static ROWS: Ty = Ty { name: "Map[Int, Row]", shape: Shape::Map(&INT, &ROW), range: None, admits: |_| true };
    static WORDS: Ty = Ty { name: "Map[Text, Int]", shape: Shape::Map(&TEXT, &INT), range: None, admits: |_| true };

    /// A directory of this test's own, removed when it ends.
    struct Dir(PathBuf);

    impl Dir {
        fn new(test: &str) -> Self {
            let path = std::env::temp_dir().join(format!("spec-lang-cli-store-{test}-{}", std::process::id()));
            assert!(fs::create_dir_all(&path).is_ok());
            Dir(path)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn read(text: &str, ty: &'static Ty) -> Option<Data> {
        Document::parse(format!("{{\"x\":{text}}}").as_bytes())?.get("x", ty)
    }

    #[test]
    fn a_failed_replace_keeps_the_old_bytes_and_no_temporary() {
        let dir = Dir::new("replace");
        // A directory in the store's place: the rename over it fails.
        let path = dir.0.join("store");
        assert!(fs::create_dir(&path).is_ok());
        assert!(atomic::replace(&path, b"new").is_err());
        let left: Vec<_> = fs::read_dir(&dir.0).into_iter().flatten().flatten().map(|e| e.file_name()).collect();
        assert_eq!(left, ["store"]);
    }

    #[test]
    fn a_missing_store_is_the_initial_world_and_an_unreadable_one_is_not() {
        let dir = Dir::new("missing");
        assert!(matches!(Store::at(dir.0.join("none")).bytes(), Ok(None)));
        assert!(matches!(Store::at(dir.0.clone()).bytes(), Err(StoreError::Unreadable)));
    }

    #[test]
    fn a_held_lock_is_contended_until_it_is_let_go() {
        let dir = Dir::new("lock");
        let path = dir.0.join("lock");
        let brief = lock::Patience { every: Duration::from_millis(1), up_to: Duration::from_millis(20) };
        let held = lock::acquire(&path, lock::Mode::Exclusive, brief);
        assert!(held.is_ok());
        assert!(matches!(lock::acquire(&path, lock::Mode::Shared, brief), Err(lock::LockError::Contended)));
        drop(held);
        assert!(lock::acquire(&path, lock::Mode::Shared, brief).is_ok());
    }

    #[test]
    fn integers_are_exact_whole_and_within_2_53() {
        assert_eq!(read("9007199254740992", &INT), Some(Data::Int(9_007_199_254_740_992)));
        assert_eq!(read(" 0 ", &INT), Some(Data::Int(0)));
        for bad in ["9007199254740993", "1.0", "1e0", "01", "+1", "\"1\"", "true"] {
            assert_eq!(read(bad, &INT), None, "{bad}");
        }
        let mut out = String::new();
        assert!(encode(&mut out, &Data::Int((1 << 53) + 1), &INT).is_err());
    }

    #[test]
    fn strings_take_every_escape_and_no_lone_surrogate() {
        assert_eq!(read(r#""A😀\n\/é""#, &TEXT), Some(Data::Text("A😀\n/é".into())));
        for bad in [r#""\ud83d""#, r#""\ude00""#, r#""\x""#, "\"a\nb\"", r#""a"#, r#""a" 1"#] {
            assert_eq!(read(bad, &TEXT), None, "{bad}");
        }
    }

    #[test]
    fn an_object_has_exactly_its_keys_each_once() {
        assert!(read(r#"{"t":"a","no":1}"#, &ROW).is_some());
        for bad in [r#"{"no":1}"#, r#"{"no":1,"t":"a","u":1}"#, r#"{"no":1,"no":1,"t":"a"}"#, r#"{"no":1,"t":"a",}"#] {
            assert_eq!(read(bad, &ROW), None, "{bad}");
        }
    }

    #[test]
    fn a_map_is_its_values_when_they_hold_their_key() {
        let rows = r#"[{"no":1,"t":"a\"b"}]"#;
        let data = read(rows, &ROWS);
        assert!(matches!(&data, Some(Data::Map(m)) if m.len() == 1 && m[0].0 == Data::Int(1)));
        let mut out = String::new();
        assert!(data.is_some_and(|d| encode(&mut out, &d, &ROWS).is_ok()));
        assert_eq!(out, rows);
        assert_eq!(read(r#"[["a",1]]"#, &WORDS), Some(Data::Map(vec![(Data::Text("a".into()), Data::Int(1))])));
    }
}
