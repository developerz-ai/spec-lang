// Projected by hand from modules hop.geom, hop.model, hop.levels, hop.physics, hop.world, hop.render, hop.screen (src/hop/*.slg) and app SkyhopRust (src/apps/rust.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! Skyhop. The simulation (`geom model levels physics world render`) touches no I/O, no raylib, no clock and no environment.

pub mod app;
pub mod cli;
pub mod geom;
pub mod levels;
pub mod model;
pub mod physics;
pub mod render;
pub mod screen;
pub mod script;
pub mod world;
