//! Links the C libraries the app's runtimes call, each as its runtime says.

#[path = "build/raylib.rs"]
mod raylib;

fn main() {
    raylib::link();
}
