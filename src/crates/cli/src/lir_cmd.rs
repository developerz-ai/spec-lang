//! `spec-lang lir`: the program lowered to the LIR, as text or JSON, or run
//! by the LIR evaluator (`--eval`), one result per check.

use crate::args::parse_test_args;
use crate::output::Out;
use spec_lang_diag::JsonWriter;
use spec_lang_lir::eval::{Config, Status};
use std::path::Path;

pub fn run(args: &[String]) -> u8 {
    let eval = args.iter().any(|a| a == "--eval");
    let rest: Vec<String> = args.iter().filter(|a| *a != "--eval").cloned().collect();
    let a = match parse_test_args(&rest) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let mut out = Out::new(a.json);
    let prog = match spec_lang_ir::load_project(Path::new(&a.project)) {
        Ok(p) => p,
        Err(f) => {
            out.diags("load", &f.errors, &f.sources);
            return 1;
        }
    };
    let errors = spec_lang_check::check(&prog);
    if !errors.is_empty() {
        out.diags("check", &errors, &prog.sources);
        return 1;
    }
    let lir = match spec_lang_lir::lower(&prog) {
        Ok(l) => l,
        Err(ds) => {
            out.diags("lir", &ds, &prog.sources);
            return 1;
        }
    };
    if !eval {
        if a.json {
            println!("{}", spec_lang_lir::print::json::program(&lir));
        } else {
            print!("{}", spec_lang_lir::print::text::program(&lir));
        }
        return 0;
    }
    let cfg = Config { seed: a.seed, runs: a.runs, traces: a.traces, steps: a.steps, prefix: 10, jobs: a.jobs };
    let results = spec_lang_lir::eval::run_all(&lir, cfg);
    let mut green = true;
    for r in &results {
        green &= r.status == Status::Pass || r.blocked_by_foreign();
        if a.json {
            let j = JsonWriter::obj()
                .set("check", JsonWriter::str(&r.name))
                .set("status", JsonWriter::str(r.status.as_str()))
                .set("diagnostic", r.diag.as_ref().map_or(JsonWriter::Null, |d| d.json(&prog.sources)));
            println!("{j}");
        } else {
            println!("{:<8} {}", r.status.as_str(), r.name);
            if let Some(d) = &r.diag {
                for line in d.human(&prog.sources).lines() {
                    println!("         {line}");
                }
            }
        }
    }
    u8::from(!green)
}
