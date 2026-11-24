//! The fixed part of a generated Bun project: runtime shims, test support,
//! configuration and scripts.

use crate::sem::app::App;
use crate::sem::prov::{Entry, Tracked, slg};

/// (path in the project, text, what it is).
const SHIMS: &[(&str, &str, &str)] = &[
    ("src/runtime/text.ts", include_str!("../../runtime/src/runtime/text.ts"), "Text: size, trim, bytes"),
    (
        "src/runtime/values.ts",
        include_str!("../../runtime/src/runtime/values.ts"),
        "claims, order, equality, Set and Map",
    ),
    ("src/runtime/outcome.ts", include_str!("../../runtime/src/runtime/outcome.ts"), "the outcome of a rule"),
    (
        "src/runtime/store/json-reader.ts",
        include_str!("../../runtime/src/runtime/store/json-reader.ts"),
        "strict JSON reader",
    ),
    ("src/runtime/store/codec.ts", include_str!("../../runtime/src/runtime/store/codec.ts"), "store codec"),
    (
        "src/runtime/store/session.ts",
        include_str!("../../runtime/src/runtime/store/session.ts"),
        "load, lock, atomic replace",
    ),
    ("src/runtime/store/lock.ts", include_str!("../../runtime/src/runtime/store/lock.ts"), "flock with a bounded wait"),
    ("src/runtime/store/libc.ts", include_str!("../../runtime/src/runtime/store/libc.ts"), "flock(2) through bun:ffi"),
    (
        "src/runtime/store/atomic-write.ts",
        include_str!("../../runtime/src/runtime/store/atomic-write.ts"),
        "atomic file replace: the tmp file is removed and signals are deferred on every way out",
    ),
    (
        "src/runtime/store/store-error.ts",
        include_str!("../../runtime/src/runtime/store/store-error.ts"),
        "store errors",
    ),
    (
        "src/runtime/store/system-error.ts",
        include_str!("../../runtime/src/runtime/store/system-error.ts"),
        "operating-system errors",
    ),
    (
        "src/runtime/cli/app.ts",
        include_str!("../../runtime/src/runtime/cli/app.ts"),
        "the app record and its store path",
    ),
    ("src/runtime/cli/arg-types.ts", include_str!("../../runtime/src/runtime/cli/arg-types.ts"), "argument types"),
    (
        "src/runtime/cli/command.ts",
        include_str!("../../runtime/src/runtime/cli/command.ts"),
        "command table rows and help",
    ),
    ("src/runtime/cli/params.ts", include_str!("../../runtime/src/runtime/cli/params.ts"), "parameter shapes"),
    ("src/runtime/cli/parse.ts", include_str!("../../runtime/src/runtime/cli/parse.ts"), "argument parser"),
    (
        "src/runtime/cli/run.ts",
        include_str!("../../runtime/src/runtime/cli/run.ts"),
        "dispatch, exit codes, process entry",
    ),
    ("src/runtime/cli/output.ts", include_str!("../../runtime/src/runtime/cli/output.ts"), "output and error lines"),
    ("src/runtime/cli/exit-codes.ts", include_str!("../../runtime/src/runtime/cli/exit-codes.ts"), "exit codes"),
    ("test/support/board.ts", include_str!("../../runtime/test/support/board.ts"), "test board"),
    ("test/support/trace.ts", include_str!("../../runtime/test/support/trace.ts"), "traces"),
    ("test/support/arb.ts", include_str!("../../runtime/test/support/arb.ts"), "generators of base types"),
    (
        "test/runtime/failure-paths.test.ts",
        include_str!("../../runtime/test/runtime/failure-paths.test.ts"),
        "failure paths: a failed or interrupted write leaves the old store or the new and no tmp file; locks are released",
    ),
];

const TSCONFIG: &str = include_str!("../../runtime/tsconfig.json");
const BIOME: &str = include_str!("../../runtime/biome.json");
const LOCK: &str = include_str!("../../runtime/bun.lock");
const GITIGNORE: &str = include_str!("../../runtime/gitignore");

fn shim(what: &str) -> Entry {
    Entry {
        decl: String::new(),
        slg: String::new(),
        role: format!("runtime shim: {what}"),
        strategy: "runtime.verbatim: the same file in every project of this backend".into(),
        fact: "the spec does not determine it per program".into(),
    }
}

pub fn files(app: &App<'_>) -> Vec<Tracked> {
    let info = &app.decl.info;
    let from = format!("app {} ({})", info.name, slg(app.p, info, info.name_span));
    let config = |role: &str, strategy: &str, fact: &str| Entry {
        decl: app.p.qualified(info),
        slg: slg(app.p, info, info.span),
        role: role.into(),
        strategy: strategy.into(),
        fact: fact.into(),
    };
    let mut out = Vec::new();
    for (path, text, what) in SHIMS {
        let header = format!(
            "// Runtime shim of the bun backend of spec-lang build: {what}. The same in every generated project; change the backend, not this file.\n\n"
        );
        out.push(Tracked {
            path: path.to_string(),
            text: format!("{header}{text}"),
            entries: vec![shim(what)],
            executable: false,
        });
    }
    let name = format!("{}-bun", app.bin);
    let package = format!(
        "{{\n  \"name\": \"{name}\",\n  \"private\": true,\n  \"type\": \"module\",\n  \"trustedDependencies\": [],\n  \"devDependencies\": {{\n    \"@biomejs/biome\": \"2.3.8\",\n    \"@types/bun\": \"1.3.3\",\n    \"fast-check\": \"4.10.2\",\n    \"typescript\": \"5.9.3\"\n  }}\n}}\n"
    );
    let plain = [
        (
            "package.json",
            package,
            "project config: package",
            "deps.pinned: no runtime dependency; the toolchain and fast-check are pinned for the gate",
        ),
        ("tsconfig.json", TSCONFIG.to_string(), "project config: TypeScript", "tsc.strict"),
        ("biome.json", BIOME.to_string(), "project config: formatter and linter", "biome.recommended"),
        ("bun.lock", LOCK.replace("todo-bun", &name), "project config: lockfile", "deps.pinned"),
        (".gitignore", GITIGNORE.to_string(), "project config: ignored files", "ignore.build-output"),
    ];
    for (path, text, role, strategy) in plain {
        out.push(Tracked {
            path: path.into(),
            text,
            entries: vec![config(role, strategy, "`@target(bun)`")],
            executable: false,
        });
    }
    let script = |what: &str, body: &str| {
        format!(
            "#!/usr/bin/env bash\n# Generated by spec-lang build from {from}. Do not edit: change the spec and build again.\n# {what}\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\n{body}"
        )
    };
    let build = format!(
        "bun build --compile --minify --bytecode --no-compile-autoload-dotenv --no-compile-autoload-bunfig src/main.ts --outfile dist/{}\n",
        app.bin
    );
    let scripts = [
        (
            "bin/build",
            script("Builds the standalone executable.", &build),
            "build script",
            "startup.bytecode-no-autoload: bytecode is compiled ahead, and no .env or bunfig.toml is looked for at start",
            "the program is fixed at build time; `@store` and its environment variable are its only inputs besides argv",
        ),
        (
            "bin/check",
            script(
                "The whole gate: lint and format, types, tests, build.",
                "bun install --frozen-lockfile\nbunx biome check .\nbunx tsc --noEmit -p .\nbun test\nbin/build\n",
            ),
            "gate script",
            "gate.bun: biome check, tsc --noEmit, bun test, build",
            "`@target(bun)`",
        ),
        (
            "bin/normalize",
            script(
                "Lets the project's own formatter have the last word on layout; run once after a build.",
                "bun install --frozen-lockfile\nbunx biome check --write .\n",
            ),
            "normalize script",
            "format.biome: the printer emits a tree, the project's formatter lays it out",
            "`@target(bun)`",
        ),
    ];
    for (path, text, role, strategy, fact) in scripts {
        out.push(Tracked { path: path.into(), text, entries: vec![config(role, strategy, fact)], executable: true });
    }
    out
}
