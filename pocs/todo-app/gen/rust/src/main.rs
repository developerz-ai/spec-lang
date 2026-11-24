//! `todo`: the app `apps.rust.TodoRust`, run by the `cli` library's runtime.

fn main() -> std::process::ExitCode {
    program::libs::cli::main(&program::manifest::APP)
}
