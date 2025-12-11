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

            // Primitive type compatibility
            (
                FacetType::Primitive(PrimitiveType::Int),
                FacetType::Primitive(PrimitiveType::Float),
            ) => true,

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

            _ => false,
        }
    }
}