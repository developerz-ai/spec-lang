//! The project around the generated code: the runtime prelude, CMake, the scripts, and `clang-format`
//! over every C++ file.

use crate::decls::Unit;
use spec_lang_diag::Diag;
use spec_lang_lir::OutFile;
use std::io::Write;
use std::process::{Command, Stdio};

/// The style every generated file is formatted with; also the project's `.clang-format`.
pub const STYLE: &str = "BasedOnStyle: Google\nColumnLimit: 120\nReflowComments: false\nDerivePointerAlignment: false\nPointerAlignment: Left\nBreakStringLiterals: false\nAllowShortFunctionsOnASingleLine: Empty\nAllowShortLambdasOnASingleLine: Inline\n";

/// The runtime prelude every project carries, verbatim.
pub const RUNTIME: &[(&str, &str)] = &[
    ("include/rt/prelude.hpp", include_str!("../runtime/include/rt/prelude.hpp")),
    ("include/rt/halt.hpp", include_str!("../runtime/include/rt/halt.hpp")),
    ("include/rt/int.hpp", include_str!("../runtime/include/rt/int.hpp")),
    ("include/rt/value.hpp", include_str!("../runtime/include/rt/value.hpp")),
    ("include/rt/text.hpp", include_str!("../runtime/include/rt/text.hpp")),
    ("include/rt/collections.hpp", include_str!("../runtime/include/rt/collections.hpp")),
    ("src/rt/value.cpp", include_str!("../runtime/src/rt/value.cpp")),
    ("src/rt/text.cpp", include_str!("../runtime/src/rt/text.cpp")),
];

/// The test runner's runtime.
pub const TEST_RUNTIME: &[(&str, &str)] = &[
    ("tests/rt/random.hpp", include_str!("../runtime/tests/rt/random.hpp")),
    ("tests/rt/random.cpp", include_str!("../runtime/tests/rt/random.cpp")),
    ("tests/rt/suite.hpp", include_str!("../runtime/tests/rt/suite.hpp")),
    ("tests/rt/suite.cpp", include_str!("../runtime/tests/rt/suite.cpp")),
];

pub const LOWER_TEMPLATE: &str = include_str!("../runtime/src/rt/lower.cpp.in");

pub fn file(path: &str, text: &str) -> OutFile {
    OutFile { path: path.to_string(), bytes: text.as_bytes().to_vec(), executable: false }
}

pub fn script(path: &str, text: &str) -> OutFile {
    OutFile { path: path.to_string(), bytes: text.as_bytes().to_vec(), executable: true }
}

/// A runtime file with the line that says where it comes from.
pub fn runtime_file(path: &str, text: &str) -> OutFile {
    file(
        path,
        &format!(
            "// The C++ printer's runtime, the same in every project `spec-lang build` prints. Do not edit.\n{text}"
        ),
    )
}

/// One file through `clang-format`.
fn format(unit: &Unit) -> Result<String, Diag> {
    let fail = |why: String| {
        Diag::load(format!("clang-format on {}: {why} (the C++ printer formats what it prints)", unit.path))
    };
    let mut child = Command::new("clang-format")
        .arg(format!("--style={{{}}}", STYLE.trim_end().replace('\n', ", ")))
        .arg(format!("--assume-filename={}", unit.path))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| fail(e.to_string()))?;
    let Some(mut stdin) = child.stdin.take() else { return Err(fail("no standard input".into())) };
    // A mark no body settled stands before a name the printer could not follow: it stays quiet.
    let text =
        unit.text.replace('\u{2}', "\u{1}").split('\u{1}').step_by(2).collect::<Vec<_>>().join("[[maybe_unused]] ");
    // Written from another thread: a large file would fill the pipe before the formatter reads it.
    let writer = std::thread::spawn(move || stdin.write_all(text.as_bytes()));
    let out = child.wait_with_output().map_err(|e| fail(e.to_string()))?;
    match writer.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(fail(e.to_string())),
        Err(_) => return Err(fail("the writer stopped".into())),
    }
    if !out.status.success() {
        return Err(fail(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    String::from_utf8(out.stdout).map_err(|e| fail(e.to_string()))
}

/// Every unit formatted, in order.
pub fn formatted(units: Vec<Unit>) -> Result<Vec<OutFile>, Vec<Diag>> {
    let results: Vec<Result<String, Diag>> = std::thread::scope(|s| {
        let handles: Vec<_> = units.iter().map(|u| s.spawn(move || format(u))).collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err(Diag::load("clang-format: a formatter thread stopped"))))
            .collect()
    });
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for (u, r) in units.iter().zip(results) {
        match r {
            Ok(text) => files.push(file(&u.path, &text)),
            Err(d) => errors.push(d),
        }
    }
    if errors.is_empty() { Ok(files) } else { Err(errors) }
}

/// The sources of a list of files that CMake compiles, one per line.
pub fn sources(files: &[OutFile], under: &str) -> String {
    let mut paths: Vec<&str> =
        files.iter().map(|f| f.path.as_str()).filter(|p| p.starts_with(under) && p.ends_with(".cpp")).collect();
    paths.sort_unstable();
    paths.iter().map(|p| format!("  {p}\n")).collect()
}

pub const CMAKE_HEAD: &str = "cmake_minimum_required(VERSION 3.28)
project(@NAME@ LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

# A spec may compare a value with itself on purpose: that is not a mistake of the generated code.
add_compile_options(-Wall -Wextra -Wpedantic -Werror -Wno-tautological-compare)
if(CMAKE_CXX_COMPILER_ID STREQUAL GNU)
  # GCC reports the standard containers' own moves as maybe-uninitialized when it inlines them.
  add_compile_options(-Wno-maybe-uninitialized)
endif()
";
