//! The runtime shims: what the spec does not determine per program (the store's
//! file handling and lock, the JSON lexer, the cli parser engine, the process edge,
//! Unicode text), lifted from the hand projection `pocs/todo-app/out/cpp` and
//! emitted verbatim (docs/idea/11-backends-and-parity.md: "backend authors write
//! printers and runtime shims").

/// `(path in the project, shim name, text)`.
pub const RUNTIME: &[(&str, &str, &str)] = &[
    ("include/rt/cli.hpp", "rt.cli", include_str!("../runtime/include/rt/cli.hpp")),
    ("include/rt/core.hpp", "rt.core", include_str!("../runtime/include/rt/core.hpp")),
    ("include/rt/driver.hpp", "rt.driver", include_str!("../runtime/include/rt/driver.hpp")),
    ("include/rt/exit_code.hpp", "rt.exit_code", include_str!("../runtime/include/rt/exit_code.hpp")),
    ("include/rt/fd.hpp", "rt.fd", include_str!("../runtime/include/rt/fd.hpp")),
    ("include/rt/file.hpp", "rt.file", include_str!("../runtime/include/rt/file.hpp")),
    ("include/rt/flat_map.hpp", "rt.flat_map", include_str!("../runtime/include/rt/flat_map.hpp")),
    ("include/rt/json_lexer.hpp", "rt.json_lexer", include_str!("../runtime/include/rt/json_lexer.hpp")),
    ("include/rt/json_reader.hpp", "rt.json_reader", include_str!("../runtime/include/rt/json_reader.hpp")),
    ("include/rt/json_writer.hpp", "rt.json_writer", include_str!("../runtime/include/rt/json_writer.hpp")),
    ("include/rt/lock.hpp", "rt.lock", include_str!("../runtime/include/rt/lock.hpp")),
    ("include/rt/main.hpp", "rt.main", include_str!("../runtime/include/rt/main.hpp")),
    ("include/rt/store_error.hpp", "rt.store_error", include_str!("../runtime/include/rt/store_error.hpp")),
    ("include/rt/text.hpp", "rt.text", include_str!("../runtime/include/rt/text.hpp")),
    ("include/rt/unicode.hpp", "rt.unicode", include_str!("../runtime/include/rt/unicode.hpp")),
    ("src/rt/file.cpp", "rt.file", include_str!("../runtime/src/rt/file.cpp")),
    ("src/rt/json_lexer.cpp", "rt.json_lexer", include_str!("../runtime/src/rt/json_lexer.cpp")),
    ("src/rt/lock.cpp", "rt.lock", include_str!("../runtime/src/rt/lock.cpp")),
    ("src/rt/text.cpp", "rt.text", include_str!("../runtime/src/rt/text.cpp")),
    ("src/rt/unicode.cpp", "rt.unicode", include_str!("../runtime/src/rt/unicode.cpp")),
    ("tests/rt/harness.cpp", "rt.harness", include_str!("../runtime/tests/rt/harness.cpp")),
    ("tests/rt/harness.hpp", "rt.harness", include_str!("../runtime/tests/rt/harness.hpp")),
    ("tests/rt/random.hpp", "rt.random", include_str!("../runtime/tests/rt/random.hpp")),
    (
        "tests/rt/store_hygiene_test.cpp",
        "rt.store_hygiene_test",
        include_str!("../runtime/tests/rt/store_hygiene_test.cpp"),
    ),
    ("tests/rt/trace.hpp", "rt.trace", include_str!("../runtime/tests/rt/trace.hpp")),
];
