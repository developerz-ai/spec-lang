// Projected by hand from note @store: load, one lock, an atomic replace; a missing store; TODO_STORE (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.
// No mocks: every test has a directory of its own.

mod support;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::Duration;

use support::{TempDir, no, three_open_one_done, title};
use todo::board::{self, Out, Refusal, Tally, World};
use todo::model::Priority;
use todo::store::codec::{Unreadable, encode};
use todo::store::lock::{self, Mode, Patience};
use todo::store::{Store, StoreError, path};

const SHORT: Patience = Patience {
    every: Duration::from_millis(5),
    up_to: Duration::from_millis(40),
};

fn names(dir: &TempDir) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir.join(""))
        .expect("the directory")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn add(store: &Store, text: &str) -> Result<Result<todo::model::TaskNo, board::Full>, StoreError> {
    store.update(|world| {
        board::add(
            world,
            &mut Out::default(),
            title(text),
            Priority::Normal,
            Default::default(),
        )
    })
}

#[test]
fn the_configured_path() {
    assert_eq!(path::configured(None), PathBuf::from(".todo.json"));
    assert_eq!(
        path::configured(Some(OsString::new())),
        PathBuf::from(".todo.json")
    );
    assert_eq!(
        path::configured(Some("lists/home.json".into())),
        PathBuf::from("lists/home.json")
    );
}

#[test]
fn a_missing_store_is_the_initial_world_and_reads_never_create_it() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("s.json"));
    assert_eq!(store.read().expect("readable"), World::default());
    assert_eq!(store.fold(0, |count, _| *count += 1).expect("readable"), 0);
    assert_eq!(names(&dir), ["s.json.lock"]);
}

#[test]
fn a_write_replaces_the_store_with_the_canonical_bytes_and_leaves_no_temporary() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("s.json"));
    assert_eq!(add(&store, "milk").expect("stored"), Ok(no(1)));
    assert_eq!(add(&store, "bread").expect("stored"), Ok(no(2)));
    assert_eq!(names(&dir), ["s.json", "s.json.lock"]);
    let world = store.read().expect("readable");
    assert_eq!(
        fs::read(dir.join("s.json")).expect("the store"),
        encode(&world)
    );
    assert_eq!(world.next_no(), no(3));
}

#[test]
fn a_refusal_writes_nothing() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("s.json"));
    let refused = store.update(|world| board::done(world, &mut Out::default(), no(1)));
    assert_eq!(refused.expect("no store error"), Err(Refusal::NoTask));
    assert_eq!(names(&dir), ["s.json.lock"]);
}

#[test]
fn a_world_already_on_disk_byte_for_byte_is_not_written_again() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("s.json"));
    add(&store, "milk").expect("stored").expect("added");
    let before = fs::metadata(dir.join("s.json")).expect("the store").ino();
    let cleared = store.update(|world| Ok::<_, Refusal>(board::clear(world, &mut Out::default())));
    assert_eq!(cleared.expect("no store error"), Ok(0));
    assert_eq!(
        fs::metadata(dir.join("s.json")).expect("the store").ino(),
        before
    );
}

#[test]
fn a_write_makes_a_loose_store_canonical() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("s.json"));
    fs::write(
        dir.join("s.json"),
        " { \"tasks\":[], \"next_no\":7, \"version\":1 } ",
    )
    .expect("seeded");
    let cleared = store.update(|world| Ok::<_, Refusal>(board::clear(world, &mut Out::default())));
    assert_eq!(cleared.expect("no store error"), Ok(0));
    assert_eq!(
        fs::read(dir.join("s.json")).expect("the store"),
        b"{\"version\":1,\"next_no\":7,\"tasks\":[]}\n"
    );
}

#[test]
fn an_unreadable_store_is_refused_and_left_alone() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("s.json"));
    fs::write(dir.join("s.json"), "{}\n").expect("seeded");
    assert!(matches!(
        store.read(),
        Err(StoreError::Unreadable(Unreadable::MissingKey))
    ));
    assert!(matches!(
        store.fold((), |_, _| ()),
        Err(StoreError::Unreadable(Unreadable::MissingKey))
    ));
    assert!(matches!(
        add(&store, "x"),
        Err(StoreError::Unreadable(Unreadable::MissingKey))
    ));
    assert_eq!(fs::read(dir.join("s.json")).expect("the store"), b"{}\n");
}

#[test]
fn a_store_that_cannot_be_read_as_a_file_is_unreadable() {
    let dir = TempDir::new();
    fs::create_dir(dir.join("s.json")).expect("a directory in the way");
    let store = Store::at(dir.join("s.json"));
    assert!(matches!(
        store.read(),
        Err(StoreError::Unreadable(Unreadable::Io(_)))
    ));
    assert!(matches!(
        add(&store, "x"),
        Err(StoreError::Unreadable(Unreadable::Io(_)))
    ));
}

#[test]
fn a_store_whose_directory_is_missing_is_unwritable_even_to_read() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("lists/home.json"));
    assert!(matches!(store.read(), Err(StoreError::Unwritable(_))));
    assert!(matches!(add(&store, "x"), Err(StoreError::Unwritable(_))));
    assert!(names(&dir).is_empty());
}

#[test]
fn a_writer_waits_for_any_lock_and_a_reader_only_for_a_writer() {
    let dir = TempDir::new();
    let store = Store::with_patience(dir.join("s.json"), SHORT);
    add(&store, "milk").expect("stored").expect("added");
    {
        let _shared =
            lock::acquire(&dir.join("s.json.lock"), Mode::Shared, SHORT).expect("the lock");
        assert!(store.read().is_ok());
        assert!(matches!(add(&store, "x"), Err(StoreError::Contended)));
    }
    {
        let _exclusive =
            lock::acquire(&dir.join("s.json.lock"), Mode::Exclusive, SHORT).expect("the lock");
        assert!(matches!(store.read(), Err(StoreError::Contended)));
        assert!(matches!(
            store.fold((), |_, _| ()),
            Err(StoreError::Contended)
        ));
        assert!(matches!(add(&store, "x"), Err(StoreError::Contended)));
    }
    assert_eq!(
        add(&store, "bread").expect("the lock is free again"),
        Ok(no(2))
    );
}

#[test]
fn the_fold_and_the_world_agree_on_disk() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("s.json"));
    let given = three_open_one_done();
    store
        .update(|world| {
            *world = given.clone();
            Ok::<_, Refusal>(())
        })
        .expect("stored")
        .expect("not refused");
    assert_eq!(store.read().expect("readable"), given);
    let tally = store
        .fold(Tally::default(), |tally, task| tally.count(task.status))
        .expect("readable");
    let (mut folded, mut by_rule) = (Out::default(), Out::default());
    tally.report(&mut folded);
    board::stats(&given, &mut by_rule);
    assert_eq!(folded.text(), "3 open, 1 done\n");
    assert_eq!(folded, by_rule);
}

#[test]
fn concurrent_writers_lose_no_number() {
    let dir = TempDir::new();
    let path = dir.join("s.json");
    let workers: Vec<_> = (0..8)
        .map(|worker| {
            let store = Store::at(path.clone());
            std::thread::spawn(move || {
                (0..10)
                    .map(|i| {
                        add(&store, &format!("job {worker}.{i}"))
                            .expect("stored")
                            .expect("added")
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut numbers: Vec<u64> = workers
        .into_iter()
        .flat_map(|worker| worker.join().expect("the worker"))
        .map(|no| no.get())
        .collect();
    numbers.sort_unstable();
    assert_eq!(numbers, (1..=80).collect::<Vec<u64>>());
    assert_eq!(Store::at(path).read().expect("readable").tasks().len(), 80);
}
