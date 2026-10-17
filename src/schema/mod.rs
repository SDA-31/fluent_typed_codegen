mod validation;

pub use validation::{Schema, schema, validate};

#[cfg(all(test, feature = "build"))]
#[path = "tests/schema.rs"]
mod tests;
