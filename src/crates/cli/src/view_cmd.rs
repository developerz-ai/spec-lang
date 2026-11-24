//! `spec-lang view`: where something is and what checks it.
//!
//! ```text
//! spec-lang view <project> <decl> [--json]        one declaration: source, notes, callers, what it names,
//!                                                 and every case, test and law that reaches it, with its result
//! spec-lang view <project> --as pseudo [<module>] pseudocode (crate `view`)
//! spec-lang view <project> --as diagram --kind <kind> [--format <f>] [<subject>]   diagrams (crate `view`)
//! ```

use crate::output::emit;
use spec_lang_diag::JsonWriter;
use spec_lang_ir::{DeclRef, Program};
use spec_lang_testing::{Config, run_all};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn run(args: &[String]) -> u8 {
    let (Some(project), rest) = (args.first(), args.get(1..).unwrap_or(&[])) else {
        eprintln!("usage: spec-lang view <project> <decl> [--json] | --as pseudo|diagram ...");
        return 2;
    };
    let prog = match spec_lang_ir::load_project(Path::new(project)) {
        Ok(p) => p,
        Err(f) => {
            eprintln!("{}", f.render());
            return 1;
        }
    };
    match rest {
        [flag, how, more @ ..] if flag == "--as" => projection(&prog, how, more),
        [decl] => card(&prog, decl, false),
        [decl, json] | [json, decl] if json == "--json" => card(&prog, decl, true),
        _ => {
            eprintln!("usage: spec-lang view <project> <decl> [--json] | --as pseudo|diagram ...");
            2
        }
    }
}

fn projection(prog: &Program, how: &str, args: &[String]) -> u8 {
    let result = match how {
        "pseudo" => view::pseudo(prog, args.first().map(String::as_str)),
        "diagram" => {
            let mut kind = None;
            let mut format = "markdown".to_string();
            let mut subject = None;
            let mut it = args.iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--kind" => kind = it.next().cloned(),
                    "--format" => format = it.next().cloned().unwrap_or_default(),
                    s => subject = Some(s.to_string()),
                }
            }
            let Some(kind) = kind else {
                eprintln!("error: --as diagram needs --kind <kind>");
                return 2;
            };
            view::DiagramKind::parse(&kind).and_then(|k| {
                view::Format::parse(&format)
                    .and_then(|f| view::diagram(prog, k, subject.as_deref(), f, view::Inputs::default()))
            })
        }
        other => {
            eprintln!("error: unknown projection {other:?}: pseudo or diagram");
            return 2;
        }
    };
    match result {
        Ok(text) => {
            emit(&text);
            if !text.ends_with('\n') {
                emit("\n");
            }
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

/// Every declaration that reaches `target` through what it names, itself included.
fn reaching(refs: &BTreeMap<Option<DeclRef>, BTreeSet<DeclRef>>, target: DeclRef) -> BTreeSet<DeclRef> {
    let mut found = BTreeSet::from([target]);
    let mut changed = true;
    while changed {
        changed = false;
        for (from, to) in refs {
            if let Some(f) = from
                && !found.contains(f)
                && to.iter().any(|t| found.contains(t))
            {
                found.insert(*f);
                changed = true;
            }
        }
    }
    found
}

fn card(prog: &Program, name: &str, json: bool) -> u8 {
    let d = match prog.find_decl(name) {
        Ok(d) => d,
        Err(candidates) => {
            eprintln!("error: `{name}` names {} declarations: {}", candidates.len(), candidates.join(", "));
            return 1;
        }
    };
    let info = prog.info(d);
    let qualified = prog.qualified(info);
    let refs = spec_lang_check::refs::decl_refs(prog);
    let names: Vec<String> = refs.get(&Some(d)).into_iter().flatten().map(|x| prog.qualified(prog.info(*x))).collect();
    let callers: Vec<String> = refs
        .iter()
        .filter_map(|(f, to)| f.filter(|f| *f != d && to.contains(&d)))
        .map(|f| prog.qualified(prog.info(f)))
        .collect();
    let reach: BTreeSet<String> = reaching(&refs, d).into_iter().map(|x| prog.qualified(prog.info(x))).collect();
    let report = run_all(prog, Config::default());
    let checks: Vec<&spec_lang_testing::CheckResult> =
        report.checks.iter().filter(|c| reach.contains(&c.decl)).collect();
    let loc = prog.location(info.span);
    let kind = match d {
        DeclRef::Type(_) => "type",
        DeclRef::Const(_) => "const",
        DeclRef::State(_) => "state",
        DeclRef::Fn(_) => "fn",
        DeclRef::Rule(_) => "rule",
        DeclRef::Test(_) => "test",
        DeclRef::Law(_) => "law",
        DeclRef::App(_) => "app",
    };
    if json {
        let list = |xs: &[String]| JsonWriter::Arr(xs.iter().map(JsonWriter::str).collect());
        let j = JsonWriter::obj()
            .set("decl", JsonWriter::str(&qualified))
            .set("kind", JsonWriter::str(kind))
            .set("pub", JsonWriter::Bool(info.is_pub))
            .set("at", JsonWriter::str(loc.to_string()))
            .set("why", info.why.as_ref().map_or(JsonWriter::Null, JsonWriter::str))
            .set("source", JsonWriter::str(prog.snippet(info.span)))
            .set("foreign", foreign_of(prog, d).map_or(JsonWriter::Null, |f| JsonWriter::str(f.to_string())))
            .set("names", list(&names))
            .set("named_by", list(&callers))
            .set(
                "checks",
                JsonWriter::Arr(
                    checks
                        .iter()
                        .map(|c| {
                            JsonWriter::obj()
                                .set("kind", JsonWriter::str(c.kind.as_str()))
                                .set("decl", JsonWriter::str(&c.decl))
                                .set("label", JsonWriter::str(&c.label))
                                .set("status", JsonWriter::str(c.status.as_str().to_lowercase()))
                                .set("evidence", JsonWriter::str(c.evidence.to_string()))
                        })
                        .collect(),
                ),
            );
        emit(&format!("{j}\n"));
    } else {
        let mut out = format!("{qualified}  {kind}{}  {loc}\n", if info.is_pub { ", pub" } else { "" });
        if let Some(w) = &info.why {
            out.push_str(&format!("why: {w}\n"));
        }
        out.push('\n');
        out.push_str(prog.snippet(info.span));
        out.push_str("\n\n");
        if let Some(f) = foreign_of(prog, d) {
            out.push_str(&format!(
                "foreign:  {} \"{}\": the interpreter blocks a check that reaches it\n",
                f.lang, f.symbol
            ));
        }
        out.push_str(&format!("names:    {}\n", if names.is_empty() { "-".into() } else { names.join(", ") }));
        out.push_str(&format!("named by: {}\n", if callers.is_empty() { "-".into() } else { callers.join(", ") }));
        out.push_str(&format!(
            "checked by ({} of {} checks reach it; {}):\n",
            checks.len(),
            report.checks.len(),
            report.settings()
        ));
        for c in &checks {
            out.push_str(&format!(
                "  {:<7} {:<8} {} {}  {}\n",
                c.status.as_str(),
                c.kind.as_str(),
                c.decl,
                c.label,
                c.evidence
            ));
        }
        emit(&out);
    }
    0
}

/// The `foreign` body of a `fn` or rule.
fn foreign_of(prog: &Program, d: DeclRef) -> Option<&spec_lang_ir::Foreign> {
    match d {
        DeclRef::Fn(f) => prog.func(f).foreign.as_ref(),
        DeclRef::Rule(r) => prog.rule(r).foreign.as_ref(),
        _ => None,
    }
}
