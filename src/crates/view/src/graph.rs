//! The node-and-edge model behind the module, effect, call and impact views,
//! with its Mermaid flowchart and JSON printers.

use crate::json::Json;
use crate::text;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// An exact graph: what is printed is all of it, and the counts say how much.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graph {
    /// The view that made it: `modules`, `effects`, `calls`, `impact`, ...
    pub view: String,
    pub title: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// Unique in the graph; ASCII letters, digits and `_`.
    pub id: String,
    pub label: String,
    /// `module`, `rule`, `fn`, `state`, `type`, `test`, `law`, `app`, `target`, ...
    pub kind: String,
    /// The box it is drawn in (a module name); empty for none.
    pub group: String,
    /// The qualified name of the declaration it stands for; empty for none.
    pub decl: String,
    /// Drawn highlighted: the subject of the view, or a contended place.
    pub hot: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    /// `calls`, `reads`, `writes`, `constructs`, `uses`, `owns`, ...
    pub kind: String,
    /// Extra text after the kind; empty for none.
    pub label: String,
}

/// How many nodes and edges, in total and by kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counts {
    pub nodes: usize,
    pub edges: usize,
    pub nodes_by_kind: BTreeMap<String, usize>,
    pub edges_by_kind: BTreeMap<String, usize>,
}

impl Counts {
    pub fn to_json(&self) -> Json {
        let by = |m: &BTreeMap<String, usize>| Json::Obj(m.iter().map(|(k, n)| (k.clone(), Json::count(*n))).collect());
        Json::obj([
            ("nodes", Json::count(self.nodes)),
            ("edges", Json::count(self.edges)),
            ("nodes_by_kind", by(&self.nodes_by_kind)),
            ("edges_by_kind", by(&self.edges_by_kind)),
        ])
    }

    /// `12 nodes (8 rule, 2 state, ...), 20 edges (...)`.
    pub fn sentence(&self) -> String {
        let list =
            |m: &BTreeMap<String, usize>| m.iter().map(|(k, n)| format!("{n} {k}")).collect::<Vec<_>>().join(", ");
        let part = |n: usize, word: &str, m: &BTreeMap<String, usize>| {
            if m.is_empty() { format!("{n} {word}") } else { format!("{n} {word} ({})", list(m)) }
        };
        format!(
            "{}, {}",
            part(self.nodes, "nodes", &self.nodes_by_kind),
            part(self.edges, "edges", &self.edges_by_kind)
        )
    }
}

impl Graph {
    pub fn new(view: &str, title: impl Into<String>) -> Self {
        Graph { view: view.to_string(), title: title.into(), nodes: Vec::new(), edges: Vec::new() }
    }

    pub fn node(&mut self, id: &str, label: &str, kind: &str, group: &str, decl: &str) {
        if !self.has_node(id) {
            self.nodes.push(Node {
                id: id.to_string(),
                label: label.to_string(),
                kind: kind.to_string(),
                group: group.to_string(),
                decl: decl.to_string(),
                hot: false,
            });
        }
    }

    pub fn has_node(&self, id: &str) -> bool {
        self.nodes.iter().any(|n| n.id == id)
    }

    /// Add an edge once: a second edge with the same ends, kind and label is the same edge.
    pub fn edge(&mut self, from: &str, to: &str, kind: &str, label: &str) {
        let e = Edge { from: from.to_string(), to: to.to_string(), kind: kind.to_string(), label: label.to_string() };
        if !self.edges.contains(&e) {
            self.edges.push(e);
        }
    }

    pub fn mark_hot(&mut self, id: &str) {
        for n in self.nodes.iter_mut().filter(|n| n.id == id) {
            n.hot = true;
        }
    }

    pub fn counts(&self) -> Counts {
        let mut nodes_by_kind = BTreeMap::new();
        for n in &self.nodes {
            *nodes_by_kind.entry(n.kind.clone()).or_insert(0) += 1;
        }
        let mut edges_by_kind = BTreeMap::new();
        for e in &self.edges {
            *edges_by_kind.entry(e.kind.clone()).or_insert(0) += 1;
        }
        Counts { nodes: self.nodes.len(), edges: self.edges.len(), nodes_by_kind, edges_by_kind }
    }

    pub fn to_json(&self) -> Json {
        let nodes = self.nodes.iter().map(|n| {
            let mut f = vec![("id", Json::str(&n.id)), ("kind", Json::str(&n.kind)), ("label", Json::str(&n.label))];
            if !n.group.is_empty() {
                f.push(("group", Json::str(&n.group)));
            }
            if !n.decl.is_empty() {
                f.push(("decl", Json::str(&n.decl)));
            }
            if n.hot {
                f.push(("hot", Json::Bool(true)));
            }
            Json::obj(f)
        });
        let edges = self.edges.iter().map(|e| {
            let mut f = vec![("from", Json::str(&e.from)), ("to", Json::str(&e.to)), ("kind", Json::str(&e.kind))];
            if !e.label.is_empty() {
                f.push(("label", Json::str(&e.label)));
            }
            Json::obj(f)
        });
        Json::obj([
            ("view", Json::str(&self.view)),
            ("title", Json::str(&self.title)),
            ("counts", self.counts().to_json()),
            ("nodes", Json::Arr(nodes.collect())),
            ("edges", Json::Arr(edges.collect())),
        ])
    }

    /// A Mermaid flowchart: one box per group, a shape per node kind, an arrow per edge kind.
    pub fn to_mermaid(&self, direction: &str) -> String {
        let mut out = format!("flowchart {direction}\n");
        let mut groups: Vec<&str> = Vec::new();
        for n in &self.nodes {
            if !groups.contains(&n.group.as_str()) {
                groups.push(&n.group);
            }
        }
        let mut ids = text::Ids::new();
        for n in &self.nodes {
            ids.fresh(&n.id);
        }
        for g in groups {
            let pad = if g.is_empty() { "  " } else { "    " };
            if !g.is_empty() {
                let _ = writeln!(out, "  subgraph {}[\"{}\"]", ids.fresh(&format!("g_{g}")), text::mermaid(g));
            }
            for n in self.nodes.iter().filter(|n| n.group == g) {
                let (open, close) = shape(&n.kind);
                let _ = writeln!(out, "{pad}{}{open}\"{}\"{close}", n.id, text::mermaid(&n.label));
            }
            if !g.is_empty() {
                out.push_str("  end\n");
            }
        }
        for e in &self.edges {
            let label = if e.label.is_empty() { e.kind.clone() } else { format!("{} {}", e.kind, e.label) };
            let _ = writeln!(out, "  {} {}|\"{}\"| {}", e.from, arrow(&e.kind), text::mermaid(&label), e.to);
        }
        let mut kinds: Vec<&str> = self.nodes.iter().map(|n| n.kind.as_str()).collect();
        kinds.sort_unstable();
        kinds.dedup();
        for k in kinds {
            if let Some(style) = class_style(k) {
                let members: Vec<&str> = self.nodes.iter().filter(|n| n.kind == k).map(|n| n.id.as_str()).collect();
                let _ = writeln!(out, "  classDef {k} {style}");
                let _ = writeln!(out, "  class {} {k}", members.join(","));
            }
        }
        let hot: Vec<&str> = self.nodes.iter().filter(|n| n.hot).map(|n| n.id.as_str()).collect();
        if !hot.is_empty() {
            out.push_str("  classDef hot stroke:#dc2626,stroke-width:4px\n");
            let _ = writeln!(out, "  class {} hot", hot.join(","));
        }
        out
    }
}

/// The Mermaid brackets that draw a node of this kind.
fn shape(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "rule" => ("([", "])"),
        "state" => ("[(", ")]"),
        "type" => ("{{", "}}"),
        "test" => ("[/", "/]"),
        "law" | "cases" => ("[\\", "\\]"),
        "app" => ("[[", "]]"),
        "target" | "vocabulary" => ("((", "))"),
        "effect" | "file" => (">", "]"),
        _ => ("[", "]"),
    }
}

fn arrow(kind: &str) -> &'static str {
    match kind {
        "reads" | "uses" | "matches" => "-.->",
        "writes" | "sets" | "owns" => "==>",
        "constructs" => "--o",
        "can race" => "<-->",
        _ => "-->",
    }
}

/// Strokes only: fills are left to the theme so the diagram reads in light and dark.
fn class_style(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "rule" => "stroke:#2563eb,stroke-width:2px",
        "state" => "stroke:#d97706,stroke-width:2px",
        "type" => "stroke:#7c3aed,stroke-width:1px",
        "test" | "cases" => "stroke:#059669,stroke-width:1px",
        "law" => "stroke:#059669,stroke-width:2px",
        "app" | "target" => "stroke:#db2777,stroke-width:2px",
        "effect" | "file" => "stroke:#6b7280,stroke-width:1px,stroke-dasharray:4 3",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Graph {
        let mut g = Graph::new("calls", "sample");
        g.node("m_pay", "pay", "rule", "bank", "bank.pay");
        g.node("m_accounts", "accounts", "state", "bank", "bank.accounts");
        g.node("fx_out", "out", "effect", "", "");
        g.edge("m_pay", "m_accounts", "writes", "");
        g.edge("m_pay", "m_accounts", "writes", "");
        g.edge("m_pay", "fx_out", "emits", "line");
        g.mark_hot("m_accounts");
        g
    }

    #[test]
    fn mermaid_flowchart_is_golden() {
        let want = "flowchart LR
  subgraph g_bank[\"bank\"]
    m_pay([\"pay\"])
    m_accounts[(\"accounts\")]
  end
  fx_out>\"out\"]
  m_pay ==>|\"writes\"| m_accounts
  m_pay -->|\"emits line\"| fx_out
  classDef effect stroke:#6b7280,stroke-width:1px,stroke-dasharray:4 3
  class fx_out effect
  classDef rule stroke:#2563eb,stroke-width:2px
  class m_pay rule
  classDef state stroke:#d97706,stroke-width:2px
  class m_accounts state
  classDef hot stroke:#dc2626,stroke-width:4px
  class m_accounts hot
";
        assert_eq!(sample().to_mermaid("LR"), want);
    }

    #[test]
    fn json_graph_is_golden_and_counts_what_it_holds() {
        let g = sample();
        let c = g.counts();
        assert_eq!((c.nodes, c.edges), (3, 2));
        assert_eq!(c.sentence(), "3 nodes (1 effect, 1 rule, 1 state), 2 edges (1 emits, 1 writes)");
        let want = "{
  \"view\": \"calls\",
  \"title\": \"sample\",
  \"counts\": {
    \"nodes\": 3,
    \"edges\": 2,
    \"nodes_by_kind\": {\"effect\": 1, \"rule\": 1, \"state\": 1},
    \"edges_by_kind\": {\"emits\": 1, \"writes\": 1}
  },
  \"nodes\": [
    {\"id\": \"m_pay\", \"kind\": \"rule\", \"label\": \"pay\", \"group\": \"bank\", \"decl\": \"bank.pay\"},
    {\"id\": \"m_accounts\", \"kind\": \"state\", \"label\": \"accounts\", \"group\": \"bank\", \"decl\": \"bank.accounts\", \"hot\": true},
    {\"id\": \"fx_out\", \"kind\": \"effect\", \"label\": \"out\"}
  ],
  \"edges\": [
    {\"from\": \"m_pay\", \"to\": \"m_accounts\", \"kind\": \"writes\"},
    {\"from\": \"m_pay\", \"to\": \"fx_out\", \"kind\": \"emits\", \"label\": \"line\"}
  ]
}
";
        assert_eq!(g.to_json().pretty(), want);
    }
}
