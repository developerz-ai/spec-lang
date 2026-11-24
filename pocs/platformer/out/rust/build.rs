// Projected by hand from the foreign c "raylib#..." declarations (src/hop/screen.slg) and contract section 3. Stand-in for spec-lang build output; change the spec, not this file.
//! Links the POC's raylib 5.5 at build time, with a run path, so the binary needs no `LD_LIBRARY_PATH`.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let lib = manifest.join("../../vendor/raylib/lib");
    let lib = match lib.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            println!(
                "cargo::error=raylib not found at {} ({error}); run pocs/platformer/bin/raylib",
                lib.display()
            );
            return;
        }
    };
    println!("cargo::rerun-if-changed=build.rs");
    println!(
        "cargo::rerun-if-changed={}",
        lib.join("libraylib.so").display()
    );
    println!("cargo::rustc-link-search=native={}", lib.display());
    println!("cargo::rustc-link-lib=dylib=raylib");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{}", lib.display());
}
