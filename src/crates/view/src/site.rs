//! `site`: every view as one static, linked set of pages.
//!
//! ```text
//! index.html                 modules, declarations, counts
//! diagrams/<kind>.html       class, state, sequence, modules, effects, calls
//! pseudo.html                the pseudocode of every module
//! module/<id>.html           one module: declarations, pseudocode, source
//! decl/<id>.html             one declaration: source, notes, diagrams, what it
//!                            names, what names it, tests and laws, emitted code
//! style.css
//! ```
//!
//! Pages are plain HTML and CSS. Diagrams are Mermaid text in `<pre class="mermaid">`,
//! drawn by the one script [`SiteOptions::mermaid_src`] names; without it the
//! text stays readable.

use crate::calls::{Scope, call_graph, neighbourhood};
use crate::class::class_diagram;
use crate::effects::effects;
use crate::facts::{Facts, Kind, Phase, Target, module_file};
use crate::impact::impact;
use crate::modules::module_graph;
use crate::provenance::Provenance;
use crate::pseudo::{pseudo_decl, pseudo_module};
use crate::sequence::{test_sequence, test_sequences};
use crate::state::{FlowEdge, state_machines};
use crate::text::{Ids, html as h};
use crate::{File, MERMAID_SRC};
use spec_lang_ir::{DeclRef, Given, Store, Trigger};
use std::fmt::Write as _;

#[derive(Clone, Debug)]
pub struct SiteOptions<'a> {
    /// The project's name, for titles.
    pub title: &'a str,
    /// The script that draws Mermaid: a pinned URL, or a path to a copy beside the site.
    pub mermaid_src: &'a str,
    pub provenance: &'a [Provenance],
    pub flows: &'a [FlowEdge],
}

impl<'a> SiteOptions<'a> {
    pub fn new(title: &'a str) -> Self {
        SiteOptions { title, mermaid_src: MERMAID_SRC, provenance: &[], flows: &[] }
    }
}

const STYLE: &str = r#":root { color-scheme: light dark; --fg: #1f2328; --bg: #ffffff; --soft: #f6f8fa; --line: #d0d7de; --link: #0969da; --dim: #59636e; }
@media (prefers-color-scheme: dark) { :root { --fg: #e6edf3; --bg: #0d1117; --soft: #161b22; --line: #30363d; --link: #58a6ff; --dim: #9198a1; } }
* { box-sizing: border-box; }
body { margin: 0; font: 15px/1.5 system-ui, sans-serif; color: var(--fg); background: var(--bg); }
a { color: var(--link); text-decoration: none; }
a:hover { text-decoration: underline; }
header { padding: 10px 16px; border-bottom: 1px solid var(--line); background: var(--soft); display: flex; flex-wrap: wrap; gap: 6px 16px; align-items: baseline; }
header strong { font-size: 16px; }
main { max-width: 1100px; margin: 0 auto; padding: 16px; }
h1 { font-size: 24px; margin: 8px 0; }
h1 code, h1 .kind { font-size: inherit; }
h2 { font-size: 18px; margin: 28px 0 8px; padding-bottom: 4px; border-bottom: 1px solid var(--line); }
h3 { font-size: 15px; margin: 18px 0 6px; }
pre, code { font: 13px/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
pre { background: var(--soft); border: 1px solid var(--line); border-radius: 6px; padding: 10px 12px; overflow-x: auto; }
pre.mermaid { background: var(--bg); }
table { border-collapse: collapse; display: block; overflow-x: auto; }
th, td { border: 1px solid var(--line); padding: 4px 8px; text-align: left; vertical-align: top; }
th { background: var(--soft); }
.kind { display: inline-block; min-width: 44px; font: 12px ui-monospace, monospace; color: var(--dim); }
.dim { color: var(--dim); }
.why { border-left: 3px solid var(--line); margin: 8px 0; padding: 2px 12px; }
ul.plain { list-style: none; padding-left: 0; }
ul.cols { columns: 260px; }
"#;

struct Site<'a, 'p> {
    f: &'a Facts<'p>,
    o: &'a SiteOptions<'a>,
    module_ids: Vec<(&'p str, String)>,
}

impl Site<'_, '_> {
    fn module_href(&self, module: &str) -> String {
        let id = self.module_ids.iter().find(|(m, _)| *m == module).map_or("", |(_, id)| id.as_str());
        format!("module/{id}.html")
    }

    fn decl_href(&self, i: usize) -> String {
        format!("decl/{}.html", self.f.decls[i].id)
    }

    /// A link to a declaration, from a page `depth` folders below the root.
    fn decl_link(&self, i: usize, up: &str) -> String {
        let d = &self.f.decls[i];
        format!("<a href=\"{up}{}\"><code>{}</code></a>", self.decl_href(i), h(&d.qname))
    }

    fn links(&self, items: &[usize], up: &str) -> String {
        if items.is_empty() {
            return "<span class=\"dim\">none</span>".to_string();
        }
        items.iter().map(|&i| self.decl_link(i, up)).collect::<Vec<_>>().join(", ")
    }

    fn by_qname(&self, names: &[String], up: &str) -> String {
        let idx: Vec<usize> = names.iter().filter_map(|n| self.f.decls.iter().position(|d| d.qname == *n)).collect();
        self.links(&idx, up)
    }

    fn page(&self, title: &str, up: &str, body: &str) -> String {
        let mut nav = String::new();
        for (name, file) in [
            ("class", "diagrams/class.html"),
            ("state", "diagrams/state.html"),
            ("sequence", "diagrams/sequence.html"),
            ("modules", "diagrams/modules.html"),
            ("effects", "diagrams/effects.html"),
            ("calls", "diagrams/calls.html"),
            ("pseudocode", "pseudo.html"),
        ] {
            let _ = write!(nav, "<a href=\"{up}{file}\">{name}</a>");
        }
        format!(
            "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
             <title>{t} - {p}</title>\n<link rel=\"stylesheet\" href=\"{up}style.css\">\n</head>\n<body>\n\
             <header><strong><a href=\"{up}index.html\">{p}</a></strong>{nav}</header>\n<main>\n{body}</main>\n\
             <script src=\"{src}\"></script>\n\
             <script>if (window.mermaid) {{ mermaid.initialize({{ startOnLoad: true, securityLevel: \"strict\", \
             theme: matchMedia(\"(prefers-color-scheme: dark)\").matches ? \"dark\" : \"default\" }}); }}</script>\n\
             </body>\n</html>\n",
            t = h(title),
            p = h(self.o.title),
            src = h(self.o.mermaid_src),
        )
    }

    fn mermaid(text: &str) -> String {
        format!("<pre class=\"mermaid\">\n{}</pre>\n", h(text))
    }

    fn decl_list(&self, module: &str, up: &str) -> String {
        let mut out = String::from("<ul class=\"plain cols\">\n");
        for (i, d) in self.f.decls.iter().enumerate().filter(|(_, d)| d.module == module) {
            let _ = writeln!(
                out,
                "<li><span class=\"kind\">{}</span> <a href=\"{up}{}\"><code>{}</code></a></li>",
                d.kind.as_str(),
                self.decl_href(i),
                h(&d.name)
            );
        }
        out.push_str("</ul>\n");
        out
    }

    fn index(&self) -> String {
        let f = self.f;
        let mut body = format!("<h1>{}</h1>\n", h(self.o.title));
        let count = |k: Kind| f.decls.iter().filter(|d| d.kind == k).count();
        let _ = writeln!(
            body,
            "<p>{} modules: {} types, {} states, {} functions, {} rules, {} tests, {} laws, {} apps. \
             Every page is computed from the source; only the source is edited.</p>",
            f.modules().len(),
            count(Kind::Type),
            count(Kind::State),
            count(Kind::Fn),
            count(Kind::Rule),
            count(Kind::Test),
            count(Kind::Law),
            count(Kind::App)
        );
        body.push_str("<h2>Modules</h2>\n");
        body.push_str(&Self::mermaid(&module_graph(f).to_mermaid("LR")));
        for m in f.modules() {
            let _ = writeln!(body, "<h3><a href=\"{}\"><code>{}</code></a></h3>", self.module_href(m), h(m));
            body.push_str(&self.decl_list(m, ""));
        }
        self.page("Explorer", "", &body)
    }

    fn module_page(&self, module: &str) -> String {
        let f = self.f;
        let up = "../";
        let mut body = format!("<h1><span class=\"kind\">module</span> <code>{}</code></h1>\n", h(module));
        let _ = writeln!(body, "<p class=\"dim\"><code>{}</code></p>", h(&module_file(module)));
        body.push_str("<h2>Declarations</h2>\n");
        body.push_str(&self.decl_list(module, up));
        let pseudo = pseudo_module(f, module);
        if !pseudo.is_empty() {
            let _ = writeln!(body, "<h2>Pseudocode</h2>\n<pre>{}</pre>", h(&pseudo));
        }
        if let Some(m) = f.p.find_module(module)
            && let Some(first) = m.decls.first()
            && let Some(file) = f.p.sources.get(crate::facts::info(f.p, *first).span.file)
        {
            let _ = writeln!(body, "<h2>Source</h2>\n<pre>{}</pre>", h(&file.text));
        }
        self.page(module, up, &body)
    }

    fn notes(&self, i: usize, up: &str) -> String {
        let f = self.f;
        let p = f.p;
        let mut rows: Vec<(String, String)> = Vec::new();
        let code = |s: &str| format!("<code>{}</code>", h(s));
        match f.decls[i].r {
            DeclRef::Rule(id) => {
                let r = p.rule(id);
                for t in &r.triggers {
                    match t {
                        Trigger::Cli { command } => {
                            rows.push(("@on".to_string(), code(&format!("cli(\"{command}\")"))))
                        }
                    }
                }
                if !r.uses.is_empty() {
                    rows.push(("@uses".to_string(), code(&r.uses.join(", "))));
                }
                for g in &r.guards {
                    rows.push((
                        "@guard".to_string(),
                        format!("{} else <code>{}</code>", code(&f.src(g.cond.span)), h(&g.error)),
                    ));
                }
            }
            DeclRef::Fn(id) => {
                for c in &p.func(id).cases {
                    rows.push(("@cases".to_string(), code(&f.src(c.span))));
                }
            }
            DeclRef::Test(id) => {
                let t = p.test(id);
                for g in &t.given {
                    match g {
                        Given::State { state, value, .. } => {
                            let s =
                                f.index_of(DeclRef::State(*state)).map(|s| self.decl_link(s, up)).unwrap_or_default();
                            rows.push(("@given".to_string(), format!("{s} -> {}", code(&f.src(value.span)))));
                        }
                        Given::Test { test, .. } => {
                            let t = f.index_of(DeclRef::Test(*test)).map(|t| self.decl_link(t, up)).unwrap_or_default();
                            rows.push(("@given".to_string(), t));
                        }
                    }
                }
                for c in &t.cases {
                    rows.push(("@cases".to_string(), code(&f.src(c.span))));
                }
            }
            DeclRef::App(id) => {
                let a = p.app(id);
                if let Some(t) = &a.target {
                    rows.push(("@target".to_string(), code(t)));
                }
                if let Some(b) = &a.bin {
                    rows.push(("@bin".to_string(), code(b)));
                }
                match &a.store {
                    Some(Store::File(path)) => rows.push(("@store".to_string(), code(&format!("file(\"{path}\")")))),
                    None => {}
                }
            }
            DeclRef::Type(_) | DeclRef::Const(_) | DeclRef::State(_) | DeclRef::Law(_) => {}
        }
        if rows.is_empty() {
            return String::new();
        }
        let mut out = String::from("<h2>Notes</h2>\n<table>\n");
        for (k, v) in rows {
            let _ = writeln!(out, "<tr><th><code>{k}</code></th><td>{v}</td></tr>");
        }
        out.push_str("</table>\n");
        out
    }

    fn decl_page(&self, i: usize, shared: &Shared) -> String {
        let f = self.f;
        let d = &f.decls[i];
        let up = "../";
        let mut body = format!(
            "<p><a href=\"{up}{}\"><code>{}</code></a></p>\n<h1><span class=\"kind\">{}</span> <code>{}</code></h1>\n",
            self.module_href(&d.module),
            h(&d.module),
            d.kind.as_str(),
            h(&d.name)
        );
        let _ = writeln!(
            body,
            "<p class=\"dim\"><code>{}</code>{}</p>",
            h(&f.location(i)),
            if d.is_pub { " · pub" } else { "" }
        );
        if let Some(why) = &d.why {
            let _ = writeln!(body, "<p class=\"why\">{}</p>", h(why));
        }
        let _ = writeln!(body, "<h2>Source</h2>\n<pre>{}</pre>", h(f.p.snippet(d.span)));
        body.push_str(&self.notes(i, up));
        let pseudo = pseudo_decl(f, i);
        if !pseudo.is_empty() {
            let _ = writeln!(body, "<h2>Pseudocode</h2>\n<pre>{}</pre>", h(&pseudo));
        }

        // Diagrams.
        let mut diagrams = String::new();
        if d.kind == Kind::Type {
            let around = shared.classes.around(&d.id);
            let _ = writeln!(
                diagrams,
                "<h3>Class: {} classes, {} relations</h3>",
                around.classes.len(),
                around.relations.len()
            );
            diagrams.push_str(&Self::mermaid(&around.to_mermaid()));
        }
        for m in &shared.machines {
            let about = m.name.starts_with(&format!("{}.", d.qname))
                || m.name == d.qname
                || (d.kind == Kind::Type && m.ty == f.short(i))
                || m.transitions.iter().any(|t| t.rule == d.qname);
            if about {
                let _ = writeln!(
                    diagrams,
                    "<h3>State machine <code>{}</code>: {} nodes, {} edges</h3>",
                    h(&m.name),
                    m.node_count(),
                    m.transitions.len()
                );
                diagrams.push_str(&Self::mermaid(&m.to_mermaid()));
            }
        }
        if let DeclRef::Test(id) = d.r {
            let s = test_sequence(f, id);
            let _ = writeln!(
                diagrams,
                "<h3>Sequence: {} participants, {} messages</h3>",
                s.participants.len(),
                s.message_count()
            );
            diagrams.push_str(&Self::mermaid(&s.to_mermaid()));
        }
        if shared.calls.has_node(&d.id) {
            let n = neighbourhood(&shared.calls, &d.id);
            if !n.edges.is_empty() {
                let _ = writeln!(
                    diagrams,
                    "<h3>Calls, reads and writes: {} nodes, {} edges</h3>",
                    n.nodes.len(),
                    n.edges.len()
                );
                diagrams.push_str(&Self::mermaid(&n.to_mermaid("LR")));
            }
        }
        let imp = impact(f, i, self.o.provenance);
        if imp.total() > 0 {
            let _ = writeln!(
                diagrams,
                "<h3>Impact: {} nodes, {} edges</h3>\n<p class=\"dim\">An arrow points at what is depended on.</p>",
                imp.graph.nodes.len(),
                imp.graph.edges.len()
            );
            diagrams.push_str(&Self::mermaid(&imp.graph.to_mermaid("RL")));
        }
        if !diagrams.is_empty() {
            body.push_str("<h2>Diagrams</h2>\n");
            body.push_str(&diagrams);
        }

        if let Some(row) = shared.effects.rows.iter().find(|r| r.rule == d.qname) {
            body.push_str("<h2>Effects</h2>\n<table>\n");
            let _ = writeln!(body, "<tr><th>reads</th><td>{}</td></tr>", self.by_qname(&row.reads, up));
            let _ = writeln!(body, "<tr><th>writes</th><td>{}</td></tr>", self.by_qname(&row.writes, up));
            let _ = writeln!(body, "<tr><th>outside</th><td><code>{}</code></td></tr>", h(&row.outside.join(", ")));
            let races: Vec<String> = shared
                .effects
                .conflicts
                .iter()
                .filter(|c| c.a == d.qname || c.b == d.qname)
                .map(|c| if c.a == d.qname { c.b.clone() } else { c.a.clone() })
                .collect();
            let _ = writeln!(body, "<tr><th>order matters with</th><td>{}</td></tr>", self.by_qname(&races, up));
            body.push_str("</table>\n");
        }

        // What it names, by kind of reference.
        let mut names = String::new();
        let mut kinds: Vec<_> = d.refs.iter().filter(|r| r.phase != Phase::Cases).map(|r| r.kind).collect();
        kinds.sort_unstable();
        kinds.dedup();
        for k in kinds {
            let mut to: Vec<usize> = Vec::new();
            let mut out_too = false;
            for r in d.refs.iter().filter(|r| r.kind == k && r.phase != Phase::Cases) {
                match r.to {
                    Target::Decl(x) if !to.contains(&x) => to.push(x),
                    Target::Decl(_) => {}
                    Target::Out => out_too = true,
                }
            }
            let mut cell = if to.is_empty() { String::new() } else { self.links(&to, up) };
            if out_too {
                if !cell.is_empty() {
                    cell.push_str(", ");
                }
                cell.push_str("<code>out</code>");
            }
            let _ = writeln!(names, "<tr><th>{}</th><td>{cell}</td></tr>", k.as_str());
        }
        if !names.is_empty() {
            let _ = writeln!(body, "<h2>Names</h2>\n<table>\n{names}</table>");
        }

        body.push_str("<h2>Depends on it</h2>\n<table>\n");
        let _ = writeln!(body, "<tr><th>directly</th><td>{}</td></tr>", self.by_qname(&imp.direct, up));
        for (kind, label) in [
            (Kind::Test, "tests"),
            (Kind::Law, "laws"),
            (Kind::Rule, "rules"),
            (Kind::Fn, "functions"),
            (Kind::App, "apps"),
        ] {
            let _ = writeln!(body, "<tr><th>{label}</th><td>{}</td></tr>", self.by_qname(imp.of(kind), up));
        }
        let _ = writeln!(body, "<tr><th><code>@cases</code> of</th><td>{}</td></tr>", self.by_qname(&imp.cases, up));
        body.push_str("</table>\n");

        if !self.o.provenance.is_empty() {
            body.push_str("<h2>Emitted code</h2>\n");
            let mut any = false;
            body.push_str("<ul>\n");
            for prov in self.o.provenance {
                for span in prov.of_decl(&d.qname) {
                    any = true;
                    let text = span.text();
                    let role = if span.role.is_empty() {
                        String::new()
                    } else {
                        format!(" <span class=\"dim\">{}</span>", h(&span.role))
                    };
                    if prov.href.is_empty() {
                        let _ = writeln!(body, "<li>{}: <code>{}</code>{role}</li>", h(&prov.backend), h(&text));
                    } else {
                        let _ = writeln!(
                            body,
                            "<li>{}: <a href=\"{up}{}{}\"><code>{}</code></a>{role}</li>",
                            h(&prov.backend),
                            h(&prov.href),
                            h(&span.file),
                            h(&text)
                        );
                    }
                }
            }
            body.push_str("</ul>\n");
            if !any {
                body.push_str("<p class=\"dim\">No emitted span names this declaration.</p>\n");
            }
        }
        self.page(&d.qname, up, &body)
    }

    fn diagram_page(&self, title: &str, sections: &[(String, String)]) -> String {
        let mut body = format!("<h1>{}</h1>\n", h(title));
        for (heading, mermaid) in sections {
            if !heading.is_empty() {
                let _ = writeln!(body, "<h2>{heading}</h2>");
            }
            body.push_str(&Self::mermaid(mermaid));
        }
        self.page(title, "../", &body)
    }
}

/// What every declaration page reads: computed once.
struct Shared {
    classes: crate::class::ClassDiagram,
    machines: Vec<crate::state::Machine>,
    calls: crate::graph::Graph,
    effects: crate::effects::Effects,
}

/// Every page of the explorer, paths relative to the site's folder.
pub fn site(f: &Facts<'_>, o: &SiteOptions<'_>) -> Vec<File> {
    let mut ids = Ids::new();
    let module_ids = f.modules().into_iter().map(|m| (m, ids.fresh(m))).collect();
    let s = Site { f, o, module_ids };
    let shared = Shared {
        classes: class_diagram(f),
        machines: state_machines(f, o.flows),
        calls: call_graph(f, Scope::WithChecks),
        effects: effects(f),
    };
    let mut files = vec![File::new("style.css", STYLE), File::new("index.html", s.index())];

    let counted = |n: usize, e: usize| format!("{n} nodes, {e} edges");
    let c = &shared.classes;
    files.push(File::new(
        "diagrams/class.html",
        s.diagram_page("Class diagram", &[(counted(c.classes.len(), c.relations.len()), c.to_mermaid())]),
    ));
    let machines: Vec<(String, String)> = shared
        .machines
        .iter()
        .map(|m| {
            (format!("<code>{}</code>: {}", h(&m.name), counted(m.node_count(), m.transitions.len())), m.to_mermaid())
        })
        .collect();
    files.push(File::new("diagrams/state.html", s.diagram_page("State diagrams", &machines)));
    let seqs: Vec<(String, String)> = test_sequences(f)
        .iter()
        .map(|q| {
            let link =
                f.decls.iter().position(|d| d.qname == q.title).map_or_else(|| h(&q.title), |i| s.decl_link(i, "../"));
            (format!("{link}: {} participants, {} messages", q.participants.len(), q.message_count()), q.to_mermaid())
        })
        .collect();
    files.push(File::new("diagrams/sequence.html", s.diagram_page("Sequence diagrams", &seqs)));
    let g = module_graph(f);
    files.push(File::new(
        "diagrams/modules.html",
        s.diagram_page("Modules", &[(g.counts().sentence(), g.to_mermaid("LR"))]),
    ));
    let e = &shared.effects;
    let cg = e.conflict_graph(f);
    files.push(File::new(
        "diagrams/effects.html",
        s.diagram_page(
            "Effects",
            &[
                (e.graph.counts().sentence(), e.graph.to_mermaid("LR")),
                (format!("Rules whose order matters: {} pairs", e.conflicts.len()), cg.to_mermaid("LR")),
            ],
        ),
    ));
    let product = call_graph(f, Scope::Product);
    files.push(File::new(
        "diagrams/calls.html",
        s.diagram_page(
            "Call graph",
            &[
                (format!("The product: {}", product.counts().sentence()), product.to_mermaid("LR")),
                (format!("With tests and laws: {}", shared.calls.counts().sentence()), shared.calls.to_mermaid("LR")),
            ],
        ),
    ));

    let mut pseudo = String::from("<h1>Pseudocode</h1>\n");
    for m in f.modules() {
        let text = pseudo_module(f, m);
        if !text.is_empty() {
            let _ = writeln!(
                pseudo,
                "<h2><a href=\"{}\"><code>{}</code></a></h2>\n<pre>{}</pre>",
                s.module_href(m),
                h(m),
                h(&text)
            );
        }
    }
    files.push(File::new("pseudo.html", s.page("Pseudocode", "", &pseudo)));

    for m in f.modules() {
        files.push(File::new(s.module_href(m), s.module_page(m)));
    }
    for i in 0..f.decls.len() {
        files.push(File::new(s.decl_href(i), s.decl_page(i, &shared)));
    }
    files
}
