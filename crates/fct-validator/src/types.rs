//! Unified FACET Type System surface for validator.
//!
//! The validator now uses the canonical FTS definitions from `fct-ast` directly.

pub use fct_ast::types::{
    AudioType, EmbeddingType, FacetType, ImageType, MultimodalType, PrimitiveType, StructField,
};

#[cfg(test)]
mod tests {
    use super::{FacetType, PrimitiveType};
    use std::any::TypeId;

    #[test]
    fn validator_types_alias_ast_types() {
        assert_eq!(
            TypeId::of::<FacetType>(),
            TypeId::of::<fct_ast::types::FacetType>()
        );
        assert_eq!(
            TypeId::of::<PrimitiveType>(),
            TypeId::of::<fct_ast::types::PrimitiveType>()
        );
    }

    #[test]
    fn assignability_available_on_reexported_type() {
        let actual = FacetType::Primitive(PrimitiveType::Int);
        let expected = FacetType::Primitive(PrimitiveType::Number);
        assert!(actual.is_assignable_to(&expected));
    }
}
