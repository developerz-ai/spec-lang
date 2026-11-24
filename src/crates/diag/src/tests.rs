use super::*;
use spec_lang_syntax::{SourceFile, SourceMap};

#[test]
fn human_and_json_carry_the_same_facts() {
    let mut map = SourceMap::new();
    let id = map.add(SourceFile::new("m.slg", "rule r() = { x = 1; }\n", 1));
    let d = Diag::error("E-NAME-UNKNOWN", Span::new(id, 13, 14), "no name `x` in scope")
        .with_decl("m.r")
        .with_at("body.stmt[0]")
        .with_edit(Span::new(id, 13, 14), "y");
    let h = d.human(&map);
    assert!(h.contains("--> m.slg:1:14-1:15  in m.r body.stmt[0]"), "{h}");
    assert!(h.contains("^"), "{h}");
    let j = d.json(&map).to_string();
    assert!(j.contains(r#""decl":"m.r""#), "{j}");
    assert!(j.contains(r#""start":{"line":1,"col":14}"#), "{j}");
}

/// Every code used in the workspace sources is in `codes.md`.
#[test]
fn every_code_is_registered() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut used = std::collections::BTreeSet::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                if !p.ends_with("target") {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "rs") {
                let text = std::fs::read_to_string(&p).unwrap();
                for (i, _) in text.match_indices('"') {
                    let rest = &text[i + 1..];
                    let end = rest.find('"').unwrap_or(0);
                    let s = &rest[..end];
                    let looks = (s.starts_with("E-") || s.starts_with("W-") || s.starts_with("F-"))
                        && s.len() > 2
                        && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-');
                    if looks {
                        used.insert(s.to_string());
                    }
                }
            }
        }
    }
    let missing: Vec<_> = used.iter().filter(|c| !CODES.contains(&format!("`{c}`"))).collect();
    assert!(missing.is_empty(), "codes missing from codes.md: {missing:?}");
}
