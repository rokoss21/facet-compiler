// ============================================================================
// LENSES MODULE - Categorical Organization
// ============================================================================

pub mod level1;
pub mod list;
pub mod map;
pub mod string;
pub mod utility;

// Re-export all lens types for convenient access
#[allow(unused_imports)]
pub use list::{EnsureListLens, FilterLens, MapLens, SortByLens};
#[allow(unused_imports)]
pub use map::{KeysLens, ValuesLens};
#[allow(unused_imports)]
pub use string::{IndentLens, LowercaseLens, ReplaceLens, SplitLens, TrimLens, UppercaseLens};
#[allow(unused_imports)]
pub use utility::{DefaultLens, JsonLens};
