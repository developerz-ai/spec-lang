// Projected by hand from test three_open_one_done (src/todo/tests/scenarios.slg) and the generators behind law and property parameters (src/todo/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use todo::board::{Command, World};
use todo::model::{Priority, Status, Tag, Task, TaskNo, Title};

pub fn no(value: u64) -> TaskNo {
    TaskNo::new(value).expect("a TaskNo is at least 1")
}

pub fn title(text: &str) -> Title {
    Title::new(text).expect("a Title")
}

pub fn tag(text: &str) -> Tag {
    Tag::new(text).expect("a Tag")
}

pub fn tags(texts: &[&str]) -> BTreeSet<Tag> {
    texts.iter().map(|text| tag(text)).collect()
}

/// `Task(no, title: title)` with the named fields changed.
pub fn task(number: u64, text: &str, status: Status, priority: Priority, tagged: &[&str]) -> Task {
    Task {
        status,
        priority,
        tags: tags(tagged),
        ..Task::new(no(number), title(text))
    }
}

/// The `@given` of `three_open_one_done`.
pub fn three_open_one_done() -> World {
    World::restore(
        no(5),
        [
            task(1, "low one", Status::Open, Priority::Low, &[]),
            task(2, "plain", Status::Open, Priority::Normal, &[]),
            task(3, "urgent", Status::Open, Priority::High, &["work"]),
            task(
                4,
                "finished",
                Status::Done,
                Priority::Normal,
                &["work", "home"],
            ),
        ],
    )
    .expect("the given world keeps the board's invariants")
}

/// SplitMix64: small, seeded, the same sequence on every machine.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound`.
    pub fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

const TITLES: [&str; 8] = [
    "milk",
    "  spaced  ",
    "caf\u{e9} \u{2615}",
    "say \"hi\"",
    "back\\slash",
    "\u{1F600}",
    "a\u{2027}b\u{84}c",
    "--all",
];
const TAGS: [&str; 5] = ["home", "work", "deep-work", "a", "42"];

pub fn random_title(rng: &mut Rng) -> Title {
    title(rng.pick(&TITLES))
}

pub fn random_tags(rng: &mut Rng) -> BTreeSet<Tag> {
    (0..rng.below(4)).map(|_| tag(rng.pick(&TAGS))).collect()
}

/// A number that is usually on the board, sometimes long gone or not yet given, now and then far away.
pub fn random_no(rng: &mut Rng, world: &World) -> TaskNo {
    match rng.below(20) {
        0 => TaskNo::BEYOND,
        1 => no(rng.next().max(1)),
        _ => no(1 + rng.below(world.next_no().get() + 1)),
    }
}

/// Any command of the board, reads included.
pub fn random_command(rng: &mut Rng, world: &World) -> Command {
    match rng.below(20) {
        0..=5 => Command::Add {
            title: random_title(rng),
            priority: *rng.pick(&Priority::ALL),
            tags: random_tags(rng),
        },
        6..=8 => Command::Done {
            no: random_no(rng, world),
        },
        9..=10 => Command::Reopen {
            no: random_no(rng, world),
        },
        11..=12 => Command::Rename {
            no: random_no(rng, world),
            title: random_title(rng),
        },
        13..=14 => Command::Remove {
            no: random_no(rng, world),
        },
        15 => Command::Clear,
        16..=17 => Command::List {
            all: rng.below(2) == 0,
            tag: (rng.below(2) == 0).then(|| tag(rng.pick(&TAGS))),
        },
        _ => Command::Stats,
    }
}

/// A directory of its own under the build's temporary directory, removed when dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "todo-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        fs::create_dir_all(&path).expect("a temporary directory");
        Self(path)
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
