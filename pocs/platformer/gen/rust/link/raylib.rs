//! Links raylib 5.5 when the app is built, with a run path, so the binary needs no `LD_LIBRARY_PATH`: the directory
//! `RAYLIB_LIB` names, else the first `vendor/raylib/lib` in the project's directory or one above it.

use std::env;
use std::path::PathBuf;

pub fn link() {
    println!("cargo::rerun-if-env-changed=RAYLIB_LIB");
    let project = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let named = env::var_os("RAYLIB_LIB").filter(|v| !v.is_empty()).map(PathBuf::from);
    let found = named.or_else(|| project.ancestors().map(|d| d.join("vendor/raylib/lib")).find(|d| d.is_dir()));
    let Some(lib) = found.and_then(|d| d.canonicalize().ok()) else {
        println!(
            "cargo::error=raylib 5.5 not found: set RAYLIB_LIB to the directory of libraylib.so, or build it into vendor/raylib/lib at or above {}",
            project.display()
        );
        return;
    };
    println!("cargo::rerun-if-changed={}", lib.join("libraylib.so").display());
    println!("cargo::rustc-link-search=native={}", lib.display());
    println!("cargo::rustc-link-lib=dylib=raylib");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{}", lib.display());
}
