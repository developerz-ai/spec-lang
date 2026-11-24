//! Load before the rule, one lock around it, an atomic replace after it
//! (docs/cli-vocabulary.md, `@store`; docs/contract.md section 3).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::lock::{self, Lock, LockError, Mode, Patience};
use super::path::sibling;
use super::{StoreError, Unreadable, atomic};

/// How the store's bytes become a world, or the part of one a rule reads.
/// Generated per program from its states.
pub trait Decode {
    type World: Default;
    fn decode(bytes: &[u8]) -> Result<Self::World, Unreadable>;
}

/// A world that is also written back.
pub trait Codec: Decode {
    fn encode(world: &Self::World) -> Result<Vec<u8>, StoreError>;
}

/// Where the world lives between runs.
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

    /// The world, for a rule that only reads it. Shared lock; the store is never
    /// created or rewritten. The lock covers the read of the bytes and no more:
    /// what was read is a whole committed world, and a rule that cannot write has
    /// nothing left to order.
    pub fn read<C: Decode>(&self) -> Result<C::World, StoreError> {
        match self.bytes(Mode::Shared)? {
            (_, Some(bytes)) => Ok(C::decode(&bytes)?),
            (_, None) => Ok(C::World::default()),
        }
    }

    /// Runs a rule that writes: exclusive lock from before the read until after the write.
    /// A refusal (`Err` from the rule) writes nothing. A world whose canonical
    /// bytes are the bytes already on disk is not written again.
    pub fn update<C: Codec, T, E>(
        &self,
        rule: impl FnOnce(&mut C::World) -> Result<T, E>,
    ) -> Result<Result<T, E>, StoreError> {
        let (_lock, before) = self.bytes(Mode::Exclusive)?;
        let mut world = match &before {
            Some(bytes) => C::decode(bytes)?,
            None => C::World::default(),
        };
        let reply = rule(&mut world);
        if reply.is_ok() {
            let after = C::encode(&world)?;
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
