//! Notes and triggers as text, for comments and for the manifest an app's library runtime reads.

use spec_lang_lir::manifest::{Note, NoteArg, NoteValue};

fn value_text(v: &NoteValue) -> String {
    match v {
        NoteValue::Int(n) => n.to_string(),
        NoteValue::Text(t) => format!("\"{t}\""),
        NoteValue::Quantity { value, unit } => format!("{value}{unit}"),
        NoteValue::Variant { name, args } => format!("{name}({})", args_text(args)),
    }
}

fn args_text(args: &[NoteArg]) -> String {
    args.iter().map(|a| value_text(&a.value)).collect::<Vec<_>>().join(", ")
}

/// `cli("add")`, `tick(60Hz)`.
pub fn note_text(n: &Note) -> String {
    format!("{}({})", n.name, args_text(&n.args))
}
