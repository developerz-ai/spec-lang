//! An app's project: the program, the Ruby runtime of each library the app uses (copied from
//! `libs/<name>/real/ruby/`), and the glue that hands the first of them the manifest. Which library runs an
//! app, what its executable is called and which gems it needs are that library's to say, in its `runtime.conf`.

use crate::code;
use crate::decls::HEADER;
use crate::gen_::{Gen, R};
use crate::project::{self, script, text};
use spec_lang_diag::{Diag, Severity};
use spec_lang_ir::LibPath;
use spec_lang_lir::manifest::NoteValue;
use spec_lang_lir::{Body, Manifest, OutFile};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The language's folder under a library's `real/`.
const LANG: &str = "ruby";
/// The one file of a runtime that is read here and not copied.
const CONF: &str = "runtime.conf";

/// What a library's `runtime.conf` says.
#[derive(Debug, Default)]
struct Runtime {
    library: String,
    /// The file the glue requires, under `lib/`.
    entry: String,
    /// The Ruby module that file defines.
    module: String,
    /// `note.param`: the app note that names the executable. Only a library that can run an app has one.
    executable: Option<(String, String)>,
    /// `name version` per gem the runtime needs when the app runs.
    gems: Vec<(String, String)>,
    files: Vec<OutFile>,
}

fn problem(message: String) -> Diag {
    Diag::new(Severity::Error, "E-UNSUPPORTED", format!("unsupported: {message}"))
}

/// Every library the app needs a runtime of: its shape's, its notes', its rules' triggers', and each a
/// foreign declaration names. The one that can run the app comes first.
fn libraries(g: &Gen<'_>, app: &Manifest) -> Vec<String> {
    let lir = g.lir;
    let mut names: Vec<String> = Vec::new();
    let mut add = |n: &str| {
        if !n.is_empty() && !names.iter().any(|x| x == n) {
            names.push(n.to_string());
        }
    };
    if let Some(shape) = &app.shape {
        add(shape);
    }
    for n in &app.notes {
        add(&n.library);
    }
    for r in &app.rules {
        for t in &lir.rule(*r).triggers {
            add(&t.library);
        }
    }
    let bodies = lir.fns.iter().map(|f| &f.body).chain(lir.rules.iter().map(|r| &r.body));
    for body in bodies {
        if let Body::Extern(x) = body {
            add(&x.library);
        }
    }
    names
}

fn read_runtime(g: &Gen<'_>, library: &str) -> R<Runtime> {
    let root = g.root_dir();
    let dir = LibPath::for_project(Path::new(&root))
        .find(library)
        .map(|d| d.join("real").join(LANG))
        .filter(|d| d.join(CONF).is_file())
        .ok_or_else(|| {
            problem(format!("library `{library}` has no runtime for {LANG} (`libs/{library}/real/{LANG}/{CONF}`)"))
        })?;
    let mut rt = Runtime { library: library.to_string(), ..Runtime::default() };
    let conf = std::fs::read_to_string(dir.join(CONF))
        .map_err(|e| Diag::load(format!("{}: {e}", dir.join(CONF).display())))?;
    for line in conf.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (key, value) = line.split_once('=').map(|(k, v)| (k.trim(), v.trim())).unwrap_or((line, ""));
        match key {
            "entry" => rt.entry = value.to_string(),
            "module" => rt.module = value.to_string(),
            "executable" => rt.executable = value.split_once('.').map(|(n, p)| (n.to_string(), p.to_string())),
            "gem" => {
                let (name, version) = value.split_once(' ').unwrap_or((value, ""));
                rt.gems.push((name.to_string(), version.trim().to_string()));
            }
            other => return Err(problem(format!("`{other}` in libs/{library}/real/{LANG}/{CONF}"))),
        }
    }
    if rt.entry.is_empty() || rt.module.is_empty() {
        return Err(problem(format!("libs/{library}/real/{LANG}/{CONF} names no `entry` and `module`")));
    }
    let mut paths = Vec::new();
    walk(&dir, &dir, &mut paths)?;
    paths.sort();
    for (rel, full) in paths {
        if rel == CONF || rel.ends_with("README.md") {
            continue;
        }
        let bytes = std::fs::read(&full).map_err(|e| Diag::load(format!("{}: {e}", full.display())))?;
        rt.files.push(OutFile { path: rel, bytes, executable: false });
    }
    Ok(rt)
}

/// Every file under `dir`, with its path relative to `base`.
fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> R<()> {
    let fail = |e: std::io::Error| Diag::load(format!("{}: {e}", dir.display()));
    for entry in std::fs::read_dir(dir).map_err(fail)? {
        let path = entry.map_err(fail)?.path();
        if path.is_dir() {
            walk(base, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.push((rel.to_string_lossy().replace('\\', "/"), path.clone()));
        }
    }
    Ok(())
}

/// The text a note of the app gives one of its parameters.
fn note_text(app: &Manifest, note: &str, param: &str) -> Option<String> {
    let n = app.notes.iter().find(|n| n.name == note)?;
    match &n.args.iter().find(|a| a.param == param)?.value {
        NoteValue::Text(t) => Some(t.clone()),
        _ => None,
    }
}

pub(crate) fn project(g: &Gen<'_>, name: &str) -> R<Vec<OutFile>> {
    let app = g.lir.apps.iter().find(|a| a.name == name).ok_or_else(|| problem(format!("no app named {name}")))?;
    let mut runtimes = Vec::new();
    for library in libraries(g, app) {
        runtimes.push(read_runtime(g, &library)?);
    }
    // The library that runs the app is the first that names an executable, and the shape's when there is one.
    let Some(runner) = runtimes.iter().position(|r| r.executable.is_some()) else {
        return Err(problem(format!("app {name} uses no library whose {LANG} runtime runs an app")));
    };
    runtimes.swap(0, runner);
    let bin =
        runtimes[0].executable.as_ref().and_then(|(note, param)| note_text(app, note, param)).ok_or_else(|| {
            problem(format!("app {name} does not name its executable (library `{}`)", runtimes[0].library))
        })?;
    if bin.is_empty() || !bin.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) {
        return Err(problem(format!("an executable named {bin:?}")));
    }
    let module = g.lir.modules.iter().find(|m| m.name == app.module);
    let origin = format!("app {} ({})", app.name, module.map_or(String::new(), |m| g.source(&m.path)));
    let generated =
        format!("# Generated by spec-lang build from {origin}. Do not edit: change the spec and build again.");

    let mut files = project::base(g)?;
    let mut taken: BTreeSet<String> = files.iter().map(|f| f.path.clone()).collect();
    for rt in &mut runtimes {
        for f in std::mem::take(&mut rt.files) {
            if !taken.insert(f.path.clone()) {
                return Err(problem(format!("two files at `{}` (library `{}`)", f.path, rt.library)));
            }
            files.push(f);
        }
    }
    let gems: Vec<&(String, String)> = runtimes.iter().flat_map(|r| &r.gems).collect();
    files.push(text("lib/app.rb", glue(app, &runtimes, &generated)));
    files.push(script(format!("exe/{bin}"), exe(&generated)));
    files.push(text("Gemfile", gemfile(&gems, &generated)));
    files.push(text(".gitignore", "/.bundle/\n/vendor/\n"));
    files.push(script("bin/build", build(&bin, !gems.is_empty(), &generated)));
    files.push(script("bin/check", check(!gems.is_empty(), &generated)));
    Ok(files)
}

fn glue(app: &Manifest, runtimes: &[Runtime], generated: &str) -> String {
    let requires: String = runtimes.iter().map(|r| format!("require_relative {}\n", code::string(&r.entry))).collect();
    let modules: Vec<String> = runtimes.iter().map(|r| r.module.clone()).collect();
    format!(
        "{HEADER}\n{generated}\n\nrequire_relative \"program\"\n{requires}\n# The app: its manifest, and the runtimes of the libraries it uses. The first runs it; the others serve it\n# (a handler's real operations, the calls of `foreign` declarations).\nmodule App\n  MANIFEST = PROGRAM.apps.fetch({})\n  LIBRARIES = [{}].freeze\n\n  # Runs one command line; gives the exit code.\n  def self.run(argv, stdin: $stdin, out: $stdout, err: $stderr, env: ENV)\n    LIBRARIES.first.run(PROGRAM, MANIFEST, argv, stdin:, out:, err:, env:, libraries: LIBRARIES.drop(1))\n  end\nend\n",
        code::string(&app.name),
        modules.join(", ")
    )
}

fn exe(generated: &str) -> String {
    format!(
        "#!/usr/bin/env -S ruby --disable-gems\n{HEADER}\n{generated}\n\n# RubyGems stays off: the program needs the standard library, and a library's runtime finds its own gems.\nrequire_relative \"../lib/app\"\n\n# Die quietly, as other tools do, when the reader of our output has gone away.\nSignal.trap(\"PIPE\", \"SYSTEM_DEFAULT\")\n$stdout.binmode\n$stderr.binmode\n$stdin.binmode\nexit App.run(ARGV)\n"
    )
}

fn gemfile(gems: &[&(String, String)], generated: &str) -> String {
    let mut out = format!("{HEADER}\n{generated}\n\nsource \"https://rubygems.org\"\n\nruby \">= 4.0\"\n");
    if !gems.is_empty() {
        out.push_str("\n# Run time: what the libraries' runtimes need.\n");
        for (name, version) in gems {
            out.push_str(&format!("gem {}, {}\n", code::string(name), code::string(version)));
        }
    }
    out.push_str("\n# Tests only.\ngroup :development do\n  gem \"minitest\", \"6.0.0\"\nend\n");
    out
}

const PARSES: &str = "while IFS= read -r file; do
  if ! report=$(ruby -wc \"$file\" 2>&1) || [ \"$report\" != \"Syntax OK\" ]; then
    printf '%s: %s\\n' \"$file\" \"$report\" >&2
    exit 1
  fi
done";

fn build(bin: &str, gems: bool, generated: &str) -> String {
    let install = if gems {
        "# The runtime's gems go into vendor/bundle with a load path that needs no RubyGems.\nbundle config set --local path vendor/bundle >/dev/null\nbundle install --quiet --standalone default\n"
    } else {
        ""
    };
    format!(
        "#!/usr/bin/env bash\n{generated}\n# Ruby has no compile step: the build makes the executable runnable and proves every product file parses without a warning.\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\n\n{install}chmod +x exe/{bin} bin/test\n{PARSES} < <(find exe lib -type f -name '*.rb' -o -type f -path 'exe/*' | sort)\necho \"built exe/{bin}\"\n"
    )
}

fn check(gems: bool, generated: &str) -> String {
    let path = if gems { "bundle config set --local path vendor/bundle >/dev/null\n" } else { "" };
    format!(
        "#!/usr/bin/env bash\n{generated}\n# The whole gate: `ruby -w` on every file, build, the program's own checks, the runtimes' tests. Any warning fails it.\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\n\n{path}bundle check >/dev/null || bundle install --quiet\n\n{PARSES} < <(find Gemfile bin/test exe lib test -type f | sort)\n\nbin/build\n\nlog=$(mktemp)\ntrap 'rm -f \"$log\"' EXIT\n\n# Every check of the program, as `spec-lang test` names and judges it: none may fail.\nruby -w bin/test 2>&1 | tee \"$log\" | grep -v '\"status\": \"pass\"' || true\nif grep -q -e '\"status\": \"fail\"' -e 'warning:' \"$log\" || ! grep -q '\"status\"' \"$log\"; then\n  echo \"the program's checks did not all pass\" >&2\n  exit 1\nfi\necho \"$(grep -c '\"status\": \"pass\"' \"$log\") checks pass, $(grep -c '\"status\": \"blocked\"' \"$log\" || true) blocked\"\n\n# The runtimes' own tests. Those that draw need a window: a virtual X server when one can be started.\nxvfb=()\nif [ -z \"${{DISPLAY:-}}\" ] && command -v xvfb-run >/dev/null; then xvfb=(xvfb-run -a); fi\n\"${{xvfb[@]}}\" bundle exec ruby -w -Ilib -Itest -e 'Dir[\"test/**/*_test.rb\"].sort.each {{ |file| require File.expand_path(file) }}' 2>&1 | tee \"$log\"\nif grep -q 'warning:' \"$log\"; then\n  echo \"the tests ran with warnings\" >&2\n  exit 1\nfi\n"
    )
}
