pub mod error;
pub mod parser;
#[cfg(test)]
pub mod test_parser;

pub use parser::parse_document;