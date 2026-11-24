//! `provenance.json`: each printed item's lines, with the declaration it comes from and where that is written. Read back
//! from the formatted files, so the lines are the ones on disk: an item's doc comment ends with
//! ``Rule `todo.board.add`, `src/todo/board.slg:29`.``

use spec_lang_diag::JsonWriter;
use spec_lang_lir::OutFile;
use syn::spanned::Spanned;

/// The kind, declaration and origin a doc comment names.
fn marker(attrs: &[syn::Attribute]) -> Option<(String, String, String)> {
    attrs.iter().rev().find_map(|a| {
        let syn::Meta::NameValue(nv) = &a.meta else { return None };
        let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value else { return None };
        let text = s.value();
        let (kind, rest) = text.trim().split_once(" `")?;
        let (decl, rest) = rest.split_once("`, `")?;
        let (origin, _) = rest.split_once("`.")?;
        origin.contains(".slg:").then(|| (kind.to_string(), decl.to_string(), origin.to_string()))
    })
}

fn attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(i) => &i.attrs,
        syn::Item::Enum(i) => &i.attrs,
        syn::Item::Fn(i) => &i.attrs,
        syn::Item::Static(i) => &i.attrs,
        syn::Item::Struct(i) => &i.attrs,
        syn::Item::Type(i) => &i.attrs,
        _ => &[],
    }
}

/// The file for every item of `files` that names its declaration, in path and line order.
pub fn file(files: &[OutFile]) -> OutFile {
    let mut sorted: Vec<&OutFile> = files.iter().filter(|f| f.path.ends_with(".rs")).collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    let mut spans = Vec::new();
    for f in sorted {
        let Ok(text) = std::str::from_utf8(&f.bytes) else { continue };
        let Ok(parsed) = syn::parse_file(text) else { continue };
        for item in &parsed.items {
            let Some((kind, decl, origin)) = marker(attrs(item)) else { continue };
            let start = attrs(item).first().map_or(item.span(), Spanned::span).start().line;
            let entry = JsonWriter::obj()
                .set("file", JsonWriter::str(&f.path))
                .set("start", JsonWriter::Int(start as i128))
                .set("end", JsonWriter::Int(item.span().end().line as i128))
                .set("kind", JsonWriter::str(kind.to_lowercase()))
                .set("decl", JsonWriter::str(&decl))
                .set("origin", JsonWriter::str(&origin));
            spans.push(entry.to_string());
        }
    }
    let text = format!("{{\"spans\":[\n{}\n]}}\n", spans.join(",\n"));
    OutFile { path: "provenance.json".into(), bytes: text.into_bytes(), executable: false }
}
