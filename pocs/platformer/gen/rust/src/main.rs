//! `skyhop`: the app `apps.rust.SkyhopRust`, run by the `game` library's runtime.

fn main() -> std::process::ExitCode {
    program::libs::game::main(&program::manifest::APP)
}
