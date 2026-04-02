pub mod error;
pub mod parser;
#[cfg(test)]
pub mod test_parser;

pub use parser::{compute_document_hash, normalize_source, parse_document, parse_document_bytes};
