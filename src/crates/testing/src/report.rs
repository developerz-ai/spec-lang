//! Results, and their human and JSON renderings.

use crate::config::Config;
use spec_lang_diag::{Diag, JsonWriter};
use spec_lang_syntax::SourceMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Case,
    Test,
    Property,
    Law,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Case => "case",
            Kind::Test => "test",
            Kind::Property => "property",
            Kind::Law => "law",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    Blocked,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "FAIL",
            Status::Blocked => "blocked",
        }
    }
}

/// How a check is known (card, Evidence). An example is one run; sampling
/// is many runs; neither is `proven`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// One run of a written example.
    Example,
    /// `n` generated runs.
    Sampled(u32),
    /// `traces` random traces of up to `steps` steps, `lawful` of them from
    /// a random world the types and laws allow, the rest from reached worlds.
    Traces { traces: u32, steps: u32, lawful: u32 },
}

impl std::fmt::Display for Evidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Evidence::Example => write!(f, "example"),
            Evidence::Sampled(n) => write!(f, "sampled({n})"),
            Evidence::Traces { traces, steps, lawful } => {
                write!(f, "sampled({traces} traces x {steps} steps, {lawful} from lawful worlds)")
            }
        }
    }
}

/// One check: a case row, a scenario, a property or a law.
#[derive(Clone, Debug)]
pub struct CheckResult {
    pub module: String,
    pub kind: Kind,
    /// Declaration path: `todo.board.add`, `todo.laws.numbers_below_next`.
    pub decl: String,
    /// What was checked, in source terms: `"milk" -> true`.
    pub label: String,
    pub status: Status,
    pub evidence: Evidence,
    pub diag: Option<Diag>,
    pub micros: u128,
}

/// Every check of a run.
#[derive(Debug)]
pub struct Report {
    pub config: Config,
    pub checks: Vec<CheckResult>,
    pub micros: u128,
}

impl Report {
    pub fn count(&self, s: Status) -> usize {
        self.checks.iter().filter(|c| c.status == s).count()
    }

    pub fn green(&self) -> bool {
        self.checks.iter().all(|c| c.status == Status::Pass)
    }

    /// The header line naming every count and the seed.
    pub fn settings(&self) -> String {
        let c = &self.config;
        format!(
            "seed {}; properties {} runs; laws {} traces x {} steps, even ones from a start reached by up to {} random calls, odd ones from a random world the types and laws allow",
            c.seed, c.runs, c.traces, c.steps, c.prefix
        )
    }

    pub fn summary(&self) -> String {
        format!(
            "{} checks: {} passed, {} failed, {} blocked in {:.2}s",
            self.checks.len(),
            self.count(Status::Pass),
            self.count(Status::Fail),
            self.count(Status::Blocked),
            self.micros as f64 / 1e6
        )
    }

    /// One line per check, grouped by module; failures followed by their diagnostic.
    pub fn human(&self, map: &SourceMap) -> String {
        let mut out = String::new();
        let mut module = "";
        let width = self.checks.iter().map(|c| c.label.chars().count() + c.decl.len()).max().unwrap_or(0).min(90);
        for c in &self.checks {
            if c.module != module {
                module = &c.module;
                out.push_str(&format!("  {module}\n"));
            }
            let what = format!("{} {}", c.decl.rsplit('.').next().unwrap_or(""), c.label);
            out.push_str(&format!(
                "    {:<7} {:<8} {:<w$}  {}\n",
                c.status.as_str(),
                c.kind.as_str(),
                what.trim_end(),
                c.evidence,
                w = width.max(20)
            ));
            if let Some(d) = &c.diag {
                for line in d.human(map).lines() {
                    out.push_str(&format!("        {line}\n"));
                }
            }
        }
        out
    }

    /// One JSON object per check.
    pub fn json_lines(&self, map: &SourceMap) -> Vec<JsonWriter> {
        self.checks
            .iter()
            .map(|c| {
                JsonWriter::obj()
                    .set("stage", JsonWriter::str("test"))
                    .set("kind", JsonWriter::str(c.kind.as_str()))
                    .set("decl", JsonWriter::str(&c.decl))
                    .set("label", JsonWriter::str(&c.label))
                    .set("status", JsonWriter::str(c.status.as_str().to_lowercase()))
                    .set("evidence", JsonWriter::str(c.evidence.to_string()))
                    .set("micros", JsonWriter::Int(c.micros as i128))
                    .set("diagnostic", c.diag.as_ref().map_or(JsonWriter::Null, |d| d.json(map)))
            })
            .collect()
    }
}
