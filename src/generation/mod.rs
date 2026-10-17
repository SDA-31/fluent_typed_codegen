mod pipeline;

pub use pipeline::{build, build_with, from_cargo, from_cargo_with, generate, generate_with};

#[cfg(test)]
#[path = "tests/generation.rs"]
mod tests;
