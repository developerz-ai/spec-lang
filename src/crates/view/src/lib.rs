//! Projections of a spec-lang program: UML diagrams, codebase graphs,
//! pseudocode and a static explorer site, computed from the IR.
//!
//! Humans read projections; only source is edited
//! (`docs/idea/10-compiler-and-ir.md`, "Projections"). Every function here is
//! a pure query over a [`spec_lang_ir::Program`]: it runs no rule, reads no
//! file, and gives the same bytes for the same program.
//!
//! The command line needs three calls:
//!
//! * [`diagram`]: `view --as diagram --kind <kind> [<subject>]` in one [`Format`];
//! * [`pseudo`]: `view --as pseudo [<module>]`;
//! * [`bundle`]: every view as files (Markdown, PlantUML, JSON, and the site of [`site::site`]).
//!
//! and [`sequence::trace_sequence`] draws a counterexample trace the engine found.
//! The modules under them hold one model per view, each with its own printers.

pub mod calls;
pub mod class;
pub mod effects;
pub mod facts;
pub mod graph;
pub mod impact;
pub mod json;
pub mod modules;
pub mod pages;
pub mod provenance;
pub mod pseudo;
pub mod sequence;
pub mod site;
pub mod state;
pub mod text;

use facts::Facts;
use json::Json;
use provenance::Provenance;
use spec_lang_ir::{DeclRef, Program};
use state::FlowEdge;
use std::fmt;
use std::path::Path;

/// The Mermaid build the site loads, and the one the committed diagrams were checked with.
pub const MERMAID_SRC: &str = "https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.min.js";

/// A diagram kind of `view --as diagram --kind <kind>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagramKind {
    Class,
    State,
    Sequence,
    Modules,
    Effects,
    Calls,
    Impact,
}

impl DiagramKind {
    pub const ALL: [DiagramKind; 7] = [
        DiagramKind::Class,
        DiagramKind::State,
        DiagramKind::Sequence,
        DiagramKind::Modules,
        DiagramKind::Effects,
        DiagramKind::Calls,
        DiagramKind::Impact,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            DiagramKind::Class => "class",
            DiagramKind::State => "state",
            DiagramKind::Sequence => "sequence",
            DiagramKind::Modules => "modules",
            DiagramKind::Effects => "effects",
            DiagramKind::Calls => "calls",
            DiagramKind::Impact => "impact",
        }
    }

    pub fn parse(name: &str) -> Result<DiagramKind, ViewError> {
        DiagramKind::ALL
            .into_iter()
            .find(|k| k.as_str() == name)
            .ok_or_else(|| ViewError::UnknownKind(name.to_string()))
    }
}

/// An output format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// One Mermaid diagram, with no fence.
    Mermaid,
    /// PlantUML; several diagrams are several `@startuml` blocks. Class, state and sequence only.
    PlantUml,
    /// Nodes, edges and counts, for agents.
    Json,
    /// A page: counts, fenced Mermaid, tables, limits.
    Markdown,
}

impl Format {
    pub fn parse(name: &str) -> Result<Format, ViewError> {
        match name {
            "mermaid" => Ok(Format::Mermaid),
            "plantuml" => Ok(Format::PlantUml),
            "json" => Ok(Format::Json),
            "markdown" | "md" => Ok(Format::Markdown),
            other => Err(ViewError::UnknownFormat(other.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewError {
    UnknownKind(String),
    UnknownFormat(String),
    /// The kind has no printer for the format (PlantUML of a graph view).
    NoPrinter {
        kind: &'static str,
        format: &'static str,
    },
    /// The subject names nothing: the declarations, tests or machines that could be meant.
    UnknownSubject {
        subject: String,
        candidates: Vec<String>,
    },
    /// One Mermaid diagram was asked for and the view has several: name one of these.
    NeedsSubject {
        kind: &'static str,
        candidates: Vec<String>,
    },
}

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewError::UnknownKind(k) => {
                write!(f, "unknown diagram kind `{k}`; one of: class, state, sequence, modules, effects, calls, impact")
            }
            ViewError::UnknownFormat(x) => write!(f, "unknown format `{x}`; one of: mermaid, plantuml, json, markdown"),
            ViewError::NoPrinter { kind, format } => write!(f, "diagram kind `{kind}` has no {format} printer"),
            ViewError::UnknownSubject { subject, candidates } if candidates.is_empty() => {
                write!(f, "`{subject}` names nothing in this program")
            }
            ViewError::UnknownSubject { subject, candidates } => {
                write!(f, "`{subject}` is not one name; candidates: {}", candidates.join(", "))
            }
            ViewError::NeedsSubject { kind, candidates } => {
                write!(f, "diagram kind `{kind}` has several diagrams; name one of: {}", candidates.join(", "))
            }
        }
    }
}

impl std::error::Error for ViewError {}

/// What the views read besides the program.
#[derive(Clone, Copy, Debug, Default)]
pub struct Inputs<'a> {
    /// `provenance.json` of generated projects, already read; empty for none.
    pub provenance: &'a [Provenance],
    /// `@edge`s of `flow`s; empty until the IR carries flows.
    pub flows: &'a [FlowEdge],
}

/// One generated file: a path with `/` separators, relative to the output folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub path: String,
    pub contents: String,
}

impl File {
    pub fn new(path: impl Into<String>, contents: impl Into<String>) -> Self {
        File { path: path.into(), contents: contents.into() }
    }
}

fn pick<'a, T>(
    kind: DiagramKind,
    subject: Option<&str>,
    items: &'a [T],
    name: impl Fn(&T) -> &str,
) -> Result<&'a T, ViewError> {
    let names = || items.iter().map(|i| name(i).to_string()).collect::<Vec<_>>();
    match (subject, items) {
        (None, [one]) => Ok(one),
        (None, _) => Err(ViewError::NeedsSubject { kind: kind.as_str(), candidates: names() }),
        (Some(s), _) => {
            let suffix = format!(".{s}");
            let hits: Vec<&T> = items.iter().filter(|i| name(i) == s || name(i).ends_with(&suffix)).collect();
            match hits.as_slice() {
                [one] => Ok(one),
                [] => Err(ViewError::UnknownSubject { subject: s.to_string(), candidates: Vec::new() }),
                many => Err(ViewError::UnknownSubject {
                    subject: s.to_string(),
                    candidates: many.iter().map(|i| name(i).to_string()).collect(),
                }),
            }
        }
    }
}

fn find_decl(f: &Facts<'_>, subject: &str) -> Result<usize, ViewError> {
    f.find(subject).map_err(|candidates| ViewError::UnknownSubject { subject: subject.to_string(), candidates })
}

/// `view --as diagram --kind <kind> [<subject>]`.
///
/// `subject` picks one diagram where a kind has several: a machine for
/// `state`, a test for `sequence`, a declaration for `impact` (by qualified
/// name, a dotted suffix of it, or a bare name that is unique). Without one,
/// Markdown, JSON and PlantUML give every diagram of the kind, and Mermaid,
/// which holds one diagram per text, is an error naming the choices.
pub fn diagram(
    program: &Program,
    kind: DiagramKind,
    subject: Option<&str>,
    format: Format,
    inputs: Inputs<'_>,
) -> Result<String, ViewError> {
    let f = &Facts::of(program);
    let no_plantuml = || ViewError::NoPrinter { kind: kind.as_str(), format: "plantuml" };
    Ok(match kind {
        DiagramKind::Class => {
            let d = class::class_diagram(f);
            let d = match subject {
                Some(s) => d.around(&f.decls[find_decl(f, s)?].id),
                None => d,
            };
            match format {
                Format::Mermaid => d.to_mermaid(),
                Format::PlantUml => d.to_plantuml(),
                Format::Json => d.to_json().pretty(),
                Format::Markdown => pages::class_page(f),
            }
        }
        DiagramKind::State => {
            let machines = state::state_machines(f, inputs.flows);
            let chosen: Vec<&state::Machine> = match subject {
                Some(_) => vec![pick(kind, subject, &machines, |m| &m.name)?],
                None => machines.iter().collect(),
            };
            match format {
                Format::Mermaid => pick(kind, subject, &machines, |m| &m.name)?.to_mermaid(),
                Format::PlantUml => chosen.iter().map(|m| m.to_plantuml()).collect::<Vec<_>>().join("\n"),
                Format::Json => states_json(&chosen).pretty(),
                Format::Markdown => pages::state_page(f, inputs.flows),
            }
        }
        DiagramKind::Sequence => {
            let seqs = sequence::test_sequences(f);
            let chosen: Vec<&sequence::Sequence> = match subject {
                Some(_) => vec![pick(kind, subject, &seqs, |s| &s.title)?],
                None => seqs.iter().collect(),
            };
            match format {
                Format::Mermaid => pick(kind, subject, &seqs, |s| &s.title)?.to_mermaid(),
                Format::PlantUml => chosen.iter().map(|s| s.to_plantuml()).collect::<Vec<_>>().join("\n"),
                Format::Json => sequences_json(&chosen).pretty(),
                Format::Markdown => pages::sequence_page(f),
            }
        }
        DiagramKind::Modules => {
            let g = modules::module_graph(f);
            match format {
                Format::Mermaid => g.to_mermaid("LR"),
                Format::PlantUml => return Err(no_plantuml()),
                Format::Json => g.to_json().pretty(),
                Format::Markdown => pages::modules_page(f),
            }
        }
        DiagramKind::Effects => {
            let e = effects::effects(f);
            match format {
                Format::Mermaid => e.graph.to_mermaid("LR"),
                Format::PlantUml => return Err(no_plantuml()),
                Format::Json => e.to_json().pretty(),
                Format::Markdown => pages::effects_page(f),
            }
        }
        DiagramKind::Calls => {
            let g = calls::call_graph(f, calls::Scope::WithChecks);
            let product = calls::call_graph(f, calls::Scope::Product);
            let (g, product) = match subject {
                Some(s) => {
                    let id = &f.decls[find_decl(f, s)?].id;
                    (calls::neighbourhood(&g, id), calls::neighbourhood(&product, id))
                }
                None => (g, product),
            };
            match format {
                Format::Mermaid => product.to_mermaid("LR"),
                Format::PlantUml => return Err(no_plantuml()),
                Format::Json => g.to_json().pretty(),
                Format::Markdown => pages::calls_page(f),
            }
        }
        DiagramKind::Impact => match (subject, format) {
            (_, Format::PlantUml) => return Err(no_plantuml()),
            (Some(s), Format::Mermaid) => impact::impact(f, find_decl(f, s)?, inputs.provenance).graph.to_mermaid("RL"),
            (Some(s), Format::Json) => impact::impact(f, find_decl(f, s)?, inputs.provenance).to_json(true).pretty(),
            (None, Format::Mermaid) => {
                let candidates = f.decls.iter().map(|d| d.qname.clone()).collect();
                return Err(ViewError::NeedsSubject { kind: kind.as_str(), candidates });
            }
            (None, Format::Json) => impacts_json(f, inputs.provenance).pretty(),
            (_, Format::Markdown) => pages::impact_page(f, inputs.provenance),
        },
    })
}

fn states_json(machines: &[&state::Machine]) -> Json {
    let nodes: usize = machines.iter().map(|m| m.node_count()).sum();
    let edges: usize = machines.iter().map(|m| m.transitions.len()).sum();
    Json::obj([
        ("view", Json::str("state")),
        (
            "counts",
            Json::obj([
                ("machines", Json::count(machines.len())),
                ("nodes", Json::count(nodes)),
                ("edges", Json::count(edges)),
            ]),
        ),
        ("machines", Json::Arr(machines.iter().map(|m| m.to_json()).collect())),
    ])
}

fn sequences_json(seqs: &[&sequence::Sequence]) -> Json {
    let nodes: usize = seqs.iter().map(|s| s.participants.len()).sum();
    let edges: usize = seqs.iter().map(|s| s.message_count()).sum();
    Json::obj([
        ("view", Json::str("sequence")),
        (
            "counts",
            Json::obj([
                ("sequences", Json::count(seqs.len())),
                ("nodes", Json::count(nodes)),
                ("edges", Json::count(edges)),
            ]),
        ),
        ("sequences", Json::Arr(seqs.iter().map(|s| s.to_json()).collect())),
    ])
}

fn impacts_json(f: &Facts<'_>, provenance: &[Provenance]) -> Json {
    let all: Vec<Json> = (0..f.decls.len()).map(|i| impact::impact(f, i, provenance).to_json(false)).collect();
    Json::obj([("view", Json::str("impact")), ("count", Json::count(all.len())), ("declarations", Json::Arr(all))])
}

/// `view --as pseudo [<module>]`: one module, or every module that has a
/// constant, state, function or rule, each under a `module` line.
pub fn pseudo(program: &Program, module: Option<&str>) -> Result<String, ViewError> {
    let f = &Facts::of(program);
    match module {
        Some(m) if f.modules().contains(&m) => Ok(pseudo::pseudo_module(f, m)),
        Some(m) => Err(ViewError::UnknownSubject {
            subject: m.to_string(),
            candidates: f.modules().iter().map(|m| m.to_string()).collect(),
        }),
        None => {
            let parts: Vec<String> =
                f.modules().iter().map(|m| pseudo::pseudo_module(f, m)).filter(|t| !t.is_empty()).collect();
            Ok(parts.join("\n"))
        }
    }
}

/// Every view of a program as the files a project commits:
///
/// ```text
/// README.md                         index, with node and edge counts
/// class.md  class.puml  class.json
/// state.md  state.puml  state.json
/// sequence.md  sequence.puml  sequence.json
/// modules.md  modules.json
/// effects.md  effects.json
/// calls.md  calls.json
/// impact.md  impact.json
/// pseudo.md
/// site/...                          the explorer
/// ```
///
/// `title` names the project in headings. `regenerate` is the command line
/// that makes these files again, printed in the index.
pub fn bundle(program: &Program, title: &str, regenerate: &str, inputs: Inputs<'_>) -> Vec<File> {
    let f = &Facts::of(program);
    let mut files = Vec::new();

    let classes = class::class_diagram(f);
    let machines = state::state_machines(f, inputs.flows);
    let machine_refs: Vec<&state::Machine> = machines.iter().collect();
    let seqs = sequence::test_sequences(f);
    let seq_refs: Vec<&sequence::Sequence> = seqs.iter().collect();
    let modules = modules::module_graph(f);
    let fx = effects::effects(f);
    let product = calls::call_graph(f, calls::Scope::Product);
    let checks = calls::call_graph(f, calls::Scope::WithChecks);

    files.push(File::new("class.md", pages::class_page(f)));
    files.push(File::new("class.puml", classes.to_plantuml()));
    files.push(File::new("class.json", classes.to_json().pretty()));
    files.push(File::new("state.md", pages::state_page(f, inputs.flows)));
    files.push(File::new("state.puml", machines.iter().map(|m| m.to_plantuml()).collect::<Vec<_>>().join("\n")));
    files.push(File::new("state.json", states_json(&machine_refs).pretty()));
    files.push(File::new("sequence.md", pages::sequence_page(f)));
    files.push(File::new("sequence.puml", seqs.iter().map(|s| s.to_plantuml()).collect::<Vec<_>>().join("\n")));
    files.push(File::new("sequence.json", sequences_json(&seq_refs).pretty()));
    files.push(File::new("modules.md", pages::modules_page(f)));
    files.push(File::new("modules.json", modules.to_json().pretty()));
    files.push(File::new("effects.md", pages::effects_page(f)));
    files.push(File::new("effects.json", fx.to_json().pretty()));
    files.push(File::new("calls.md", pages::calls_page(f)));
    files.push(File::new("calls.json", checks.to_json().pretty()));
    files.push(File::new("impact.md", pages::impact_page(f, inputs.provenance)));
    files.push(File::new("impact.json", impacts_json(f, inputs.provenance).pretty()));
    files.push(File::new("pseudo.md", pages::pseudo_page(f)));

    let options =
        site::SiteOptions { title, mermaid_src: MERMAID_SRC, provenance: inputs.provenance, flows: inputs.flows };
    let pages = site::site(f, &options);
    let site_pages = pages.len();
    files.extend(pages.into_iter().map(|p| File::new(format!("site/{}", p.path), p.contents)));

    let count = |k: facts::Kind| f.decls.iter().filter(|d| d.kind == k).count();
    let machine_nodes: usize = machines.iter().map(|m| m.node_count()).sum();
    let machine_edges: usize = machines.iter().map(|m| m.transitions.len()).sum();
    let seq_nodes: usize = seqs.iter().map(|s| s.participants.len()).sum();
    let seq_edges: usize = seqs.iter().map(|s| s.message_count()).sum();
    let with_dependents = (0..f.decls.len()).filter(|&i| !impact::dependents(f, i).is_empty()).count();
    let tests = f.decls.iter().filter(|d| matches!(d.r, DeclRef::Test(_))).count();
    let mut index = format!(
        "# Views of {title}\n\n> **Generated.** Every file in this folder is a projection of the program's IR, printed by the `view` crate. \
         Do not edit: change the source and regenerate. Same source, same bytes, so a stale file is a failing check.\n\n\
         The program: {} modules, {} types, {} states, {} functions, {} rules, {} tests, {} laws, {} apps.\n\n\
         | View | Read it | Also | Diagrams | Nodes | Edges |\n|---|---|---|---|---|---|\n",
        f.modules().len(),
        count(facts::Kind::Type),
        count(facts::Kind::State),
        count(facts::Kind::Fn),
        count(facts::Kind::Rule),
        tests,
        count(facts::Kind::Law),
        count(facts::Kind::App),
    );
    let mut row = |view: &str, also: &str, diagrams: usize, nodes: usize, edges: usize| {
        index.push_str(&format!("| {view} | [{view}.md]({view}.md) | {also} | {diagrams} | {nodes} | {edges} |\n"));
    };
    row("class", "[puml](class.puml), [json](class.json)", 1, classes.classes.len(), classes.relations.len());
    row("state", "[puml](state.puml), [json](state.json)", machines.len(), machine_nodes, machine_edges);
    row("sequence", "[puml](sequence.puml), [json](sequence.json)", seqs.len(), seq_nodes, seq_edges);
    row("modules", "[json](modules.json)", 1, modules.nodes.len(), modules.edges.len());
    row("effects", "[json](effects.json)", 1, fx.graph.nodes.len(), fx.graph.edges.len());
    row("calls", "[json](calls.json)", 1, product.nodes.len(), product.edges.len());
    index.push_str(&format!(
        "\nMore in the same pages: the effects view also lists **{} pairs of rules whose order matters**; the call graph with tests and laws has \
         **{} nodes, {} edges**; [impact.md](impact.md) ([json](impact.json)) has one graph for each of the {with_dependents} declarations something depends on; \
         [pseudo.md](pseudo.md) is every function and rule as pseudocode.\n\n\
         The explorer is [site/index.html](site/index.html): {site_pages} files, one page per module and per declaration with its source, notes, \
         diagrams, what it names, what depends on it and its tests and laws. Open it from a checkout; it loads one pinned script to draw Mermaid \
         (`{MERMAID_SRC}`) and shows the diagram text without it.\n\n\
         Regenerate:\n\n```bash\n{regenerate}\n```\n\n",
        fx.conflicts.len(),
        checks.nodes.len(),
        checks.edges.len(),
    ));
    index.push_str(
        "## Limits\n\n\
         - Every view is read off the declarations without running a rule: read and write sets are what a rule can touch, sequences are a test's written steps.\n\
         - Counts are of the diagram as printed. They change when the source does, and that diff is the point.\n\
         - Each page ends with what its view does not show.\n",
    );
    files.insert(0, File::new("README.md", index));
    files
}

/// Write files under `dir`, creating folders, and remove nothing: a caller
/// that wants a clean folder empties it first.
pub fn write_files(dir: &Path, files: &[File]) -> std::io::Result<()> {
    for file in files {
        let path = dir.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &file.contents)?;
    }
    Ok(())
}
