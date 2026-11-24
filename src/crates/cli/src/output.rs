//! One writer for both renderings: text for people, JSON lines for agents.

use spec_lang_diag::{Diag, JsonWriter};
use spec_lang_syntax::SourceMap;
use spec_lang_testing::Report;
use std::io::Write;

pub struct Out {
    json: bool,
}

impl Out {
    pub fn new(json: bool) -> Self {
        Out { json }
    }

    /// A stage's verdict: `check   ok   ...`.
    pub fn stage(&mut self, stage: &str, status: &str, detail: &str) {
        if self.json {
            let j = JsonWriter::obj()
                .set("stage", JsonWriter::str(stage))
                .set("status", JsonWriter::str(status))
                .set("detail", JsonWriter::str(detail));
            emit(&format!("{j}\n"));
        } else {
            emit(&format!("{stage:<7} {status:<8} {detail}\n"));
        }
    }

    pub fn diags(&mut self, stage: &str, diags: &[Diag], map: &SourceMap) {
        for d in diags {
            if self.json {
                let j = match d.json(map) {
                    JsonWriter::Obj(mut items) => {
                        items.insert(0, ("stage".into(), JsonWriter::str(stage)));
                        JsonWriter::Obj(items)
                    }
                    other => other,
                };
                emit(&format!("{j}\n"));
            } else {
                emit(&indent(&d.human(map)));
            }
        }
    }

    pub fn report(&mut self, r: &Report, map: &SourceMap) {
        if self.json {
            for j in r.json_lines(map) {
                emit(&format!("{j}\n"));
            }
            let s = JsonWriter::obj()
                .set("stage", JsonWriter::str("summary"))
                .set("checks", JsonWriter::Int(r.checks.len() as i128))
                .set("passed", JsonWriter::Int(r.count(spec_lang_testing::Status::Pass) as i128))
                .set("failed", JsonWriter::Int(r.count(spec_lang_testing::Status::Fail) as i128))
                .set("blocked", JsonWriter::Int(r.count(spec_lang_testing::Status::Blocked) as i128))
                .set("micros", JsonWriter::Int(r.micros as i128));
            emit(&format!("{s}\n"));
        } else {
            emit(&r.human(map));
            emit(&format!("{}\n", r.summary()));
        }
    }
}

fn indent(s: &str) -> String {
    s.lines().map(|l| format!("  {l}\n")).collect()
}

/// Write to standard output; a reader that went away (`| head`) is not an error.
pub fn emit(s: &str) {
    let _ = std::io::stdout().lock().write_all(s.as_bytes());
}
