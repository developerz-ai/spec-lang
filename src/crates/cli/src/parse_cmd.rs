//! `spec-lang parse`: form only, as the Lark checker does it.

use crate::sources::collect;
use spec_lang_syntax::{SourceFile, SourceMap, parse_file};

pub fn run(args: &[String]) -> u8 {
    if args.is_empty() {
        eprintln!("usage: spec-lang parse <path>...");
        return 2;
    }
    let sources = match collect(args) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let mut map = SourceMap::new();
    let mut failed = 0;
    let checked = sources.len();
    for s in sources {
        let text = s.text.clone();
        let id = map.add(SourceFile::new(s.path, s.text, s.first_line));
        if let Err(d) = parse_file(id, &text) {
            failed += 1;
            eprintln!("{}", d.render(&map));
        }
    }
    crate::output::emit(&format!("slg: {checked} blocks, {failed} failed\n"));
    if failed > 0 || checked == 0 { 1 } else { 0 }
}
