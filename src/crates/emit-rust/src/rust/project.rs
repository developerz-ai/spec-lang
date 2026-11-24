//! The crate around the program: `lib.rs`, `main.rs`, `app.rs`, `Cargo.toml`,
//! build configuration and the gate scripts, all from the `app` declaration.

use std::collections::BTreeSet;

use quote::quote;
use spec_lang_diag::Diag;
use spec_lang_ir::Store;

use super::layout::{Layout, ident};
use super::{Out, header, package, simple_file};
use crate::front::Front;
use crate::provenance::{FileBuf, Origin};

pub fn app_files(front: &Front<'_>, layout: &Layout, out: &mut Out) -> Result<(), Diag> {
    let app = front.app();
    let at = front.at(app.info.span);
    let path = front.decl_path(&app.info);
    let bin = package(front);
    let origin = |role: &str| Origin::new(path.clone(), at.clone(), role);
    let Some(Store::File(store)) = &app.store else {
        return Err(Diag::unsupported(app.info.span, "an app without @store(file(...))").with_decl(path.clone()));
    };
    let variable = format!("{}_STORE", bin.to_uppercase());

    // lib.rs
    let mut mods: Vec<String> = layout.modules.iter().map(|m| m.name.clone()).collect();
    mods.extend(["app", "cli", "lang", "out", "store"].map(String::from));
    mods.sort();
    let ids: Vec<_> = mods.iter().map(|m| ident(m)).collect();
    out.add(simple_file(
        "src/lib.rs",
        &format!("app {}", app.info.name),
        origin("crate root"),
        quote!(#(pub mod #ids;)*),
        &[],
        &BTreeSet::new(),
        None,
    )?);

    // app.rs
    let board = &layout.board;
    let mut uses: BTreeSet<String> = BTreeSet::new();
    for u in [
        format!("crate::{board}::Command"),
        format!("crate::{board}::Refusal"),
        format!("crate::{board}::apply"),
        "crate::cli::Failure".into(),
        "crate::out::Out".into(),
        "crate::store::Store".into(),
        "crate::store::codec::Json".into(),
    ] {
        uses.insert(u);
    }
    // Rules with a fused read: their own arm, reading only what they read.
    let mut fused = Vec::new();
    let mut fused_names = Vec::new();
    for (index, r) in front.rules.iter().enumerate() {
        let Some(p) = &front.facts.rules[index].projection else { continue };
        if !layout.on("store.read.projection") {
            continue;
        }
        let rname = front.prog.rule(r.id).info.name.clone();
        let reader = format!("{}Read", super::layout::camel(&rname));
        uses.insert(format!("crate::store::codec::{reader}"));
        uses.insert(format!("crate::{board}::{rname}"));
        let reader = ident(&reader);
        let f = ident(&rname);
        let v = ident(&super::layout::camel(&rname));
        let names: Vec<_> = r.params.iter().map(|p| ident(&p.name)).collect();
        let pat = if names.is_empty() { quote!(Command::#v) } else { quote!(Command::#v { #(#names),* }) };
        let call = quote!(#f(&store.read::<#reader>()?, &mut out #(, #names)*));
        fused.push(if r.guards.is_empty() {
            quote!(#pat => { #call; })
        } else {
            quote!(#pat => { #call.map_err(Failure::Refused)?; })
        });
        fused_names.push((rname, p.licence.clone()));
    }
    let doc_fused = if fused.is_empty() {
        quote!()
    } else {
        quote!(
            ///
            /// A rule that reads only some fields of the world reads only those
            /// (projection pushdown): its own arm, its own reader.
        )
    };
    let ts = quote! {
        /// One command against the store: load, the rule, commit. The lines come back
        /// only when the new world is on disk, so the caller prints after commit.
        ///
        /// The rule's write set picks the path: a rule that writes takes the exclusive
        /// lock and may replace the store; a rule that only reads takes the shared lock,
        /// and no code that writes the store is reachable from it.
        #doc_fused
        pub fn execute(store: &Store, command: Command) -> Result<Out, Failure<Refusal>> {
            let mut out = Out::default();
            match command {
                #(#fused)*
                command if command.writes() => {
                    store.update::<Json, _, _>(|world| apply(world, &mut out, command))?.map_err(Failure::Refused)?;
                }
                command => {
                    apply(&mut store.read::<Json>()?, &mut out, command).map_err(Failure::Refused)?;
                }
            }
            Ok(out)
        }
    };
    let mut app_origin = origin("app: load, rule, commit").strategy(
        "store.lock-by-write-set",
        "each rule's write set (its assignments; @uses agrees): read-only rules take the shared lock and never write",
    );
    if let Some((rname, licence)) = fused_names.first() {
        app_origin = origin("app: load, rule, commit").strategy(
            "store.read.projection",
            format!("rule {rname}: {licence}; and each rule's write set picks the lock"),
        );
    }
    out.add(simple_file(
        "src/app.rs",
        &format!("app {} with the cli vocabulary", app.info.name),
        app_origin,
        ts,
        &[],
        &uses,
        None,
    )?);

    // main.rs
    let store_lit = proc_macro2::Literal::string(store);
    let var_lit = proc_macro2::Literal::string(&variable);
    let uses: BTreeSet<String> = [
        "std::env",
        "std::io",
        "std::process::ExitCode",
        "{pkg}::app",
        "{pkg}::cli",
        "{pkg}::cli::table::COMMANDS",
        "{pkg}::cli::table::HELP",
        "{pkg}::store::Store",
        "{pkg}::store::path",
    ]
    .iter()
    .map(|u| u.replace("{pkg}", &bin))
    .collect();
    let ts = quote! {
        /// The only place that touches the process: arguments, environment, the two streams, the exit code.
        fn main() -> ExitCode {
            let arguments = env::args_os().skip(1).collect();
            let store = Store::at(path::configured(env::var_os(#var_lit), #store_lit));
            let exit = cli::run(
                arguments,
                &COMMANDS,
                HELP,
                &store,
                app::execute,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            );
            ExitCode::from(exit.code())
        }
    };
    out.add(simple_file(
        "src/main.rs",
        &format!("app {}, @bin(\"{bin}\"), @store(file(\"{store}\"))", app.info.name),
        origin("process edge"),
        ts,
        &[],
        &uses,
        None,
    )?);

    // Project files.
    let why = app.info.why.clone().unwrap_or_default();
    let statik = why.contains("static binary");
    let head = |c: &str| format!("{c} {}\n", header(&format!("app {}", app.info.name), &at));
    let mut cargo = head("#");
    cargo.push_str(&format!(
        "# A project of its own, never a member of an enclosing workspace.\n[workspace]\n\n[package]\nname = \"{bin}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"1.100\"\ndescription = \"{}\"\npublish = false\n\n[lib]\nname = \"{bin}\"\npath = \"src/lib.rs\"\n\n[[bin]]\nname = \"{bin}\"\npath = \"src/main.rs\"\n\n[dependencies]\n\n[lints.rust]\nunsafe_code = \"forbid\"\n\n[lints.clippy]\nunwrap_used = \"deny\"\nexpect_used = \"deny\"\npanic = \"deny\"\n\n[profile.release]\nopt-level = 3\nlto = \"fat\"\ncodegen-units = 1\npanic = \"abort\"\nstrip = true\n",
        why.replace('"', "'")
    ));
    project_file(out, "Cargo.toml", &cargo, origin("package manifest"), false);
    let lock = format!(
        "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"{bin}\"\nversion = \"0.1.0\"\n"
    );
    project_file(out, "Cargo.lock", &lock, origin("lock file: no dependencies"), false);
    if statik && layout.on("link.static") {
        let mut cfg = head("#");
        cfg.push_str(&format!(
            "# @why(\"{why}\"): link the C runtime statically where the target allows it.\n[target.x86_64-unknown-linux-gnu]\nrustflags = [\"-C\", \"target-feature=+crt-static\"]\n\n[target.aarch64-unknown-linux-gnu]\nrustflags = [\"-C\", \"target-feature=+crt-static\"]\n"
        ));
        project_file(
            out,
            ".cargo/config.toml",
            &cfg,
            origin("build configuration").strategy("link.static", format!("the app's @why(\"{why}\")")),
            false,
        );
    }
    let mut clippy = head("#");
    clippy.push_str("allow-unwrap-in-tests = true\nallow-expect-in-tests = true\nallow-panic-in-tests = true\n");
    project_file(out, "clippy.toml", &clippy, origin("lint configuration"), false);
    project_file(out, ".gitignore", "target/\n", origin("ignore build output"), false);
    let mut build = format!("#!/usr/bin/env bash\n{}", head("#"));
    build.push_str(&format!("# Builds target/release/{bin}.\nset -euo pipefail\ncd \"$(dirname \"${{BASH_SOURCE[0]}}\")/..\"\ncargo build --release --locked\n"));
    project_file(out, "bin/build", &build, origin("build script"), true);
    let mut check = format!("#!/usr/bin/env bash\n{}", head("#"));
    check.push_str("# The gate: lints, tests, then the release build. Formatting is the printer's (prettyplease).\nset -euo pipefail\ncd \"$(dirname \"${BASH_SOURCE[0]}\")/..\"\ncargo clippy --locked --all-targets -- -D warnings\ncargo test --locked\nbin/build\n");
    project_file(out, "bin/check", &check, origin("gate script"), true);
    Ok(())
}

fn project_file(out: &mut Out, path: &str, text: &str, origin: Origin, executable: bool) {
    let mut buf = FileBuf::new(path);
    if executable {
        buf = buf.executable();
    }
    buf.push(text, origin);
    out.add(buf);
}
