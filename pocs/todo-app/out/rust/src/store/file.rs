// Projected by hand from note @store: load before the rule, one lock around it, an atomic replace after it (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::codec::{self, TaskView, Unreadable};
use super::lock::{self, Lock, LockError, Mode, Patience};
use super::path::sibling;
use super::{StoreError, atomic};
use crate::board::World;

/// Where the board's state lives between runs.
#[derive(Clone, Debug)]
pub struct Store {
    path: PathBuf,
    patience: Patience,
}

impl Store {
    pub fn at(path: PathBuf) -> Self {
        Self::with_patience(path, Patience::CONTRACT)
    }

    pub fn with_patience(path: PathBuf, patience: Patience) -> Self {
        Self { path, patience }
    }

    /// The path as configured, for messages.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The world, for a rule that only reads it. Shared lock; the store is never created or rewritten.
    /// The lock covers the read of the bytes and no more: what was read is a
    /// whole committed world, and a rule that cannot write has nothing left to order.
    pub fn read(&self) -> Result<World, StoreError> {
        match self.bytes(Mode::Shared)? {
            (_, Some(bytes)) => Ok(codec::decode(&bytes)?),
            (_, None) => Ok(World::default()),
        }
    }

    /// A fold over the stored tasks for a rule whose read set is narrower than
    /// the world. The store is checked in full exactly as [`Store::read`] checks
    /// it, but no task is built.
    pub fn fold<T>(
        &self,
        mut state: T,
        mut step: impl FnMut(&mut T, TaskView<'_>),
    ) -> Result<T, StoreError> {
        if let (_, Some(bytes)) = self.bytes(Mode::Shared)? {
            codec::scan(&bytes, |task| {
                step(&mut state, task);
                Ok(())
            })?;
        }
        Ok(state)
    }

    /// Runs a rule that writes: exclusive lock from before the read until after the write.
    /// A refusal (`Err` from the rule) writes nothing. A world whose canonical
    /// bytes are the bytes already on disk is not written again.
    pub fn update<T, E>(
        &self,
        rule: impl FnOnce(&mut World) -> Result<T, E>,
    ) -> Result<Result<T, E>, StoreError> {
        let (_lock, before) = self.bytes(Mode::Exclusive)?;
        let mut world = match &before {
            Some(bytes) => codec::decode(bytes)?,
            None => World::default(),
        };
        let reply = rule(&mut world);
        if reply.is_ok() {
            let after = codec::encode(&world);
            if before.as_deref() != Some(after.as_slice()) {
                atomic::replace(&self.path, &after).map_err(StoreError::Unwritable)?;
            }
        }
        Ok(reply)
    }

    /// Takes the lock, then reads the store's bytes; `None` is a missing store, which is the initial world.
    fn bytes(&self, mode: Mode) -> Result<(Lock, Option<Vec<u8>>), StoreError> {
        let lock =
            lock::acquire(&sibling(&self.path, ".lock"), mode, self.patience).map_err(|why| {
                match why {
                    LockError::Contended => StoreError::Contended,
                    LockError::Unavailable(why) => StoreError::Unwritable(why),
                }
            })?;
        match fs::read(&self.path) {
            Ok(bytes) => Ok((lock, Some(bytes))),
            Err(why) if why.kind() == io::ErrorKind::NotFound => Ok((lock, None)),
            Err(why) => Err(StoreError::Unreadable(Unreadable::Io(why))),
        }
    }
}
