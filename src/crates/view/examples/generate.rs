//! Write every view of a project.
//!
//! ```text
//! cargo run -p view --example generate -- <src dir> <out dir> [--title <name>]
//!     [--regenerate <command printed in the index>]
//!     [--provenance <backend>=<provenance.json>=<href of the generated project from site/>]...
//! ```
//!
//! Exit codes: 0 written, 1 the project does not load or a file cannot be written, 2 usage.

use std::path::Path;
use std::process::ExitCode;
use view::provenance::Provenance;

const USAGE: &str = "usage: generate <src dir> <out dir> [--title <name>] [--regenerate <command>] [--provenance <backend>=<file>=<href>]...";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(src), Some(out)) = (args.first(), args.get(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let mut title = "program".to_string();
    let mut regenerate = "cargo run -p view --example generate -- <src dir> <this folder>".to_string();
    let mut provenance = Vec::new();
    let mut rest = args.iter().skip(2);
    while let Some(flag) = rest.next() {
        let Some(value) = rest.next() else {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        };
        match flag.as_str() {
            "--title" => title = value.clone(),
            "--regenerate" => regenerate = value.clone(),
            "--provenance" => {
                let parts: Vec<&str> = value.splitn(3, '=').collect();
                let [backend, file, href] = parts.as_slice() else {
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                };
                let parsed = std::fs::read_to_string(file)
                    .map_err(|e| e.to_string())
                    .and_then(|text| Provenance::parse(&text, backend, href));
                match parsed {
                    Ok(p) => provenance.push(p),
                    Err(e) => {
                        eprintln!("{file}: {e}");
                        return ExitCode::from(1);
                    }
                }
            }
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let program = match spec_lang_ir::load_project(Path::new(src)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e.render());
            return ExitCode::from(1);
        }
    };
    let inputs = view::Inputs { provenance: &provenance, flows: &[] };
    let files = view::bundle(&program, &title, &regenerate, inputs);
    // The explorer is wholly generated: a page of a declaration that is gone must not stay.
    match std::fs::remove_dir_all(Path::new(out).join("site")) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("{out}/site: {e}");
            return ExitCode::from(1);
        }
    }
    if let Err(e) = view::write_files(Path::new(out), &files) {
        eprintln!("{out}: {e}");
        return ExitCode::from(1);
    }
    println!("{} files written to {out}", files.len());
    ExitCode::SUCCESS
}
