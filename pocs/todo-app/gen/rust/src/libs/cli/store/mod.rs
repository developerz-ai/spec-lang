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
    crate::rt::data::Ty { name: "version", shape: crate::rt::data::Shape::Int, admits: |_| true };

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
