//! # FACET Type System (FTS)
//!
//! This module contains the core type definitions for the FACET language compiler.
//! It defines primitive types, composite types, multimodal types, and the complete
//! type system hierarchy.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Primitive types in the FACET Type System (FTS).
///
/// These are the fundamental building blocks of the FACET type system.
/// All complex types are built from combinations of these primitive types.
///
/// # Examples
///
/// ```rust
/// use fct_validator::{PrimitiveType, FacetType};
///
/// let string_type = FacetType::Primitive(PrimitiveType::String);
/// let int_type = FacetType::Primitive(PrimitiveType::Int);
///
/// // Type checking example
/// fn is_numeric_type(t: &FacetType) -> bool {
///     matches!(t, FacetType::Primitive(PrimitiveType::Int | PrimitiveType::Float))
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveType {
    /// String type for text values. Supports Unicode and can include
    /// constraints like regex patterns and length limits.
    String,

    /// Integer type for whole numbers. Supports 64-bit signed integers
    /// with optional range constraints.
    Int,

    /// Float type for decimal numbers. IEEE 754 64-bit floating point
    /// with optional range and precision constraints.
    Float,

    /// Boolean type for true/false values. No additional constraints
    /// are supported for boolean types.
    Bool,

    /// Null type representing the absence of a value. This is a distinct
    /// type from optional types - use Union types for optionals.
    Null,

    /// Any type (dynamic typing). Discouraged in production code as it
    /// bypasses compile-time type checking, but allowed for generic lens
    /// operations and legacy code compatibility.
    #[serde(rename = "any")]
    Any,
}

/// Multimodal types for rich media content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageType {
    pub max_dim: Option<u32>,
    pub format: Option<String>, // "png" | "jpeg" | "webp"
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioType {
    pub max_duration: Option<f64>,
    pub format: Option<String>, // "mp3" | "wav" | "ogg"
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingType {
    pub size: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MultimodalType {
    Image(ImageType),
    Audio(AudioType),
    Embedding(EmbeddingType),
}

/// Composite types for structured data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructType {
    pub fields: HashMap<String, FacetType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListType {
    pub element_type: Box<FacetType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapType {
    pub value_type: Box<FacetType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnionType {
    pub types: Vec<FacetType>,
}

/// The core type enum of the FACET Type System (FTS).
///
/// This enum represents all possible types in the FACET language, from simple
/// primitive types to complex composite and multimodal types. Each variant
/// represents a different category of types with specific semantics and use cases.
///
/// # Type Categories
///
/// - **Primitive**: Basic types like String, Int, Float, Bool, Null
/// - **Multimodal**: Rich media types like Image, Audio, Embedding
/// - **Struct**: Named collections with typed fields (objects/records)
/// - **List**: Homogeneous collections of elements (arrays/vectors)
/// - **Map**: Key-value collections with string keys and typed values
/// - **Union**: Sum types that can be one of several types (optionals, variants)
///
/// # Examples
///
/// ```rust
/// use fct_validator::{FacetType, PrimitiveType, StructType, ListType};
/// use std::collections::HashMap;
///
/// // Create a primitive string type
/// let string_type = FacetType::Primitive(PrimitiveType::String);
///
/// // Create a struct type representing a user
/// let mut fields = HashMap::new();
/// fields.insert("name".to_string(), FacetType::Primitive(PrimitiveType::String));
/// fields.insert("age".to_string(), FacetType::Primitive(PrimitiveType::Int));
/// let user_type = FacetType::Struct(StructType { fields });
///
/// // Create a list of users
/// let user_list_type = FacetType::List(ListType {
///     element_type: Box::new(user_type)
/// });
/// ```
///
/// # Type Compatibility
///
/// The `is_assignable_to()` method determines type compatibility according
/// to FACET's type system rules, including subtyping, union compatibility,
/// and structural typing rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FacetType {
    /// Primitive types (String, Int, Float, Bool, Null, Any)
    ///
    /// These are the fundamental building blocks of the type system.
    /// Primitive types have direct value representation and can be
    /// used in constraints and type checking.
    Primitive(PrimitiveType),

    /// Multimodal types for rich media content
    ///
    /// These types represent AI/ML multimodal content including images,
    /// audio, and vector embeddings. They often have size and format
    /// constraints for efficient processing and validation.
    Multimodal(MultimodalType),

    /// Struct type for named field collections
    ///
    /// Represents objects or records with named fields, each with its
    /// own type. Structs provide structural typing - two structs are
    /// compatible if they have compatible fields, regardless of name.
    Struct(StructType),

    /// List type for ordered collections
    ///
    /// Represents arrays or vectors containing elements of a single
    /// type. Lists are homogeneous - all elements must be of the
    /// specified element type.
    List(ListType),

    /// Map type for key-value collections
    ///
    /// Represents dictionaries or hash maps with string keys and
    /// values of a specific type. Maps are useful for dynamic data
    /// and configuration objects.
    Map(MapType),

    /// Union type for sum types and optionals
    ///
    /// Represents values that can be one of several types. This is
    /// used for optional values (Union of type + Null), variant
    /// types, and any situation where multiple types are acceptable.
    Union(UnionType),
}

impl FacetType {
    /// Check if this type is assignable to another type
    pub fn is_assignable_to(&self, other: &FacetType) -> bool {
        match (self, other) {
            // Same type
            (a, b) if a == b => true,

            // Any accepts everything
            (_, FacetType::Primitive(PrimitiveType::Any)) => true,

            // Primitive types must match exactly (except Any above)
            (FacetType::Primitive(a), FacetType::Primitive(b)) => a == b,

            // Multimodal assignability with constraint satisfaction
            (
                FacetType::Multimodal(MultimodalType::Image(actual)),
                FacetType::Multimodal(MultimodalType::Image(expected)),
            ) => {
                let format_ok = match (&actual.format, &expected.format) {
                    (_, None) => true,
                    (Some(a), Some(e)) => a == e,
                    (None, Some(_)) => false,
                };
                let dim_ok = match (actual.max_dim, expected.max_dim) {
                    (_, None) => true,
                    (Some(a), Some(e)) => a <= e,
                    (None, Some(_)) => false,
                };
                format_ok && dim_ok
            }
            (
                FacetType::Multimodal(MultimodalType::Audio(actual)),
                FacetType::Multimodal(MultimodalType::Audio(expected)),
            ) => {
                let format_ok = match (&actual.format, &expected.format) {
                    (_, None) => true,
                    (Some(a), Some(e)) => a == e,
                    (None, Some(_)) => false,
                };
                let duration_ok = match (actual.max_duration, expected.max_duration) {
                    (_, None) => true,
                    (Some(a), Some(e)) => a <= e,
                    (None, Some(_)) => false,
                };
                format_ok && duration_ok
            }
            (
                FacetType::Multimodal(MultimodalType::Embedding(actual)),
                FacetType::Multimodal(MultimodalType::Embedding(expected)),
            ) => actual.size == expected.size,

            // Struct assignability: all required fields of expected must exist in actual.
            (FacetType::Struct(actual), FacetType::Struct(expected)) => {
                expected.fields.iter().all(|(field, expected_ty)| {
                    actual
                        .fields
                        .get(field)
                        .map(|actual_ty| actual_ty.is_assignable_to(expected_ty))
                        .unwrap_or(false)
                })
            }

            // List element type compatibility
            (FacetType::List(l1), FacetType::List(l2)) => {
                l1.element_type.is_assignable_to(&l2.element_type)
            }

            // Map value type compatibility
            (FacetType::Map(m1), FacetType::Map(m2)) => {
                m1.value_type.is_assignable_to(&m2.value_type)
            }

            // Union types
            (t, FacetType::Union(union)) => union.types.iter().any(|ut| t.is_assignable_to(ut)),
            (FacetType::Union(union), t) => union.types.iter().all(|ut| ut.is_assignable_to(t)),

            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioType, EmbeddingType, FacetType, ImageType, MultimodalType, PrimitiveType, StructType,
        UnionType,
    };
    use std::collections::HashMap;

    #[test]
    fn primitive_assignability_is_exact_except_any_target() {
        let int_ty = FacetType::Primitive(PrimitiveType::Int);
        let float_ty = FacetType::Primitive(PrimitiveType::Float);
        let any_ty = FacetType::Primitive(PrimitiveType::Any);

        assert!(!int_ty.is_assignable_to(&float_ty));
        assert!(int_ty.is_assignable_to(&any_ty));
    }

    #[test]
    fn struct_assignability_requires_expected_fields_only() {
        let mut actual_fields = HashMap::new();
        actual_fields.insert(
            "name".to_string(),
            FacetType::Primitive(PrimitiveType::String),
        );
        actual_fields.insert("age".to_string(), FacetType::Primitive(PrimitiveType::Int));
        actual_fields.insert(
            "city".to_string(),
            FacetType::Primitive(PrimitiveType::String),
        );

        let mut expected_fields = HashMap::new();
        expected_fields.insert(
            "name".to_string(),
            FacetType::Primitive(PrimitiveType::String),
        );
        expected_fields.insert("age".to_string(), FacetType::Primitive(PrimitiveType::Int));

        let actual = FacetType::Struct(StructType {
            fields: actual_fields,
        });
        let expected = FacetType::Struct(StructType {
            fields: expected_fields,
        });

        assert!(actual.is_assignable_to(&expected));
    }

    #[test]
    fn multimodal_constraints_are_checked_during_assignability() {
        let actual = FacetType::Multimodal(MultimodalType::Image(ImageType {
            max_dim: Some(512),
            format: Some("jpeg".to_string()),
        }));
        let expected = FacetType::Multimodal(MultimodalType::Image(ImageType {
            max_dim: Some(1024),
            format: Some("jpeg".to_string()),
        }));
        let wrong_format = FacetType::Multimodal(MultimodalType::Image(ImageType {
            max_dim: Some(1024),
            format: Some("png".to_string()),
        }));

        assert!(actual.is_assignable_to(&expected));
        assert!(!actual.is_assignable_to(&wrong_format));
    }

    #[test]
    fn union_assignability_works_for_left_and_right_unions() {
        let string_or_null = FacetType::Union(UnionType {
            types: vec![
                FacetType::Primitive(PrimitiveType::String),
                FacetType::Primitive(PrimitiveType::Null),
            ],
        });
        let any_ty = FacetType::Primitive(PrimitiveType::Any);
        let string_ty = FacetType::Primitive(PrimitiveType::String);

        assert!(string_ty.is_assignable_to(&string_or_null));
        assert!(string_or_null.is_assignable_to(&any_ty));
        assert!(!string_or_null.is_assignable_to(&string_ty));
    }

    #[test]
    fn embedding_and_audio_assignability_require_matching_constraints() {
        let emb_3 = FacetType::Multimodal(MultimodalType::Embedding(EmbeddingType { size: 3 }));
        let emb_4 = FacetType::Multimodal(MultimodalType::Embedding(EmbeddingType { size: 4 }));
        assert!(!emb_3.is_assignable_to(&emb_4));

        let short_audio = FacetType::Multimodal(MultimodalType::Audio(AudioType {
            max_duration: Some(3.0),
            format: Some("wav".to_string()),
        }));
        let long_audio = FacetType::Multimodal(MultimodalType::Audio(AudioType {
            max_duration: Some(10.0),
            format: Some("wav".to_string()),
        }));
        assert!(short_audio.is_assignable_to(&long_audio));
    }
}
