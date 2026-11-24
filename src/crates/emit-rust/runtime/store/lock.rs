//! One `flock(2)` around a rule (docs/contract.md section 3, Lock).

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

/// Shared for rules that only read the state, exclusive for rules that write it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Shared,
    Exclusive,
}

/// How long to keep trying, and how often.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Patience {
    pub every: Duration,
    pub up_to: Duration,
}

impl Patience {
    /// Every 10 ms for up to 5 s.
    pub const CONTRACT: Self = Self {
        every: Duration::from_millis(10),
        up_to: Duration::from_secs(5),
    };
}

#[derive(Debug)]
pub enum LockError {
    /// Someone else held it for the whole of the patience.
    Contended,
    /// The lock file cannot be created, opened or locked.
    Unavailable(io::Error),
}

/// A held `flock(2)`. Dropping it closes the file, which releases the lock.
#[derive(Debug)]
pub struct Lock {
    _file: File,
}

/// Creates the lock file if missing (it is never deleted) and takes the lock without ever blocking in the kernel.
pub fn acquire(path: &Path, mode: Mode, patience: Patience) -> Result<Lock, LockError> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(LockError::Unavailable)?;
    let deadline = Instant::now() + patience.up_to;
    loop {
        let attempt = match mode {
            Mode::Shared => file.try_lock_shared(),
            Mode::Exclusive => file.try_lock(),
        };
        match attempt {
            Ok(()) => return Ok(Lock { _file: file }),
            Err(TryLockError::Error(why)) => return Err(LockError::Unavailable(why)),
            Err(TryLockError::WouldBlock) if Instant::now() >= deadline => {
                return Err(LockError::Contended);
            }
            Err(TryLockError::WouldBlock) => thread::sleep(patience.every),
        }
    }
}
