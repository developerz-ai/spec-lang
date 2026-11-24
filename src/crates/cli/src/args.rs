//! Option parsing for `spec-lang test`.

/// Options of `spec-lang test`.
#[derive(Debug)]
pub struct TestArgs {
    pub project: String,
    pub seed: u64,
    pub runs: u32,
    pub traces: u32,
    pub steps: u32,
    /// Worker threads; results do not depend on it.
    pub jobs: usize,
    /// One JSON object per line instead of text.
    pub json: bool,
    /// Lint warnings fail the run.
    pub deny_warnings: bool,
}

pub fn parse_test_args(args: &[String]) -> Result<TestArgs, String> {
    let mut project = None;
    let d = spec_lang_testing::Config::default();
    let mut out = TestArgs {
        project: String::new(),
        seed: d.seed,
        runs: d.runs,
        traces: d.traces,
        steps: d.steps,
        jobs: d.jobs,
        json: false,
        deny_warnings: false,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| -> Result<u64, String> {
            let v = it.next().ok_or_else(|| format!("{name} needs a value"))?;
            v.parse::<u64>().map_err(|_| format!("{name}: not a number: {v}"))
        };
        match a.as_str() {
            "--seed" => out.seed = value("--seed")?,
            "--runs" => out.runs = narrow(value("--runs")?, "--runs")?,
            "--traces" => out.traces = narrow(value("--traces")?, "--traces")?,
            "--steps" => out.steps = narrow(value("--steps")?, "--steps")?,
            "--jobs" => {
                out.jobs = usize::try_from(value("--jobs")?).map_err(|_| "--jobs: too large".to_string())?.max(1)
            }
            "--json" => out.json = true,
            "--deny" => match it.next().map(String::as_str) {
                Some("warnings") => out.deny_warnings = true,
                other => return Err(format!("--deny takes `warnings`, found {other:?}")),
            },
            s if s.starts_with("--") => return Err(format!("unknown flag {s}")),
            s if project.is_none() => project = Some(s.to_string()),
            s => return Err(format!("unexpected argument {s:?}")),
        }
    }
    out.project = project.ok_or("missing <project>")?;
    Ok(out)
}

fn narrow(v: u64, name: &str) -> Result<u32, String> {
    u32::try_from(v).map_err(|_| format!("{name}: too large"))
}
