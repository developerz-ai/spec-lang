//! Each view on a program small enough to check by eye.

// A helper that cannot load its fixture has nothing to return: failing the test is the handling.
#![allow(clippy::panic)]

use view::facts::Facts;

const LAMPS: &str = r#"
type Light = Red | Green;
type Lamp(id: Int, light: Light = Red);

state lamps: Map[Int, Lamp];
state count: Int = 0;

fn label(l: Lamp) -> Text = "lamp {l.id}";

fn total() -> Int = count;

@guard(id in lamps else NoLamp, lamps[id].light == Red else AlreadyGreen)
rule go(id: Int) = {
  lamps[id].light = Green;
}

@guard(id in lamps else NoLamp)
rule stop(id: Int) = {
  lamps[id].light = Red;
}

rule install(id: Int) = {
  lamps[id] = Lamp(id);
  count = count + 1;
}

rule scrap(id: Int) = {
  lamps.remove(id);
}

test green_once = {
  install(1);
  go(1);
  go(1) -> fail AlreadyGreen;
  assert total() == 1;
}
"#;

fn program() -> spec_lang_ir::Program {
    let sources = vec![("street".to_string(), "street.slg".to_string(), LAMPS.to_string())];
    spec_lang_ir::load_sources(sources).unwrap_or_else(|e| panic!("{}", e.render()))
}

#[test]
fn state_machine_is_read_off_assignments_and_guards() {
    let p = program();
    let f = Facts::of(&p);
    let machines = view::state::state_machines(&f, &[]);
    assert_eq!(machines.len(), 1);
    let want = "stateDiagram-v2
  state \"street.Lamp.light #58; Light\" as machine {
    state \"Red\" as Red
    state \"Green\" as Green
    Red --> Green : go [id in lamps]
    Red --> Red : stop [id in lamps]
    Green --> Red : stop [id in lamps]
    [*] --> Red : install
  }
  machine --> [*] : scrap
";
    assert_eq!(machines[0].to_mermaid(), want);
    assert_eq!((machines[0].node_count(), machines[0].transitions.len()), (3, 5));
}

#[test]
fn class_diagram_has_fields_defaults_and_the_state_association() {
    let p = program();
    let d = view::class::class_diagram(&Facts::of(&p));
    let want = "classDiagram
  class street_Light[\"Light\"] {
    <<enumeration>>
    Red
    Green
  }
  class street_Lamp[\"Lamp\"] {
    <<record>>
    id: Int
    light: Light = Red
  }
  class street[\"street\"] {
    <<module>>
    count: Int = 0
    -go(id: Int)
    -stop(id: Int)
    -install(id: Int)
    -scrap(id: Int)
  }
  street_Lamp \"1\" *-- \"1\" street_Light : light
  street \"1\" o-- \"0..*\" street_Lamp : lamps [Int]
";
    assert_eq!(d.to_mermaid(), want);
    assert!(d.to_plantuml().contains("street \"1\" o-- \"0..*\" street_Lamp : lamps [Int]\n"));
}

#[test]
fn effects_list_the_pairs_whose_order_matters() {
    let p = program();
    let f = Facts::of(&p);
    let e = view::effects::effects(&f);
    let install = e.rows.iter().find(|r| r.rule == "street.install").unwrap();
    assert_eq!(install.reads, ["street.count"]);
    assert_eq!(install.writes, ["street.lamps", "street.count"]);
    // Private rules have no @uses: everything found is undeclared.
    assert_eq!(install.undeclared, ["lamps.write", "count.write"]);
    // Four writers of `lamps`: every pair of them, and each with itself.
    assert_eq!(e.conflicts.len(), 10);
    let both = e.conflicts.iter().find(|c| c.a == "street.install" && c.b == "street.install").unwrap();
    assert_eq!(both.states, ["street.lamps", "street.count"]);
}

#[test]
fn sequence_shows_guard_reads_only_for_a_refusal() {
    let p = program();
    let f = Facts::of(&p);
    let seqs = view::sequence::test_sequences(&f);
    let want = "sequenceDiagram
  actor caller as green_once
  participant m_street as street
  participant s_lamps as lamps
  participant s_count as count
  caller->>m_street: install(1)
  m_street->>s_lamps: write
  m_street->>s_count: read, write
  m_street-->>caller: ok
  caller->>m_street: go(1)
  m_street->>s_lamps: read, write
  m_street-->>caller: ok
  caller->>m_street: go(1)
  m_street->>s_lamps: read
  m_street--xcaller: fail AlreadyGreen
  Note over caller: assert total() == 1
";
    assert_eq!(seqs[0].to_mermaid(), want);
}

#[test]
fn impact_follows_names_transitively() {
    let p = program();
    let f = Facts::of(&p);
    let count = f.find("count").unwrap();
    let i = view::impact::impact(&f, count, &[]);
    assert_eq!(i.direct, ["street.total", "street.install"]);
    assert_eq!(i.of(view::facts::Kind::Test), ["street.green_once"]);
    let light = view::impact::impact(&f, f.find("street.Light").unwrap(), &[]);
    assert_eq!(light.of(view::facts::Kind::Rule), ["street.go", "street.stop", "street.install", "street.scrap"]);
    assert!(f.find("nothing").is_err());
}

#[test]
fn call_graph_counts_what_it_draws() {
    let p = program();
    let f = Facts::of(&p);
    let g = view::calls::call_graph(&f, view::calls::Scope::Product);
    let c = g.counts();
    assert_eq!(c.nodes, g.nodes.len());
    assert_eq!(c.edges, g.edges.len());
    assert_eq!(c.edges_by_kind["writes"], 5);
    assert!(g.edges.iter().any(|e| e.from == "street_total" && e.to == "street_count" && e.kind == "reads"));
    assert!(g.to_json().pretty().contains("\"counts\""));
}

#[test]
fn pseudocode_puts_guards_first() {
    let p = program();
    let text = view::pseudo(&p, None).unwrap();
    let want = "rule go(id: Int)
  require id in lamps else fail NoLamp
  require lamps[id].light = Red else fail AlreadyGreen
  lamps[id].light ← Green
end
";
    assert!(text.contains(want), "{text}");
    assert!(text.contains("function label(l: Lamp) → Text\n  return \"lamp {l.id}\"\nend\n"), "{text}");
}

#[test]
fn provenance_reaches_impact_and_the_site() {
    let p = program();
    let f = Facts::of(&p);
    let prov = view::provenance::Provenance::parse(
        r#"{"target": "rust", "spans": [{"file": "src/go.rs", "start": 1, "end": 9, "decl": "street.go", "role": "rule body"}]}"#,
        "rust",
        "../../out/rust/",
    )
    .unwrap();
    let provs = [prov];
    let i = view::impact::impact(&f, f.find("street.Light").unwrap(), &provs);
    assert_eq!(i.files.len(), 1);
    assert_eq!(i.files[0].at, "src/go.rs:1-9");
    let options = view::site::SiteOptions { provenance: &provs, ..view::site::SiteOptions::new("lamps") };
    let pages = view::site::site(&f, &options);
    let go = pages.iter().find(|x| x.path == "decl/street_go.html").unwrap();
    assert!(go.contents.contains(
        "<a href=\"../../../out/rust/src/go.rs\"><code>src/go.rs:1-9</code></a> <span class=\"dim\">rule body</span>"
    ));
    assert!(pages.iter().any(|x| x.path == "index.html"));
}
